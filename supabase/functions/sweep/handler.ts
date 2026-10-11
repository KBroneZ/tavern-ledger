// Daily sweep (T-104d). POST /functions/v1/sweep with the service key the
// function itself uses (the new secret key, D-042; never a legacy key, D-053),
// or with the sweep token that the scheduled job reads from Vault
// (X-Sweep-Token; checked by the database, migration 20261010090000):
//   1. game files with no summary row (an upload that wrote its file and then
//      failed before the row), untouched for an hour, through the Storage
//      API (deleting storage.objects rows in SQL would leave the bytes);
//   2. public.sweep_housekeeping(): audit entries of deleted accounts (the
//      T-104a leftover) and every audit entry older than 90 days (D-048),
//      upload times older than two days and IP counters older than an hour;
//   3. game summaries for the website (T-502, D-071): games stored before
//      the upload function wrote them, or summarized by an older version, get
//      one from their stored report, read through the Storage API, checked
//      again by validateRecord() and against the row's SHA-256. At most 500
//      per run and 100 seconds; the rest wait for the next run.
// Each run ends by recording in private.sweep_runs (T-524, D-096) the files
// deleted and the backfill's counts, skips by reason, next to the
// housekeeping counts; a step that fails is recorded too, so a run that
// reported nothing is never read as one that did nothing. The response
// carries the same counts.
// Safe to run any time and as often as wanted. No dependencies.

import { summarize, SUMMARY_VERSION } from "../_shared/game_summary.ts";
import { validateRecord } from "../upload-game/validate.ts";

export interface Env {
  url: string;
  serviceKey: string;
  /** Clock in ms, for tests; Date.now by default. */
  now?: () => number;
}

export type Fetch = (input: string, init?: RequestInit) => Promise<Response>;

const TIMEOUT_MS = 10_000;
// sweep_orphan_files() returns at most 1000 names per call.
const MAX_ROUNDS = 20;
const GAME_FILE =
  /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}\/Hearthstone_\d{4}(_\d{2}){5}-[1-9][0-9]{0,3}\.json\.gz$/;
// Summaries filled per run; a stored game is about 2.5 KB gzipped.
const BACKFILL_LIMIT = 500;
// No summary is started after this long, so a slow Storage cannot run the
// function past its wall-clock limit; the rest wait for the next run.
const BACKFILL_BUDGET_MS = 100_000;
// The upload function stores at most 64 KiB, inflating to at most 512 KiB.
const MAX_FILE = 64 * 1024;
const MAX_INFLATED = 512 * 1024;
const SHA256 = /^[0-9a-f]{64}$/;

class StepError extends Error {
  constructor(readonly step: string, readonly status: number) {
    super(`${step} failed with HTTP ${status}`);
  }
}

function json(status: number, body: unknown): Response {
  return new Response(JSON.stringify(body), {
    status,
    headers: { "Content-Type": "application/json" },
  });
}

function serviceHeaders(env: Env): Record<string, string> {
  const headers: Record<string, string> = {
    apikey: env.serviceKey,
    "Content-Type": "application/json",
  };
  return headers;
}

/** Compares two strings in time that does not depend on where they differ. */
function sameSecret(a: string, b: string): boolean {
  const x = new TextEncoder().encode(a);
  const y = new TextEncoder().encode(b);
  let diff = x.length ^ y.length;
  for (let i = 0; i < Math.max(x.length, y.length); i++) diff |= (x[i] ?? 0) ^ (y[i] ?? 0);
  return diff === 0;
}

function isServiceRole(req: Request, env: Env): boolean {
  const bearer = req.headers.get("Authorization") ?? "";
  const token = bearer.startsWith("Bearer ") ? bearer.slice(7) : "";
  const apikey = req.headers.get("apikey") ?? "";
  return env.serviceKey !== "" &&
    (sameSecret(token, env.serviceKey) || sameSecret(apikey, env.serviceKey));
}

// The token the migration puts in Vault: 32 random bytes as hex.
const SWEEP_TOKEN = /^[0-9a-f]{64}$/;

/** True if the request carries the scheduled job's sweep token. */
async function hasSweepToken(req: Request, env: Env, fetchFn: Fetch): Promise<boolean> {
  const token = req.headers.get("X-Sweep-Token") ?? "";
  if (!SWEEP_TOKEN.test(token)) return false;
  const res = await call(fetchFn, "check token", `${env.url}/rest/v1/rpc/sweep_token_valid`, {
    method: "POST",
    headers: serviceHeaders(env),
    body: JSON.stringify({ p_token: token }),
  });
  return (await res.json()) === true;
}

async function call(fetchFn: Fetch, step: string, url: string, init: RequestInit) {
  let res: Response;
  try {
    res = await fetchFn(url, { ...init, signal: AbortSignal.timeout(TIMEOUT_MS) });
  } catch (_e) {
    throw new StepError(step, 0);
  }
  if (!res.ok) {
    await res.body?.cancel();
    throw new StepError(step, res.status);
  }
  return res;
}

async function orphanFiles(env: Env, fetchFn: Fetch): Promise<string[]> {
  const res = await call(fetchFn, "list files", `${env.url}/rest/v1/rpc/sweep_orphan_files`, {
    method: "POST",
    headers: serviceHeaders(env),
    body: JSON.stringify({ min_age: "1 hour" }),
  });
  const names = await res.json();
  if (!Array.isArray(names) || !names.every((n) => typeof n === "string")) {
    throw new StepError("list files", 502);
  }
  // Never touch anything that is not shaped like a game file.
  return names.filter((n) => GAME_FILE.test(n));
}

async function deleteOrphans(env: Env, fetchFn: Fetch): Promise<number> {
  let deleted = 0;
  let previous = "";
  for (let round = 0; round < MAX_ROUNDS; round++) {
    const names = await orphanFiles(env, fetchFn);
    if (names.length === 0) return deleted;
    // The same list twice: the delete did not take; stop instead of looping.
    if (names.join("\n") === previous) throw new StepError("delete files", 508);
    previous = names.join("\n");
    const res = await call(fetchFn, "delete files", `${env.url}/storage/v1/object/games`, {
      method: "DELETE",
      headers: serviceHeaders(env),
      body: JSON.stringify({ prefixes: names }),
    });
    await res.body?.cancel();
    deleted += names.length;
  }
  throw new StepError("delete files", 508);
}

interface Missing {
  user_id: string;
  session: string;
  game_index: number;
  storage_path: string;
  content_sha256: string;
}

/** A row of game_stats_backfill_list() whose file name is the one its key gives. */
function isMissing(v: unknown): v is Missing {
  if (typeof v !== "object" || v === null) return false;
  const m = v as Record<string, unknown>;
  return typeof m.user_id === "string" && typeof m.session === "string" &&
    Number.isInteger(m.game_index) && typeof m.storage_path === "string" &&
    typeof m.content_sha256 === "string" && SHA256.test(m.content_sha256) &&
    GAME_FILE.test(m.storage_path) &&
    m.storage_path === `${m.user_id}/${m.session}-${m.game_index}.json.gz`;
}

async function readCapped(stream: ReadableStream<Uint8Array>, max: number): Promise<Uint8Array> {
  const reader = stream.getReader();
  const chunks: Uint8Array[] = [];
  let size = 0;
  try {
    while (true) {
      const { done, value } = await reader.read();
      if (done) break;
      size += value.length;
      if (size > max) throw new Error("too large");
      chunks.push(value);
    }
  } finally {
    await reader.cancel().catch(() => {});
  }
  const out = new Uint8Array(size);
  let at = 0;
  for (const c of chunks) {
    out.set(c, at);
    at += c.length;
  }
  return out;
}

async function sha256Hex(bytes: Uint8Array): Promise<string> {
  const digest = new Uint8Array(await crypto.subtle.digest("SHA-256", bytes as BufferSource));
  return Array.from(digest, (b) => b.toString(16).padStart(2, "0")).join("");
}

/** Why a game got no summary; the keys private.sweep_stats_valid() checks. */
const SKIP_REASONS = [
  "no_file", // the stored file could not be read
  "bad_file", // too large, not gzip or JSON, or it fails the upload check
  "file_mismatch", // its SHA-256 is not the row's
  "write_failed", // the write call failed or gave an unknown answer
  "missing", // the game is gone (game_stats_backfill_put)
  "changed", // another copy was uploaded meanwhile
  "newer", // a summary of a newer version is there
] as const;
type SkipReason = typeof SKIP_REASONS[number];
const PUT_SKIPS: readonly unknown[] = ["missing", "changed", "newer"];

export interface BackfillCounts {
  listed: number;
  looked_at: number;
  filled: number;
  left: number;
  skipped: Record<SkipReason, number>;
}

/**
 * The summary of one stored game, or why there is none: its file is missing,
 * is not the copy the row names, or no longer passes the upload check.
 */
async function summaryOf(
  env: Env,
  fetchFn: Fetch,
  game: Missing,
): Promise<{ stats: unknown } | { skip: SkipReason }> {
  let res: Response;
  try {
    res = await fetchFn(`${env.url}/storage/v1/object/authenticated/games/${game.storage_path}`, {
      headers: { apikey: env.serviceKey },
      signal: AbortSignal.timeout(TIMEOUT_MS),
    });
  } catch (_e) {
    return { skip: "no_file" };
  }
  if (!res.ok || !res.body) {
    await res.body?.cancel();
    return { skip: "no_file" };
  }
  try {
    const file = await readCapped(res.body, MAX_FILE);
    const bytes = await readCapped(
      new Blob([file as BlobPart]).stream().pipeThrough(new DecompressionStream("gzip")),
      MAX_INFLATED,
    );
    if ((await sha256Hex(bytes)) !== game.content_sha256) return { skip: "file_mismatch" };
    const record = JSON.parse(new TextDecoder("utf-8", { fatal: true }).decode(bytes));
    if (!validateRecord(record, game.session, game.game_index).ok) return { skip: "bad_file" };
    return { stats: summarize(record.report) };
  } catch (_e) {
    return { skip: "bad_file" };
  }
}

/** Fills missing or outdated game summaries; counts what happened to each. */
async function backfillStats(env: Env, fetchFn: Fetch): Promise<BackfillCounts> {
  const res = await call(
    fetchFn,
    "list summaries",
    `${env.url}/rest/v1/rpc/game_stats_backfill_list`,
    {
      method: "POST",
      headers: serviceHeaders(env),
      body: JSON.stringify({ p_version: SUMMARY_VERSION, p_limit: BACKFILL_LIMIT }),
    },
  );
  const games = await res.json();
  // Never fetch anything that is not shaped like the game the row names.
  if (!Array.isArray(games) || !games.every(isMissing)) {
    throw new StepError("list summaries", 502);
  }
  const now = env.now ?? Date.now;
  const deadline = now() + BACKFILL_BUDGET_MS;
  const skipped = Object.fromEntries(SKIP_REASONS.map((r) => [r, 0])) as Record<SkipReason, number>;
  const counts: BackfillCounts = {
    listed: games.length,
    looked_at: 0,
    filled: 0,
    left: 0,
    skipped,
  };
  for (const game of games) {
    if (now() >= deadline) {
      counts.left += 1;
      continue;
    }
    counts.looked_at += 1;
    const summary = await summaryOf(env, fetchFn, game);
    if ("skip" in summary) {
      skipped[summary.skip] += 1;
      continue;
    }
    let result: unknown;
    try {
      const put = await call(
        fetchFn,
        "write summary",
        `${env.url}/rest/v1/rpc/game_stats_backfill_put`,
        {
          method: "POST",
          headers: serviceHeaders(env),
          body: JSON.stringify({
            target: game.user_id,
            p_session: game.session,
            p_game_index: game.game_index,
            p_sha256: game.content_sha256,
            p_summary_version: SUMMARY_VERSION,
            p_stats: summary.stats,
          }),
        },
      );
      result = await put.json();
    } catch (_e) {
      skipped.write_failed += 1;
      continue;
    }
    // changed, newer or missing: another copy or version won; nothing to do.
    if (result === "stored") counts.filled += 1;
    else if (PUT_SKIPS.includes(result)) skipped[result as SkipReason] += 1;
    else skipped.write_failed += 1;
  }
  const failed = skipped.no_file + skipped.bad_file + skipped.file_mismatch + skipped.write_failed;
  if (failed > 0) {
    // Counts only: no user id, session or file name in the logs.
    console.error(`sweep: ${failed} of ${counts.listed} game summaries could not be made`);
  }
  return counts;
}

/** Records the step a run failed at; a failure to record is logged, not thrown. */
async function recordFailure(env: Env, fetchFn: Fetch, runId: number | null, step: string) {
  try {
    const res = await call(
      fetchFn,
      "record failure",
      `${env.url}/rest/v1/rpc/sweep_record_failure`,
      {
        method: "POST",
        headers: serviceHeaders(env),
        body: JSON.stringify({ p_run_id: runId, p_step: step }),
      },
    );
    await res.body?.cancel();
  } catch (e) {
    console.error(`sweep: ${e instanceof StepError ? e.message : "record failure failed"}`);
  }
}

/** Deletes orphan files, does the housekeeping and the backfill, records the run. */
async function sweep(env: Env, fetchFn: Fetch): Promise<Response> {
  let runId: number | null = null;
  try {
    const files = await deleteOrphans(env, fetchFn);
    const res = await call(
      fetchFn,
      "housekeeping",
      `${env.url}/rest/v1/rpc/sweep_housekeeping`,
      { method: "POST", headers: serviceHeaders(env), body: "{}" },
    );
    const { run_id, ...counts } = await res.json();
    if (!Number.isSafeInteger(run_id)) throw new StepError("housekeeping", 502);
    runId = run_id;
    const stats = await backfillStats(env, fetchFn);
    const done = await call(fetchFn, "record run", `${env.url}/rest/v1/rpc/sweep_record_result`, {
      method: "POST",
      headers: serviceHeaders(env),
      body: JSON.stringify({ p_run_id: runId, p_files: files, p_stats: stats }),
    });
    await done.body?.cancel();
    return json(200, { run_id: runId, files, ...counts, stats });
  } catch (e) {
    const step = e instanceof StepError ? e.step : "unexpected";
    console.error(`sweep: ${e instanceof StepError ? e.message : "unexpected error"}`);
    await recordFailure(env, fetchFn, runId, step);
    return step === "unexpected"
      ? json(500, { error: "the sweep did not finish; run it again" })
      : json(500, { error: "the sweep did not finish; run it again", step });
  }
}

export async function handle(req: Request, env: Env, fetchFn: Fetch = fetch): Promise<Response> {
  try {
    if (!isServiceRole(req, env) && !(await hasSweepToken(req, env, fetchFn))) {
      return json(401, { error: "service role only" });
    }
  } catch (e) {
    // Callers without a valid key may reach this: say nothing about the
    // database, and record nothing (anyone could fill the record).
    console.error(`sweep: ${e instanceof StepError ? e.message : "token check failed"}`);
    return json(503, { error: "try again later" });
  }
  if (req.method !== "POST") return json(405, { error: "use POST" });
  return await sweep(env, fetchFn);
}

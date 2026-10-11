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
//      again by validateRecord() and against the row's SHA-256. At most 200
//      per run; the rest wait for the next run.
// Safe to run any time and as often as wanted. No dependencies.

import { summarize, SUMMARY_VERSION } from "../_shared/game_summary.ts";
import { validateRecord } from "../upload-game/validate.ts";

export interface Env {
  url: string;
  serviceKey: string;
}

export type Fetch = (input: string, init?: RequestInit) => Promise<Response>;

const TIMEOUT_MS = 10_000;
// sweep_orphan_files() returns at most 1000 names per call.
const MAX_ROUNDS = 20;
const GAME_FILE =
  /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}\/Hearthstone_\d{4}(_\d{2}){5}-[1-9][0-9]{0,3}\.json\.gz$/;
// Summaries filled per run; a stored game is about 2.5 KB gzipped.
const BACKFILL_LIMIT = 200;
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

/**
 * The summary of one stored game, or null when its file is missing, is not
 * the copy the row names, or no longer passes the upload check.
 */
async function summaryOf(env: Env, fetchFn: Fetch, game: Missing) {
  let res: Response;
  try {
    res = await fetchFn(`${env.url}/storage/v1/object/authenticated/games/${game.storage_path}`, {
      headers: { apikey: env.serviceKey },
      signal: AbortSignal.timeout(TIMEOUT_MS),
    });
  } catch (_e) {
    return null;
  }
  if (!res.ok || !res.body) {
    await res.body?.cancel();
    return null;
  }
  try {
    const file = await readCapped(res.body, MAX_FILE);
    const bytes = await readCapped(
      new Blob([file as BlobPart]).stream().pipeThrough(new DecompressionStream("gzip")),
      MAX_INFLATED,
    );
    if ((await sha256Hex(bytes)) !== game.content_sha256) return null;
    const record = JSON.parse(new TextDecoder("utf-8", { fatal: true }).decode(bytes));
    if (!validateRecord(record, game.session, game.game_index).ok) return null;
    return summarize(record.report);
  } catch (_e) {
    return null;
  }
}

/** Fills missing or outdated game summaries; counts what happened to each. */
async function backfillStats(env: Env, fetchFn: Fetch) {
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
  const counts = { listed: games.length, stored: 0, skipped: 0, failed: 0 };
  for (const game of games) {
    const stats = await summaryOf(env, fetchFn, game);
    if (stats === null) {
      counts.failed += 1;
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
            p_stats: stats,
          }),
        },
      );
      result = await put.json();
    } catch (_e) {
      counts.failed += 1;
      continue;
    }
    // changed, newer or missing: another copy or version won; nothing to do.
    if (result === "stored") counts.stored += 1;
    else counts.skipped += 1;
  }
  if (counts.failed > 0) {
    // Counts only: no user id, session or file name in the logs.
    console.error(`sweep: ${counts.failed} of ${counts.listed} game summaries could not be made`);
  }
  return counts;
}

export async function handle(req: Request, env: Env, fetchFn: Fetch = fetch): Promise<Response> {
  try {
    if (!isServiceRole(req, env) && !(await hasSweepToken(req, env, fetchFn))) {
      return json(401, { error: "service role only" });
    }
    if (req.method !== "POST") return json(405, { error: "use POST" });
    const files = await deleteOrphans(env, fetchFn);
    const res = await call(
      fetchFn,
      "housekeeping",
      `${env.url}/rest/v1/rpc/sweep_housekeeping`,
      { method: "POST", headers: serviceHeaders(env), body: "{}" },
    );
    const counts = await res.json();
    const stats = await backfillStats(env, fetchFn);
    return json(200, { files, ...counts, stats });
  } catch (e) {
    if (e instanceof StepError && e.step === "check token") {
      // Callers without a valid key may reach this: say nothing about the database.
      console.error(`sweep: ${e.message}`);
      return json(503, { error: "try again later" });
    }
    if (e instanceof StepError) {
      console.error(`sweep: ${e.message}`);
      return json(500, { error: "the sweep did not finish; run it again", step: e.step });
    }
    console.error("sweep: unexpected error");
    return json(500, { error: "the sweep did not finish; run it again" });
  }
}

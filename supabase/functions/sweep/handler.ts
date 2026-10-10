// Daily sweep (T-104d). POST /functions/v1/sweep with the service key the
// function itself uses (the new secret key where the platform gives it, D-042),
// or with the sweep token that the scheduled job reads from Vault
// (X-Sweep-Token; checked by the database, migration 20261010090000):
//   1. game files with no summary row (an upload that wrote its file and then
//      failed before the row), untouched for an hour, through the Storage
//      API (deleting storage.objects rows in SQL would leave the bytes);
//   2. public.sweep_housekeeping(): audit entries of deleted accounts (the
//      T-104a leftover), upload times older than two days and IP counters
//      older than an hour.
// Safe to run any time and as often as wanted. No dependencies.

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
  if (!env.serviceKey.startsWith("sb_")) {
    headers.Authorization = `Bearer ${env.serviceKey}`;
  }
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
    return json(200, { files, ...counts });
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

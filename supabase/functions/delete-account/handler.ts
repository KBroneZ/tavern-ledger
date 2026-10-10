// Account deletion (T-104a, CLAUDE.md rule 6). The signed-in user calls
// DELETE /functions/v1/delete-account with their access token. In this order:
//   1. the user's files in the `games` bucket, through the Storage API
//      (deleting storage.objects rows in SQL would leave the bytes behind);
//   2. their rows: public.delete_user_data() refuses while files remain;
//   3. the auth user (identities, sessions and refresh tokens go with it);
//   4. the audit entry step 3 writes (it holds the email).
// Every step can be repeated, so a failure halfway is fixed by calling again.
// No dependencies: plain fetch against the project's own HTTP APIs.
//
// Before any step (T-104c): a browser request must come from the website's
// own origin (SITE_ORIGINS), and this session must come from a sign-in within
// the last MAX_SIGN_IN_AGE_MS (both the account's last sign-in and the
// token's own `amr` time), so a stolen or forgotten session cannot delete the
// account. Requests with no Origin header (not a browser) skip the CORS
// check; they still need a valid token and a recent sign-in.

export interface Env {
  url: string;
  serviceKey: string;
  /** Exact origins of the website, e.g. https://example.org. Empty: none. */
  allowedOrigins: readonly string[];
  /** Milliseconds since 1970; injectable for tests. */
  now?: () => number;
}

export type Fetch = (input: string, init?: RequestInit) => Promise<Response>;

const TIMEOUT_MS = 10_000;
// The Storage API removes at most 1000 objects per request.
const FILE_BATCH = 1000;
// Upload quota is 10,000 games (web-stack.md), so 20 rounds is plenty.
const MAX_FILE_ROUNDS = 20;
const UUID = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/;
export const MAX_SIGN_IN_AGE_MS = 10 * 60 * 1000;
// Clocks of this function and the auth server may differ a little.
const CLOCK_SKEW_MS = 60 * 1000;
const CORS_ALLOW_HEADERS = "authorization, apikey, content-type, x-client-info";

/** Comma-separated SITE_ORIGINS to a list of exact http(s) origins. */
export function parseOrigins(value: string | undefined): string[] {
  if (!value) return [];
  return value
    .split(",")
    .map((s) => s.trim())
    .filter((s) => {
      try {
        const u = new URL(s);
        return (u.protocol === "https:" || u.protocol === "http:") && u.origin === s;
      } catch (_e) {
        return false;
      }
    });
}

class StepError extends Error {
  constructor(readonly step: string, readonly status: number) {
    super(`${step} failed with HTTP ${status}`);
  }
}

function json(status: number, body: unknown, extra: Record<string, string> = {}): Response {
  return new Response(JSON.stringify(body), {
    status,
    headers: { "Content-Type": "application/json", ...extra },
  });
}

function serviceHeaders(env: Env): Record<string, string> {
  // The secret key (sb_secret_...) goes in `apikey` only, never as a bearer
  // token; the gateway turns it into a service-role token (D-042, D-053).
  const headers: Record<string, string> = {
    apikey: env.serviceKey,
    "Content-Type": "application/json",
  };
  return headers;
}

async function call(
  fetchFn: Fetch,
  step: string,
  url: string,
  init: RequestInit,
): Promise<Response> {
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

/** Like call(), for answers whose body is not needed. */
async function send(fetchFn: Fetch, step: string, url: string, init: RequestInit) {
  const res = await call(fetchFn, step, url, init);
  await res.body?.cancel();
}

/**
 * When this session signed in: the newest `amr` timestamp of the access token,
 * in milliseconds, or NaN. Only called after /auth/v1/user has accepted the
 * token, so its signature is already checked.
 */
function sessionSignInAt(token: string): number {
  const parts = token.split(".");
  if (parts.length !== 3) return NaN;
  try {
    const b64 = parts[1].replaceAll("-", "+").replaceAll("_", "/");
    const payload = JSON.parse(atob(b64.padEnd(Math.ceil(b64.length / 4) * 4, "=")));
    if (!Array.isArray(payload?.amr)) return NaN;
    const times = payload.amr
      .map((a: { timestamp?: unknown }) => a?.timestamp)
      .filter((t: unknown): t is number => typeof t === "number" && Number.isFinite(t));
    return times.length === 0 ? NaN : Math.max(...times) * 1000;
  } catch (_e) {
    return NaN;
  }
}

function isRecent(at: number, now: number): boolean {
  return Number.isFinite(at) && now - at <= MAX_SIGN_IN_AGE_MS && at - now <= CLOCK_SKEW_MS;
}

interface CurrentUser {
  id: string;
  /** Milliseconds since 1970, or NaN if the answer had no usable time. */
  lastSignInAt: number;
}

/** The user the access token belongs to, or null if it is not valid. */
async function currentUser(
  env: Env,
  fetchFn: Fetch,
  authorization: string,
): Promise<CurrentUser | null> {
  let res: Response;
  try {
    res = await fetchFn(`${env.url}/auth/v1/user`, {
      headers: { apikey: env.serviceKey, Authorization: authorization },
      signal: AbortSignal.timeout(TIMEOUT_MS),
    });
  } catch (_e) {
    throw new StepError("auth", 0);
  }
  if (res.status === 401 || res.status === 403) {
    await res.body?.cancel();
    return null;
  }
  if (!res.ok) {
    await res.body?.cancel();
    throw new StepError("auth", res.status);
  }
  const user = await res.json();
  const id = user && typeof user.id === "string" ? user.id : null;
  if (!id || !UUID.test(id)) return null;
  const last = typeof user.last_sign_in_at === "string" ? Date.parse(user.last_sign_in_at) : NaN;
  return { id, lastSignInAt: last };
}

async function fileNames(env: Env, fetchFn: Fetch, userId: string): Promise<string[]> {
  const res = await call(fetchFn, "list files", `${env.url}/rest/v1/rpc/user_file_names`, {
    method: "POST",
    headers: serviceHeaders(env),
    body: JSON.stringify({ target: userId }),
  });
  const names = await res.json();
  if (!Array.isArray(names) || !names.every((n) => typeof n === "string")) {
    throw new StepError("list files", 502);
  }
  // Never touch anything outside the user's folder, whatever the list says.
  return names.filter((n) => n.startsWith(`${userId}/`));
}

async function deleteFiles(env: Env, fetchFn: Fetch, userId: string): Promise<number> {
  let deleted = 0;
  for (let round = 0; round < MAX_FILE_ROUNDS; round++) {
    const names = await fileNames(env, fetchFn, userId);
    if (names.length === 0) return deleted;
    const batch = names.slice(0, FILE_BATCH);
    await send(fetchFn, "delete files", `${env.url}/storage/v1/object/games`, {
      method: "DELETE",
      headers: serviceHeaders(env),
      body: JSON.stringify({ prefixes: batch }),
    });
    deleted += batch.length;
  }
  throw new StepError("delete files", 508);
}

export async function handle(req: Request, env: Env, fetchFn: Fetch = fetch): Promise<Response> {
  const origin = req.headers.get("Origin");
  if (origin !== null && !env.allowedOrigins.includes(origin)) {
    return json(403, { error: "origin not allowed" });
  }
  const cors: Record<string, string> = origin === null
    ? {}
    : { "Access-Control-Allow-Origin": origin, Vary: "Origin" };
  if (req.method === "OPTIONS") {
    return new Response(null, {
      status: 204,
      headers: {
        ...cors,
        "Access-Control-Allow-Methods": "DELETE, OPTIONS",
        "Access-Control-Allow-Headers": CORS_ALLOW_HEADERS,
        "Access-Control-Max-Age": "600",
      },
    });
  }
  if (req.method !== "DELETE") {
    return json(405, { error: "method not allowed" }, cors);
  }
  const authorization = req.headers.get("Authorization") ?? "";
  if (!/^Bearer [A-Za-z0-9._~+/-]+=*$/.test(authorization)) {
    return json(401, { error: "sign in first" }, cors);
  }
  try {
    const user = await currentUser(env, fetchFn, authorization);
    if (user === null) {
      return json(401, { error: "sign in first" }, cors);
    }
    const now = (env.now ?? Date.now)();
    const token = authorization.slice("Bearer ".length);
    if (!isRecent(user.lastSignInAt, now) || !isRecent(sessionSignInAt(token), now)) {
      return json(
        403,
        { error: "sign in again to delete your account", reason: "reauthenticate" },
        cors,
      );
    }
    const userId = user.id;
    const files = await deleteFiles(env, fetchFn, userId);
    const rowsRes = await call(fetchFn, "delete rows", `${env.url}/rest/v1/rpc/delete_user_data`, {
      method: "POST",
      headers: serviceHeaders(env),
      body: JSON.stringify({ target: userId }),
    });
    const rows = await rowsRes.json();
    await send(fetchFn, "delete auth user", `${env.url}/auth/v1/admin/users/${userId}`, {
      method: "DELETE",
      headers: serviceHeaders(env),
    });
    const eventsRes = await call(
      fetchFn,
      "delete auth events",
      `${env.url}/rest/v1/rpc/delete_user_auth_events`,
      { method: "POST", headers: serviceHeaders(env), body: JSON.stringify({ target: userId }) },
    );
    const lateEvents = await eventsRes.json();
    const authEvents = Number(rows?.auth_events ?? 0) + Number(lateEvents ?? 0);
    return json(200, { deleted: { files, ...rows, auth_events: authEvents, account: 1 } }, cors);
  } catch (e) {
    if (e instanceof StepError) {
      // Step and status only: no token, id or email in the logs.
      console.error(`delete-account: ${e.message}`);
      return json(500, { error: "deletion did not finish; try again", step: e.step }, cors);
    }
    console.error("delete-account: unexpected error");
    return json(500, { error: "deletion did not finish; try again" }, cors);
  }
}

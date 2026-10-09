// Account deletion (T-104a, CLAUDE.md rule 6). The signed-in user calls
// DELETE /functions/v1/delete-account with their access token. In this order:
//   1. the user's files in the `games` bucket, through the Storage API
//      (deleting storage.objects rows in SQL would leave the bytes behind);
//   2. their rows: public.delete_user_data() refuses while files remain;
//   3. the auth user (identities, sessions and refresh tokens go with it).
// Every step can be repeated, so a failure halfway is fixed by calling again.
// No dependencies: plain fetch against the project's own HTTP APIs.

export interface Env {
  url: string;
  serviceKey: string;
}

export type Fetch = (input: string, init?: RequestInit) => Promise<Response>;

const TIMEOUT_MS = 10_000;
// The Storage API removes at most 1000 objects per request.
const FILE_BATCH = 1000;
// Upload quota is 10,000 games (web-stack.md), so 20 rounds is plenty.
const MAX_FILE_ROUNDS = 20;
const UUID = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/;

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
  // New-style secret keys (sb_secret_...) go in `apikey` only; the gateway
  // turns them into a service-role token. Legacy keys are JWTs.
  const headers: Record<string, string> = {
    apikey: env.serviceKey,
    "Content-Type": "application/json",
  };
  if (!env.serviceKey.startsWith("sb_")) {
    headers.Authorization = `Bearer ${env.serviceKey}`;
  }
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

/** Id of the user the access token belongs to, or null if it is not valid. */
async function currentUserId(
  env: Env,
  fetchFn: Fetch,
  authorization: string,
): Promise<string | null> {
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
  return id && UUID.test(id) ? id : null;
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
    await call(fetchFn, "delete files", `${env.url}/storage/v1/object/games`, {
      method: "DELETE",
      headers: serviceHeaders(env),
      body: JSON.stringify({ prefixes: batch }),
    });
    deleted += batch.length;
  }
  throw new StepError("delete files", 508);
}

export async function handle(req: Request, env: Env, fetchFn: Fetch = fetch): Promise<Response> {
  if (req.method !== "DELETE") {
    return json(405, { error: "method not allowed" });
  }
  const authorization = req.headers.get("Authorization") ?? "";
  if (!/^Bearer [A-Za-z0-9._~+/-]+=*$/.test(authorization)) {
    return json(401, { error: "sign in first" });
  }
  try {
    const userId = await currentUserId(env, fetchFn, authorization);
    if (userId === null) {
      return json(401, { error: "sign in first" });
    }
    const files = await deleteFiles(env, fetchFn, userId);
    const rowsRes = await call(fetchFn, "delete rows", `${env.url}/rest/v1/rpc/delete_user_data`, {
      method: "POST",
      headers: serviceHeaders(env),
      body: JSON.stringify({ target: userId }),
    });
    const rows = await rowsRes.json();
    await call(fetchFn, "delete auth user", `${env.url}/auth/v1/admin/users/${userId}`, {
      method: "DELETE",
      headers: serviceHeaders(env),
    });
    return json(200, { deleted: { files, ...rows, account: 1 } });
  } catch (e) {
    if (e instanceof StepError) {
      // Step and status only: no token, id or email in the logs.
      console.error(`delete-account: ${e.message}`);
      return json(500, { error: "deletion did not finish; try again", step: e.step });
    }
    console.error("delete-account: unexpected error");
    return json(500, { error: "deletion did not finish; try again" });
  }
}

// Game upload (T-104d, D-021). The desktop app calls
//   PUT /functions/v1/upload-game/v1/games/<session>/<index>
// with the user's access token and one game record, gzipped
// (Content-Type: application/gzip). In this order:
//   1. address, method and type; the body is read up to 64 KiB;
//   2. the client's IP (hashed here, salted by the database: D-042) is not
//      blocked; the token is valid (a refused token counts against the IP);
//   3. the body inflates to at most 512 KiB of UTF-8 JSON that passes
//      validateRecord(): no player names, no fields outside the report;
//   4. upload_begin(): revision, rate limits and total quota;
//   5. the file, re-compressed here from the checked bytes, is written with
//      the service role, then upload_commit() writes the summary row.
// Same bytes again: 200 without writing. Answers carry a code and the
// validator's version (D-047), never the values sent, a token or an id. No
// dependencies: plain fetch against the project's own HTTP APIs.

import { parsePath, type Summary, validateRecord, VALIDATOR_VERSION } from "./validate.ts";

export interface Env {
  url: string;
  serviceKey: string;
}

export type Fetch = (input: string, init?: RequestInit) => Promise<Response>;

type Bytes = Uint8Array<ArrayBuffer>;

/** Largest gzipped body read (the largest real game is about 3.3 KB). */
export const MAX_BODY = 64 * 1024;
/** Largest inflated record (the largest real game is about 29 KB). */
export const MAX_INFLATED = 512 * 1024;
const TIMEOUT_MS = 10_000;
/** Seconds a blocked IP is told to wait (the IP window is 10 minutes). */
const IP_RETRY_AFTER = 600;
const UUID = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/;

class StepError extends Error {
  constructor(readonly step: string, readonly status: number) {
    super(`${step} failed with HTTP ${status}`);
  }
}

class TooLarge extends Error {}

/** Every answer says which validator ran, so the app can retry games it refused. */
function json(status: number, body: object, extra: Record<string, string> = {}): Response {
  return new Response(JSON.stringify({ ...body, validator: VALIDATOR_VERSION }), {
    status,
    headers: { "Content-Type": "application/json", "Cache-Control": "no-store", ...extra },
  });
}

const refuse = (status: number, code: string, error: string, extra = {}) =>
  json(status, { error, code }, extra);

function serviceHeaders(env: Env): Record<string, string> {
  // The secret key (sb_secret_...) goes in `apikey` only, never as a bearer
  // token; the gateway turns it into a service-role token (D-042, D-053).
  const headers: Record<string, string> = {
    apikey: env.serviceKey,
    "Content-Type": "application/json",
  };
  return headers;
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

async function rpc(env: Env, fetchFn: Fetch, step: string, name: string, args: unknown) {
  const res = await call(fetchFn, step, `${env.url}/rest/v1/rpc/${name}`, {
    method: "POST",
    headers: serviceHeaders(env),
    body: JSON.stringify(args),
  });
  const text = await res.text();
  return text ? JSON.parse(text) : null;
}

/** Reads a stream into memory, or throws TooLarge past `max` bytes. */
async function readCapped(stream: ReadableStream<Uint8Array>, max: number): Promise<Bytes> {
  const reader = stream.getReader();
  const chunks: Uint8Array[] = [];
  let size = 0;
  try {
    while (true) {
      const { done, value } = await reader.read();
      if (done) break;
      size += value.length;
      if (size > max) throw new TooLarge();
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

const streamOf = (bytes: Bytes) => new Blob([bytes]).stream();

async function gzip(bytes: Bytes): Promise<Bytes> {
  return await readCapped(streamOf(bytes).pipeThrough(new CompressionStream("gzip")), Infinity);
}

async function sha256Hex(bytes: Bytes): Promise<string> {
  const digest = new Uint8Array(await crypto.subtle.digest("SHA-256", bytes));
  return Array.from(digest, (b) => b.toString(16).padStart(2, "0")).join("");
}

/**
 * The client's IP: the platform's own header when there is one, else the
 * first X-Forwarded-For entry (which a client can forge: it only feeds the
 * limit on refused tokens, and a valid token is never refused for it).
 * Which header the hosted gateway sets is **unverified**.
 */
function clientIp(req: Request): string {
  const platform = req.headers.get("CF-Connecting-IP")?.trim();
  const forwarded = req.headers.get("X-Forwarded-For")?.split(",")[0]?.trim();
  return platform || forwarded || req.headers.get("X-Real-IP")?.trim() || "unknown";
}

/** The user the access token belongs to, or null if it is not valid. */
async function currentUser(env: Env, fetchFn: Fetch, authorization: string) {
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

type Decoded =
  | { ok: true; bytes: Bytes; value: unknown }
  | { ok: false; response: Response };

async function decode(body: Bytes): Promise<Decoded> {
  let bytes: Bytes;
  try {
    bytes = await readCapped(
      streamOf(body).pipeThrough(new DecompressionStream("gzip")),
      MAX_INFLATED,
    );
  } catch (e) {
    if (e instanceof TooLarge) {
      return { ok: false, response: refuse(413, "too_large", "the game is too large") };
    }
    return { ok: false, response: refuse(400, "bad_gzip", "the body is not gzip") };
  }
  const notJson = () =>
    ({ ok: false, response: refuse(400, "invalid_json", "the game is not compact JSON") }) as const;
  let value: unknown;
  try {
    const text = new TextDecoder("utf-8", { fatal: true }).decode(bytes);
    value = JSON.parse(text);
    // Only compact JSON with no repeated keys: then the bytes stored are
    // exactly what was checked (JSON.parse keeps only the last of a repeated
    // key, so a first copy could hide anything). The app sends serde_json's
    // compact output, which JSON.stringify reproduces.
    if (JSON.stringify(value) !== text) return notJson();
  } catch (_e) {
    return notJson();
  }
  return { ok: true, bytes, value };
}

function commitArgs(
  userId: string,
  key: { session: string; index: number },
  s: Summary,
  sha: string,
  size: number,
) {
  return {
    target: userId,
    p_session: key.session,
    p_game_index: key.index,
    p_game_type: s.game_type,
    p_status: s.status,
    p_hero_card_id: s.hero_card_id,
    p_final_place: s.final_place,
    p_build: s.build,
    p_played_on: s.played_on,
    p_tribes_offered: s.tribes_offered,
    p_saved_at: s.saved_at,
    p_sha256: sha,
    p_size: size,
    p_parser_version: s.parser_version,
  };
}

const BEGIN_REFUSALS: Record<string, [number, string]> = {
  older: [409, "a newer copy of this game is already stored"],
  quota_games: [403, "your account holds the most games it can"],
  quota_bytes: [403, "your account holds the most data it can"],
};

async function store(
  req: Request,
  env: Env,
  fetchFn: Fetch,
  key: { session: string; index: number },
  body: Bytes,
): Promise<Response> {
  const ipKey = await sha256Hex(new TextEncoder().encode(clientIp(req)));
  const blocked = await rpc(env, fetchFn, "ip check", "upload_ip_blocked", { p_ip_key: ipKey });
  const tooMany = () =>
    refuse(429, "rate_limited", "too many failed requests; try again later", {
      "Retry-After": String(IP_RETRY_AFTER),
    });
  const authorization = req.headers.get("Authorization") ?? "";
  const wellFormed = /^Bearer [A-Za-z0-9._~+/-]+=*$/.test(authorization);
  if (blocked && !wellFormed) return tooMany();
  // A blocked IP still serves a valid token: the IP may be shared or forged.
  const userId = wellFormed ? await currentUser(env, fetchFn, authorization) : null;
  if (userId === null) {
    if (blocked) return tooMany();
    await rpc(env, fetchFn, "ip count", "upload_ip_failed", { p_ip_key: ipKey });
    return refuse(401, "unauthorized", "sign in first");
  }

  const decoded = await decode(body);
  if (!decoded.ok) return decoded.response;
  const checked = validateRecord(decoded.value, key.session, key.index);
  if (!checked.ok) {
    return json(400, {
      error: checked.code === "player_name"
        ? "the game record holds something shaped like a player name"
        : "the game record is not valid",
      code: checked.code,
      field: checked.field,
    });
  }
  const sha = await sha256Hex(decoded.bytes);
  // Store only the checked bytes: a gzip header can carry a name or comment.
  const file = await gzip(decoded.bytes);
  if (file.length > MAX_BODY) return refuse(413, "too_large", "the game is too large");

  const begin = await rpc(env, fetchFn, "begin", "upload_begin", {
    target: userId,
    p_session: key.session,
    p_game_index: key.index,
    p_saved_at: checked.summary.saved_at,
    p_sha256: sha,
    p_size: file.length,
  });
  const result = begin?.result;
  if (result === "unchanged") return json(200, { result: "unchanged", sha256: sha });
  if (result === "rate_limited") {
    const wait = Number.isInteger(begin.retry_after) ? begin.retry_after : 3600;
    return refuse(429, "rate_limited", "too many uploads; try again later", {
      "Retry-After": String(Math.max(1, wait)),
    });
  }
  if (result in BEGIN_REFUSALS) {
    const [status, message] = BEGIN_REFUSALS[result];
    return refuse(status, result === "older" ? "older_revision" : result, message);
  }
  if (result !== "create" && result !== "replace") throw new StepError("begin", 502);

  const path = `${userId}/${key.session}-${key.index}.json.gz`;
  const write = await call(fetchFn, "write file", `${env.url}/storage/v1/object/games/${path}`, {
    method: "POST",
    headers: { ...serviceHeaders(env), "Content-Type": "application/gzip", "x-upsert": "true" },
    body: file,
  });
  await write.body?.cancel();

  const committed = await rpc(
    env,
    fetchFn,
    "write row",
    "upload_commit",
    commitArgs(userId, key, checked.summary, sha, file.length),
  );
  if (committed === "created") return json(201, { result: "created", sha256: sha });
  if (committed === "replaced") return json(200, { result: "replaced", sha256: sha });
  if (committed === "older") {
    return refuse(409, "older_revision", "a newer copy of this game is already stored");
  }
  throw new StepError("write row", 502);
}

export async function handle(req: Request, env: Env, fetchFn: Fetch = fetch): Promise<Response> {
  // Only the desktop app calls this; a browser page never may.
  if (req.headers.get("Origin") !== null) {
    return refuse(403, "origin_not_allowed", "browsers cannot upload games");
  }
  if (req.method !== "PUT") {
    return refuse(405, "method_not_allowed", "use PUT", { Allow: "PUT" });
  }
  const key = parsePath(req.url);
  if (key === null) return refuse(404, "not_found", "no such game address");
  const type = (req.headers.get("Content-Type") ?? "").split(";")[0].trim().toLowerCase();
  if (type !== "application/gzip") {
    return refuse(415, "unsupported_media_type", "send the game as application/gzip");
  }
  const length = Number(req.headers.get("Content-Length") ?? "0");
  if (!Number.isFinite(length) || length > MAX_BODY) {
    return refuse(413, "too_large", "the game is too large");
  }
  let body: Bytes;
  try {
    body = req.body ? await readCapped(req.body, MAX_BODY) : new Uint8Array();
  } catch (e) {
    if (e instanceof TooLarge) return refuse(413, "too_large", "the game is too large");
    return refuse(400, "bad_body", "the body could not be read");
  }
  try {
    return await store(req, env, fetchFn, key, body);
  } catch (e) {
    if (e instanceof StepError) {
      // Step and status only: no token, id or IP in the logs.
      console.error(`upload-game: ${e.message}`);
      return json(500, { error: "the upload did not finish; try again", step: e.step });
    }
    console.error("upload-game: unexpected error");
    return json(500, { error: "the upload did not finish; try again" });
  }
}

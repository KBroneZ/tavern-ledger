// The Supabase calls in src/lib/api.ts without a server: a real client with
// a stubbed fetch for the database calls, and small fake clients elsewhere.
// The same functions run against the local stack in test/local/.
import { afterEach, test } from "node:test";
import assert from "node:assert/strict";
import type { SupabaseClient, SupportedStorage } from "@supabase/supabase-js";
import {
  deleteAccount,
  exportAll,
  finishAuthLink,
  loadPublicProfile,
  makeClient,
  requestPasswordReset,
  saveProfile,
  setNewPassword,
  signUp,
} from "../../src/lib/api.ts";
import { RESET_SENT } from "../../src/lib/auth.ts";

const CONFIG = { url: "http://127.0.0.1:54321", anonKey: "sb_publishable_test" };
const USER = "0f8fad5b-d9cb-469f-a165-70867728950e";
const realFetch = globalThis.fetch;

afterEach(() => {
  globalThis.fetch = realFetch;
});

function memoryStorage(): SupportedStorage {
  const items = new Map<string, string>();
  return {
    getItem: (k) => items.get(k) ?? null,
    setItem: (k, v) => void items.set(k, v),
    removeItem: (k) => void items.delete(k),
  };
}

type Route = (url: URL, init: RequestInit) => Response | Promise<Response>;

/** Stubs fetch; each request goes to the first route whose path matches. */
function stubFetch(routes: Record<string, Route>): string[] {
  const seen: string[] = [];
  globalThis.fetch = (async (input: RequestInfo | URL, init: RequestInit = {}) => {
    const url = new URL(input instanceof Request ? input.url : String(input));
    seen.push(`${init.method ?? "GET"} ${url.pathname}`);
    const route = Object.entries(routes).find(([path]) => url.pathname === path);
    if (!route) return new Response("{}", { status: 404 });
    return route[1](url, init);
  }) as typeof fetch;
  return seen;
}

function json(body: unknown, status = 200): Response {
  return new Response(JSON.stringify(body), { status, headers: { "Content-Type": "application/json" } });
}

/** PostgREST answers an array, or one object when the client asks for one. */
function rows(list: unknown[]): Route {
  return (_url, init) => {
    const accept = new Headers(init.headers).get("Accept") ?? "";
    if (accept.includes("vnd.pgrst.object")) {
      return list.length === 1 ? json(list[0]) : json({ code: "PGRST116", message: "no rows" }, 406);
    }
    return json(list);
  };
}

const client = () => makeClient(CONFIG, memoryStorage());

// --- loadPublicProfile ---

test("public profile: no public row is not-found, and games are not asked for", async () => {
  const seen = stubFetch({ "/rest/v1/profiles": rows([]) });
  assert.deepEqual(await loadPublicProfile(client(), USER), { kind: "not-found" });
  assert.deepEqual(seen, ["GET /rest/v1/profiles"]);
});

test("public profile: the query asks for public profiles only", async () => {
  let query = "";
  stubFetch({
    "/rest/v1/profiles": (url, init) => {
      query = url.search;
      return rows([])(url, init);
    },
  });
  await loadPublicProfile(client(), USER);
  assert.match(decodeURIComponent(query), /is_public=eq\.true/);
  assert.match(decodeURIComponent(query), new RegExp(`user_id=eq\\.${USER}`));
});

test("public profile: name and games through gameView", async () => {
  stubFetch({
    "/rest/v1/profiles": rows([{ display_name: "Keeper" }]),
    "/rest/v1/games": rows([
      { game_type: "GT_BATTLEGROUNDS", status: "ok", hero_card_id: "BG20_HERO_202", final_place: 1, played_on: "2026-10-09" },
      { game_type: "GT_BATTLEGROUNDS_DUO", status: "incomplete", hero_card_id: null, final_place: null, played_on: "2026-10-08" },
    ]),
  });
  assert.deepEqual(await loadPublicProfile(client(), USER), {
    kind: "found",
    name: "Keeper",
    games: [
      { date: "2026-10-09", mode: "Solo", hero: "BG20_HERO_202", place: "1st" },
      { date: "2026-10-08", mode: "Duos", hero: "unknown", place: "unfinished" },
    ],
  });
});

test("public profile: server and network errors are errors, not not-found", async () => {
  stubFetch({ "/rest/v1/profiles": () => json({ message: "boom" }, 500) });
  assert.deepEqual(await loadPublicProfile(client(), USER), { kind: "error" });
  stubFetch({
    "/rest/v1/profiles": rows([{ display_name: null }]),
    "/rest/v1/games": () => json({ message: "boom" }, 500),
  });
  assert.deepEqual(await loadPublicProfile(client(), USER), { kind: "error" });
  globalThis.fetch = (() => Promise.reject(new TypeError("offline"))) as typeof fetch;
  assert.deepEqual(await loadPublicProfile(client(), USER), { kind: "error" });
});

// --- saveProfile ---

test("save: one row back is saved", async () => {
  stubFetch({ "/rest/v1/profiles": () => json([{ user_id: USER }]) });
  assert.equal(await saveProfile(client(), USER, { displayName: "Keeper", isPublic: true }), null);
});

test("save: no row back means nothing was saved (row-level security)", async () => {
  stubFetch({ "/rest/v1/profiles": () => json([]) });
  assert.match((await saveProfile(client(), USER, { displayName: "Keeper", isPublic: false })) ?? "", /sign in/i);
});

test("save: the database's name check reads as a name rule", async () => {
  stubFetch({ "/rest/v1/profiles": () => json({ code: "23514", message: "violates check" }, 400) });
  assert.match((await saveProfile(client(), USER, { displayName: "x", isPublic: false })) ?? "", /letters/);
});

// --- exportAll ---

function fakeExportClient(download: () => Promise<{ data: Blob | null; error: unknown }>, rpcError: unknown = null) {
  const data = {
    format: "tavern-ledger-export",
    account: { id: USER },
    games: [],
    files: [{ bucket: "games", name: `${USER}/Hearthstone_2026_10_09_18_30_00-1.json.gz` }],
  };
  return {
    rpc: async () => ({ data: rpcError ? null : data, error: rpcError }),
    storage: { from: () => ({ download }) },
  } as unknown as SupabaseClient;
}

test("export: adds each file's bytes", async () => {
  const out = await exportAll(fakeExportClient(async () => ({ data: new Blob([new Uint8Array([1, 2, 3])]), error: null })));
  assert.deepEqual(out.file_contents, [
    { name: `${USER}/Hearthstone_2026_10_09_18_30_00-1.json.gz`, encoding: "gzip+base64", data: "AQID" },
  ]);
});

test("export: a file that cannot be read fails the export instead of leaving it out", async () => {
  await assert.rejects(exportAll(fakeExportClient(async () => ({ data: null, error: { message: "denied" } }))), /file/);
});

test("export: a failed rpc fails the export", async () => {
  await assert.rejects(exportAll(fakeExportClient(async () => ({ data: null, error: null }), { message: "x" })), /export/);
});

// --- deleteAccount ---

const withSession = (token: string | null) =>
  ({
    auth: { getSession: async () => ({ data: { session: token ? { access_token: token } : null } }) },
  }) as unknown as SupabaseClient;

test("delete: no session is signed-out, with no request", async () => {
  const seen = stubFetch({});
  assert.deepEqual(await deleteAccount(withSession(null), CONFIG), { kind: "signed-out" });
  assert.deepEqual(seen, []);
});

test("delete: sends the token and reads the answer", async () => {
  let auth = "";
  stubFetch({
    "/functions/v1/delete-account": (_url, init) => {
      auth = new Headers(init.headers).get("Authorization") ?? "";
      return json({ error: "sign in again", reason: "reauthenticate" }, 403);
    },
  });
  assert.deepEqual(await deleteAccount(withSession("tok"), CONFIG), { kind: "reauthenticate" });
  assert.equal(auth, "Bearer tok");
});

test("delete: a network failure says nothing was deleted yet", async () => {
  globalThis.fetch = (() => Promise.reject(new TypeError("offline"))) as typeof fetch;
  const outcome = await deleteAccount(withSession("tok"), CONFIG);
  assert.equal(outcome.kind, "error");
  assert.match(outcome.kind === "error" ? outcome.message : "", /Nothing was deleted/);
});

test("delete: a non-JSON answer is an error, not a crash", async () => {
  stubFetch({ "/functions/v1/delete-account": () => new Response("<html>", { status: 502 }) });
  assert.equal((await deleteAccount(withSession("tok"), CONFIG)).kind, "error");
});

// --- email links and password reset (D-048) ---

function b64url(value: unknown): string {
  return Buffer.from(JSON.stringify(value)).toString("base64url");
}

/** An Auth error as the server answers it (API version 2024-01-01). */
function authError(code: string, status: number): Response {
  return new Response(JSON.stringify({ code, msg: "refused" }), {
    status,
    headers: { "Content-Type": "application/json", "X-Supabase-Api-Version": "2024-01-01" },
  });
}

const ACCESS_TOKEN = `${b64url({ alg: "ES256" })}.${b64url({ sub: USER, exp: 4102444800, role: "authenticated" })}.c2ln`;

function session(): Record<string, unknown> {
  return {
    access_token: ACCESS_TOKEN,
    token_type: "bearer",
    expires_in: 3600,
    expires_at: Math.floor(Date.now() / 1000) + 3600,
    refresh_token: "refresh-token",
    user: { id: USER, aud: "authenticated", email: "someone@example.test" },
  };
}

test("reset request: sends a PKCE challenge and the account page as the return address", async () => {
  let body: Record<string, unknown> = {};
  let query = "";
  stubFetch({
    "/auth/v1/recover": async (url, init) => {
      body = JSON.parse(String(init.body));
      query = url.search;
      return json({});
    },
  });
  const outcome = await requestPasswordReset(client(), " someone@example.test ", "https://tavernledger.net/account/");
  assert.equal(outcome.kind, "check-email");
  assert.equal(body.email, "someone@example.test");
  assert.equal(body.code_challenge_method, "s256");
  assert.equal(typeof body.code_challenge, "string");
  assert.equal(new URLSearchParams(query).get("redirect_to"), "https://tavernledger.net/account/");
});

test("reset request: a refusal reads like a sent email, a rate limit and no connection say so", async () => {
  stubFetch({ "/auth/v1/recover": () => authError("user_not_found", 400) });
  const refused = await requestPasswordReset(client(), "someone@example.test", "https://x.test/account/");
  assert.deepEqual(refused, { kind: "check-email", message: RESET_SENT });
  stubFetch({ "/auth/v1/recover": () => authError("over_email_send_rate_limit", 429) });
  const again = await requestPasswordReset(client(), "someone@example.test", "https://x.test/account/");
  assert.deepEqual(again, { kind: "check-email", message: RESET_SENT }, "a second email to an account looks like the first");
  stubFetch({ "/auth/v1/recover": () => authError("over_request_rate_limit", 429) });
  const limited = await requestPasswordReset(client(), "someone@example.test", "https://x.test/account/");
  assert.equal(limited.kind, "error");
  assert.match(limited.message, /Too many attempts/);
  globalThis.fetch = (() => Promise.reject(new TypeError("offline"))) as typeof fetch;
  const offline = await requestPasswordReset(client(), "someone@example.test", "https://x.test/account/");
  assert.match(offline.message, /Could not reach/);
});

test("link: a reset code started in this browser opens the new password form", async () => {
  const c = client();
  stubFetch({ "/auth/v1/recover": () => json({}) });
  await requestPasswordReset(c, "someone@example.test", "https://x.test/account/");
  let verifier = "";
  stubFetch({
    "/auth/v1/token": (url, init) => {
      assert.equal(url.searchParams.get("grant_type"), "pkce");
      verifier = JSON.parse(String(init.body)).code_verifier;
      return json(session());
    },
  });
  const outcome = await finishAuthLink(c, CONFIG, { kind: "code", code: "0b5c4a1e-1111", flowId: null });
  assert.deepEqual(outcome, { recovery: true, message: null, tone: "info" });
  assert.ok(verifier.length >= 43);
  assert.equal((await c.auth.getSession()).data.session?.access_token, ACCESS_TOKEN);
});

test("link: a code opened in another browser says what to do, with no request", async () => {
  const seen = stubFetch({});
  const outcome = await finishAuthLink(client(), CONFIG, { kind: "code", code: "0b5c4a1e-1111", flowId: null });
  assert.equal(outcome.recovery, false);
  assert.equal(outcome.tone, "error");
  assert.match(outcome.message ?? "", /another browser/);
  assert.deepEqual(seen, []);
});

test("link: a sign-up code started here signs in and says the email is confirmed", async () => {
  const c = client();
  stubFetch({ "/auth/v1/signup": () => json({ id: USER }) });
  await signUp(c, "someone@example.test", "0123456789ab", "https://x.test/account/");
  stubFetch({ "/auth/v1/token": () => json(session()) });
  const outcome = await finishAuthLink(c, CONFIG, { kind: "code", code: "0b5c4a1e-1111", flowId: null });
  assert.deepEqual(outcome, { recovery: false, message: "Your email is confirmed.", tone: "ok" });
});

test("link: an expired code from the server is explained", async () => {
  const c = client();
  stubFetch({ "/auth/v1/recover": () => json({}) });
  await requestPasswordReset(c, "someone@example.test", "https://x.test/account/");
  stubFetch({ "/auth/v1/token": () => authError("flow_state_not_found", 404) });
  const outcome = await finishAuthLink(c, CONFIG, { kind: "code", code: "0b5c4a1e-1111", flowId: null });
  assert.match(outcome.message ?? "", /expired or was already used/);
});

test("link: a session in the hash is never kept, and is ended on the server", async () => {
  const c = client();
  let auth = "";
  const seen = stubFetch({
    "/auth/v1/logout": (_url, init) => {
      auth = new Headers(init.headers).get("Authorization") ?? "";
      return new Response(null, { status: 204 });
    },
  });
  const outcome = await finishAuthLink(c, CONFIG, { kind: "implicit", type: "invite", accessToken: ACCESS_TOKEN });
  assert.equal(outcome.recovery, false);
  assert.match(outcome.message ?? "", /confirmed.*Forgot password/);
  assert.equal(auth, `Bearer ${ACCESS_TOKEN}`);
  assert.deepEqual(seen, ["POST /auth/v1/logout"]);
  assert.equal((await c.auth.getSession()).data.session, null);
});

test("link: a session in the hash that is not shaped like a token sends nothing", async () => {
  const seen = stubFetch({});
  const outcome = await finishAuthLink(client(), CONFIG, { kind: "implicit", type: "signup", accessToken: null });
  assert.equal(outcome.message, "Your email is confirmed. Sign in to continue.");
  assert.deepEqual(seen, []);
});

test("link: a token_hash reset keeps the session for the new password form", async () => {
  const c = client();
  let body: Record<string, unknown> = {};
  stubFetch({
    "/auth/v1/verify": (_url, init) => {
      body = JSON.parse(String(init.body));
      return json(session());
    },
  });
  const outcome = await finishAuthLink(c, CONFIG, { kind: "token-hash", tokenHash: "token-hash-example", type: "recovery" });
  assert.deepEqual(outcome, { recovery: true, message: null, tone: "info" });
  assert.equal(body.token_hash, "token-hash-example");
  assert.equal(body.type, "recovery");
});

test("link: a token_hash confirmation confirms but does not stay signed in", async () => {
  const c = client();
  const seen = stubFetch({
    "/auth/v1/verify": () => json(session()),
    "/auth/v1/logout": () => new Response(null, { status: 204 }),
  });
  const outcome = await finishAuthLink(c, CONFIG, { kind: "token-hash", tokenHash: "token-hash-example", type: "signup" });
  assert.deepEqual(outcome, { recovery: false, message: "Your email is confirmed. Sign in to continue.", tone: "ok" });
  assert.deepEqual(seen, ["POST /auth/v1/verify", "POST /auth/v1/logout"]);
  assert.equal((await c.auth.getSession()).data.session, null);
});

test("link: an error in the address and a notice need no request", async () => {
  const seen = stubFetch({});
  const failed = await finishAuthLink(client(), CONFIG, { kind: "error", code: "otp_expired" });
  assert.equal(failed.tone, "error");
  assert.match(failed.message ?? "", /expired/);
  assert.equal((await finishAuthLink(client(), CONFIG, { kind: "notice" })).tone, "ok");
  assert.deepEqual(await finishAuthLink(client(), CONFIG, { kind: "none" }), { recovery: false, message: null, tone: "info" });
  assert.deepEqual(seen, []);
});

test("new password: saved with the reset session, and refusals in words", async () => {
  const c = client();
  stubFetch({ "/auth/v1/verify": () => json(session()) });
  await finishAuthLink(c, CONFIG, { kind: "token-hash", tokenHash: "token-hash-example", type: "recovery" });
  let body: Record<string, unknown> = {};
  let logout = "";
  stubFetch({
    "/auth/v1/user": (_url, init) => {
      body = JSON.parse(String(init.body));
      return json(session().user);
    },
    "/auth/v1/logout": (url) => {
      logout = url.searchParams.get("scope") ?? "";
      return new Response(null, { status: 204 });
    },
  });
  assert.equal(await setNewPassword(c, "a-new-password"), null);
  assert.equal(body.password, "a-new-password");
  assert.equal(logout, "others", "every other session of the account is ended");
  assert.ok((await c.auth.getSession()).data.session, "this one stays");
  stubFetch({ "/auth/v1/user": () => authError("same_password", 422) });
  assert.match((await setNewPassword(c, "a-new-password")) ?? "", /different/);
  stubFetch({ "/auth/v1/user": () => authError("over_request_rate_limit", 429) });
  assert.match((await setNewPassword(c, "a-new-password")) ?? "", /Too many attempts/);
});

test("new password: without a session it asks for a new link", async () => {
  stubFetch({});
  assert.match((await setNewPassword(client(), "a-new-password")) ?? "", /new link/);
});

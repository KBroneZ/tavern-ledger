// Run: deno test supabase/functions/delete-account/
// Fake HTTP APIs; the end-to-end test against the local stack is
// tests/test_supabase_local.py.
import { type Env, type Fetch, handle, MAX_SIGN_IN_AGE_MS, parseOrigins } from "./handler.ts";

const NOW = Date.parse("2026-10-09T12:00:00Z");
const SITE = "https://site.test";
const ENV: Env = {
  url: "http://api.test",
  serviceKey: "sb_secret_test",
  allowedOrigins: [SITE],
  now: () => NOW,
};
const USER = "00000000-0000-4000-8000-00000000000a";
const RECENT = new Date(NOW - 60_000).toISOString();

/** A token shaped like Supabase's access token. Only /auth/v1/user (faked
 *  here) checks the signature; the function only reads `amr` after that. */
function fakeToken(payload: unknown): string {
  const b64 = (o: unknown) =>
    btoa(JSON.stringify(o)).replaceAll("+", "-").replaceAll("/", "_").replace(/=+$/, "");
  return `${b64({ alg: "ES256", typ: "JWT" })}.${b64(payload)}.c2lnbmF0dXJl`;
}
const amrAt = (...msAgo: number[]) =>
  fakeToken({
    sub: USER,
    amr: msAgo.map((ago) => ({ method: "password", timestamp: Math.floor((NOW - ago) / 1000) })),
  });
const TOKEN = amrAt(60_000);

function assertEquals(actual: unknown, expected: unknown, msg = ""): void {
  const a = JSON.stringify(actual);
  const e = JSON.stringify(expected);
  if (a !== e) throw new Error(`${msg}\n  actual:   ${a}\n  expected: ${e}`);
}

interface Call {
  method: string;
  path: string;
  body: unknown;
  headers: Record<string, string>;
}

/** Fake project APIs. `files` is the bucket; `fail` makes one step fail. */
function fakeApi(
  opts: {
    files?: string[];
    fail?: string;
    userStatus?: number;
    stickyFiles?: boolean;
    lastSignIn?: unknown;
  } = {},
) {
  let files = [...(opts.files ?? [])];
  let fail = opts.fail;
  const calls: Call[] = [];
  const reply = (status: number, body: unknown) =>
    Promise.resolve(new Response(JSON.stringify(body), { status }));
  const fetchFn: Fetch = (input, init = {}) => {
    const path = input.replace(ENV.url, "");
    const method = init.method ?? "GET";
    const body = init.body ? JSON.parse(init.body as string) : null;
    calls.push({ method, path, body, headers: init.headers as Record<string, string> });
    if (fail === path) return reply(503, {});
    if (path === "/auth/v1/user") {
      if (opts.userStatus) return reply(opts.userStatus, {});
      const lastSignIn = "lastSignIn" in opts ? opts.lastSignIn : RECENT;
      return reply(200, { id: USER, last_sign_in_at: lastSignIn });
    }
    if (path === "/rest/v1/rpc/user_file_names") return reply(200, files);
    if (path === "/storage/v1/object/games" && method === "DELETE") {
      if (!opts.stickyFiles) files = files.filter((f) => !body.prefixes.includes(f));
      return reply(200, []);
    }
    if (path === "/rest/v1/rpc/delete_user_data") {
      return reply(200, { games: 2, profiles: 1, auth_events: 3 });
    }
    if (path === `/auth/v1/admin/users/${USER}` && method === "DELETE") return reply(200, {});
    if (path === "/rest/v1/rpc/delete_user_auth_events") return reply(200, 1);
    return reply(404, {});
  };
  return { fetchFn, calls, files: () => files, heal: () => (fail = undefined) };
}

const request = (
  method = "DELETE",
  auth = `Bearer ${TOKEN}`,
  origin: string | null = null,
) => {
  const headers: Record<string, string> = {};
  if (auth) headers.Authorization = auth;
  if (origin !== null) headers.Origin = origin;
  return new Request("http://fn.test/delete-account", { method, headers });
};

Deno.test("deletes files, then rows, then the auth user", async () => {
  const api = fakeApi({ files: [`${USER}/a.json.gz`, `${USER}/b.json.gz`] });
  const res = await handle(request(), ENV, api.fetchFn);
  assertEquals(res.status, 200);
  assertEquals(await res.json(), {
    deleted: { files: 2, games: 2, profiles: 1, auth_events: 4, account: 1 },
  });
  assertEquals(api.files(), []);
  assertEquals(
    api.calls.map((c) => `${c.method} ${c.path}`),
    [
      "GET /auth/v1/user",
      "POST /rest/v1/rpc/user_file_names",
      "DELETE /storage/v1/object/games",
      "POST /rest/v1/rpc/user_file_names",
      "POST /rest/v1/rpc/delete_user_data",
      `DELETE /auth/v1/admin/users/${USER}`,
      "POST /rest/v1/rpc/delete_user_auth_events",
    ],
  );
});

Deno.test("the user is taken from the token, never from the request", async () => {
  const api = fakeApi();
  await handle(request(), ENV, api.fetchFn);
  assertEquals(api.calls[0].headers.Authorization, `Bearer ${TOKEN}`);
  const rows = api.calls.find((c) => c.path === "/rest/v1/rpc/delete_user_data");
  assertEquals(rows?.body, { target: USER });
});

Deno.test("files outside the user's folder are never deleted", async () => {
  const other = "00000000-0000-4000-8000-00000000000b/x.json.gz";
  const api = fakeApi({ files: [`${USER}/a.json.gz`, other] });
  const res = await handle(request(), ENV, api.fetchFn);
  assertEquals(res.status, 200);
  assertEquals(api.files(), [other]);
});

Deno.test("only DELETE is accepted", async () => {
  const api = fakeApi();
  const res = await handle(request("POST"), ENV, api.fetchFn);
  assertEquals(res.status, 405);
  assertEquals(api.calls.length, 0);
});

Deno.test("no or malformed token is 401 with no call made", async () => {
  for (const auth of ["", "Basic abc", "Bearer a b"]) {
    const api = fakeApi();
    const res = await handle(request("DELETE", auth), ENV, api.fetchFn);
    assertEquals(res.status, 401, auth);
    assertEquals(api.calls.length, 0, auth);
  }
});

Deno.test("an expired or revoked token is 401 and nothing is deleted", async () => {
  const api = fakeApi({ files: [`${USER}/a.json.gz`], userStatus: 401 });
  const res = await handle(request(), ENV, api.fetchFn);
  assertEquals(res.status, 401);
  assertEquals(api.files(), [`${USER}/a.json.gz`]);
  assertEquals(api.calls.length, 1);
});

Deno.test("a failed file step stops before rows and account", async () => {
  const api = fakeApi({ files: [`${USER}/a.json.gz`], fail: "/storage/v1/object/games" });
  const res = await handle(request(), ENV, api.fetchFn);
  assertEquals(res.status, 500);
  assertEquals((await res.json()).step, "delete files");
  assertEquals(api.calls.some((c) => c.path.includes("delete_user_data")), false);
  assertEquals(api.calls.some((c) => c.path.includes("/admin/users/")), false);
});

Deno.test("a failed row step keeps the account, so a retry can finish", async () => {
  const api = fakeApi({ fail: "/rest/v1/rpc/delete_user_data" });
  const res = await handle(request(), ENV, api.fetchFn);
  assertEquals(res.status, 500);
  assertEquals((await res.json()).step, "delete rows");
  assertEquals(api.calls.some((c) => c.path.includes("/admin/users/")), false);
});

Deno.test("the error answer and log hold no token or id", async () => {
  const logged: string[] = [];
  const original = console.error;
  console.error = (...args: unknown[]) => logged.push(args.join(" "));
  try {
    const api = fakeApi({ fail: `/auth/v1/admin/users/${USER}` });
    const res = await handle(request(), ENV, api.fetchFn);
    const text = await res.text() + logged.join("\n");
    assertEquals(res.status, 500);
    const payload = TOKEN.split(".")[1];
    assertEquals(text.includes(payload) || /0000000a|sb_secret_test/.test(text), false, text);
  } finally {
    console.error = original;
  }
});

Deno.test("new-style secret keys are not sent as a bearer token", async () => {
  const api = fakeApi();
  await handle(request(), { ...ENV, serviceKey: "sb_secret_test" }, api.fetchFn);
  const rows = api.calls.find((c) => c.path === "/rest/v1/rpc/delete_user_data");
  assertEquals(rows?.headers.Authorization, undefined);
  assertEquals(rows?.headers.apikey, "sb_secret_test");
});

Deno.test("calling again after a failed row step finishes the deletion", async () => {
  const api = fakeApi({ files: [`${USER}/a.json.gz`], fail: "/rest/v1/rpc/delete_user_data" });
  assertEquals((await handle(request(), ENV, api.fetchFn)).status, 500);
  api.heal();
  const res = await handle(request(), ENV, api.fetchFn);
  assertEquals(res.status, 200);
  assertEquals(api.files(), []);
  assertEquals(api.calls.at(-1)?.path, "/rest/v1/rpc/delete_user_auth_events");
});

Deno.test("files that will not go away stop the deletion before the rows", async () => {
  const api = fakeApi({ files: [`${USER}/a.json.gz`], stickyFiles: true });
  const res = await handle(request(), ENV, api.fetchFn);
  assertEquals(res.status, 500);
  assertEquals((await res.json()).step, "delete files");
  assertEquals(api.calls.some((c) => c.path.includes("delete_user_data")), false);
});

Deno.test("more than 1000 files are deleted in batches", async () => {
  const many = Array.from({ length: 1500 }, (_, i) => `${USER}/g${i}.json.gz`);
  const api = fakeApi({ files: many });
  const res = await handle(request(), ENV, api.fetchFn);
  assertEquals(res.status, 200);
  const batches = api.calls.filter((c) => c.path === "/storage/v1/object/games");
  assertEquals(batches.map((c) => (c.body as { prefixes: string[] }).prefixes.length), [1000, 500]);
});

Deno.test("a network error is a 500 with the step, not a crash", async () => {
  const failing: Fetch = () => Promise.reject(new TypeError("network down"));
  const res = await handle(request(), ENV, failing);
  assertEquals(res.status, 500);
  assertEquals((await res.json()).step, "auth");
});

// --- CORS: only the site's own origin (T-104c) ---

Deno.test("the site's origin gets the CORS headers on the answer", async () => {
  const api = fakeApi();
  const res = await handle(request("DELETE", undefined, SITE), ENV, api.fetchFn);
  assertEquals(res.status, 200);
  assertEquals(res.headers.get("Access-Control-Allow-Origin"), SITE);
  assertEquals(res.headers.get("Vary"), "Origin");
});

Deno.test("a preflight from the site's origin is allowed without a token", async () => {
  const api = fakeApi();
  const res = await handle(request("OPTIONS", "", SITE), ENV, api.fetchFn);
  assertEquals(res.status, 204);
  assertEquals(res.headers.get("Access-Control-Allow-Origin"), SITE);
  assertEquals(res.headers.get("Access-Control-Allow-Methods"), "DELETE, OPTIONS");
  const allowed = (res.headers.get("Access-Control-Allow-Headers") ?? "").split(", ");
  for (const h of ["authorization", "apikey", "content-type", "x-client-info"]) {
    assertEquals(allowed.includes(h), true, h);
  }
  assertEquals(api.calls.length, 0);
});

Deno.test("another origin is refused before any call, preflight or not", async () => {
  for (const origin of ["https://evil.test", "https://site.test.evil.test", "null", ""]) {
    for (const method of ["OPTIONS", "DELETE"]) {
      const api = fakeApi({ files: [`${USER}/a.json.gz`] });
      const res = await handle(request(method, undefined, origin), ENV, api.fetchFn);
      assertEquals(res.status, 403, `${method} ${origin}`);
      assertEquals(res.headers.get("Access-Control-Allow-Origin"), null, origin);
      assertEquals(api.calls.length, 0, origin);
      assertEquals(api.files(), [`${USER}/a.json.gz`]);
    }
  }
});

Deno.test("with no site configured, every browser origin is refused", async () => {
  const api = fakeApi();
  const res = await handle(
    request("DELETE", undefined, SITE),
    { ...ENV, allowedOrigins: [] },
    api.fetchFn,
  );
  assertEquals(res.status, 403);
  assertEquals(api.calls.length, 0);
});

Deno.test("SITE_ORIGINS keeps exact origins only", () => {
  assertEquals(parseOrigins(undefined), []);
  assertEquals(parseOrigins(""), []);
  assertEquals(
    parseOrigins(
      " https://site.test , http://127.0.0.1:3000,https://x.test/path,*,null,ftp://a.test",
    ),
    ["https://site.test", "http://127.0.0.1:3000"],
  );
});

// --- Recent sign-in (T-104c) ---

Deno.test("a sign-in older than the limit is refused and nothing is deleted", async () => {
  const stale = new Date(NOW - MAX_SIGN_IN_AGE_MS - 1000).toISOString();
  const api = fakeApi({ files: [`${USER}/a.json.gz`], lastSignIn: stale });
  const res = await handle(request(), ENV, api.fetchFn);
  assertEquals(res.status, 403);
  assertEquals((await res.json()).reason, "reauthenticate");
  assertEquals(api.files(), [`${USER}/a.json.gz`]);
  assertEquals(api.calls.map((c) => c.path), ["/auth/v1/user"]);
});

Deno.test("a sign-in just inside the limit is accepted", async () => {
  const fresh = new Date(NOW - MAX_SIGN_IN_AGE_MS + 1000).toISOString();
  const api = fakeApi({ lastSignIn: fresh });
  assertEquals((await handle(request(), ENV, api.fetchFn)).status, 200);
});

Deno.test("the limit is ten minutes", () => {
  assertEquals(MAX_SIGN_IN_AGE_MS, 10 * 60 * 1000);
});

Deno.test("a missing or unreadable sign-in time is refused", async () => {
  for (const lastSignIn of [undefined, null, "", "yesterday", 12345]) {
    const api = fakeApi({ lastSignIn });
    const res = await handle(request(), ENV, api.fetchFn);
    assertEquals(res.status, 403, String(lastSignIn));
    assertEquals(api.calls.length, 1, String(lastSignIn));
  }
});

// --- This session's own sign-in time (security review M2) ---

Deno.test("an old session is refused even if the account signed in recently elsewhere", async () => {
  const api = fakeApi({ files: [`${USER}/a.json.gz`] });
  const old = amrAt(MAX_SIGN_IN_AGE_MS + 1000);
  const res = await handle(request("DELETE", `Bearer ${old}`), ENV, api.fetchFn);
  assertEquals(res.status, 403);
  assertEquals((await res.json()).reason, "reauthenticate");
  assertEquals(api.files(), [`${USER}/a.json.gz`]);
});

Deno.test("the newest of the session's sign-in methods counts", async () => {
  const token = amrAt(MAX_SIGN_IN_AGE_MS * 3, 30_000);
  const api = fakeApi();
  assertEquals((await handle(request("DELETE", `Bearer ${token}`), ENV, api.fetchFn)).status, 200);
});

Deno.test("a token with no readable sign-in time is refused", async () => {
  const tokens = [
    fakeToken({ sub: USER }),
    fakeToken({ sub: USER, amr: [] }),
    fakeToken({ sub: USER, amr: [{ method: "password", timestamp: "now" }] }),
    fakeToken({ sub: USER, amr: "password" }),
    "not.a.jwt",
    "opaque-token",
  ];
  for (const token of tokens) {
    const api = fakeApi();
    const res = await handle(request("DELETE", `Bearer ${token}`), ENV, api.fetchFn);
    assertEquals(res.status, 403, token);
    assertEquals(api.calls.length, 1, token);
  }
});

Deno.test("sign-in times far in the future are refused", async () => {
  const ahead = new Date(NOW + 5 * 60_000).toISOString();
  const api = fakeApi({ lastSignIn: ahead });
  assertEquals((await handle(request(), ENV, api.fetchFn)).status, 403);
  const future = amrAt(-5 * 60_000);
  const api2 = fakeApi();
  assertEquals(
    (await handle(request("DELETE", `Bearer ${future}`), ENV, api2.fetchFn)).status,
    403,
  );
});

Deno.test("a small clock difference is tolerated", async () => {
  const ahead = new Date(NOW + 30_000).toISOString();
  const api = fakeApi({ lastSignIn: ahead });
  const token = amrAt(-30_000);
  assertEquals((await handle(request("DELETE", `Bearer ${token}`), ENV, api.fetchFn)).status, 200);
});

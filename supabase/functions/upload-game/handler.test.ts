// Run: deno test supabase/functions/upload-game/
// Fake HTTP APIs; the end-to-end test against the local stack is
// tests/test_supabase_local.py (and the desktop app's own, T-104d).
import { type Env, type Fetch, handle, MAX_BODY, MAX_INFLATED } from "./handler.ts";
import { realDuos, realRecord, record, SESSION } from "./testdata.ts";
import { VALIDATOR_VERSION } from "./validate.ts";

type Bytes = Uint8Array<ArrayBuffer>;

const ENV: Env = { url: "http://api.test", serviceKey: "service-key-for-tests" };
const USER = "00000000-0000-4000-8000-00000000000a";
const TOKEN = "made-up-access-token-for-tests";
const PATH = `/upload-game/v1/games/${SESSION}/1`;
const FILE = `/storage/v1/object/games/${USER}/${SESSION}-1.json.gz`;

function assertEquals(actual: unknown, expected: unknown, msg = ""): void {
  const a = JSON.stringify(actual);
  const e = JSON.stringify(expected);
  if (a !== e) throw new Error(`${msg}\n  actual:   ${a}\n  expected: ${e}`);
}

async function gzip(bytes: Bytes): Promise<Bytes> {
  const stream = new Blob([bytes]).stream().pipeThrough(new CompressionStream("gzip"));
  return new Uint8Array(await new Response(stream).arrayBuffer());
}

async function gunzip(bytes: Bytes): Promise<Bytes> {
  const stream = new Blob([bytes]).stream().pipeThrough(new DecompressionStream("gzip"));
  return new Uint8Array(await new Response(stream).arrayBuffer());
}

async function sha256(bytes: Bytes): Promise<string> {
  const d = new Uint8Array(await crypto.subtle.digest("SHA-256", bytes));
  return Array.from(d, (b) => b.toString(16).padStart(2, "0")).join("");
}

const enc = (v: unknown): Bytes => new TextEncoder().encode(JSON.stringify(v)) as Bytes;

interface Call {
  method: string;
  path: string;
  body: unknown;
  raw: Bytes | null;
  headers: Record<string, string>;
}

/** Fake project APIs. `begin` and `commit` are what the RPCs answer. */
function fakeApi(opts: {
  begin?: unknown;
  commit?: unknown;
  user?: number;
  blocked?: boolean;
  fail?: string;
} = {}) {
  const calls: Call[] = [];
  const reply = (status: number, body: unknown) =>
    Promise.resolve(new Response(JSON.stringify(body), { status }));
  const fetchFn: Fetch = async (input, init = {}) => {
    const path = input.replace(ENV.url, "");
    const method = init.method ?? "GET";
    const headers = init.headers as Record<string, string>;
    let body: unknown = null;
    let raw: Bytes | null = null;
    if (init.body instanceof Uint8Array) raw = init.body as Bytes;
    else if (typeof init.body === "string") body = JSON.parse(init.body);
    calls.push({ method, path, body, raw, headers });
    if (opts.fail === path) return reply(503, {});
    switch (path) {
      case "/rest/v1/rpc/upload_ip_blocked":
        return reply(200, opts.blocked ?? false);
      case "/rest/v1/rpc/upload_ip_failed":
        return new Response(null, { status: 204 });
      case "/auth/v1/user":
        if (opts.user) return reply(opts.user, {});
        return reply(200, { id: USER });
      case "/rest/v1/rpc/upload_begin":
        return reply(200, opts.begin ?? { result: "create" });
      case "/rest/v1/rpc/upload_commit":
        return reply(200, opts.commit ?? "created");
      case FILE:
        return reply(200, { Key: `games/${USER}/${SESSION}-1.json.gz` });
    }
    await Promise.resolve();
    return reply(404, {});
  };
  const paths = () => calls.map((c) => `${c.method} ${c.path}`);
  return { fetchFn, calls, paths };
}

function request(
  body: BodyInit | null,
  opts: { method?: string; path?: string; auth?: string; type?: string; headers?: HeadersInit } =
    {},
) {
  const headers = new Headers(opts.headers);
  if (opts.auth !== "") headers.set("Authorization", opts.auth ?? `Bearer ${TOKEN}`);
  headers.set("Content-Type", opts.type ?? "application/gzip");
  headers.set("X-Forwarded-For", "203.0.113.7, 10.0.0.1");
  return new Request(`http://fn.test${opts.path ?? PATH}`, {
    method: opts.method ?? "PUT",
    headers,
    body,
  });
}

const upload = async (rec: unknown = record()) => request(await gzip(enc(rec)));

Deno.test("a new game: checks, then the file, then the row", async () => {
  const api = fakeApi();
  const raw = enc(record());
  const res = await handle(request(await gzip(raw)), ENV, api.fetchFn);
  assertEquals(res.status, 201);
  const sha = await sha256(raw);
  assertEquals(await res.json(), { result: "created", sha256: sha, validator: VALIDATOR_VERSION });
  assertEquals(api.paths(), [
    "POST /rest/v1/rpc/upload_ip_blocked",
    "GET /auth/v1/user",
    "POST /rest/v1/rpc/upload_begin",
    `POST ${FILE}`,
    "POST /rest/v1/rpc/upload_commit",
  ]);
  const begin = api.calls[2].body as Record<string, unknown>;
  assertEquals(begin.target, USER);
  assertEquals(begin.p_sha256, sha);
  const file = api.calls[3];
  assertEquals(file.headers["x-upsert"], "true");
  assertEquals(file.headers["Content-Type"], "application/gzip");
  assertEquals(await gunzip(file.raw!), raw, "the stored file inflates to the checked bytes");
  assertEquals(begin.p_size, file.raw!.length, "the quota counts the stored size");
  const commit = api.calls[4].body as Record<string, unknown>;
  assertEquals(commit, {
    target: USER,
    p_session: SESSION,
    p_game_index: 1,
    p_game_type: "GT_BATTLEGROUNDS_DUO",
    p_status: "ok",
    p_hero_card_id: record().report.hero,
    p_final_place: 1,
    p_build: record().report.build,
    p_played_on: "2026-10-09",
    p_tribes_offered: { BEAST: 4, RACE_99: 1 },
    p_saved_at: 1791000000,
    p_sha256: sha,
    p_size: file.raw!.length,
    p_parser_version: "0.1.0+r1",
  });
});

Deno.test("the file stored is re-compressed by the server, not the bytes sent", async () => {
  // A gzip header may carry a file name; a forged one could hide a BattleTag there.
  const body = await gzip(enc(record()));
  const name = new TextEncoder().encode("Someone#1234\0");
  const forged = new Uint8Array(body.length + name.length);
  forged.set(body.subarray(0, 10));
  forged[3] |= 0x08; // FNAME
  forged.set(name, 10);
  forged.set(body.subarray(10), 10 + name.length);
  const api = fakeApi();
  const res = await handle(request(forged), ENV, api.fetchFn);
  assertEquals(res.status, 201);
  const stored = api.calls.find((c) => c.path === FILE)!.raw!;
  assertEquals(new TextDecoder().decode(stored).includes("Someone"), false);
});

Deno.test("only compact JSON is stored: duplicate keys or padding are refused", async () => {
  const compact = JSON.stringify(record());
  const hidden = compact.replace('"saved_at":', '"saved_at":"Someone 1234 hidden","saved_at":');
  for (const text of [hidden, ` ${compact}`, `${compact}\n`, JSON.stringify(record(), null, 2)]) {
    const api = fakeApi();
    const res = await handle(
      request(await gzip(new TextEncoder().encode(text) as Bytes)),
      ENV,
      api.fetchFn,
    );
    assertEquals([res.status, (await res.json()).code], [400, "invalid_json"], text.slice(0, 40));
    assertEquals(api.paths().some((p) => p.includes("upload_begin")), false);
  }
});

Deno.test("a valid token is served even when its IP is blocked", async () => {
  const api = fakeApi({ blocked: true });
  const res = await handle(await upload(), ENV, api.fetchFn);
  assertEquals(res.status, 201);
});

Deno.test("a replaced game is 200", async () => {
  const api = fakeApi({ begin: { result: "replace" }, commit: "replaced" });
  const res = await handle(await upload(), ENV, api.fetchFn);
  assertEquals(res.status, 200);
  assertEquals((await res.json()).result, "replaced");
});

Deno.test("the same bytes again are 200 and nothing is written", async () => {
  const api = fakeApi({ begin: { result: "unchanged" } });
  const res = await handle(await upload(), ENV, api.fetchFn);
  assertEquals(res.status, 200);
  assertEquals((await res.json()).result, "unchanged");
  assertEquals(api.paths().some((p) => p.includes("/storage/") || p.includes("commit")), false);
});

Deno.test("an older revision is 409 and nothing is written", async () => {
  const api = fakeApi({ begin: { result: "older" } });
  const res = await handle(await upload(), ENV, api.fetchFn);
  assertEquals(res.status, 409);
  assertEquals((await res.json()).code, "older_revision");
  assertEquals(api.paths().some((p) => p.includes("/storage/")), false);
});

Deno.test("a newer row landing between file and row is 409", async () => {
  const api = fakeApi({ commit: "older" });
  const res = await handle(await upload(), ENV, api.fetchFn);
  assertEquals(res.status, 409);
});

Deno.test("too many uploads is 429 with Retry-After", async () => {
  const api = fakeApi({ begin: { result: "rate_limited", retry_after: 1800 } });
  const res = await handle(await upload(), ENV, api.fetchFn);
  assertEquals(res.status, 429);
  assertEquals(res.headers.get("Retry-After"), "1800");
  assertEquals((await res.json()).code, "rate_limited");
  assertEquals(api.paths().some((p) => p.includes("/storage/")), false);
});

Deno.test("a full quota is 403 with its code", async () => {
  for (const result of ["quota_games", "quota_bytes"]) {
    const api = fakeApi({ begin: { result } });
    const res = await handle(await upload(), ENV, api.fetchFn);
    assertEquals(res.status, 403, result);
    assertEquals((await res.json()).code, result);
  }
});

Deno.test("a forged upload with a BattleTag is refused before any write", async () => {
  const rec = record();
  rec.report.card_names[rec.report.hero] = "Someone#1234";
  const api = fakeApi();
  const res = await handle(await upload(rec), ENV, api.fetchFn);
  assertEquals(res.status, 400);
  const body = await res.json();
  assertEquals(body.code, "player_name");
  assertEquals(JSON.stringify(body).includes("Someone"), false, "the answer does not echo it");
  assertEquals(api.paths().some((p) => p.includes("upload_begin")), false);
});

Deno.test("a forged upload with extra fields is refused before any write", async () => {
  const rec = record();
  rec.report.lobby[0].battle_tag = "Someone";
  const api = fakeApi();
  const res = await handle(await upload(rec), ENV, api.fetchFn);
  assertEquals(res.status, 400);
  assertEquals(await res.json(), {
    error: "the game record is not valid",
    code: "invalid_report",
    field: "report.lobby[0].battle_tag",
    validator: VALIDATOR_VERSION,
  });
  assertEquals(api.paths().some((p) => p.includes("upload_begin")), false);
});

Deno.test("a real game from the current parser is stored", async () => {
  const api = fakeApi();
  const res = await handle(await upload(realRecord(realDuos[0])), ENV, api.fetchFn);
  assertEquals(res.status, 201);
});

Deno.test("every answer says which validator version the server runs", async () => {
  const cases: Array<[string, Request | Promise<Request>, Parameters<typeof fakeApi>[0]]> = [
    ["stored", upload(), {}],
    ["unchanged", upload(), { begin: { result: "unchanged" } }],
    ["refused record", upload(record({ mmr: 1 })), {}],
    ["older revision", upload(), { begin: { result: "older" } }],
    ["no sign-in", upload(), { user: 401 }],
    ["not gzip", request(enc(record())), {}],
    ["server error", upload(), { fail: "/rest/v1/rpc/upload_begin" }],
  ];
  for (const [what, req, opts] of cases) {
    const res = await handle(await req, ENV, fakeApi(opts).fetchFn);
    assertEquals((await res.json()).validator, VALIDATOR_VERSION, what);
  }
});

Deno.test("a body over 64 KiB is 413, with or without Content-Length", async () => {
  const big = new Uint8Array(MAX_BODY + 1);
  const api = fakeApi();
  assertEquals((await handle(request(big), ENV, api.fetchFn)).status, 413);
  assertEquals(api.calls.length, 0, "refused on Content-Length before any call");
  const stream = new Blob([big]).stream();
  const res = await handle(request(stream), ENV, fakeApi().fetchFn);
  assertEquals(res.status, 413);
});

Deno.test("a gzip bomb stops at 512 KiB", async () => {
  const bomb = await gzip(new Uint8Array(20 * MAX_INFLATED));
  assertEquals(bomb.length < MAX_BODY, true, "setup: the bomb fits in the body limit");
  const api = fakeApi();
  const res = await handle(request(bomb), ENV, api.fetchFn);
  assertEquals(res.status, 413);
  assertEquals((await res.json()).code, "too_large");
});

Deno.test("not gzip, not UTF-8 or not JSON is 400", async () => {
  const cases: Array<[Bytes, string]> = [
    [enc(record()), "bad_gzip"],
    [await gzip(new Uint8Array([0xff, 0xfe, 0x00])), "invalid_json"],
    [await gzip(new TextEncoder().encode("{not json") as Bytes), "invalid_json"],
  ];
  for (const [body, code] of cases) {
    const res = await handle(request(body), ENV, fakeApi().fetchFn);
    assertEquals([res.status, (await res.json()).code], [400, code]);
  }
});

Deno.test("only PUT of application/gzip to a game address", async () => {
  const body = await gzip(enc(record()));
  const cases: Array<[Request, number]> = [
    [request(null, { method: "GET" }), 405],
    [request(body, { method: "POST" }), 405],
    [request(body, { type: "application/json" }), 415],
    [request(body, { path: "/upload-game/v1/games/../1" }), 404],
    [request(body, { path: `/upload-game/v1/games/${SESSION}/1001` }), 404],
  ];
  for (const [req, status] of cases) {
    const api = fakeApi();
    const res = await handle(req, ENV, api.fetchFn);
    assertEquals(res.status, status, `${req.method} ${req.url}`);
    assertEquals(api.calls.length, 0);
  }
});

Deno.test("browsers are refused: the desktop app sends no Origin", async () => {
  const api = fakeApi();
  const req = request(await gzip(enc(record())), { headers: { Origin: "https://evil.test" } });
  const res = await handle(req, ENV, api.fetchFn);
  assertEquals(res.status, 403);
  assertEquals(res.headers.get("Access-Control-Allow-Origin"), null);
  assertEquals(api.calls.length, 0);
});

Deno.test("no or a bad token is 401 and counts against the IP", async () => {
  for (const [auth, user] of [["", 0], ["Basic abc", 0], [`Bearer ${TOKEN}`, 401]] as const) {
    const api = fakeApi({ user });
    const res = await handle(request(await gzip(enc(record())), { auth }), ENV, api.fetchFn);
    assertEquals(res.status, 401, auth);
    assertEquals(api.paths().at(-1), "POST /rest/v1/rpc/upload_ip_failed", auth);
    assertEquals(api.paths().some((p) => p.includes("upload_begin")), false);
  }
});

Deno.test("only a hash of the client's IP leaves the function", async () => {
  const api = fakeApi({ user: 401 });
  await handle(await upload(), ENV, api.fetchFn);
  const key = await sha256(new TextEncoder().encode("203.0.113.7") as Bytes);
  for (const c of api.calls.filter((c) => c.path.includes("upload_ip"))) {
    assertEquals(c.body, { p_ip_key: key });
  }
  assertEquals(JSON.stringify(api.calls).includes("203.0.113.7"), false);
});

Deno.test("a blocked IP without a valid token is 429 and costs no token check", async () => {
  const api = fakeApi({ blocked: true });
  const res = await handle(request(await gzip(enc(record())), { auth: "" }), ENV, api.fetchFn);
  assertEquals(res.status, 429);
  assertEquals(res.headers.get("Retry-After"), "600");
  assertEquals(api.paths(), ["POST /rest/v1/rpc/upload_ip_blocked"]);
  const bad = fakeApi({ blocked: true, user: 401 });
  assertEquals((await handle(await upload(), ENV, bad.fetchFn)).status, 429);
  assertEquals(bad.paths().includes("POST /rest/v1/rpc/upload_ip_failed"), false);
});

Deno.test("the platform's client IP header wins over X-Forwarded-For", async () => {
  const api = fakeApi({ user: 401 });
  const req = request(await gzip(enc(record())), {
    headers: { "CF-Connecting-IP": "198.51.100.9" },
  });
  await handle(req, ENV, api.fetchFn);
  const key = await sha256(new TextEncoder().encode("198.51.100.9") as Bytes);
  assertEquals(api.calls[0].body, { p_ip_key: key });
});

Deno.test("the user is taken from the token, never from the request", async () => {
  const api = fakeApi();
  const rec = record();
  rec.user_id = "00000000-0000-4000-8000-00000000000b";
  const res = await handle(await upload(rec), ENV, api.fetchFn);
  assertEquals(res.status, 400, "a user id in the body is an unknown field");
  const ok = fakeApi();
  await handle(await upload(), ENV, ok.fetchFn);
  assertEquals(ok.calls[1].headers.Authorization, `Bearer ${TOKEN}`);
});

Deno.test("a failed file write stops before the row; the answer is 500", async () => {
  const api = fakeApi({ fail: FILE });
  const res = await handle(await upload(), ENV, api.fetchFn);
  assertEquals(res.status, 500);
  assertEquals((await res.json()).step, "write file");
  assertEquals(api.paths().some((p) => p.includes("upload_commit")), false);
});

Deno.test("a network error is 500 with the step, and holds no token or user id", async () => {
  const logged: string[] = [];
  const original = console.error;
  console.error = (...args: unknown[]) => logged.push(args.join(" "));
  try {
    const failing: Fetch = () => Promise.reject(new TypeError("network down"));
    const res = await handle(await upload(), ENV, failing);
    const body = await res.text();
    const text = body + "\n" + logged.join("\n");
    assertEquals(res.status, 500);
    assertEquals(JSON.parse(body).step, "ip check");
    assertEquals(
      text.includes(TOKEN) || text.includes(USER) || text.includes("service-key"),
      false,
    );
  } finally {
    console.error = original;
  }
});

Deno.test("new-style secret keys are not sent as a bearer token", async () => {
  const api = fakeApi();
  await handle(await upload(), { ...ENV, serviceKey: "sb_secret_test" }, api.fetchFn);
  const begin = api.calls.find((c) => c.path === "/rest/v1/rpc/upload_begin")!;
  assertEquals(begin.headers.Authorization, undefined);
  assertEquals(begin.headers.apikey, "sb_secret_test");
});

Deno.test("with a new-style secret key no service call carries a bearer token", async () => {
  const api = fakeApi();
  const res = await handle(await upload(), { ...ENV, serviceKey: "sb_secret_test" }, api.fetchFn);
  assertEquals(res.status, 201);
  for (const c of api.calls) {
    assertEquals(c.headers.apikey, "sb_secret_test", c.path);
    // Only the user lookup sends a bearer, and it is the user's own token.
    const bearer = c.path === "/auth/v1/user" ? `Bearer ${TOKEN}` : undefined;
    assertEquals(c.headers.Authorization, bearer, c.path);
  }
});

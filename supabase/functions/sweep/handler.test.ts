// Run: deno test supabase/functions/sweep/
// Fake HTTP APIs; tests/test_supabase_local.py runs it against the local stack.
import { type Env, type Fetch, handle } from "./handler.ts";
import { summarize, SUMMARY_VERSION } from "../_shared/game_summary.ts";
import { type Json, realDuos, realRecord, SESSION } from "../upload-game/testdata.ts";

const ENV: Env = { url: "http://api.test", serviceKey: "sb_secret_test" };
const A = "00000000-0000-4000-8000-00000000000a";
const orphan = (i: number) => `${A}/Hearthstone_2026_10_09_18_30_00-${i}.json.gz`;

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

const TOKEN = "ab".repeat(32);
const RUN = 7;

const NO_SKIPS = {
  no_file: 0,
  bad_file: 0,
  file_mismatch: 0,
  write_failed: 0,
  missing: 0,
  changed: 0,
  newer: 0,
};
/** The backfill's counts; skips by reason, zero unless given. */
const counts = (
  listed: number,
  looked_at: number,
  filled: number,
  left: number,
  skips: Partial<typeof NO_SKIPS> = {},
) => ({ listed, looked_at, filled, left, skipped: { ...NO_SKIPS, ...skips } });

type Bytes = Uint8Array<ArrayBuffer>;

interface Missing {
  user_id: string;
  session: string;
  game_index: number;
  storage_path: string;
  content_sha256: string;
}

function fakeApi(
  opts: {
    files?: string[];
    fail?: string;
    sticky?: boolean;
    token?: string;
    missing?: Missing[];
    objects?: Record<string, Bytes>;
    put?: string;
  } = {},
) {
  let files = [...(opts.files ?? [])];
  const calls: Call[] = [];
  const reply = (status: number, body: unknown) =>
    Promise.resolve(new Response(JSON.stringify(body), { status }));
  const fetchFn: Fetch = (input, init = {}) => {
    const path = input.replace(ENV.url, "");
    const method = init.method ?? "GET";
    const body = init.body ? JSON.parse(init.body as string) : null;
    const headers = (init.headers ?? {}) as Record<string, string>;
    calls.push({ method, path, body, headers });
    if (opts.fail === path) return reply(503, {});
    if (path === "/rest/v1/rpc/sweep_orphan_files") return reply(200, files.slice(0, 1000));
    if (path === "/rest/v1/rpc/sweep_token_valid") {
      return reply(200, body.p_token === (opts.token ?? TOKEN));
    }
    if (path === "/storage/v1/object/games" && method === "DELETE") {
      if (!opts.sticky) files = files.filter((f) => !body.prefixes.includes(f));
      return reply(200, []);
    }
    if (path === "/rest/v1/rpc/sweep_housekeeping") {
      return reply(200, { auth_events: 1, upload_events: 2, ip_failures: 3, run_id: RUN });
    }
    if (path === "/rest/v1/rpc/sweep_record_result") return reply(200, null);
    if (path === "/rest/v1/rpc/sweep_record_failure") return reply(200, RUN);
    if (path === "/rest/v1/rpc/game_stats_backfill_list") return reply(200, opts.missing ?? []);
    if (path === "/rest/v1/rpc/game_stats_backfill_put") return reply(200, opts.put ?? "stored");
    const prefix = "/storage/v1/object/authenticated/games/";
    if (method === "GET" && path.startsWith(prefix)) {
      const bytes = opts.objects?.[path.slice(prefix.length)];
      return Promise.resolve(
        bytes ? new Response(bytes, { status: 200 }) : new Response("{}", { status: 400 }),
      );
    }
    return reply(404, {});
  };
  return { fetchFn, calls, files: () => files };
}

const request = (key: string | null = ENV.serviceKey, method = "POST") => {
  const headers: Record<string, string> = {};
  if (key !== null) headers.Authorization = `Bearer ${key}`;
  return new Request("http://fn.test/sweep", { method, headers });
};

Deno.test("deletes orphan files through the Storage API, then housekeeping", async () => {
  const api = fakeApi({ files: [orphan(1), orphan(2)] });
  const res = await handle(request(), ENV, api.fetchFn);
  assertEquals(res.status, 200);
  assertEquals(await res.json(), {
    run_id: RUN,
    files: 2,
    auth_events: 1,
    upload_events: 2,
    ip_failures: 3,
    stats: counts(0, 0, 0, 0),
  });
  assertEquals(api.files(), []);
  assertEquals(api.calls.map((c) => `${c.method} ${c.path}`), [
    "POST /rest/v1/rpc/sweep_orphan_files",
    "DELETE /storage/v1/object/games",
    "POST /rest/v1/rpc/sweep_orphan_files",
    "POST /rest/v1/rpc/sweep_housekeeping",
    "POST /rest/v1/rpc/game_stats_backfill_list",
    "POST /rest/v1/rpc/sweep_record_result",
  ]);
  assertEquals(api.calls.at(-1)?.body, { p_run_id: RUN, p_files: 2, p_stats: counts(0, 0, 0, 0) });
  assertEquals(api.calls[0].body, { min_age: "1 hour" });
});

Deno.test("only the service role may run it", async () => {
  for (const key of [null, "", "anon-key", `${ENV.serviceKey}x`]) {
    const api = fakeApi({ files: [orphan(1)] });
    const res = await handle(request(key), ENV, api.fetchFn);
    assertEquals(res.status, 401, String(key));
    assertEquals(api.calls.length, 0);
  }
  const api = fakeApi();
  const res = await handle(request(ENV.serviceKey, "GET"), ENV, api.fetchFn);
  assertEquals(res.status, 405);
});

Deno.test("a new-style secret key in apikey is accepted", async () => {
  const env = { ...ENV, serviceKey: "sb_secret_test" };
  const req = new Request("http://fn.test/sweep", {
    method: "POST",
    headers: { apikey: "sb_secret_test" },
  });
  assertEquals((await handle(req, env, fakeApi().fetchFn)).status, 200);
});

Deno.test("service calls send the secret key in apikey only, never as a bearer", async () => {
  const api = fakeApi({ files: [orphan(1)] });
  assertEquals((await handle(request(), ENV, api.fetchFn)).status, 200);
  for (const c of api.calls) {
    assertEquals(c.headers.apikey, ENV.serviceKey, c.path);
    assertEquals(c.headers.Authorization, undefined, c.path);
  }
});

Deno.test("names that are not game files are never deleted", async () => {
  const odd = [`${A}/../x.json.gz`, "x/y.json.gz", `${A}/Hearthstone_2026_10_09_18_30_00-1.txt`];
  const api = fakeApi({ files: [orphan(1), ...odd], sticky: false });
  const res = await handle(request(), ENV, api.fetchFn);
  assertEquals(res.status, 200);
  const deleted = api.calls.filter((c) => c.method === "DELETE")
    .flatMap((c) => (c.body as { prefixes: string[] }).prefixes);
  assertEquals(deleted, [orphan(1)]);
});

Deno.test("more than 1000 orphans are deleted in rounds", async () => {
  const many = Array.from({ length: 1500 }, (_, i) => orphan(i + 1));
  const api = fakeApi({ files: many });
  const res = await handle(request(), ENV, api.fetchFn);
  assertEquals((await res.json()).files, 1500);
  const batches = api.calls.filter((c) => c.method === "DELETE")
    .map((c) => (c.body as { prefixes: string[] }).prefixes.length);
  assertEquals(batches, [1000, 500]);
});

Deno.test("files that do not go away stop after a few rounds", async () => {
  const api = fakeApi({ files: [orphan(1)], sticky: true });
  const res = await handle(request(), ENV, api.fetchFn);
  assertEquals(res.status, 500);
  assertEquals((await res.json()).step, "delete files");
});

Deno.test("a failed step is a 500 naming it", async () => {
  const api = fakeApi({ files: [orphan(1)], fail: "/storage/v1/object/games" });
  const res = await handle(request(), ENV, api.fetchFn);
  assertEquals(res.status, 500);
  assertEquals((await res.json()).step, "delete files");
  const failing: Fetch = () => Promise.reject(new TypeError("network down"));
  const res2 = await handle(request(), ENV, failing);
  assertEquals([res2.status, (await res2.json()).step], [500, "list files"]);
});

const tokenRequest = (token: string, extra: Record<string, string> = {}) =>
  new Request("http://fn.test/sweep", {
    method: "POST",
    headers: { "X-Sweep-Token": token, ...extra },
  });

Deno.test("the scheduled job's sweep token is checked by the database, then runs", async () => {
  const api = fakeApi({ files: [orphan(1)] });
  const res = await handle(tokenRequest(TOKEN), ENV, api.fetchFn);
  assertEquals(res.status, 200);
  assertEquals(api.calls[0].path, "/rest/v1/rpc/sweep_token_valid");
  assertEquals(api.calls[0].body, { p_token: TOKEN });
  assertEquals(api.calls.at(-1)?.path, "/rest/v1/rpc/sweep_record_result");
});

Deno.test("a wrong or malformed sweep token runs nothing", async () => {
  // Wrong token: asked once, refused. Malformed: refused without asking.
  for (
    const [token, asked] of [["cd".repeat(32), 1], ["short", 0], [`${TOKEN}0`, 0], ["", 0]] as const
  ) {
    const api = fakeApi({ files: [orphan(1)] });
    const res = await handle(tokenRequest(token), ENV, api.fetchFn);
    assertEquals(res.status, 401, token);
    assertEquals(api.calls.length, asked, token);
  }
});

Deno.test("a sweep token that cannot be checked is a plain 503, not a run", async () => {
  const api = fakeApi({ files: [orphan(1)], fail: "/rest/v1/rpc/sweep_token_valid" });
  const res = await handle(tokenRequest(TOKEN), ENV, api.fetchFn);
  assertEquals(res.status, 503);
  assertEquals(await res.json(), { error: "try again later" });
  assertEquals(api.calls.length, 1);
});

Deno.test("a sweep token with the wrong method is refused after the check", async () => {
  const api = fakeApi();
  const req = new Request("http://fn.test/sweep", {
    method: "GET",
    headers: { "X-Sweep-Token": TOKEN },
  });
  assertEquals((await handle(req, ENV, api.fetchFn)).status, 405);
});

// ------------------------------------------------------------------ summaries (T-502)

async function gzip(bytes: Bytes): Promise<Bytes> {
  const stream = new Blob([bytes]).stream().pipeThrough(new CompressionStream("gzip"));
  return new Uint8Array(await new Response(stream).arrayBuffer());
}

async function sha256(bytes: Bytes): Promise<string> {
  const d = new Uint8Array(await crypto.subtle.digest("SHA-256", bytes));
  return Array.from(d, (b) => b.toString(16).padStart(2, "0")).join("");
}

const enc = (v: unknown): Bytes => new TextEncoder().encode(JSON.stringify(v)) as Bytes;

/** A stored game file and the row the backfill list gives for it. */
async function stored(index: number, record: Json = realRecord(realDuos[0])) {
  const rec = { ...record, index };
  if (rec.report?.status) rec.report = { ...rec.report, index };
  const bytes = enc(rec);
  const name = `${A}/${SESSION}-${index}.json.gz`;
  const row: Missing = {
    user_id: A,
    session: SESSION,
    game_index: index,
    storage_path: name,
    content_sha256: await sha256(bytes),
  };
  return { row, name, file: await gzip(bytes) };
}

Deno.test("games with no summary get one from their stored report", async () => {
  const g = await stored(1);
  const api = fakeApi({ missing: [g.row], objects: { [g.name]: g.file } });
  const res = await handle(request(), ENV, api.fetchFn);
  assertEquals(res.status, 200);
  assertEquals((await res.json()).stats, counts(1, 1, 1, 0));
  const list = api.calls.find((c) => c.path === "/rest/v1/rpc/game_stats_backfill_list")!;
  assertEquals(list.body, { p_version: SUMMARY_VERSION, p_limit: 500 });
  const get = api.calls.find((c) => c.method === "GET")!;
  assertEquals(get.path, `/storage/v1/object/authenticated/games/${g.name}`);
  assertEquals(get.headers.apikey, ENV.serviceKey);
  assertEquals(get.headers.Authorization, undefined);
  const put = api.calls.find((c) => c.path === "/rest/v1/rpc/game_stats_backfill_put")!;
  assertEquals(put.body, {
    target: A,
    p_session: SESSION,
    p_game_index: 1,
    p_sha256: g.row.content_sha256,
    p_summary_version: SUMMARY_VERSION,
    p_stats: summarize({ ...realDuos[0], index: 1 }),
  });
});

Deno.test("a stored file that differs from its row, fails the check or is gone is not summarized", async () => {
  const good = await stored(1);
  const other = await stored(2);
  const changed = { ...other, row: { ...other.row, content_sha256: "0".repeat(64) } };
  const invalid = await stored(3, { session: SESSION, index: 3, report: {} });
  const absent = await stored(4);
  const api = fakeApi({
    missing: [good.row, changed.row, invalid.row, absent.row],
    objects: { [good.name]: good.file, [changed.name]: changed.file, [invalid.name]: invalid.file },
  });
  const res = await handle(request(), ENV, api.fetchFn);
  assertEquals(res.status, 200);
  assertEquals(
    (await res.json()).stats,
    counts(4, 4, 1, 0, { file_mismatch: 1, bad_file: 1, no_file: 1 }),
  );
  const puts = api.calls.filter((c) => c.path === "/rest/v1/rpc/game_stats_backfill_put");
  assertEquals(puts.length, 1);
});

Deno.test("a summary the database turns down is skipped by its reason, not an error", async () => {
  const g = await stored(1);
  for (const put of ["missing", "changed", "newer"]) {
    const api = fakeApi({ missing: [g.row], objects: { [g.name]: g.file }, put });
    const res = await handle(request(), ENV, api.fetchFn);
    assertEquals((await res.json()).stats, counts(1, 1, 0, 0, { [put]: 1 }), put);
  }
  // An answer the database never gives is a failed write, not a fill.
  const api = fakeApi({ missing: [g.row], objects: { [g.name]: g.file }, put: "maybe" });
  const res = await handle(request(), ENV, api.fetchFn);
  assertEquals((await res.json()).stats, counts(1, 1, 0, 0, { write_failed: 1 }));
});

Deno.test("a failed write of one summary is counted and the others go on", async () => {
  const g = await stored(1);
  const api = fakeApi({
    missing: [g.row],
    objects: { [g.name]: g.file },
    fail: "/rest/v1/rpc/game_stats_backfill_put",
  });
  const res = await handle(request(), ENV, api.fetchFn);
  assertEquals(res.status, 200);
  assertEquals((await res.json()).stats, counts(1, 1, 0, 0, { write_failed: 1 }));
});

Deno.test("rows that do not look like game rows are never fetched", async () => {
  const g = await stored(1);
  const odd = { ...g.row, storage_path: `${A}/../secret.json.gz` };
  const api = fakeApi({ missing: [odd], objects: { [g.name]: g.file } });
  const res = await handle(request(), ENV, api.fetchFn);
  assertEquals(res.status, 500);
  assertEquals((await res.json()).step, "list summaries");
  assertEquals(api.calls.some((c) => c.method === "GET"), false);
});

Deno.test("a failed list of missing summaries is a 500 naming it", async () => {
  const api = fakeApi({ fail: "/rest/v1/rpc/game_stats_backfill_list" });
  const res = await handle(request(), ENV, api.fetchFn);
  assertEquals([res.status, (await res.json()).step], [500, "list summaries"]);
});

Deno.test("no summary is started past the time budget; the rest are left for the next run", async () => {
  const games = [await stored(1), await stored(2), await stored(3)];
  const api = fakeApi({
    missing: games.map((g) => g.row),
    objects: Object.fromEntries(games.map((g) => [g.name, g.file])),
  });
  // Each look at the clock moves it 40 seconds on.
  let t = 0;
  const env = { ...ENV, now: () => (t += 40_000) };
  const res = await handle(request(), env, api.fetchFn);
  assertEquals(res.status, 200);
  assertEquals((await res.json()).stats, counts(3, 2, 2, 1));
});

// ------------------------------------------------------------------ run record (T-524)

const failures = (api: ReturnType<typeof fakeApi>) =>
  api.calls.filter((c) => c.path === "/rest/v1/rpc/sweep_record_failure").map((c) => c.body);

Deno.test("the run's counts are recorded on the row housekeeping made", async () => {
  const g = await stored(1);
  const api = fakeApi({ files: [orphan(1)], missing: [g.row], objects: { [g.name]: g.file } });
  const res = await handle(request(), ENV, api.fetchFn);
  assertEquals(res.status, 200);
  const done = api.calls.filter((c) => c.path === "/rest/v1/rpc/sweep_record_result");
  assertEquals(done.map((c) => c.body), [{
    p_run_id: RUN,
    p_files: 1,
    p_stats: counts(1, 1, 1, 0),
  }]);
  assertEquals(failures(api), []);
});

Deno.test("a step that fails before housekeeping leaves a row of its own", async () => {
  const api = fakeApi({ files: [orphan(1)], fail: "/storage/v1/object/games" });
  const res = await handle(request(), ENV, api.fetchFn);
  assertEquals([res.status, (await res.json()).step], [500, "delete files"]);
  assertEquals(failures(api), [{ p_run_id: null, p_step: "delete files" }]);
});

Deno.test("a step that fails after housekeeping marks that run's row", async () => {
  const api = fakeApi({ fail: "/rest/v1/rpc/game_stats_backfill_list" });
  const res = await handle(request(), ENV, api.fetchFn);
  assertEquals([res.status, (await res.json()).step], [500, "list summaries"]);
  assertEquals(failures(api), [{ p_run_id: RUN, p_step: "list summaries" }]);
  assertEquals(api.calls.some((c) => c.path === "/rest/v1/rpc/sweep_record_result"), false);
});

Deno.test("counts that cannot be recorded are a failed run, not a silent one", async () => {
  const api = fakeApi({ fail: "/rest/v1/rpc/sweep_record_result" });
  const res = await handle(request(), ENV, api.fetchFn);
  assertEquals([res.status, (await res.json()).step], [500, "record run"]);
  assertEquals(failures(api), [{ p_run_id: RUN, p_step: "record run" }]);
});

Deno.test("housekeeping with no run id is a failed step", async () => {
  const api = fakeApi();
  const fetchFn: Fetch = (input, init) =>
    input.endsWith("/rpc/sweep_housekeeping")
      ? Promise.resolve(new Response(JSON.stringify({ auth_events: 0 }), { status: 200 }))
      : api.fetchFn(input, init);
  const res = await handle(request(), ENV, fetchFn);
  assertEquals([res.status, (await res.json()).step], [500, "housekeeping"]);
  assertEquals(failures(api), [{ p_run_id: null, p_step: "housekeeping" }]);
});

Deno.test("a failure that cannot be recorded still answers 500 naming the step", async () => {
  const api = fakeApi({
    fail: "/rest/v1/rpc/sweep_record_failure",
    sticky: true,
    files: [orphan(1)],
  });
  const res = await handle(request(), ENV, api.fetchFn);
  assertEquals([res.status, (await res.json()).step], [500, "delete files"]);
});

Deno.test("callers that are refused or cannot be checked record nothing", async () => {
  for (
    const [req, fail] of [
      [request("anon-key"), undefined],
      [tokenRequest("cd".repeat(32)), undefined],
      [tokenRequest(TOKEN), "/rest/v1/rpc/sweep_token_valid"],
    ] as const
  ) {
    const api = fakeApi({ fail });
    const res = await handle(req, ENV, api.fetchFn);
    assertEquals(res.status === 401 || res.status === 503, true);
    assertEquals(failures(api), []);
  }
});

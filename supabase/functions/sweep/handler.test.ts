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

/** What sweep_housekeeping() measures (T-543); far from the limits. */
const CAPACITY = {
  database: { bytes: 13_276_851, limit: 500_000_000, level: "none" },
  files: { bytes: 292_626, count: 41, limit: 1_000_000_000, level: "none" },
  level: "none",
  limits_read_on: "2026-10-12",
};

/** file_retention_list() in report-only mode, nothing due (T-544). */
const REPORT = {
  live: false,
  days: 28,
  due: 0,
  skipped: { exempt: 0, recent: 0, no_summary: 0 },
  no_summary_oldest_days: null,
  files: [],
};
/** The retention's counts; zero unless given. */
const retained = (over: Record<string, unknown> = {}) => ({
  mode: "report",
  days: 28,
  due: 0,
  listed: 0,
  changed: 0,
  deleted: 0,
  failed: 0,
  unmarked: 0,
  left: 0,
  skipped: { exempt: 0, recent: 0, no_summary: 0 },
  no_summary_oldest_days: null,
  ...over,
});

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
    /** sweep_housekeeping()'s capacity; CAPACITY unless given, left out if undefined. */
    capacity?: unknown;
    /** file_retention_list()'s answer; REPORT unless given. */
    retention?: unknown;
    /** True for the retention deletes (by their first file) that Storage refuses. */
    deleteFails?: (first: string) => boolean;
    /** file_retention_check()'s answer for a batch; all of it unless given. */
    stillDue?: (paths: string[]) => unknown;
    /** file_retention_mark()'s answer for a batch; all of it unless given. */
    marked?: (n: number) => unknown;
  } = {},
) {
  let files = [...(opts.files ?? [])];
  // Retention files the fake Storage deleted; the fake mark confirms these.
  const gone = new Set<string>();
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
      if (opts.deleteFails?.(body.prefixes[0])) return reply(503, {});
      if (!opts.sticky) {
        for (const p of body.prefixes) gone.add(p);
        files = files.filter((f) => !body.prefixes.includes(f));
      }
      return reply(200, []);
    }
    if (path === "/rest/v1/rpc/file_retention_list") {
      return reply(200, "retention" in opts ? opts.retention : REPORT);
    }
    if (path === "/rest/v1/rpc/file_retention_check") {
      return reply(200, opts.stillDue ? opts.stillDue(body.p_paths) : body.p_paths);
    }
    if (path === "/rest/v1/rpc/file_retention_mark") {
      const n = body.p_paths.filter((p: string) => gone.has(p)).length;
      return reply(200, opts.marked ? opts.marked(n) : n);
    }
    if (path === "/rest/v1/rpc/sweep_housekeeping") {
      const capacity = "capacity" in opts ? opts.capacity : CAPACITY;
      return reply(200, {
        auth_events: 1,
        upload_events: 2,
        ip_failures: 3,
        capacity,
        run_id: RUN,
      });
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

Deno.test("housekeeping, then orphan files deleted through the Storage API", async () => {
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
    capacity: CAPACITY,
    retention: retained(),
  });
  assertEquals(api.files(), []);
  assertEquals(api.calls.map((c) => `${c.method} ${c.path}`), [
    "POST /rest/v1/rpc/sweep_housekeeping",
    "POST /rest/v1/rpc/sweep_orphan_files",
    "DELETE /storage/v1/object/games",
    "POST /rest/v1/rpc/sweep_orphan_files",
    "POST /rest/v1/rpc/game_stats_backfill_list",
    "POST /rest/v1/rpc/file_retention_list",
    // 404 in this fake: a database from before the rename step. Nothing to report.
    "POST /rest/v1/rpc/game_files_to_move",
    "POST /rest/v1/rpc/sweep_record_result",
  ]);
  assertEquals(api.calls.at(-1)?.body, {
    p_run_id: RUN,
    p_files: 2,
    p_stats: counts(0, 0, 0, 0),
    p_retention: retained(),
  });
  assertEquals(api.calls[1].body, { min_age: "1 hour" });
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
  const body = await res2.json();
  assertEquals([res2.status, body.step], [500, "housekeeping"]);
  assertEquals(body.failed, ["housekeeping", "list files", "list summaries", "record run"]);
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

Deno.test("a signed, time-bound sweep token is checked by the database, then runs", async () => {
  const signed = `1791000000.${TOKEN}`;
  const api = fakeApi({ files: [orphan(1)], token: signed });
  const res = await handle(tokenRequest(signed), ENV, api.fetchFn);
  assertEquals(res.status, 200);
  assertEquals(api.calls[0].body, { p_token: signed });
});

Deno.test("a wrong or malformed sweep token runs nothing", async () => {
  // Wrong token: asked once, refused. Malformed: refused without asking.
  for (
    const [token, asked] of [["cd".repeat(32), 1], ["short", 0], [`${TOKEN}0`, 0], ["", 0],
      [`.${TOKEN}`, 0], [`1.2.${TOKEN}`, 0], [`17910000000000.${TOKEN}`, 0]] as const
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
    p_retention: retained(),
  }]);
  assertEquals(failures(api), []);
});

const records = (api: ReturnType<typeof fakeApi>) =>
  api.calls.filter((c) => c.path === "/rest/v1/rpc/sweep_record_result").map((c) => c.body);
const ran = (api: ReturnType<typeof fakeApi>, path: string) =>
  api.calls.some((c) => c.path === `/rest/v1/rpc/${path}`);

Deno.test("a failed orphan delete is recorded on the run's row; the other steps still run", async () => {
  const g = await stored(1);
  const api = fakeApi({
    files: [orphan(1)],
    fail: "/storage/v1/object/games",
    missing: [g.row],
    objects: { [g.name]: g.file },
  });
  const res = await handle(request(), ENV, api.fetchFn);
  const body = await res.json();
  assertEquals([res.status, body.step, body.failed], [500, "delete files", ["delete files"]]);
  assertEquals([body.run_id, body.files, body.auth_events], [RUN, 0, 1]);
  assertEquals(body.stats, counts(1, 1, 1, 0));
  assertEquals(body.retention, retained());
  assertEquals(records(api), [{
    p_run_id: RUN,
    p_files: 0,
    p_stats: counts(1, 1, 1, 0),
    p_retention: retained(),
    p_failed: ["delete files"],
  }]);
  assertEquals(failures(api), []);
});

Deno.test("a failed list of missing summaries is recorded with no counts; retention still runs", async () => {
  const api = fakeApi({ files: [orphan(1)], fail: "/rest/v1/rpc/game_stats_backfill_list" });
  const res = await handle(request(), ENV, api.fetchFn);
  const body = await res.json();
  assertEquals([res.status, body.step, body.stats, body.files], [500, "list summaries", null, 1]);
  assertEquals(ran(api, "file_retention_list"), true);
  assertEquals(records(api), [{
    p_run_id: RUN,
    p_files: 1,
    p_stats: null,
    p_retention: retained(),
    p_failed: ["list summaries"],
  }]);
  assertEquals(failures(api), []);
});

Deno.test("an orphan file with no row is cleaned and a backlog does not stall the run", async () => {
  // Files that never go away, and a clock that moves 20 seconds per look:
  // the orphan rounds stop, and housekeeping (first), the backfill and the
  // retention all run and are recorded.
  const many = Array.from({ length: 3000 }, (_, i) => orphan(i + 1));
  const api = fakeApi({ files: many });
  let t = 0;
  const env = { ...ENV, now: () => (t += 20_000) };
  const res = await handle(request(), env, api.fetchFn);
  const body = await res.json();
  assertEquals([res.status, body.step, body.files], [500, "delete files", 2000]);
  assertEquals(api.files().length, 1000, "the rest wait for the next run");
  assertEquals(api.calls[0].path, "/rest/v1/rpc/sweep_housekeeping");
  assertEquals(
    ["game_stats_backfill_list", "file_retention_list"].map((p) => ran(api, p)),
    [true, true],
  );
  assertEquals((records(api)[0] as Record<string, unknown>).p_failed, ["delete files"]);
  // One orphan, a clock that stands still: cleaned, and the run is clean.
  const one = fakeApi({ files: [orphan(1)] });
  const ok = await handle(request(), ENV, one.fetchFn);
  assertEquals([ok.status, (await ok.json()).files, one.files()], [200, 1, []]);
});

Deno.test("counts that cannot be recorded are a failed run, not a silent one", async () => {
  const api = fakeApi({ fail: "/rest/v1/rpc/sweep_record_result" });
  const res = await handle(request(), ENV, api.fetchFn);
  assertEquals([res.status, (await res.json()).step], [500, "record run"]);
  assertEquals(failures(api), [{ p_run_id: RUN, p_step: "record run" }]);
});

Deno.test("housekeeping with no run id is a failed step; the others run on a row of their own", async () => {
  const api = fakeApi({ files: [orphan(1)] });
  const fetchFn: Fetch = (input, init) =>
    input.endsWith("/rpc/sweep_housekeeping")
      ? Promise.resolve(new Response(JSON.stringify({ auth_events: 0 }), { status: 200 }))
      : api.fetchFn(input, init);
  const res = await handle(request(), ENV, fetchFn);
  const body = await res.json();
  assertEquals([res.status, body.step, body.run_id, body.capacity], [
    500,
    "housekeeping",
    null,
    null,
  ]);
  assertEquals("auth_events" in body, false);
  assertEquals(records(api), [{
    p_run_id: null,
    p_files: 1,
    p_stats: counts(0, 0, 0, 0),
    p_retention: retained(),
    p_failed: ["housekeeping"],
  }]);
  assertEquals(failures(api), []);
});

Deno.test("a failed housekeeping call is recorded the same way", async () => {
  const api = fakeApi({ fail: "/rest/v1/rpc/sweep_housekeeping" });
  const res = await handle(request(), ENV, api.fetchFn);
  assertEquals([res.status, (await res.json()).failed], [500, ["housekeeping"]]);
  assertEquals((records(api)[0] as Record<string, unknown>).p_run_id, null);
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

Deno.test("a file cut off while reading is no_file; one too large is bad_file", async () => {
  const cut = await stored(1);
  const big = await stored(2);
  const api = fakeApi({ missing: [cut.row, big.row] });
  const fetchFn: Fetch = (input, init) => {
    if (input.endsWith(cut.name)) {
      const body = new ReadableStream<Uint8Array>({
        pull(c) {
          c.error(new TypeError("connection reset"));
        },
      });
      return Promise.resolve(new Response(body, { status: 200 }));
    }
    if (input.endsWith(big.name)) {
      return Promise.resolve(new Response(new Uint8Array(64 * 1024 + 1), { status: 200 }));
    }
    return api.fetchFn(input, init);
  };
  const res = await handle(request(), ENV, fetchFn);
  assertEquals((await res.json()).stats, counts(2, 2, 0, 0, { no_file: 1, bad_file: 1 }));
});

/** Runs fn with console.warn and console.error captured. */
async function captureLogs(fn: () => Promise<void>): Promise<string[]> {
  const lines: string[] = [];
  const { warn, error } = console;
  console.warn = (...a: unknown[]) => lines.push(a.join(" "));
  console.error = (...a: unknown[]) => lines.push(a.join(" "));
  try {
    await fn();
  } finally {
    console.warn = warn;
    console.error = error;
  }
  return lines;
}

const finished = (api: ReturnType<typeof fakeApi>) =>
  api.calls.some((c) => c.path === "/rest/v1/rpc/sweep_record_result");

Deno.test("the response carries the capacity; far from the limits nothing is logged", async () => {
  const api = fakeApi();
  let body: Record<string, unknown> = {};
  const logs = await captureLogs(async () => {
    body = await (await handle(request(), ENV, api.fetchFn)).json();
  });
  assertEquals(body.capacity, CAPACITY);
  assertEquals(logs, []);
});

Deno.test("50 % or 80 % of a limit is logged with the percentages only", async () => {
  const at80 = {
    ...CAPACITY,
    files: { bytes: 820_000_000, count: 54_000, limit: 1_000_000_000, level: "80%" },
    database: { bytes: 260_000_000, limit: 500_000_000, level: "50%" },
    level: "80%",
  };
  const api = fakeApi({ capacity: at80 });
  let res = new Response();
  const logs = await captureLogs(async () => {
    res = await handle(request(), ENV, api.fetchFn);
  });
  assertEquals(res.status, 200);
  assertEquals((await res.json()).capacity, at80);
  assertEquals(logs, ["sweep: storage at 80% of a Free plan limit (database 52 %, files 82 %)"]);
  assertEquals(finished(api), true);
});

Deno.test("a failed measurement is passed on and logged, never fatal", async () => {
  const failed = { failed: true, sqlstate: "XX000" };
  const api = fakeApi({ capacity: failed });
  let res = new Response();
  const logs = await captureLogs(async () => {
    res = await handle(request(), ENV, api.fetchFn);
  });
  assertEquals(res.status, 200);
  const body = await res.json();
  assertEquals([body.capacity, body.stats], [failed, counts(0, 0, 0, 0)]);
  assertEquals(logs, ["sweep: capacity not measured (SQLSTATE XX000)"]);
  assertEquals(finished(api), true);
});

Deno.test("a missing or odd capacity is null in the response, logged, never fatal", async () => {
  const odd: unknown[] = [
    undefined, // a database from before the migration
    null,
    "full",
    { ...CAPACITY, level: "90%" },
    { ...CAPACITY, files: { ...CAPACITY.files, bytes: -1 } },
    { ...CAPACITY, database: { ...CAPACITY.database, bytes: "13 MB" } },
    { ...CAPACITY, database: { ...CAPACITY.database, limit: 0 } },
    { ...CAPACITY, limits_read_on: "yesterday" },
    { failed: true, sqlstate: "not a state" },
    { failed: false, sqlstate: "XX000" },
  ];
  for (const capacity of odd) {
    // JSON leaves out an undefined capacity, as a database before T-543 does.
    const api = fakeApi({ capacity });
    let res = new Response();
    const logs = await captureLogs(async () => {
      res = await handle(request(), ENV, api.fetchFn);
    });
    assertEquals(res.status, 200, JSON.stringify(capacity));
    assertEquals((await res.json()).capacity, null, JSON.stringify(capacity));
    assertEquals(logs, ["sweep: capacity missing or out of shape"], JSON.stringify(capacity));
    assertEquals(finished(api), true);
  }
});

Deno.test("only the known capacity fields reach the response", async () => {
  const extra = { ...CAPACITY, note: "x", files: { ...CAPACITY.files, name: "y" } };
  const api = fakeApi({ capacity: extra });
  const body = await (await handle(request(), ENV, api.fetchFn)).json();
  assertEquals(body.capacity, CAPACITY);
});

// ------------------------------------------------------------ file retention (T-544)

const B = "00000000-0000-4000-8000-00000000000b";
const old = (i: number, user = B) => ({
  path: `${user}/Hearthstone_2026_08_01_18_30_00-${i}.json.gz`,
  sha256: (i % 16).toString(16).repeat(64),
});
const live = (n: number, over: Record<string, unknown> = {}) => ({
  live: true,
  days: 28,
  due: n,
  skipped: { exempt: 2, recent: 4, no_summary: 1 },
  no_summary_oldest_days: 40,
  files: Array.from({ length: n }, (_, i) => old(i + 1)),
  ...over,
});
const retentionCalls = (api: ReturnType<typeof fakeApi>) =>
  api.calls.filter((c) =>
    c.path === "/rest/v1/rpc/file_retention_mark" ||
    c.path === "/rest/v1/rpc/file_retention_check" ||
    (c.path === "/storage/v1/object/games" &&
      (c.body as { prefixes: string[] }).prefixes[0].includes("2026_08_01"))
  );
const recorded = (api: ReturnType<typeof fakeApi>) =>
  (api.calls.find((c) => c.path === "/rest/v1/rpc/sweep_record_result")?.body as {
    p_retention: unknown;
  }).p_retention;

/** Runs the sweep with the logs captured; its answer's body and the logs. */
async function run(api: ReturnType<typeof fakeApi>, env: Env = ENV, fetchFn = api.fetchFn) {
  let status = 0;
  let body: Record<string, unknown> = {};
  const logs = await captureLogs(async () => {
    const res = await handle(request(), env, fetchFn);
    status = res.status;
    body = await res.json();
  });
  return { status, body, logs };
}

Deno.test("retention, report-only: counts what is due and deletes nothing", async () => {
  const api = fakeApi({
    retention: {
      ...REPORT,
      due: 5,
      skipped: { exempt: 2, recent: 4, no_summary: 1 },
      no_summary_oldest_days: 40,
    },
  });
  const { status, body } = await run(api);
  assertEquals(status, 200);
  const expected = retained({
    due: 5,
    skipped: { exempt: 2, recent: 4, no_summary: 1 },
    no_summary_oldest_days: 40,
  });
  assertEquals(body.retention, expected);
  assertEquals(recorded(api), expected);
  assertEquals(retentionCalls(api), []);
  const list = api.calls.find((c) => c.path === "/rest/v1/rpc/file_retention_list");
  assertEquals(list?.body, { p_limit: 1000, p_version: SUMMARY_VERSION });
});

Deno.test("retention, live: deletes through the Storage API, then marks the games", async () => {
  const api = fakeApi({ retention: live(3) });
  const { status, body, logs } = await run(api);
  assertEquals(status, 200);
  const expected = retained({
    mode: "live",
    due: 3,
    listed: 3,
    deleted: 3,
    skipped: { exempt: 2, recent: 4, no_summary: 1 },
    no_summary_oldest_days: 40,
  });
  assertEquals(body.retention, expected);
  assertEquals(recorded(api), expected);
  assertEquals(logs, []);
  const calls = retentionCalls(api);
  assertEquals(calls.map((c) => `${c.method} ${c.path}`), [
    "POST /rest/v1/rpc/file_retention_check",
    "DELETE /storage/v1/object/games",
    "POST /rest/v1/rpc/file_retention_mark",
  ]);
  const named = {
    p_paths: [old(1).path, old(2).path, old(3).path],
    p_sha256: [old(1).sha256, old(2).sha256, old(3).sha256],
  };
  assertEquals(calls[0].body, { ...named, p_version: SUMMARY_VERSION });
  assertEquals(calls[1].body, { prefixes: [old(1).path, old(2).path, old(3).path] });
  assertEquals(calls[2].body, named);
  assertEquals(calls[1].headers.apikey, ENV.serviceKey);
  // After the backfill (which needs the files) and before the run is recorded.
  const order = api.calls.map((c) => c.path);
  const at = (p: string) => order.indexOf(p);
  assertEquals(
    at("/rest/v1/rpc/game_stats_backfill_list") < at("/rest/v1/rpc/file_retention_list") &&
      at("/rest/v1/rpc/file_retention_list") < at("/rest/v1/rpc/sweep_record_result"),
    true,
  );
});

Deno.test("retention, live: batches of 100", async () => {
  const api = fakeApi({ retention: live(250) });
  const { body } = await run(api);
  assertEquals((body.retention as { deleted: number }).deleted, 250);
  const deletes = retentionCalls(api).filter((c) => c.method === "DELETE");
  assertEquals(deletes.map((c) => (c.body as { prefixes: string[] }).prefixes.length), [
    100,
    100,
    50,
  ]);
});

Deno.test("retention: a failed delete is counted, still checked, and its games stay unmarked", async () => {
  const api = fakeApi({ retention: live(250), deleteFails: (first) => first === old(101).path });
  const { status, body, logs } = await run(api);
  assertEquals(status, 200);
  const r = body.retention as Record<string, number>;
  assertEquals([r.listed, r.deleted, r.failed, r.unmarked, r.left], [250, 150, 100, 0, 0]);
  const marks = retentionCalls(api).filter((c) => c.path.endsWith("file_retention_mark"));
  // Marked after every delete, failed or not: the database counts the attempt.
  assertEquals(marks.map((c) => (c.body as { p_paths: string[] }).p_paths[0]), [
    old(1).path,
    old(101).path,
    old(201).path,
  ]);
  assertEquals(logs, [
    "sweep: file retention deleted 150 of 250 (100 failed, 0 unmarked, 0 left)",
  ]);
});

Deno.test("retention: a failed or short mark is counted as unmarked, never fatal", async () => {
  const cases: [(n: number) => unknown, number][] = [
    [(n) => n - 1, 1],
    [() => "odd", 3],
    [() => 99, 3],
    [() => -1, 3],
  ];
  for (const [marked, unmarked] of cases) {
    const { status, body } = await run(fakeApi({ retention: live(3), marked }));
    assertEquals(status, 200);
    const r = body.retention as Record<string, number>;
    // "deleted" is what the mark confirmed; the rest is unmarked.
    assertEquals([r.deleted, r.unmarked], [3 - unmarked, unmarked], String(marked));
  }
  const api = fakeApi({ retention: live(3), fail: "/rest/v1/rpc/file_retention_mark" });
  const { status, body } = await run(api);
  assertEquals(status, 200);
  const r = body.retention as Record<string, number>;
  assertEquals([r.deleted, r.unmarked], [0, 3]);
});

Deno.test("retention: no batch starts after the time budget; the rest is left", async () => {
  let t = 0;
  const api = fakeApi({ retention: live(250) });
  const fetchFn: Fetch = (input, init) => {
    if (input.endsWith("/rest/v1/rpc/file_retention_mark")) t += 15_000;
    return api.fetchFn(input, init);
  };
  const { body } = await run(api, { ...ENV, now: () => t }, fetchFn);
  const r = body.retention as Record<string, number>;
  assertEquals([r.listed, r.deleted, r.left], [250, 200, 50]);
});

Deno.test("retention: a failed list is recorded, never fatal, and deletes nothing", async () => {
  const api = fakeApi({ fail: "/rest/v1/rpc/file_retention_list" });
  const { status, body, logs } = await run(api);
  assertEquals(status, 200);
  assertEquals(body.retention, { error: "list" });
  assertEquals(recorded(api), { error: "list" });
  assertEquals(retentionCalls(api), []);
  assertEquals(logs, ["sweep: file retention not run (list failed or out of shape)"]);
});

Deno.test("retention: an answer out of shape deletes nothing", async () => {
  const bad: unknown[] = [
    null,
    [],
    "x",
    { ...REPORT, files: [old(1)], due: 1 }, // report-only naming a file
    live(1, { files: [{ path: "../etc/passwd", sha256: old(1).sha256 }] }),
    live(1, { files: [{ path: `${A}/notes.txt`, sha256: old(1).sha256 }] }),
    live(1, { files: [{ path: old(1).path, sha256: "x" }] }),
    live(1, { files: [old(1).path] }),
    live(2, { due: 1 }), // more files than due
    live(1001), // over the limit
    live(1, { days: 0 }),
    live(1, { days: 1.5 }),
    live(1, { live: "yes" }),
    live(1, { due: -1 }),
    live(1, { skipped: { exempt: 1, no_summary: 0 } }),
    live(1, { skipped: null }),
    live(1, { no_summary_oldest_days: -1 }),
    live(1, { no_summary_oldest_days: "40" }),
    live(1, { no_summary_oldest_days: undefined }),
  ];
  for (const retention of bad) {
    const api = fakeApi({ retention });
    const { status, body } = await run(api);
    const label = JSON.stringify(retention)?.slice(0, 80);
    assertEquals(status, 200, label);
    assertEquals(body.retention, { error: "list" }, label);
    assertEquals(retentionCalls(api), [], label);
  }
});

Deno.test("retention: the logs hold counts, never a user id or file name", async () => {
  const { logs } = await run(fakeApi({ retention: live(3), deleteFails: () => true }));
  assertEquals(logs.length, 1);
  assertEquals(/[0-9a-f]{8}-|Hearthstone|\.json/.test(logs.join("\n")), false);
});

Deno.test("retention: a file no longer due at its batch's check is kept and counted", async () => {
  const api = fakeApi({
    retention: live(3),
    stillDue: (paths) => paths.filter((p) => p !== old(2).path),
  });
  const { status, body } = await run(api);
  assertEquals(status, 200);
  const r = body.retention as Record<string, number>;
  assertEquals([r.listed, r.changed, r.deleted, r.unmarked], [3, 1, 2, 0]);
  const del = retentionCalls(api).find((c) => c.method === "DELETE");
  assertEquals(del?.body, { prefixes: [old(1).path, old(3).path] });
});

Deno.test("retention: nothing still due, no delete; a failed or odd check deletes nothing", async () => {
  const none = fakeApi({ retention: live(3), stillDue: () => [] });
  const r0 = (await run(none)).body.retention as Record<string, number>;
  assertEquals([r0.changed, r0.deleted], [3, 0]);
  assertEquals(retentionCalls(none).filter((c) => c.method === "DELETE"), []);
  const odd: ((paths: string[]) => unknown)[] = [
    () => null,
    () => "x",
    () => [old(9).path], // not in the batch
    (paths) => [paths[0], paths[0]], // twice
    (paths) => [...paths, old(9).path], // longer than the batch
  ];
  for (const stillDue of odd) {
    const api = fakeApi({ retention: live(3), stillDue });
    const { status, body } = await run(api);
    assertEquals(status, 200);
    const r = body.retention as Record<string, number>;
    assertEquals([r.failed, r.deleted], [3, 0], String(stillDue));
    assertEquals(retentionCalls(api).filter((c) => c.method === "DELETE"), [], String(stillDue));
  }
  const failing = fakeApi({ retention: live(3), fail: "/rest/v1/rpc/file_retention_check" });
  const r1 = (await run(failing)).body.retention as Record<string, number>;
  assertEquals([r1.failed, r1.deleted], [3, 0]);
});

Deno.test("retention: no batch starts past the run's deadline, whatever the backfill took", async () => {
  let t = 0;
  const api = fakeApi({ retention: live(3) });
  const fetchFn: Fetch = (input, init) => {
    // The backfill's list call takes the run past its 130 s.
    if (input.endsWith("/rest/v1/rpc/game_stats_backfill_list")) t += 131_000;
    return api.fetchFn(input, init);
  };
  const { body } = await run(api, { ...ENV, now: () => t }, fetchFn);
  const r = body.retention as Record<string, number>;
  assertEquals([r.listed, r.deleted, r.left], [3, 0, 3]);
  assertEquals(retentionCalls(api), []);
});

Deno.test("retention: a delete Storage answered but did not do is unmarked, not deleted", async () => {
  const api = fakeApi({ retention: live(3), sticky: true });
  const { status, body } = await run(api);
  assertEquals(status, 200);
  const r = body.retention as Record<string, number>;
  assertEquals([r.listed, r.deleted, r.unmarked, r.failed], [3, 0, 3, 0]);
});

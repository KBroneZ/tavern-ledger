// Run: deno test supabase/functions/sweep/
// Fake HTTP APIs; tests/test_supabase_local.py runs it against the local stack.
import { type Env, type Fetch, handle } from "./handler.ts";

const ENV: Env = { url: "http://api.test", serviceKey: "service-key-for-tests" };
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
}

function fakeApi(opts: { files?: string[]; fail?: string; sticky?: boolean } = {}) {
  let files = [...(opts.files ?? [])];
  const calls: Call[] = [];
  const reply = (status: number, body: unknown) =>
    Promise.resolve(new Response(JSON.stringify(body), { status }));
  const fetchFn: Fetch = (input, init = {}) => {
    const path = input.replace(ENV.url, "");
    const method = init.method ?? "GET";
    const body = init.body ? JSON.parse(init.body as string) : null;
    calls.push({ method, path, body });
    if (opts.fail === path) return reply(503, {});
    if (path === "/rest/v1/rpc/sweep_orphan_files") return reply(200, files.slice(0, 1000));
    if (path === "/storage/v1/object/games" && method === "DELETE") {
      if (!opts.sticky) files = files.filter((f) => !body.prefixes.includes(f));
      return reply(200, []);
    }
    if (path === "/rest/v1/rpc/sweep_housekeeping") {
      return reply(200, { auth_events: 1, upload_events: 2, ip_failures: 3 });
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
    files: 2,
    auth_events: 1,
    upload_events: 2,
    ip_failures: 3,
  });
  assertEquals(api.files(), []);
  assertEquals(api.calls.map((c) => `${c.method} ${c.path}`), [
    "POST /rest/v1/rpc/sweep_orphan_files",
    "DELETE /storage/v1/object/games",
    "POST /rest/v1/rpc/sweep_orphan_files",
    "POST /rest/v1/rpc/sweep_housekeeping",
  ]);
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

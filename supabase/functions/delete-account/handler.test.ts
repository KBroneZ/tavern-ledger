// Run: deno test supabase/functions/delete-account/
// Fake HTTP APIs; the end-to-end test against the local stack is
// tests/test_supabase_local.py.
import { type Env, type Fetch, handle } from "./handler.ts";

const ENV: Env = { url: "http://api.test", serviceKey: "service-key-for-tests" };
const USER = "00000000-0000-4000-8000-00000000000a";

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
function fakeApi(opts: { files?: string[]; fail?: string; userStatus?: number } = {}) {
  let files = [...(opts.files ?? [])];
  const calls: Call[] = [];
  const reply = (status: number, body: unknown) =>
    Promise.resolve(new Response(JSON.stringify(body), { status }));
  const fetchFn: Fetch = (input, init = {}) => {
    const path = input.replace(ENV.url, "");
    const method = init.method ?? "GET";
    const body = init.body ? JSON.parse(init.body as string) : null;
    calls.push({ method, path, body, headers: init.headers as Record<string, string> });
    if (opts.fail === path) return reply(503, {});
    if (path === "/auth/v1/user") {
      return opts.userStatus ? reply(opts.userStatus, {}) : reply(200, { id: USER });
    }
    if (path === "/rest/v1/rpc/user_file_names") return reply(200, files);
    if (path === "/storage/v1/object/games" && method === "DELETE") {
      files = files.filter((f) => !body.prefixes.includes(f));
      return reply(200, []);
    }
    if (path === "/rest/v1/rpc/delete_user_data") {
      return reply(200, { games: 2, profiles: 1, auth_events: 3 });
    }
    if (path === `/auth/v1/admin/users/${USER}` && method === "DELETE") return reply(200, {});
    return reply(404, {});
  };
  return { fetchFn, calls, files: () => files };
}

const request = (method = "DELETE", auth = "Bearer header.payload.sig") =>
  new Request("http://fn.test/delete-account", {
    method,
    headers: auth ? { Authorization: auth } : {},
  });

Deno.test("deletes files, then rows, then the auth user", async () => {
  const api = fakeApi({ files: [`${USER}/a.json.gz`, `${USER}/b.json.gz`] });
  const res = await handle(request(), ENV, api.fetchFn);
  assertEquals(res.status, 200);
  assertEquals(await res.json(), {
    deleted: { files: 2, games: 2, profiles: 1, auth_events: 3, account: 1 },
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
    ],
  );
});

Deno.test("the user is taken from the token, never from the request", async () => {
  const api = fakeApi();
  await handle(request(), ENV, api.fetchFn);
  assertEquals(api.calls[0].headers.Authorization, "Bearer header.payload.sig");
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
    assertEquals(/header\.payload|0000000a|service-key/.test(text), false, text);
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

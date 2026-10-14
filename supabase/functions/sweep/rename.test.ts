// Run: deno test supabase/functions/sweep/
// The step that renames old-named game files to their opaque names (T-586).
import type { Env, Fetch } from "./handler.ts";
import { renameFiles } from "./rename.ts";

const ENV: Env = { url: "http://api.test", serviceKey: "sb_secret_test" };
const A = "00000000-0000-4000-8000-00000000000a";
const B = "00000000-0000-4000-8000-00000000000b";
const ID = (n: number) => `3f2b8c1e-5d4a-4b7e-9c6d-${String(n).padStart(12, "0")}`;
const old = (user: string, i: number) => `${user}/Hearthstone_2026_10_09_18_30_00-${i}.json.gz`;
const opaque = (user: string, n: number) => `${user}/${ID(n)}.json.gz`;
const FAR = Number.MAX_SAFE_INTEGER;
const AUTHENTICATED = "/storage/v1/object/authenticated/games/";

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

/** Fake Storage and database: `objects` is the bucket, `rows` the games' storage_path. */
function fakeApi(opts: {
  list: unknown;
  objects: string[];
  rows?: string[];
  /** Moves of these names fail with 500. */
  failMove?: string[];
  listStatus?: number;
}) {
  const objects = new Set(opts.objects);
  const rows = new Set(opts.rows ?? []);
  const calls: Call[] = [];
  const reply = (status: number, body: unknown = {}) =>
    Promise.resolve(new Response(JSON.stringify(body), { status }));
  const fetchFn: Fetch = (input, init = {}) => {
    const path = input.replace(ENV.url, "");
    const method = init.method ?? "GET";
    const body = typeof init.body === "string" ? JSON.parse(init.body) : null;
    calls.push({ method, path, body });
    if (path === "/rest/v1/rpc/game_files_to_move") {
      return reply(opts.listStatus ?? 200, opts.list);
    }
    if (path === "/storage/v1/object/move") {
      const { sourceKey, destinationKey } = body as { sourceKey: string; destinationKey: string };
      if (opts.failMove?.includes(sourceKey)) return reply(500);
      if (!objects.has(sourceKey)) return reply(404);
      objects.delete(sourceKey);
      objects.add(destinationKey);
      return reply(200, { message: "Successfully moved" });
    }
    if (path.startsWith(AUTHENTICATED) && method === "HEAD") {
      return reply(objects.has(path.slice(AUTHENTICATED.length)) ? 200 : 404);
    }
    if (path === "/rest/v1/rpc/game_file_moved") {
      const { p_old, p_new } = body as { p_old: string; p_new: string };
      const had = rows.delete(p_old);
      if (had) rows.add(p_new);
      return reply(200, had);
    }
    return reply(404);
  };
  return { fetchFn, calls, objects, rows };
}

Deno.test("old files are moved, then their rows say so", async () => {
  const api = fakeApi({
    list: [
      { old_path: old(A, 1), new_path: opaque(A, 1) },
      { old_path: old(B, 2), new_path: opaque(B, 2) },
    ],
    objects: [old(A, 1), old(B, 2)],
    rows: [old(A, 1), old(B, 2)],
  });
  const counts = await renameFiles(ENV, api.fetchFn, FAR);
  assertEquals(counts, { listed: 2, renamed: 2, failed: 0, left: 0 });
  assertEquals([...api.objects].sort(), [opaque(A, 1), opaque(B, 2)].sort());
  assertEquals([...api.rows].sort(), [opaque(A, 1), opaque(B, 2)].sort());
  assertEquals(api.calls[0].body, { p_limit: 200 });
  assertEquals(api.calls.map((c) => c.path), [
    "/rest/v1/rpc/game_files_to_move",
    "/storage/v1/object/move",
    "/rest/v1/rpc/game_file_moved",
    "/storage/v1/object/move",
    "/rest/v1/rpc/game_file_moved",
  ]);
});

Deno.test("a run that stopped between the move and the row finishes the row", async () => {
  // The file is already under its new name; the old one is gone.
  const api = fakeApi({
    list: [{ old_path: old(A, 1), new_path: opaque(A, 1) }],
    objects: [opaque(A, 1)],
    rows: [old(A, 1)],
  });
  const counts = await renameFiles(ENV, api.fetchFn, FAR);
  assertEquals(counts, { listed: 1, renamed: 1, failed: 0, left: 0 });
  assertEquals([...api.rows], [opaque(A, 1)]);
});

Deno.test("a file that is nowhere is counted, tried again next run, and its row is not touched", async () => {
  const api = fakeApi({
    list: [{ old_path: old(A, 1), new_path: opaque(A, 1) }],
    objects: [],
    rows: [old(A, 1)],
  });
  const counts = await renameFiles(ENV, api.fetchFn, FAR);
  assertEquals(counts, { listed: 1, renamed: 0, failed: 1, left: 0 });
  assertEquals([...api.rows], [old(A, 1)]);
  assertEquals(api.calls.some((c) => c.path === "/rest/v1/rpc/game_file_moved"), false);
});

Deno.test("a failed move does not stop the others", async () => {
  const api = fakeApi({
    list: [
      { old_path: old(A, 1), new_path: opaque(A, 1) },
      { old_path: old(A, 2), new_path: opaque(A, 2) },
    ],
    objects: [old(A, 1), old(A, 2)],
    rows: [old(A, 1), old(A, 2)],
    failMove: [old(A, 1)],
  });
  const counts = await renameFiles(ENV, api.fetchFn, FAR);
  assertEquals(counts, { listed: 2, renamed: 1, failed: 1, left: 0 });
  assertEquals([...api.rows].sort(), [old(A, 1), opaque(A, 2)].sort());
});

Deno.test("no rename starts after the deadline", async () => {
  const api = fakeApi({
    list: [{ old_path: old(A, 1), new_path: opaque(A, 1) }],
    objects: [old(A, 1)],
    rows: [old(A, 1)],
  });
  const counts = await renameFiles({ ...ENV, now: () => 1_000 }, api.fetchFn, 500);
  assertEquals(counts, { listed: 1, renamed: 0, failed: 0, left: 1 });
  assertEquals(api.calls.length, 1, "only the list");
});

Deno.test("a database from before the step, or a failed list, reports nothing", async () => {
  for (const listStatus of [404, 500]) {
    const api = fakeApi({ list: {}, objects: [], listStatus });
    assertEquals(await renameFiles(ENV, api.fetchFn, FAR), null, String(listStatus));
    assertEquals(api.calls.length, 1);
  }
});

Deno.test("a list that is not shaped as asked is never acted on", async () => {
  const lists: unknown[] = [
    "no",
    [{ old_path: old(A, 1) }],
    // Another user's folder, a name that is not an old one, an old name as the target.
    [{ old_path: old(A, 1), new_path: opaque(B, 1) }],
    [{ old_path: opaque(A, 1), new_path: opaque(A, 2) }],
    [{ old_path: old(A, 1), new_path: old(A, 2) }],
    [{ old_path: `${A}/../${B}/x.json.gz`, new_path: opaque(A, 2) }],
    // One good row among bad ones: none is trusted.
    [{ old_path: old(A, 1), new_path: opaque(A, 1) }, { old_path: "x", new_path: "y" }],
  ];
  for (const list of lists) {
    const api = fakeApi({ list, objects: [old(A, 1)], rows: [old(A, 1)] });
    assertEquals(await renameFiles(ENV, api.fetchFn, FAR), null, JSON.stringify(list));
    assertEquals(api.calls.length, 1, JSON.stringify(list));
  }
});

Deno.test("an empty list costs one call", async () => {
  const api = fakeApi({ list: [], objects: [] });
  assertEquals(await renameFiles(ENV, api.fetchFn, FAR), { listed: 0, renamed: 0, failed: 0, left: 0 });
  assertEquals(api.calls.length, 1);
});

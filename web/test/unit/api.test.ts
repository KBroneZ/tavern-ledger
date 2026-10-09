// The Supabase calls in src/lib/api.ts without a server: a real client with
// a stubbed fetch for the database calls, and small fake clients elsewhere.
// The same functions run against the local stack in test/local/.
import { afterEach, test } from "node:test";
import assert from "node:assert/strict";
import type { SupabaseClient, SupportedStorage } from "@supabase/supabase-js";
import { deleteAccount, exportAll, loadPublicProfile, makeClient, saveProfile } from "../../src/lib/api.ts";

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

// The data pages' reads (src/lib/dataApi.ts) without a server: a real
// client with a stubbed fetch. Who may see a profile, what each read asks
// for, and that any failure throws instead of showing a half answer.
import { afterEach, test } from "node:test";
import assert from "node:assert/strict";
import { makeClient } from "../../src/lib/api.ts";
import { CHART_GAMES, LAST_GAMES, loadDashboard, loadGames, loadHeroes, profileAccess } from "../../src/lib/dataApi.ts";
import { DEFAULT_FILTERS, parseFilters } from "../../src/lib/data/filters.ts";
import { DataError } from "../../src/lib/data/rows.ts";
import { json, memoryStorage, rows, stubFetch, type Route } from "../helpers/fetchStub.ts";
import { gameRaw, heroRaw, overviewRow, recordRaw } from "../helpers/rows.ts";

const CONFIG = { url: "http://127.0.0.1:54321", anonKey: "sb_publishable_test" };
const USER = "0f8fad5b-d9cb-469f-a165-70867728950e";
const OTHER = "1b4e28ba-2fa1-11d2-883f-0016d3cca427";
const realFetch = globalThis.fetch;
const client = () => makeClient(CONFIG, memoryStorage());

afterEach(() => {
  globalThis.fetch = realFetch;
});

test("profile access: a public profile, its name and whether it is the viewer's own", async () => {
  const seen = stubFetch({ "/rest/v1/profiles": rows([{ display_name: "Keeper", is_public: true }]) });
  assert.deepEqual(await profileAccess(client(), USER, null), { kind: "public", name: "Keeper", own: false });
  assert.deepEqual(await profileAccess(client(), USER, USER), { kind: "public", name: "Keeper", own: true });
  assert.match(seen[0], new RegExp(`user_id=eq\\.${USER}`));
});

test("profile access: private and missing look the same to others; the owner sees their own", async () => {
  stubFetch({ "/rest/v1/profiles": rows([]) });
  assert.deepEqual(await profileAccess(client(), USER, OTHER), { kind: "private" });
  stubFetch({ "/rest/v1/profiles": rows([{ display_name: null, is_public: false }]) });
  assert.deepEqual(await profileAccess(client(), USER, USER), { kind: "own-private", name: null });
  // Never the case under row-level security, but a private row is still never shown to someone else.
  assert.deepEqual(await profileAccess(client(), USER, OTHER), { kind: "private" });
});

test("profile access: a server or network error throws instead of saying private", async () => {
  stubFetch({ "/rest/v1/profiles": () => json({ message: "boom" }, 500) });
  await assert.rejects(profileAccess(client(), USER, null), DataError);
  globalThis.fetch = (() => Promise.reject(new TypeError("offline"))) as typeof fetch;
  await assert.rejects(profileAccess(client(), USER, null), DataError);
});

function dashboardRoutes(over: Record<string, Route> = {}): string[] {
  return stubFetch({
    "/rest/v1/user_overview": rows([overviewRow(), overviewRow({ mode: "duos", avg_place: 2.2 })]),
    "/rest/v1/user_heroes": rows([heroRaw(), heroRaw({ mode: "duos" })]),
    "/rest/v1/user_records": rows([recordRaw(), recordRaw({ record: "unknown_new_one" })]),
    "/rest/v1/game_summaries": (url, init) =>
      url.searchParams.get("select") === "counted_place"
        ? rows([{ counted_place: 2 }, { counted_place: null }, { counted_place: 9 }], 1200)(url, init)
        : rows([gameRaw()])(url, init),
    ...over,
  });
}

test("dashboard: both modes read for one user, places newest first and capped", async () => {
  const seen = dashboardRoutes();
  const dash = await loadDashboard(client(), USER);
  assert.equal(dash.modes.solo.overview?.games, 31);
  assert.equal(dash.modes.duos.overview?.avgPlace, 2.2);
  assert.deepEqual(dash.modes.solo.places, [2, null, null], "out of range is unknown");
  assert.equal(dash.modes.solo.total, 1200);
  assert.equal(dash.modes.solo.lastGames.length, 1);
  assert.equal(dash.records.length, 1, "a record this page does not know is left out");
  for (const request of seen) assert.match(request, new RegExp(`user_id=eq\\.${USER}`), request);
  const places = seen.filter((r) => r.includes("select=counted_place"));
  assert.equal(places.length, 2);
  for (const r of places) {
    assert.match(r, new RegExp(`limit=${CHART_GAMES}`));
    assert.match(r, /order=play_order\.desc/);
  }
  assert.ok(seen.some((r) => r.includes("mode=eq.duos") && r.includes(`limit=${LAST_GAMES}`)));
  // Only what the page shows: no storage path, build, tribes or board.
  for (const r of seen.filter((x) => x.includes("game_summaries"))) assert.doesNotMatch(r, /storage_path|tribes|final_board|build|\*/);
});

test("dashboard: a mode with no games has no overview", async () => {
  dashboardRoutes({ "/rest/v1/user_overview": rows([overviewRow()]) });
  const dash = await loadDashboard(client(), USER);
  assert.equal(dash.modes.duos.overview, null);
});

test("dashboard: one failed or unreadable read fails the whole load", async () => {
  dashboardRoutes({ "/rest/v1/user_records": () => json({ message: "boom" }, 500) });
  await assert.rejects(loadDashboard(client(), USER), DataError);
  dashboardRoutes({ "/rest/v1/user_overview": rows([overviewRow({ games: "lots" })]) });
  await assert.rejects(loadDashboard(client(), USER), DataError);
});

test("games list: the filters reach the query, one page of 25 with the total", async () => {
  const seen = stubFetch({ "/rest/v1/game_summaries": rows([gameRaw(), gameRaw({ status: "incomplete" })], 52) });
  const { filters } = parseFilters("?mode=solo&hero=BG20_HERO_202&from=2026-10-01&to=2026-10-31&place=bottom&page=2");
  const page = await loadGames(client(), USER, filters);
  assert.equal(page.total, 52);
  assert.equal(page.rows.length, 2);
  const q = seen[0];
  for (const part of [
    `user_id=eq.${USER}`,
    "mode=eq.solo",
    "hero=eq.BG20_HERO_202",
    "played_on=gte.2026-10-01",
    "played_on=lte.2026-10-31",
    "top_half=eq.false",
    "order=play_order.desc",
    "offset=25",
    "limit=25",
  ]) {
    assert.ok(q.includes(part), `${part} in ${q}`);
  }
});

test("games list: past the last page there are no rows but the total still comes back", async () => {
  stubFetch({
    "/rest/v1/game_summaries": (_url, init) =>
      init.method === "HEAD"
        ? new Response(null, { status: 200, headers: { "Content-Range": "*/30" } })
        : json({ code: "PGRST103", message: "Requested range not satisfiable" }, 416),
  });
  const page = await loadGames(client(), USER, { ...DEFAULT_FILTERS, page: 9 });
  assert.deepEqual(page, { rows: [], total: 30 });
});

test("games list: an error is an error, not an empty list", async () => {
  stubFetch({ "/rest/v1/game_summaries": () => json({ message: "boom" }, 500) });
  await assert.rejects(loadGames(client(), USER, DEFAULT_FILTERS), DataError);
});

test("hero filter: heroes with games in either mode, most games first", async () => {
  stubFetch({
    "/rest/v1/user_heroes": rows([
      heroRaw({ hero: "A_HERO", games: 2 }),
      heroRaw({ hero: "B_HERO", games: 2, mode: "duos" }),
      heroRaw({ hero: "B_HERO", games: 3 }),
      heroRaw({ hero: "OFFERED", games: 0, placed_games: 0, avg_place: null, top_half: 0, wins: 0, best_place: null }),
    ]),
  });
  assert.deepEqual(await loadHeroes(client(), USER), ["B_HERO", "A_HERO"]);
});

test("a request that hangs ends in an error after the timeout, without retries", async () => {
  let calls = 0;
  globalThis.fetch = ((_input: RequestInfo | URL, init: RequestInit = {}) => {
    calls += 1;
    return new Promise((_resolve, reject) => {
      init.signal?.addEventListener("abort", () => reject(init.signal?.reason));
    });
  }) as typeof fetch;
  const started = Date.now();
  await assert.rejects(profileAccess(makeClient(CONFIG, memoryStorage(), 50), USER, null), DataError);
  assert.equal(calls, 1, "a timed-out read is not tried again");
  assert.ok(Date.now() - started < 1000);
});

// The community hero stats page (T-507): row and state checks, the sorts,
// the address, the cells, the reads with a stubbed server, the page body
// (tables, the honest empty state, the stale notice, the explanations) and
// the account page's switch calls.
import { afterEach, test } from "node:test";
import assert from "node:assert/strict";
import { loadCommunityConsent, makeClient, saveCommunityConsent } from "../../src/lib/api.ts";
import { loadCommunityStats, type CommunityStats } from "../../src/lib/community/api.ts";
import {
  communityHero,
  communityHeroes,
  communityState,
  heroCells,
  heroColumns,
  MIN_GAMES,
  MIN_PLAYERS,
  parsePeriod,
  parseSort,
  sortHeroes,
  withParam,
  type CommunityHero,
  type CommunityHeroes,
  type CommunityState,
} from "../../src/lib/community/heroes.ts";
import { CH_STRINGS, ct } from "../../src/lib/community/strings.ts";
import { howFold, JOIN_HREF, modePanels, periodPanel, POLICY_HREF, updatedLine } from "../../src/lib/community/view.ts";
import { DataError } from "../../src/lib/data/rows.ts";
import { communityErrorMessage } from "../../src/lib/profile.ts";
import { elements, textOf, toHtml } from "../helpers/html.ts";
import { json, memoryStorage, stubFetch } from "../helpers/fetchStub.ts";

const CONFIG = { url: "http://127.0.0.1:54321", anonKey: "sb_publishable_test" };
const USER = "0f8fad5b-d9cb-469f-a165-70867728950e";
const realFetch = globalThis.fetch;

afterEach(() => {
  globalThis.fetch = realFetch;
});

function raw(over: Record<string, unknown> = {}): Record<string, unknown> {
  return {
    mode: "solo",
    period: "30d",
    hero: "BG20_HERO_202",
    games: 60,
    players: 5,
    avg_place: 3.9,
    top_half_rate: 60,
    win_rate: 20,
    offered_games: 100,
    offered_players: 5,
    pick_rate: 55,
    ...over,
  };
}

const STATE = { refreshed_at: "2026-10-11T04:05:00+00:00", counted_until: "2026-10-10", available: true };
const state = (over: Partial<CommunityState> = {}): CommunityState => ({ ...communityState([STATE]), ...over });
const hero = (over: Partial<CommunityHero> = {}): CommunityHero => ({ ...communityHero(raw()).hero, ...over });
const empty = (): CommunityHeroes => ({ solo: { "30d": [], all: [] }, duos: { "30d": [], all: [] } });

// ------------------------------------------------------------------ rows

test("a row is checked, named and placed in its mode and period", () => {
  assert.deepEqual(communityHero(raw()), {
    mode: "solo",
    period: "30d",
    hero: { hero: "BG20_HERO_202", games: 60, players: 5, avgPlace: 3.9, topHalfRate: 60, winRate: 20, pick: { rate: 55, offeredGames: 100, offeredPlayers: 5 } },
  });
  assert.equal(communityHero(raw({ offered_games: null, offered_players: null, pick_rate: null })).hero.pick, null);
  assert.equal(communityHero(raw({ avg_place: 1.1 })).hero.avgPlace, 1.1, "one decimal survives floating point");
  const grouped = communityHeroes([raw(), raw({ period: "all" }), raw({ mode: "duos", avg_place: 2.5 })]);
  assert.equal(grouped.solo["30d"].length, 1);
  assert.equal(grouped.solo.all.length, 1);
  assert.equal(grouped.duos["30d"][0].avgPlace, 2.5);
  assert.equal(grouped.duos.all.length, 0);
});

test("the thresholds and steps are the database's: 50 games from 5 players, floored to tens and fives", () => {
  assert.equal(MIN_GAMES, 50);
  assert.equal(MIN_PLAYERS, 5);
  assert.equal(communityHero(raw({ games: 50, players: 5 })).hero.games, 50);
});

test("anything unexpected in a row throws instead of showing", () => {
  const bad: Record<string, unknown>[] = [
    { mode: "ranked" },
    { period: "7d" },
    { period: "old" },
    { hero: "<script>" },
    { hero: null },
    { games: 40 },
    { games: 55 },
    { players: 4 },
    { players: 6 },
    { players: 70 },
    { games: "60x" },
    { avg_place: 0.5 },
    { avg_place: 8.5 },
    { avg_place: 3.95 },
    { avg_place: "3.9" },
    { top_half_rate: 101 },
    { top_half_rate: 60.5 },
    { win_rate: -1 },
    { win_rate: 70 },
    { pick_rate: null },
    { offered_games: null },
    { offered_games: 40 },
    { offered_games: 105 },
    { offered_players: 3 },
    { pick_rate: 120 },
  ];
  for (const over of bad) assert.throws(() => communityHero(raw(over)), DataError, JSON.stringify(over));
  assert.throws(() => communityHero(raw({ mode: "duos", avg_place: 4.5 })), DataError, "Duos places stop at 4");
  assert.throws(() => communityHero("row"), DataError);
});

test("a list with a hero twice in one mode and period, or far too long, is refused", () => {
  assert.throws(() => communityHeroes([raw(), raw()]), DataError);
  assert.throws(() => communityHeroes(Array.from({ length: 1001 }, (_, i) => raw({ hero: `H${i}` }))), DataError);
  assert.throws(() => communityHeroes({}), DataError);
  assert.equal(communityHeroes([]).solo["30d"].length, 0);
});

test("the state: built time, last day counted, available", () => {
  assert.deepEqual(communityState([STATE]), { refreshedAt: STATE.refreshed_at, countedUntil: "2026-10-10", available: true });
  assert.deepEqual(communityState([{ refreshed_at: null, counted_until: null, available: false }]), { refreshedAt: null, countedUntil: null, available: false });
  const bad: Record<string, unknown>[] = [
    { refreshed_at: "yesterday" },
    { counted_until: "10 Oct" },
    { counted_until: null },
    { available: "yes" },
    { refreshed_at: null, counted_until: null, available: true },
  ];
  for (const over of bad) assert.throws(() => communityState([{ ...STATE, ...over }]), DataError, JSON.stringify(over));
  assert.throws(() => communityState([]), DataError);
  assert.throws(() => communityState([STATE, STATE]), DataError);
});

// ------------------------------------------------------------------ sorts and the address

test("each sort, then more games, then the card id", () => {
  const rows = [
    hero({ hero: "C", games: 80, avgPlace: 4.5, topHalfRate: 50, pick: null }),
    hero({ hero: "D", games: 70, avgPlace: 4.6, topHalfRate: 40, pick: null }),
    hero({ hero: "A", games: 60, avgPlace: 3.9, topHalfRate: 60, pick: { rate: 20, offeredGames: 60, offeredPlayers: 5 } }),
    hero({ hero: "B", games: 60, avgPlace: 3.9, topHalfRate: 70, pick: { rate: 40, offeredGames: 60, offeredPlayers: 5 } }),
  ];
  const order = (sort: Parameters<typeof sortHeroes>[1]) => sortHeroes(rows, sort).map((r) => r.hero);
  assert.deepEqual(order("games"), ["C", "D", "A", "B"]);
  assert.deepEqual(order("avg"), ["A", "B", "C", "D"]);
  assert.deepEqual(order("top"), ["B", "A", "C", "D"]);
  assert.deepEqual(order("pick"), ["B", "A", "C", "D"], "no pick rate goes last, by games");
  assert.deepEqual(rows.map((r) => r.hero), ["C", "D", "A", "B"], "the input is left as it was");
});

test("period and sort live in the address; defaults leave none", () => {
  assert.equal(parsePeriod(""), "30d");
  assert.equal(parsePeriod("?period=all"), "all");
  assert.equal(parsePeriod("?period=old"), "30d");
  assert.equal(parseSort("?sort=pick"), "pick");
  assert.equal(parseSort("?sort=rating"), "games");
  assert.equal(withParam("https://x.test/heroes/?mode=duos", "period", "all"), "/heroes/?mode=duos&period=all");
  assert.equal(withParam("https://x.test/heroes/?period=all&sort=avg", "period", "30d"), "/heroes/?sort=avg");
  assert.equal(withParam("https://x.test/heroes/?sort=avg#t", "sort", "games"), "/heroes/#t");
});

// ------------------------------------------------------------------ cells

test("cells: hero, games, players, pick, average, top half; Advanced adds wins and offers", () => {
  const cols = heroColumns("solo");
  assert.deepEqual(
    cols.map((c) => c.label),
    ["Hero", "Games", "Players", "Picked", "Avg place", "Top 4", "1st", "Offered", "Players offered"],
  );
  assert.deepEqual(cols.filter((c) => c.advanced).map((c) => c.label), ["1st", "Offered", "Players offered"]);
  assert.equal(heroColumns("duos")[5].label, "Top 2");
  assert.deepEqual(heroCells(hero()).slice(1), ["60+", "5+", "55%", "3.9", "60%", "20%", "100+", "5+"]);
  assert.equal(heroCells(hero({ avgPlace: 2 }))[4], "2.0");
});

test("a pick rate below the thresholds is a dash read as 'Not enough hero choices yet'", () => {
  const cells = heroCells(hero({ pick: null }));
  for (const i of [3, 7, 8]) {
    const cell = cells[i] as { node: Parameters<typeof textOf>[0][] };
    assert.match(cell.node.map(textOf).join(" "), /Not enough hero choices yet/);
  }
});

// ------------------------------------------------------------------ the page body

test("no hero above the thresholds: the empty state says why and how to join", () => {
  const html = toHtml(periodPanel([], "solo", "30d", "games"));
  assert.match(html, /Not enough games yet/);
  assert.match(html, /at least 50 games with it, in the last 30 days, come from at least 5 players/);
  assert.match(toHtml(periodPanel([], "solo", "all", "games")), /games with it, over all time, come from/);
  assert.match(html, new RegExp(`href="${JOIN_HREF}"`));
  assert.doesNotMatch(html, /<table/);
});

test("heroes above the thresholds: a sorted table with its caption, the sort switch and the rounding note", () => {
  const rows = [hero({ hero: "BG20_HERO_202", games: 60 }), hero({ hero: "BG23_HERO_303", games: 80, avgPlace: 6.8, pick: null })];
  const panel = periodPanel(rows, "duos", "all", "avg");
  const html = toHtml(panel);
  assert.match(html, /<caption>Heroes, Duos, over all time, sorted by average place<\/caption>/);
  const heroes = elements(panel).filter((e) => e.tag === "th" && e.attrs.some(([k, v]) => k === "scope" && v === "row"));
  assert.deepEqual(heroes.map((e) => textOf(e).trim()), ["BG20_HERO_202", "BG23_HERO_303"], "better average place first");
  assert.match(html, /50\+ means 50 to 59 games/);
  const pressed = elements(panel).filter((e) => e.tag === "button" && e.attrs.some(([k, v]) => k === "aria-pressed" && v === "true"));
  assert.deepEqual(pressed.map((e) => textOf(e)), ["Avg place"]);
  assert.match(html, /data-period-panel="all"/);
});

test("22 heroes: the top 20 and 'Show all' (21 show in full)", () => {
  const rows = Array.from({ length: 22 }, (_, i) => hero({ hero: `H${String(i).padStart(2, "0")}` }));
  assert.match(toHtml(periodPanel(rows, "solo", "30d", "games")), /Show all 22/);
});

test("both modes, each with both periods; unique ids", () => {
  const heroes = empty();
  heroes.solo["30d"] = [hero()];
  heroes.solo.all = [hero()];
  const nodes = modePanels({ heroes, state: state() }, "games");
  assert.deepEqual(nodes.map((n) => n.attrs.find(([k]) => k === "data-mode-panel")?.[1]), ["solo", "duos"]);
  const ids = nodes.flatMap(elements).flatMap((e) => e.attrs.filter(([k]) => k === "id").map(([, v]) => v));
  assert.equal(new Set(ids).size, ids.length);
  assert.equal(nodes.flatMap(elements).filter((e) => e.tag === "table").length, 2);
});

test("a snapshot too old to show: one notice, not 'not enough games'", () => {
  const stats: CommunityStats = { heroes: empty(), state: state({ available: false }) };
  const html = modePanels(stats, "games").map(toHtml).join("");
  assert.match(html, /Being rebuilt/);
  assert.doesNotMatch(html, /Not enough games yet/);
  // Before the first rebuild there is nothing to rebuild: the empty state.
  const never = modePanels({ heroes: empty(), state: { refreshedAt: null, countedUntil: null, available: false } }, "games");
  assert.match(never.map(toHtml).join(""), /Not enough games yet/);
});

test("the explanation: opt-in only, thresholds, rounding, self-reported, validator, no player, the policy", () => {
  const text = textOf(howFold());
  assert.match(text, /Include my games in community hero stats/);
  assert.match(text, /off for everyone until they turn it on/);
  assert.match(text, /at least 50 games from at least 5 different players/);
  assert.match(text, /more than half/);
  assert.match(text, /three quarters/);
  assert.match(text, /before the last 30 days pass the same rule/);
  assert.match(text, /tens of games and fives of players/);
  assert.match(text, /Self-reported/);
  assert.match(text, /checks each report/);
  assert.match(text, /no player, name or game is stored/);
  assert.match(text, /once a day/);
  assert.match(toHtml(howFold()), new RegExp(`href="${POLICY_HREF}"`));
});

test("the updated line names the build time and the last day counted", () => {
  assert.equal(updatedLine(state()), "Updated 11 Oct 2026, 04:05 UTC, with games up to 10 Oct 2026.");
  assert.equal(updatedLine({ refreshedAt: null, countedUntil: null, available: false }), "Not built yet.");
});

test("strings: placeholders are required, and no player names or ratings are promised", () => {
  assert.throws(() => ct("empty.body", { min: 1 }));
  for (const text of Object.values(CH_STRINGS)) assert.doesNotMatch(text, /BattleTag|MMR|ratings? (?:of|for)/i);
});

// ------------------------------------------------------------------ reads

test("reads every mode and period in one call, with the state, and refuses a bad answer", async () => {
  const seen = stubFetch({
    "/rest/v1/rpc/community_heroes": () => json([raw(), raw({ mode: "duos", period: "all" })]),
    "/rest/v1/rpc/community_heroes_state": () => json([STATE]),
  });
  const stats = await loadCommunityStats(makeClient(CONFIG, memoryStorage()));
  assert.equal(seen.length, 2);
  assert.equal(stats.heroes.solo["30d"][0].hero, "BG20_HERO_202");
  assert.equal(stats.heroes.duos.all.length, 1);
  assert.equal(stats.state.countedUntil, "2026-10-10");

  for (const routes of [
    { "/rest/v1/rpc/community_heroes": () => json({ message: "no" }, 500), "/rest/v1/rpc/community_heroes_state": () => json([STATE]) },
    { "/rest/v1/rpc/community_heroes": () => json([raw({ games: 10 })]), "/rest/v1/rpc/community_heroes_state": () => json([STATE]) },
    { "/rest/v1/rpc/community_heroes": () => json([]), "/rest/v1/rpc/community_heroes_state": () => json({ message: "no" }, 500) },
    // Rows from a snapshot the server says is too old: a fault, not data.
    { "/rest/v1/rpc/community_heroes": () => json([raw()]), "/rest/v1/rpc/community_heroes_state": () => json([{ ...STATE, available: false }]) },
  ]) {
    stubFetch(routes);
    await assert.rejects(loadCommunityStats(makeClient(CONFIG, memoryStorage())), DataError);
  }
});

// ------------------------------------------------------------------ the account page's switch

test("the switch reads off when the account never set it, and its own value otherwise", async () => {
  stubFetch({ "/rest/v1/community_consent": () => json([]) });
  assert.equal(await loadCommunityConsent(makeClient(CONFIG, memoryStorage()), USER), false);
  stubFetch({ "/rest/v1/community_consent": () => json([{ hero_stats: true }]) });
  assert.equal(await loadCommunityConsent(makeClient(CONFIG, memoryStorage()), USER), true);
  for (const answer of [json({ message: "no" }, 500), json([{ hero_stats: "yes" }])]) {
    stubFetch({ "/rest/v1/community_consent": () => answer });
    await assert.rejects(loadCommunityConsent(makeClient(CONFIG, memoryStorage()), USER));
  }
});

test("saving the switch calls the owner-only function and checks its answer", async () => {
  const bodies: unknown[] = [];
  stubFetch({
    "/rest/v1/rpc/set_community_consent": (_url, init) => {
      bodies.push(JSON.parse(String(init.body)));
      return json(true);
    },
  });
  assert.equal(await saveCommunityConsent(makeClient(CONFIG, memoryStorage()), true), null);
  assert.deepEqual(bodies, [{ p_hero_stats: true }]);
  stubFetch({ "/rest/v1/rpc/set_community_consent": () => json(true) });
  assert.match((await saveCommunityConsent(makeClient(CONFIG, memoryStorage()), false)) ?? "", /Could not save/);
  stubFetch({ "/rest/v1/rpc/set_community_consent": () => json({ code: "42501", message: "not signed in" }, 401) });
  assert.match((await saveCommunityConsent(makeClient(CONFIG, memoryStorage()), true)) ?? "", /Sign in again/);
  stubFetch({ "/rest/v1/rpc/set_community_consent": () => json({ message: "down" }, 500) });
  assert.match((await saveCommunityConsent(makeClient(CONFIG, memoryStorage()), false)) ?? "", /Could not save/);
  assert.equal(communityErrorMessage({}), "Could not save. Try again in a moment.");
});

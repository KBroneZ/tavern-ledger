// The heroes and curious stats pages' bodies and reads (T-505): a panel per
// mode, Simple and Advanced columns, the sort switch, the states, unique ids,
// the dashboard's links to both pages, and reads that ask only for the
// player's rows and throw on any failure.
import { afterEach, test } from "node:test";
import assert from "node:assert/strict";
import { makeClient } from "../../src/lib/api.ts";
import { DataError, gameRow, heroRow, overview, recordRow, type RecordRow } from "../../src/lib/data/rows.ts";
import type { Dashboard } from "../../src/lib/dataApi.ts";
import { heroStat } from "../../src/lib/heroes/heroes.ts";
import { curiousRecord, type CuriousRecord } from "../../src/lib/heroes/curious.ts";
import { loadCurious, loadHeroStats, type Curious } from "../../src/lib/heroes/load.ts";
import { curiousAbout, curiousEmpty, curiousPanels, heroesEmpty, heroesPanel, heroesPanels, TOP_HEROES } from "../../src/lib/heroes/views.ts";
import { modePanel } from "../../src/lib/view/dashboardView.ts";
import type { VNode } from "../../src/lib/view/node.ts";
import { elements, textOf, toHtml } from "../helpers/html.ts";
import { json, memoryStorage, rows, stubFetch } from "../helpers/fetchStub.ts";
import { gameRaw, heroRaw, overviewRow, recordRaw } from "../helpers/rows.ts";

const ID = "0f8fad5b-d9cb-469f-a165-70867728950e";
const attr = (node: VNode, name: string) => node.attrs.find(([k]) => k === name)?.[1];
const hasClass = (node: VNode, name: string) => (attr(node, "class") ?? "").split(" ").includes(name);

const heroes = Array.from({ length: 13 }, (_, i) =>
  heroStat(heroRaw({ hero: `BG_HERO_${String(i).padStart(2, "0")}`, games: 20 - i, placed_games: 5, top_half: 3, wins: 1, worst_place: 8, offered_games: 6, picked_games: 2 })),
);
const offered = heroStat(heroRaw({ hero: "BG_NEVER", games: 0, placed_games: 0, avg_place: null, top_half: 0, wins: 0, best_place: null, offered_games: 2, picked_games: 0 }));

test("heroes: a panel per mode; a mode with no hero says so", () => {
  const panels = heroesPanels([...heroes, offered], { profileId: null, sort: "games" });
  assert.deepEqual(panels.map((p) => attr(p, "data-mode-panel")), ["solo", "duos"]);
  assert.match(textOf(panels[1]), /No Duos games yet/);
  assert.equal(heroesEmpty([offered]), true);
  assert.equal(heroesEmpty(heroes), false);
});

test("heroes: Simple shows the main columns, Advanced adds wins, best, worst and picks", () => {
  const panel = heroesPanel("solo", heroes, { profileId: null, sort: "games" });
  const heads = elements(panel).filter((e) => e.tag === "th" && attr(e, "scope") === "col");
  assert.deepEqual(heads.filter((e) => !hasClass(e, "adv")).map(textOf), ["Hero", "Games", "Avg place", "Top 4"]);
  assert.deepEqual(heads.filter((e) => hasClass(e, "adv")).map(textOf), ["Wins", "Best", "Worst", "Picked when offered"]);
  // Every body row has as many cells as there are columns, with the same Advanced ones.
  for (const row of elements(panel).filter((e) => e.tag === "tr").slice(1)) {
    const cells = row.children.filter((c): c is VNode => typeof c !== "string");
    assert.deepEqual(cells.map((c) => hasClass(c, "adv")), heads.map((h) => hasClass(h, "adv")));
  }
});

test("heroes: top 10 with Show all; each hero links to its games", () => {
  const html = toHtml(heroesPanel("solo", heroes, { profileId: ID, sort: "games" }));
  assert.equal((html.match(/<tr class="more">/g) ?? []).length, heroes.length - TOP_HEROES);
  assert.match(html, /Show all 13/);
  assert.match(html, new RegExp(`<a href="/games/\\?id=${ID}&amp;mode=solo&amp;hero=BG_HERO_00">Unknown hero</a><span class="adv cell-note hero-id">BG_HERO_00</span>`));
});

test("heroes: the sort switch shows the sort in force and drives the table", () => {
  for (const sort of ["games", "avg"] as const) {
    const panel = heroesPanel("solo", heroes, { profileId: null, sort });
    const buttons = elements(panel).filter((e) => e.tag === "button" && attr(e, "data-sort"));
    assert.deepEqual(buttons.map((b) => [attr(b, "data-sort"), attr(b, "aria-pressed")]), [["games", String(sort === "games")], ["avg", String(sort === "avg")]]);
    const group = elements(panel).find((e) => attr(e, "role") === "group");
    assert.equal(attr(group as VNode, "aria-labelledby"), "heroes-solo-sort");
    assert.equal(textOf(elements(panel).find((e) => attr(e, "id") === "heroes-solo-sort") as VNode), "Sort by");
  }
  assert.match(textOf(heroesPanel("solo", heroes, { profileId: null, sort: "avg" })), /by average place, best first/);
});

test("heroes: never-played heroes are an Advanced fold; the few-games note shows only where a mark does", () => {
  const html = toHtml(heroesPanel("solo", [...heroes, offered], { profileId: null, sort: "games" }));
  assert.match(html, /<div class="adv"><details class="fold">.*Offered but never played \(1\)/);
  assert.doesNotMatch(html, /mark-few/, "every hero has enough games and choices: no mark, no legend");
  // Few hero choices only: the picks are marked in Advanced, and so is the legend.
  const fewPicks = heroes.map((r) => ({ ...r, offeredGames: 2, pickedGames: 1 }));
  const picks = toHtml(heroesPanel("solo", fewPicks, { profileId: null, sort: "games" }));
  assert.match(picks, /<p class="data-note adv"><span class="mark-src mark-few"/);
  assert.match(picks, /fewer than 5 finished games/);
});

test("heroes and curious: ids are unique across both panels", () => {
  const data = curious();
  for (const panels of [heroesPanels(heroes, { profileId: null, sort: "games" }), curiousPanels(data, ID)]) {
    const ids = panels.flatMap(elements).map((e) => attr(e, "id")).filter(Boolean);
    assert.equal(new Set(ids).size, ids.length, ids.join(", "));
  }
});

function curious(over: Partial<Curious> = {}): Curious {
  return {
    overviews: [overview(overviewRow()), overview(overviewRow({ mode: "duos", games: 0, incomplete_games: 0, placed_games: 0, top_half: 0, wins: 0, summarized_games: 0, combat_games: 0, damage_dealt_games: 0, damage_taken_games: 0, gold_spent_games: 0, gold_beyond_games: 0, sell_gold_games: 0, shop_games: 0, apm_games: 0, triples_games: 0 }))],
    records: [recordRaw({ session: "Hearthstone_2026_10_02_20_00_00", game_index: 0 }), recordRaw({ record: "damage_dealt_max", value: 30, inferred: true })]
      .map(curiousRecord)
      .filter((r): r is CuriousRecord => r !== null),
    ...over,
  };
}

test("curious: records with their games, Simple totals and Advanced totals, per mode", () => {
  const [solo, duos] = curiousPanels(curious(), ID);
  const html = toHtml(solo);
  assert.match(html, /Records, the game that set each one/);
  assert.match(html, /href="\/game\/\?u=/);
  assert.match(html, /mark-inferred/);
  // Each record's name heads its row, so its value and game are read with it.
  assert.match(html, /<th scope="row">Most combats won in a row<\/th>/);
  const tiles = elements(solo).filter((e) => hasClass(e, "tiles"));
  assert.equal(tiles.length, 2);
  assert.equal(hasClass(tiles[1], "adv"), true);
  assert.match(textOf(duos), /No Duos games yet/);
  assert.equal(curiousEmpty(curious()), false);
  assert.equal(curiousEmpty(curious({ overviews: [] })), true);
});

test("curious: no records yet says so; the gaps fold lists what is not here yet", () => {
  assert.match(textOf(curiousPanels(curious({ records: [] }), ID)[0]), /No records yet/);
  const about = curiousAbout().map(toHtml).join("");
  assert.match(about, /Not on the website yet/);
  assert.match(about, /most bought minion/);
});

test("the dashboard links to both pages when given their addresses", () => {
  const dash: Dashboard = {
    modes: {
      solo: { overview: overview(overviewRow()), places: [3], total: 31, lastGames: [gameRow(gameRaw())] },
      duos: { overview: null, places: [], total: 0, lastGames: [] },
    },
    heroes: [heroRow(heroRaw())],
    records: [recordRow(recordRaw())].filter((r): r is RecordRow => r !== null),
  };
  const base = { gamesHref: () => "/games/", owner: true, userId: ID };
  const html = toHtml(modePanel("solo", dash, { ...base, heroesHref: () => "/player-heroes/", curiousHref: () => "/curious/" }));
  assert.match(html, /href="\/player-heroes\/">All Solo heroes/);
  assert.match(html, /href="\/curious\/">All records and totals/);
  // Without them (an older caller) the dashboard is as before.
  assert.doesNotMatch(toHtml(modePanel("solo", dash, base)), /player-heroes|curious\//);
});

// ---------- Reads, with a real client and a stubbed server ----------

const CONFIG = { url: "http://127.0.0.1:54321", anonKey: "sb_publishable_test" };
const realFetch = globalThis.fetch;
const client = () => makeClient(CONFIG, memoryStorage());

afterEach(() => {
  globalThis.fetch = realFetch;
});

test("heroes read: one player's rows with the pick counts", async () => {
  const seen = stubFetch({ "/rest/v1/user_heroes": rows([heroRaw({ worst_place: 6, offered_games: 3, picked_games: 2 })]) });
  const list = await loadHeroStats(client(), ID);
  assert.equal(list[0].pickedGames, 2);
  assert.match(seen[0], new RegExp(`user_id=eq\\.${ID}`));
  assert.match(seen[0], /select=.*worst_place,offered_games,picked_games/);
});

test("curious read: the overview and the records with their games", async () => {
  const seen = stubFetch({
    "/rest/v1/user_overview": rows([overviewRow()]),
    "/rest/v1/user_records": rows([recordRaw({ session: "Hearthstone_2026_10_02_20_00_00", game_index: 3 }), recordRaw({ record: "new_one" })]),
  });
  const data = await loadCurious(client(), ID);
  assert.equal(data.overviews.length, 1);
  assert.deepEqual(data.records.map((r) => [r.record, r.gameIndex]), [["win_streak", 3]]);
  assert.ok(seen.every((s) => s.includes(`user_id=eq.${ID}`)));
  assert.ok(seen.some((s) => /user_records\?select=.*session,game_index/.test(s)));
});

test("reads: a server error or a broken answer throws instead of showing part of it", async () => {
  stubFetch({ "/rest/v1/user_heroes": () => json({ message: "boom" }, 500) });
  await assert.rejects(loadHeroStats(client(), ID), DataError);
  stubFetch({ "/rest/v1/user_heroes": rows([heroRaw({ offered_games: 1, picked_games: 2 })]) });
  await assert.rejects(loadHeroStats(client(), ID), DataError);
  stubFetch({ "/rest/v1/user_overview": rows([overviewRow()]), "/rest/v1/user_records": () => json({ message: "boom" }, 500) });
  await assert.rejects(loadCurious(client(), ID), DataError);
});

test("heroes: when every hero has few games, one line says so instead of a mark on each row (T-559)", () => {
  const few = Array.from({ length: 3 }, (_, i) => heroStat(heroRaw({ hero: `BG_FEW_${i}`, games: 2, placed_games: 2, top_half: 1, wins: 0, offered_games: 6, picked_games: 2 })));
  const html = toHtml(heroesPanel("solo", few, { profileId: null, sort: "games" }));
  assert.doesNotMatch(html, /mark-few/);
  assert.match(html, /Every hero here has fewer than 5 finished games/);
  // A mix keeps the mark, once per row, on the few-games heroes only.
  const mixed = toHtml(heroesPanel("solo", [...heroes, ...few], { profileId: null, sort: "games" }));
  assert.equal((mixed.match(/class="mark-src mark-few"/g) ?? []).length, few.length + 1, "each few row and the note");
});

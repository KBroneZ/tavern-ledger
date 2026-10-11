// What the dashboard and the games list show (T-503): key numbers first,
// each with the games behind it; unknown is a dash, never 0; inferred is marked.
import { test } from "node:test";
import assert from "node:assert/strict";
import { dateText, gameCells, gameColumns, heroCells, overviewTiles, perGame, recentPlaces, recordCells } from "../../src/lib/data/dashboard.ts";
import { gameRow, heroRow, overview, recordRow, type RecordRow } from "../../src/lib/data/rows.ts";
import { gameRaw, heroRaw, overviewRow, recordRaw } from "../helpers/rows.ts";
import { textOf, toHtml } from "../helpers/html.ts";
import type { VNode } from "../../src/lib/view/node.ts";

test("perGame: a sum over the games that have it; no game is unknown", () => {
  assert.equal(perGame(4060, 29), 140);
  assert.equal(perGame(null, 29), null);
  assert.equal(perGame(10, 0), null);
});

test("dateText: a short date, a dash when unknown", () => {
  assert.equal(dateText("2026-10-09"), "Oct 9, 2026");
  assert.equal(dateText(null), "—");
});

test("Simple tiles: average place, games, top 4 and wins, each with its games", () => {
  const { simple } = overviewTiles(overview(overviewRow()), [3, 1, 5]);
  assert.deepEqual(
    simple.map((x) => [x.label, x.value, x.backing]),
    [
      ["Average place", "3.42", "in 30 of 31 games"],
      ["Games", "31", "1 without a place (unfinished or unreadable)"],
      ["Top 4", "63%", "in 30 of 31 games"],
      ["Wins (1st place)", "4", "in 30 of 31 games"],
    ],
  );
  assert.deepEqual(simple[0].spark, { values: [3, 1, 5], min: 1, max: 8, invert: true });
  assert.equal(simple[0].hero, true);
});

test("Duos says top 2 and its line runs to 4th", () => {
  const { simple } = overviewTiles(overview(overviewRow({ mode: "duos", avg_place: 2.1 })), [1, 2]);
  assert.equal(simple[2].label, "Top 2");
  assert.equal(simple[0].spark?.max, 4);
});

test("no finished game: places and wins are unknown, not 0", () => {
  const { simple } = overviewTiles(overview(overviewRow({ games: 2, placed_games: 0, incomplete_games: 2, avg_place: null, top_half: 0, wins: 0, summarized_games: 0, combat_games: 0, damage_dealt_games: 0, damage_taken_games: 0, gold_spent_games: 0, gold_beyond_games: 0, sell_gold_games: 0, shop_games: 0, apm_games: 0, triples_games: 0 })), []);
  assert.deepEqual(simple.map((x) => x.value), ["—", "2", "—", "—"]);
  assert.equal(simple[0].spark, undefined);
  assert.equal(simple[0].backing, "no data in 2 games");
});

test("Advanced tiles: per-game numbers with their games; inferred and unknown marked", () => {
  const { advanced } = overviewTiles(overview(overviewRow({ apm_games: 0, avg_apm: null })), []);
  const by = Object.fromEntries(advanced.map((x) => [x.label, x]));
  assert.equal(by["Combats won"].value, "52%");
  assert.equal(by["Combats won"].source, "inferred");
  assert.equal(by["Damage dealt per game"].source, "inferred");
  assert.equal(by["Gold spent per game"].value, "140.00");
  assert.equal(by["Gold spent per game"].backing, "in 29 of 31 games");
  assert.equal(by["Gold beyond income per game"].value, "14.50");
  assert.equal(by["Gold beyond income per game"].backing, "in 20 of 31 games");
  assert.equal(by["Rolls per game"].value, "10.00");
  assert.equal(by["Triples per game"].value, "2.07");
  assert.equal(by["Actions per minute"].value, "—");
  assert.equal(by["Actions per minute"].source, "unknown");
  assert.equal(by["Actions per minute"].backing, "no data in 31 games");
});

test("combats from the log only carry no inferred mark", () => {
  const { advanced } = overviewTiles(overview(overviewRow({ combats_inferred: 0 })), []);
  assert.equal(advanced[0].source, undefined);
});

test("heroes: one mode, most games first, then the better average, then the name; offered-only heroes left out", () => {
  const rows = [
    heroRaw({ hero: "TB_BaconShop_HERO_41", games: 3, avg_place: 4 }),
    heroRaw({ hero: "BG22_HERO_002", games: 3, avg_place: 2.5 }),
    heroRaw({ hero: "BG20_HERO_202", games: 7 }),
    heroRaw({ hero: "OFFERED", games: 0, placed_games: 0, avg_place: null, top_half: 0, wins: 0, best_place: null }),
    heroRaw({ hero: "DUOS_HERO", mode: "duos" }),
  ].map(heroRow);
  const cells = heroCells(rows, "solo");
  assert.deepEqual(cells.map((r) => (r[0] as { name: string }).name), ["Master Nguyen", "Drek'Thar", "Reno Jackson"]);
  assert.deepEqual(cells[1].slice(1), [3, "2.50", "60%", { place: 1, mode: "solo" }]);
});

test("records: fixed order, value with its mark, date and the games that back it", () => {
  const rows = [
    recordRaw({ record: "apm", value: 52.345 }),
    recordRaw({ record: "damage_dealt_max", value: 24, inferred: true }),
    recordRaw({ record: "top_half_streak", value: 5, games: 30 }),
    recordRaw({ mode: "duos" }),
  ].map(recordRow).filter((r): r is RecordRow => r !== null);
  const cells = recordCells(rows, "solo", 31);
  assert.deepEqual(cells.map((r) => r[0]), ["Most top-4 finishes in a row", "Most damage dealt in one combat", "Highest actions per minute"]);
  const value = (c: unknown) => (c as { node: VNode[] }).node.map(toHtml).join("");
  assert.equal(value(cells[1][1]), '<span>24</span><span class="mark-src mark-inferred">inferred</span>');
  assert.equal(value(cells[2][1]), "<span>52.35</span>");
  assert.deepEqual(cells[0].slice(2), ["Oct 2, 2026", "in 30 of 31 games"]);
});

test("a game row: hero, place badge, date and the key numbers; unknown read as unknown", () => {
  const cells = gameCells(gameRow(gameRaw({ apm: null, combats_inferred: 2 })), false);
  assert.equal(cells.length, gameColumns(false).length);
  assert.deepEqual(cells.slice(0, 4), [{ hero: "BG20_HERO_202", name: "Master Nguyen", href: undefined, unknownId: null }, { place: 3, mode: "solo" }, "Oct 9, 2026", "13"]);
  const html = (c: unknown) => [(c as { node: VNode | VNode[] }).node].flat().map(toHtml).join("");
  assert.equal(html(cells[4]), '<span>7–5</span><span class="mark-src mark-inferred">inferred</span>');
  assert.equal(html(cells[6]), '<span aria-hidden="true">—</span><span class="visually-hidden">Unknown</span>');
});

test("an unfinished game says so instead of a place; the list of every mode adds the mode", () => {
  const cells = gameCells(gameRow(gameRaw({ status: "incomplete", mode: "duos", hero: null, rounds: null })), true);
  assert.equal(cells.length, gameColumns(true).length);
  assert.deepEqual(cells[0], { hero: null, name: "Hero not recorded", href: undefined, unknownId: null });
  assert.match(textOf((cells[1] as { node: VNode[] }).node[1]), /unfinished/);
  assert.equal(cells[3], "Duos");
  assert.ok("node" in (cells[4] as object), "rounds unknown");
});

test("recentPlaces: the newest n, oldest first", () => {
  assert.deepEqual(recentPlaces([1, 2, 3, 4], 3), [3, 2, 1]);
  assert.deepEqual(recentPlaces([], 20), []);
});

test("a game's date links to the game viewer when its key is known, each value encoded", () => {
  const id = "0f8fad5b-d9cb-469f-a165-70867728950e";
  const cells = gameCells(gameRow(gameRaw()), false, id);
  assert.equal(
    toHtml((cells[2] as { node: VNode }).node),
    `<a href="/game/?u=${id}&amp;s=Hearthstone_2026_10_09_18_30_00&amp;g=2">Oct 9, 2026<span class="visually-hidden">: game with Master Nguyen</span></a>`,
  );
  assert.equal(gameCells(gameRow(gameRaw({ session: "a/b?c" })), false, id)[2], "Oct 9, 2026", "an odd key gets no link");
  assert.equal(gameCells(gameRow(gameRaw()), false)[2], "Oct 9, 2026");
});

test("combats won: unknown combats are said next to the share of known ones", () => {
  const { advanced } = overviewTiles(overview(overviewRow({ combats_won: 5, combats_lost: 2, combats_tied: 0, combats_unknown: 6, combats_inferred: 0 })), []);
  assert.equal(advanced[0].value, "71%");
  assert.equal(advanced[0].backing, "of 7 combats with a result; 6 unknown");
});

test("heroes: games without a place are said next to the games, since places count finished games only", () => {
  const cells = heroCells([heroRow(heroRaw({ games: 10, placed_games: 2, avg_place: 2, top_half: 2 }))], "solo");
  assert.deepEqual(cells[0].slice(1, 4), ["10 (2 placed)", "2.00", "100%"]);
});

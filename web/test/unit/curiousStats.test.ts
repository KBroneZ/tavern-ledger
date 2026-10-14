// The curious stats page's data shaping (T-505): records with the game they
// came from, the totals with the games behind them, inferred and unknown
// marks, and the app's stats the website cannot show yet.
import { test } from "node:test";
import assert from "node:assert/strict";
import { overview } from "../../src/lib/data/rows.ts";
import { CURIOUS_RECORD_COLUMNS_VIEW, curiousRecord, gapItems, modeRecords, NOT_YET, recordCellsOf, totalTile, totalTiles, TOTALS, type CuriousRecord } from "../../src/lib/heroes/curious.ts";
import { textOf, toHtml } from "../helpers/html.ts";
import { overviewRow, recordRaw, type RawRow } from "../helpers/rows.ts";

const ID = "0f8fad5b-d9cb-469f-a165-70867728950e";
const GAME = "3f2b8c1e-5d4a-4b7e-9c6d-1a2b3c4d5e6f";

const rec = (over: RawRow = {}): CuriousRecord => {
  const r = curiousRecord(recordRaw({ public_id: GAME, game_index: 1, ...over }));
  assert.ok(r);
  return r;
};

const cellHtml = (cell: unknown): string =>
  typeof cell === "object" && cell !== null && "node" in cell ? [(cell as { node: unknown }).node].flat().map((n) => toHtml(n as never)).join("") : String(cell);

test("a record keeps its game's key; a record this page does not know is left out", () => {
  const r = rec();
  assert.equal(r.publicId, GAME);
  assert.equal(r.gameIndex, 1);
  assert.equal(curiousRecord(recordRaw({ record: "brand_new" })), null);
  // A key not shaped as expected gives no link, never a broken address.
  assert.equal(rec({ public_id: "../etc", game_index: -1 }).publicId, null);
  assert.equal(rec({ public_id: "../etc", game_index: -1 }).gameIndex, null);
});

test("records of one mode, in the fixed order", () => {
  const rows = [rec({ record: "apm", value: 51.5 }), rec({ record: "win_streak" }), rec({ mode: "duos", record: "rolls" }), rec({ record: "damage_dealt_max" })];
  assert.deepEqual(modeRecords(rows, "solo").map((r) => r.record), ["win_streak", "damage_dealt_max", "apm"]);
  assert.deepEqual(modeRecords(rows, "duos").map((r) => r.record), ["rolls"]);
});

test("a record's cells: label, value with its inferred mark, the game's link and the games behind it", () => {
  const [label, value, game, back] = recordCellsOf(rec({ record: "damage_taken_max", value: 21, inferred: true, games: 12 }), ID, 31);
  assert.equal(label, "Most damage taken in one combat");
  assert.match(cellHtml(value), /21.*mark-inferred/);
  const link = cellHtml(game);
  assert.match(link, new RegExp(`href="/game/\\?u=${ID}&amp;id=${GAME}"`));
  assert.match(link, /Oct 2, 2026/);
  assert.match(link, /visually-hidden">: the game that set the record Most damage taken in one combat/);
  assert.equal(back, "in 12 of 31 games");
});

test("APM is whole from 10 up (one decimal below); a game with no key is a date only; the top streak names its mode", () => {
  assert.match(cellHtml(recordCellsOf(rec({ record: "apm", value: 51.5 }), ID, 31)[1]), /<span>52<\/span>/);
  assert.match(cellHtml(recordCellsOf(rec({ record: "apm", value: 8.25 }), ID, 31)[1]), /<span>8\.3<\/span>/);
  assert.equal(recordCellsOf(rec({ public_id: null }), ID, 31)[2], "Oct 2, 2026");
  assert.equal(recordCellsOf(rec({ mode: "duos", record: "top_half_streak" }), ID, 31)[0], "Most top-2 finishes in a row");
  // A record backed by more games than the mode says it has stays honest.
  assert.equal(recordCellsOf(rec({ games: 40 }), ID, 31)[3], "in all 40 games");
});

test("totals: sums over the games that have them, with the games behind and the source", () => {
  const ov = overview(overviewRow());
  const tiles = totalTiles(ov);
  assert.equal(tiles.simple.length, 4);
  assert.equal(tiles.simple.length + tiles.advanced.length, TOTALS.length);
  // A sum over only some games keeps its line in view (T-559).
  assert.deepEqual(tiles.simple[0], { label: "Gold spent", value: "4,060", backing: "in 29 of 31 games", backingAlways: true, source: undefined });
  const dealt = tiles.advanced.find((x) => x.label === "Damage dealt");
  assert.equal(dealt?.source, "inferred");
});

test("a total no game has is a dash marked unknown, never 0", () => {
  const ov = overview(overviewRow({ gold_spent: null, gold_spent_games: 0, triples: 0, triples_games: 0 }));
  const gold = totalTile(TOTALS[0], ov);
  assert.equal(gold.value, "—");
  assert.equal(gold.source, "unknown");
  assert.equal(gold.backing, "no data in 31 games");
  // A sum the view sent with no game behind it is not shown either.
  const triples = totalTile(TOTALS[2], ov);
  assert.equal(triples.value, "—");
});

test("the app's curious stats the website cannot show are each listed with the reason", () => {
  const text = gapItems().map(textOf).join("\n");
  assert.equal(gapItems().length, NOT_YET.length);
  assert.match(text, /Golden Brann or Titus sold/);
  assert.match(text, /most bought minion/);
  assert.match(text, /Solo and Duos together/);
});

test("totals: the games line stays in view only when a total misses some games; the records' games are Advanced (T-559)", () => {
  const full = totalTiles(overview(overviewRow({ gold_spent_games: 31 }))).simple[0];
  assert.equal(full.backingAlways, false);
  const none = totalTiles(overview(overviewRow({ gold_spent_games: 0 }))).simple[0];
  assert.equal(none.value, "—");
  assert.equal(none.backingAlways, false, "unknown is a dash, not a partial sum");
  assert.deepEqual(CURIOUS_RECORD_COLUMNS_VIEW.filter((c) => c.advanced).map((c) => c.label), ["Based on"]);
});

test("damage totals are inferred only when some game's total is worked out (T-559)", () => {
  const dealt = (n: number) => totalTiles(overview(overviewRow({ damage_dealt_inferred_games: n }))).advanced.find((x) => x.label === "Damage dealt")?.source;
  assert.equal(dealt(0), undefined);
  assert.equal(dealt(1), "inferred");
});

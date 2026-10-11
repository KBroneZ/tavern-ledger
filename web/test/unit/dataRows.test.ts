// Rows from the data API's views, read as untrusted input (T-503): a broken
// count refuses the answer, a value out of range is unknown (null), never 0.
import { test } from "node:test";
import assert from "node:assert/strict";
import { amount, asRows, cardId, count, DataError, day, gameRow, heroRow, num, overview, place, recordRow } from "../../src/lib/data/rows.ts";
import { overviewRow } from "../helpers/rows.ts";

test("num reads numbers and numeric strings, nothing else", () => {
  assert.equal(num(3), 3);
  assert.equal(num("2.75"), 2.75);
  assert.equal(num("-1"), -1);
  for (const bad of [null, undefined, "", "1e9", "3 ", "abc", Number.NaN, Number.POSITIVE_INFINITY, true, {}]) assert.equal(num(bad), null, String(bad));
});

test("count refuses anything that is not a whole number from 0 up", () => {
  assert.equal(count(0, "x"), 0);
  assert.equal(count("12", "x"), 12);
  for (const bad of [null, -1, 1.5, "a"]) assert.throws(() => count(bad, "games"), DataError);
});

test("amount: a negative or missing value is unknown, 0 stays 0", () => {
  assert.equal(amount(0), 0);
  assert.equal(amount(null), null);
  assert.equal(amount(-5), null);
});

test("place: whole numbers within the mode's places only", () => {
  assert.equal(place(8, "solo"), 8);
  assert.equal(place(5, "duos"), null);
  assert.equal(place(0, "solo"), null);
  assert.equal(place(2.5, "solo"), null);
});

test("card ids and days are checked before they reach the page", () => {
  assert.equal(cardId("BG20_HERO_202"), "BG20_HERO_202");
  assert.equal(cardId("<img>"), null);
  assert.equal(cardId("x".repeat(65)), null);
  assert.equal(day("2026-10-09"), "2026-10-09");
  assert.equal(day("2026-02-30"), null);
  assert.equal(day("2026-10-09T00:00"), null);
});

test("asRows refuses an answer that is not a list of objects", () => {
  assert.throws(() => asRows({}), DataError);
  assert.throws(() => asRows([1]), DataError);
  assert.deepEqual(asRows([]), []);
});

test("overview: reads every count and sum, unknown sums stay null", () => {
  const ov = overview(overviewRow({ gold_spent: null, gold_spent_games: 0 }));
  assert.equal(ov.mode, "solo");
  assert.equal(ov.games, 31);
  assert.equal(ov.placedGames, 30);
  assert.equal(ov.avgPlace, 3.42);
  assert.equal(ov.goldSpent, null);
  assert.equal(ov.goldSpentGames, 0);
  assert.equal(ov.combatsInferred, 4);
});

test("overview: a part larger than the whole, a broken count or an unknown mode refuses the answer", () => {
  assert.throws(() => overview(overviewRow({ placed_games: 40 })), /more than games/);
  assert.throws(() => overview(overviewRow({ apm_games: null })), DataError);
  assert.throws(() => overview(overviewRow({ mode: "arena" })), DataError);
});

test("overview: an average place outside the mode's places is unknown", () => {
  assert.equal(overview(overviewRow({ avg_place: 9 })).avgPlace, null);
  assert.equal(overview(overviewRow({ mode: "duos", avg_place: 4.5 })).avgPlace, null);
});

test("heroRow: a hero id that is not a card id is unknown", () => {
  const row = heroRow({ mode: "solo", hero: "<b>", games: 2, placed_games: 2, avg_place: 3, top_half: 1, wins: 0, best_place: 2 });
  assert.equal(row.hero, null);
  assert.equal(row.bestPlace, 2);
});

test("recordRow: known records only; an unknown or valueless one is left out", () => {
  assert.deepEqual(recordRow({ mode: "duos", record: "apm", value: "41.5", played_on: "2026-10-01", games: 3, inferred: false }), {
    mode: "duos",
    record: "apm",
    value: 41.5,
    playedOn: "2026-10-01",
    games: 3,
    inferred: false,
  });
  assert.equal(recordRow({ mode: "solo", record: "new_record", value: 3, games: 1 }), null);
  assert.equal(recordRow({ mode: "solo", record: "rolls", value: null, games: 1 }), null);
  assert.equal(recordRow({ mode: "solo", record: "damage_dealt_max", value: 9, games: 2, inferred: true })?.inferred, true);
});

test("gameRow: only a finished game has a place; unknown values stay null", () => {
  const base = { mode: "solo", played_on: "2026-10-09", hero: "BG20_HERO_202", counted_place: 4, rounds: 12 };
  assert.equal(gameRow({ ...base, status: "ok" }).place, 4);
  assert.equal(gameRow({ ...base, status: "incomplete" }).place, null);
  assert.equal(gameRow({ ...base, status: "weird" }).status, "unknown");
  const g = gameRow({ ...base, status: "ok", apm: null, triples: -1 });
  assert.equal(g.apm, null);
  assert.equal(g.triples, null);
  assert.equal(g.rounds, 12);
});

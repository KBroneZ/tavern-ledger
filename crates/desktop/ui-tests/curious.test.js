"use strict";
const test = require("node:test");
const assert = require("node:assert/strict");
const TLCurious = require("../ui/curious.js");

const stat = (over) => ({
  key: "gold_spent", label: "Gold spent", value: 1234, card: null, source: "log",
  games: 14, missing: null, basis: "Gold you spent.", ...over,
});

test("each stat says how many games back it", () => {
  assert.equal(TLCurious.coverage(14, 29), "In 14 of 29 games.");
  assert.equal(TLCurious.coverage(29, 29), "In all 29 games.");
  assert.equal(TLCurious.coverage(0, 29), "In none of your 29 games.");
  assert.equal(TLCurious.coverage(1, 1), "In all 1 game.");
  assert.equal(TLCurious.coverage(0, 0), "No games yet.");
});

test("a stat with a value shows it with its source and game count", () => {
  const [row] = TLCurious.rows({ games: 29, stats: [stat()], dropped: [] });
  assert.equal(row.value, "1,234");
  assert.equal(row.source, "log");
  assert.equal(row.coverage, "In 14 of 29 games.");
  assert.equal(row.why, null);
  assert.equal(row.basis, "Gold you spent.");
});

test("a stat with no data says so and why, never a 0", () => {
  const [row] = TLCurious.rows({
    games: 3,
    stats: [stat({ value: null, source: "unknown", games: 0, missing: "No game has a shop record." })],
  });
  assert.equal(row.value, "No data");
  assert.equal(row.source, "unknown");
  assert.equal(row.why, "No game has a shop record.");
  assert.equal(row.coverage, "In none of your 3 games.");
});

test("a real zero is shown as 0 with its source", () => {
  const [row] = TLCurious.rows({ games: 2, stats: [stat({ key: "longest_win_streak", value: 0, games: 2 })] });
  assert.equal(row.value, "0");
  assert.equal(row.source, "log");
});

test("an inferred stat keeps its source", () => {
  const [row] = TLCurious.rows({ games: 2, stats: [stat({ key: "most_damage_one_combat", value: 22, source: "inferred" })] });
  assert.equal(row.source, "inferred");
});

test("the most bought minion names its card and the times", () => {
  const [row] = TLCurious.rows({ games: 5, stats: [stat({ key: "most_bought", value: 7, card: "BG_A" })] });
  assert.equal(row.card, "BG_A");
  assert.equal(row.times, "bought 7 times");
  const [once] = TLCurious.rows({ games: 5, stats: [stat({ key: "most_bought", value: 1, card: "BG_A" })] });
  assert.equal(once.times, "bought 1 time");
});

test("dropped stats are listed with the reason", () => {
  assert.deepEqual(TLCurious.dropped({ dropped: [{ label: "Triples", why: "Not in the record." }] }), ["Triples: Not in the record."]);
  assert.deepEqual(TLCurious.dropped(null), []);
});

test("no answer from the app yet gives no rows instead of an error", () => {
  assert.deepEqual(TLCurious.rows(null), []);
  assert.deepEqual(TLCurious.rows({ games: 0 }), []);
});

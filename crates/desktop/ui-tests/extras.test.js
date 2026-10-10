"use strict";
const test = require("node:test");
const assert = require("node:assert/strict");
const X = require("../ui/extras.js");

test("results say how many came from the log, were inferred or are unknown", () => {
  const line = X.resultsLine({ from_log: 9, inferred: 2, unknown: 1, recorded: true });
  assert.equal(line.text, "Results: 9 from the game's log, 2 inferred from health, 1 unknown.");
  assert.equal(line.source, "inferred", "any inferred result marks the record inferred");
  assert.equal(X.resultsLine({ from_log: 5, inferred: 0, unknown: 0, recorded: true }).source, "log");
});

test("an older game says its own results were not recorded", () => {
  const line = X.resultsLine({ from_log: 0, inferred: 6, unknown: 1, recorded: false });
  assert.match(line.text, /not recorded for this game/);
  assert.match(line.text, /--reparse/);
  assert.equal(X.resultsLine({ from_log: 0, inferred: 0, unknown: 0, recorded: true }).text, "No combat in the log.");
});

const pick = (over) => ({
  state: "recorded", source: "log", rerolls: 1,
  offered: [{ id: "H_A", name: "Alpha" }, { id: "H_B", name: null }, { id: "H_C", name: "Gamma" }],
  picked: { id: "H_B", name: null }, ...over,
});

test("the hero pick lists the offers with the picked one marked", () => {
  const p = X.heroPick(pick());
  assert.equal(p.empty, null);
  assert.deepEqual(p.offered.map((h) => [h.text, h.picked]), [["Alpha", false], ["H_B", true], ["Gamma", false]]);
  assert.equal(p.rerolls, "1 reroll");
  assert.equal(p.picked, "H_B");
  assert.equal(X.heroPick(pick({ rerolls: 0 })).rerolls, "No reroll");
  assert.equal(X.heroPick(pick({ rerolls: 2 })).rerolls, "2 rerolls");
});

test("a pick the log lacks is unknown, an older game not recorded", () => {
  assert.equal(X.heroPick(pick({ picked: null })).pickedSource, "unknown");
  assert.match(X.heroPick({ state: "not_recorded" }).empty, /not recorded/);
  assert.match(X.heroPick({ state: "no_data" }).empty, /no hero choice/);
  assert.match(X.heroPick(undefined).empty, /not recorded/);
});

const none = { buys: 3, extra_gold: null, sell_gold: null, gold_beyond_income: null, buy_gold: null, spell_gold: null, free_refreshes: null, passes: null };

test("shop extras say not recorded for an older game, never 0", () => {
  const old = X.shopExtras(none, false);
  assert.ok(old.every((f) => f.value === X.NOT_RECORDED && f.source === "unknown"));
  assert.equal(old.length, 6, "no Duos passes in Solo");
  const fresh = X.shopExtras({ buys: 3, extra_gold: 0, sell_gold: 5, gold_beyond_income: 5, buy_gold: 9, spell_gold: 2, free_refreshes: 1, passes: 2 }, true);
  assert.deepEqual(fresh.map((f) => f.value), ["0", "5", "5", "9", "2", "1", "2"]);
  assert.ok(fresh.every((f) => f.source === "log"), "a recorded 0 is a real 0");
  assert.equal(fresh[6].label, "Cards passed to your teammate");
  assert.deepEqual(X.shopExtras(null, false), []);
});

test("a total the log lacks for one turn says not every turn, not not recorded", () => {
  const turns = [{ extra_gold: null, passes: null }, { extra_gold: 2, passes: null }];
  const facts = X.shopExtras(none, false, turns);
  assert.equal(facts[0].value, "not every turn");
  assert.equal(facts[0].source, "unknown");
  assert.match(facts[0].tip, /at least one turn/);
  assert.equal(facts[1].value, X.NOT_RECORDED, "no turn has sell gold: not recorded");
});

test("shop extra averages give the games they cover", () => {
  const facts = X.shopExtraStats({ extra_gold: 2.25, sell_gold: 4, gold_beyond_income: 6.25, free_refreshes: null, passes: 0, extras_games: 4, free_refresh_games: 0 });
  assert.deepEqual(facts.map((f) => f.value), ["2.3", "4.0", "6.3", "not recorded", "0.0"]);
  assert.equal(facts[0].tip, "Average over 4 games with this record.");
  assert.equal(facts[3].source, "unknown");
  assert.equal(facts[3].tip, "No game has this record yet.");
});

test("per-hero pick stats give rate and place, and the games left out", () => {
  const s = X.heroPickStats({
    games: 3, not_recorded: 2, no_data: 1, source: "log",
    heroes: [
      { hero: "H_A", name: "Alpha", offered: 3, picked: 2, pick_rate: 2 / 3, placed_when_picked: 2, average_place_when_picked: 2.5 },
      { hero: "H_B", name: null, offered: 1, picked: 0, pick_rate: 0, placed_when_picked: 0, average_place_when_picked: null },
    ],
  });
  assert.deepEqual(s.rows.map((r) => [r.text, r.offered, r.picked, r.rate, r.place]), [
    ["Alpha", "3", "2", "67%", "2.50"],
    ["H_B", "1", "0", "0%", "—"],
  ]);
  assert.match(s.note, /^Over 3 games whose hero pick is in the log\./);
  assert.match(s.note, /2 saved by an older version \(re-read them with tavern-watch --reparse/);
  assert.match(s.note, /1 with no hero choice/);
  assert.equal(s.empty, null);
  assert.equal(X.heroPickStats({ games: 0, not_recorded: 0, no_data: 0, heroes: [] }).empty, "No hero pick in the log yet.");
});

"use strict";
const test = require("node:test");
const assert = require("node:assert/strict");
const R = require("../ui/round-two.js");
const I18n = require("../ui/i18n.js");
require("../ui/strings/common.js");
require("../ui/strings/details.js");

test("keywords are short words with the full name, and an older record is not none", () => {
  const marks = R.keywordMarks(["taunt", "divine_shield", "new_thing"]);
  assert.deepEqual(marks.map((k) => [k.short, k.full]), [["Taunt", "Taunt"], ["Shield", "Divine Shield"], ["New Thing", "New Thing"]]);
  assert.deepEqual(R.keywordMarks([]), [], "recorded, none");
  assert.equal(R.keywordMarks(null), null, "not recorded");
  assert.equal(R.keywordMarks(undefined), null);
});

const seat = {
  seat: 3, hero: "H_3", own: false, triples: 2,
  triples_by_turn: [[4, 1], [7, 2]],
  trinkets: [
    { turn: 6, slot: "first", dbf_id: 10, card_id: "T_A" },
    { turn: 9, slot: "first", dbf_id: 11, card_id: "T_A2" },
    { turn: 10, slot: "second", dbf_id: 12, card_id: null },
  ],
};

test("the leaderboard as of a turn: triples so far and the latest trinket per slot", () => {
  assert.deepEqual(R.seatAt(seat, 3), { triples: 0, trinkets: [] });
  const six = R.seatAt(seat, 6);
  assert.equal(six.triples, 1);
  assert.deepEqual(six.trinkets.map((t) => t.card_id), ["T_A"]);
  const end = R.seatAt(seat, Infinity);
  assert.equal(end.triples, 2);
  assert.deepEqual(end.trinkets.map((t) => t.dbf_id), [11, 12], "an upgrade replaces the slot's trinket");
});

const choices = (over) => ({
  state: "recorded", source: "log", leaderboard_state: "recorded",
  leaderboard: [seat, { seat: 1, hero: "H_ME", own: true, triples: null, triples_by_turn: [], trinkets: [] }],
  old_god_dbf: 555, old_god: "OG_A",
  choices: [
    { turn: 6, kind: "trinket", offered: [{ card_id: "T_A", gift: null }, { card_id: "T_B", gift: null }], picked: 1 },
    { turn: 5, kind: "dark_gift", offered: [{ card_id: "M_X", gift: "G_A" }], picked: 0 },
    { turn: 8, kind: "triple_reward", offered: [{ card_id: "M_Y", gift: null }], picked: null },
  ],
  ...over,
});

test("each opponent gets its boards with the leaderboard of that turn", () => {
  const opponent = {
    seats: [3],
    boards: [
      { round: 6, seat: 3, hero: null, minions: [{ card_id: "M_A", atk: 2, health: null, golden: true, keywords: ["taunt"] }] },
      { round: 8, seat: 3, hero: null, minions: [{ card_id: "M_B", atk: 1, health: 1, golden: false }] },
    ],
  };
  const d = R.opponentDetails(opponent, choices());
  assert.equal(d.leaderboardMissing, null);
  assert.equal(d.end[0].triples, 2);
  assert.equal(d.boards[0].board.triples, 1);
  assert.deepEqual(d.boards[0].minions[0], { id: "M_A", stats: "2/?", golden: true, keywords: [{ key: "taunt", short: "Taunt", full: "Taunt" }] });
  assert.equal(d.boards[0].keywordsRecorded, true);
  assert.equal(d.boards[1].keywordsRecorded, false, "an older board says not recorded");
});

test("an older record says not recorded, never zero triples", () => {
  const d = R.opponentDetails({ seats: [3], boards: [] }, choices({ leaderboard_state: "not_recorded", leaderboard: [] }));
  assert.match(d.leaderboardMissing, /not recorded/);
  assert.match(d.leaderboardMissing, /--reparse/);
  assert.deepEqual(d.end, []);
  assert.match(R.opponentDetails({ seats: [3] }, undefined).leaderboardMissing, /not recorded/);
});

test("choices are listed with the taken one and the trinkets taken and Old God", () => {
  const c = R.choiceRows(choices());
  assert.equal(c.empty, null);
  assert.deepEqual(c.rows.map((r) => [r.turn, r.kind]), [[6, "Trinket offer"], [5, "Dark Gift"], [8, "Triple reward"]]);
  assert.deepEqual(c.rows[0].options.map((o) => o.taken), [false, true]);
  assert.equal(c.rows[2].pickKnown, false, "the log does not say: not a guess");
  assert.deepEqual(c.trinketsTaken, ["T_B"]);
  assert.deepEqual(c.oldGod, { id: "OG_A", dbf: 555 });
  assert.deepEqual(R.choiceRows(choices({ old_god: null })).oldGod, { id: null, dbf: 555 });
  assert.equal(R.choiceRows(choices({ old_god_dbf: null })).oldGod, null);
});

test("older or unreadable choices say so", () => {
  assert.match(R.choiceRows({ state: "not_recorded" }).empty, /not recorded.*--reparse/);
  assert.match(R.choiceRows({ state: "no_data" }).empty, /no choices/);
  assert.match(R.choiceRows(undefined).empty, /not recorded/);
  assert.equal(R.choiceRows(choices({ choices: [] })).empty, "No choice in the log for this game.");
});

test("the stats table has rates, places and the games behind it", () => {
  const stats = {
    games: 3, not_recorded: 2, no_data: 0,
    trinkets: [{ card_id: "T_A", offered: 4, picked: 1, pick_rate: 0.25, placed_when_picked: 1, average_place_when_picked: 3 }],
    dark_gifts: [],
  };
  const t = R.choiceTable(stats, "trinkets");
  assert.deepEqual(t.rows[0], { id: "T_A", offered: "4", picked: "1", rate: "25%", place: "3.00", placeTip: "Over 1 game with a place that counts.",
    sort: { offered: 4, picked: 1, rate: 0.25, place: 3 } });
  assert.match(t.note, /^Over 3 games whose choices are in the log\. Not counted: 2 saved by an older version/);
  assert.equal(R.choiceTable(stats, "dark_gifts").empty, "None offered in these games yet.");
  assert.equal(R.choiceTable(null, "trinkets").empty, "No choice in the log yet.");
});

test("in Spanish keywords, kinds and counts follow the language", () => {
  I18n.setLang("es");
  try {
    const marks = R.keywordMarks(["taunt", "divine_shield"]);
    assert.deepEqual(marks.map((k) => [k.short, k.full]), [["Provocar", "Provocar"], ["Escudo", "Escudo divino"]]);
    assert.equal(R.choiceRows(choices()).rows[1].kind, "Don siniestro");
    const t = R.choiceTable({ games: 3, not_recorded: 0, no_data: 0, trinkets: [{ card_id: "T_A", offered: 4, picked: 1, pick_rate: 0.25, placed_when_picked: 1, average_place_when_picked: 2.5 }], dark_gifts: [] }, "trinkets");
    assert.equal(t.rows[0].place, "2,50");
    assert.match(t.note, /^En 3 partidas cuyas elecciones/);
    assert.match(R.choiceRows({ state: "not_recorded" }).empty, /^Elecciones no registradas/);
  } finally {
    I18n.setLang("en");
  }
  assert.equal(R.keywordMarks(["taunt"])[0].full, "Taunt");
});

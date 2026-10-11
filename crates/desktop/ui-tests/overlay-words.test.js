"use strict";
const test = require("node:test");
const assert = require("node:assert/strict");
const I18n = require("../ui/i18n.js");
require("../ui/strings/common.js");
require("../ui/strings/overlay.js");
const words = require("../ui/overlay-words.js");

const names = { BG30_MagicItem_1: "Made-up Trinket" };
const nameOf = (id) => (Object.prototype.hasOwnProperty.call(names, id) ? names[id] : null);

test("every keyword the parser writes has a short mark and its full name", () => {
  for (const key of ["taunt", "divine_shield", "reborn", "venomous", "windfury", "stealth", "deathrattle", "rally"]) {
    const [mark] = words.keywordMarks([key]);
    assert.ok(mark.short.length <= 2, key);
    assert.ok(mark.full.length > mark.short.length, key);
  }
  assert.deepEqual(words.keywordMarks(["divine_shield"]), [{ key: "divine_shield", short: "DS", full: "Divine Shield" }]);
});

test("keywords an older record did not keep are not recorded, not none", () => {
  assert.equal(words.keywordMarks(undefined), null);
  assert.equal(words.keywordMarks(null), null);
  assert.deepEqual(words.keywordMarks([]), []);
});

test("a keyword this version does not know still shows in words", () => {
  assert.deepEqual(words.keywordMarks(["magnetic", 7, ""]), [{ key: "magnetic", short: "Magnetic", full: "Magnetic" }]);
});

test("the legend spells out every mark", () => {
  const legend = words.keywordLegend();
  assert.match(legend, /^T Taunt · DS Divine Shield/);
  assert.match(legend, /Ra Rally$/);
});

test("a triple count the log never gave is said in words, not 0", () => {
  assert.equal(words.triplesText(null), "no triple in the log yet");
  assert.equal(words.triplesText(undefined), "no triple in the log yet");
  assert.equal(words.triplesText(1), "1 triple");
  assert.equal(words.triplesText(3), "3 triples");
  assert.equal(words.triplesText(0), "0 triples");
});

test("a trinket is named by the card data, else its card id, else its database id", () => {
  assert.deepEqual(words.trinketText({ card_id: "BG30_MagicItem_1", dbf_id: 1 }, nameOf), { text: "Made-up Trinket", named: true, known: true });
  assert.deepEqual(words.trinketText({ card_id: "BG30_Other", dbf_id: 2 }, nameOf), { text: "BG30_Other", named: false, known: false });
  assert.deepEqual(words.trinketText({ card_id: "BG30_Wait", dbf_id: 2 }, () => undefined), { text: "BG30_Wait", named: false, known: true });
  assert.deepEqual(words.trinketText({ card_id: null, dbf_id: 133 }, nameOf), { text: "trinket #133", named: false, known: false });
});

test("a seat's words say which trinket became a hero power and which are unnamed", () => {
  const seat = {
    seat: 2,
    triples: 2,
    trinkets: [
      { slot: "first", dbf_id: 1, card_id: "BG30_MagicItem_1" },
      { slot: "hero_power", dbf_id: 9, card_id: null },
    ],
  };
  const w = words.seatWords(seat, nameOf);
  assert.equal(w.triples, "2 triples");
  assert.deepEqual(w.trinkets.map((t) => [t.text, t.hero_power]), [["Made-up Trinket", false], ["trinket #9", true]]);
  assert.equal(w.named, true);
  assert.equal(w.unknown, true);
});

test("the next opponent is the one the game marks, or none", () => {
  const a = { seats: [2], next: false };
  const b = { seats: [3], next: true };
  assert.equal(words.nextOpponent([a, b]), b);
  assert.equal(words.nextOpponent([a]), null);
  assert.equal(words.nextOpponent(undefined), null);
});

test("the marked opponent is the next one in the shop and the current one during its combat", () => {
  const shop = { turn: { value: 6 }, last_combat: { value: 5 }, is_duos: false };
  const combat = { turn: { value: 6 }, last_combat: { value: 6 }, is_duos: true };
  assert.deepEqual(words.nextLabels(shop), { badge: "Next", heading: "Next opponent" });
  assert.deepEqual(words.nextLabels(combat), { badge: "Now", heading: "Opposing team now" });
  assert.deepEqual(words.nextLabels({ turn: { value: null }, last_combat: { value: null } }), { badge: "Next", heading: "Next opponent" });
});

test("in Spanish the marks, the triple count and the labels are Spanish, and English comes back", () => {
  I18n.setLang("es");
  try {
    assert.deepEqual(words.keywordMarks(["divine_shield"]), [{ key: "divine_shield", short: "ED", full: "Escudo divino" }]);
    assert.equal(words.triplesText(1), "1 triple");
    assert.equal(words.triplesText(3), "3 triples");
    assert.equal(words.triplesText(null), "aún sin triples en el registro");
    assert.deepEqual(words.nextLabels({ turn: { value: null }, last_combat: { value: null } }), { badge: "Siguiente", heading: "Siguiente rival" });
    assert.match(words.keywordLegend(), /^P Provocar · ED Escudo divino/);
  } finally {
    I18n.setLang("en");
  }
  assert.equal(words.keywordMarks(["taunt"])[0].short, "T");
});

// The hover board (T-333): each hero of a Duos team with its own board.
const team = {
  heroes: [{ id: "HERO_A" }, { id: "HERO_B" }],
  seats: [5, 8],
  tiers: [{ value: 4, source: "log" }, { value: null, source: "log" }],
  boards: [{ seat: 8, round: 6, minions: [] }],
  shown: [{ seat: 5, triples: 1, trinkets: [] }, { seat: 8, triples: 0, trinkets: [] }],
  shown_source: "log",
  possible: [{ seat: 5, tier: 4 }],
};

test("a team's boards go with their own hero, matched by seat", () => {
  const [a, b] = words.seatViews(team);
  assert.equal(a.hero.id, "HERO_A");
  assert.equal(a.board, null, "the first hero was not met: no board, never the teammate's");
  assert.equal(a.tier, 4);
  assert.equal(a.shown.triples, 1);
  assert.equal(a.possible.length, 1);
  assert.equal(b.hero.id, "HERO_B");
  assert.equal(b.board.round, 6);
  assert.equal(b.tier, null, "unknown, not 0");
  assert.equal(b.possible.length, 0);
});

test("a Solo board kept without a seat belongs to the only hero", () => {
  const [only] = words.seatViews({ heroes: [{ id: "X" }], seats: [3], boards: [{ round: 2, minions: [] }], tiers: [] });
  assert.equal(only.board.round, 2);
  assert.equal(only.shown, null, "leaderboard values not recorded");
});

test("leaderboard values not from the log are not shown", () => {
  const [a] = words.seatViews({ ...team, shown_source: "unknown" });
  assert.equal(a.shown, null);
  assert.deepEqual(words.seatViews(null), []);
});

test("when a board was seen, in turns", () => {
  I18n.setLang("en");
  assert.equal(words.seenWords(6, 6), "Turn 6 · this turn");
  assert.equal(words.seenWords(6, 7), "Turn 6 · 1 turn ago");
  assert.equal(words.seenWords(4, 7), "Turn 4 · 3 turns ago");
  assert.equal(words.seenWords(6, null), "Turn 6");
  assert.equal(words.seenWords(null, 6), "No board seen yet");
  I18n.setLang("es");
  assert.equal(words.seenWords(4, 7), "Turno 4 · hace 3 turnos");
  I18n.setLang("en");
});

// Session 059 (T-339, T-340): the Simple side panel and the single-hero hover.

const duoTeam = {
  heroes: [{ id: "HERO_A" }, { id: "HERO_B" }],
  seats: [3, 4],
  tiers: [{ value: 4 }, { value: null }],
  health: { value: 22 },
  next: true,
  shown_source: "log",
  shown: [{ seat: 3, triples: 2, trinkets: [] }, { seat: 4, triples: 1, trinkets: [] }],
};

test("a Simple row has one face per hero with its known tier, no names", () => {
  const row = words.simpleRow(duoTeam);
  assert.deepEqual(row.faces, [{ hero: { id: "HERO_A" }, tier: 4 }, { hero: { id: "HERO_B" }, tier: null }]);
  assert.equal(row.health, 22);
  assert.equal(row.triples, 3, "the team's triples together");
  assert.equal(row.next, true);
  assert.equal(row.out, false);
});

test("a Simple row hides what is unknown instead of showing ?", () => {
  const row = words.simpleRow({ heroes: [], seats: [5], tiers: [], health: { value: null }, shown_source: "unknown", shown: [] });
  assert.deepEqual(row.faces, [{ hero: null, tier: null }]);
  assert.equal(row.health, null);
  assert.equal(row.triples, 0, "no triple count: nothing shown");
  assert.equal(row.next, false);
});

test("a Simple row counts triples only from the log and marks a team that is out", () => {
  assert.equal(words.simpleRow({ ...duoTeam, shown_source: "unknown" }).triples, 0);
  assert.equal(words.simpleRow({ ...duoTeam, shown: [{ seat: 3, triples: null }] }).triples, 0);
  assert.equal(words.simpleRow({ ...duoTeam, health: { value: 0 } }).out, true);
  assert.deepEqual(words.simpleRow(null).faces, []);
});

test("Simple tribes: from the game or entered with no mark, confirmed by the tavern marked inferred", () => {
  assert.deepEqual(words.simpleTribes({ lobby_tribes: { basis: "game", tribes: ["BEAST", "NAGA"], complete: true, source: "game" } }), {
    tribes: ["BEAST", "NAGA"],
    inferred: false,
    count: null,
  });
  assert.deepEqual(words.simpleTribes({ lobby_tribes: { basis: "entered", tribes: ["BEAST"], complete: true, source: "entered" } }).inferred, false);
  assert.deepEqual(words.simpleTribes({ lobby_tribes: { basis: "tavern_confirmed", tribes: ["MECHANICAL", "UNDEAD"], complete: false, source: "inferred" } }), {
    tribes: ["MECHANICAL", "UNDEAD"],
    inferred: true,
    count: 2,
  });
  assert.equal(words.simpleTribes({ lobby_tribes: { basis: "tavern_confirmed", tribes: ["A", "B", "C", "D", "E"], complete: true, source: "inferred" } }).count, null);
});

test("Simple tribes: nothing when only the tavern's offers or nothing is known", () => {
  assert.equal(words.simpleTribes({ lobby_tribes: { basis: "tavern_seen", tribes: ["BEAST"], source: "inferred" } }), null);
  assert.equal(words.simpleTribes({ lobby_tribes: { basis: "unknown", tribes: [], source: "unknown" } }), null);
  assert.equal(words.simpleTribes({ lobby_tribes: { basis: "game", tribes: [], source: "game" } }), null);
  assert.equal(words.simpleTribes(null), null);
});

test("a Duos hover shows only the hovered hero's view", () => {
  const views = words.seatViews({ ...duoTeam, boards: [] });
  assert.deepEqual(words.hoveredViews(views, 0).map((v) => v.seat), [3]);
  assert.deepEqual(words.hoveredViews(views, 1).map((v) => v.seat), [4]);
});

test("without a known portrait the hover keeps the whole team", () => {
  const views = words.seatViews({ ...duoTeam, boards: [] });
  assert.equal(words.hoveredViews(views, null).length, 2);
  assert.equal(words.hoveredViews(views, undefined).length, 2);
  assert.equal(words.hoveredViews(views, 2).length, 2, "out of range");
  const solo = words.seatViews({ heroes: [{ id: "X" }], seats: [3], boards: [], tiers: [] });
  assert.equal(words.hoveredViews(solo, 1).length, 1, "Solo has one hero anyway");
});

test("the damage cap is the game's own number, in its words, or nothing", () => {
  assert.deepEqual(words.damageCap({ damage_cap: 15 }).text, "Max damage 15");
  assert.match(words.damageCap({ damage_cap: 15 }).about, /lost combat/);
  for (const cap of [null, undefined, 0, -1, 1.5, "15"]) assert.equal(words.damageCap({ damage_cap: cap }), null, String(cap));
  assert.equal(words.damageCap(null), null);
  I18n.setLang("es");
  try {
    assert.equal(words.damageCap({ damage_cap: 15 }).text, "Daño máximo 15");
  } finally {
    I18n.setLang("en");
  }
});

"use strict";
const test = require("node:test");
const assert = require("node:assert/strict");
const I18n = require("../ui/i18n.js");
require("../ui/strings/common.js");
require("../ui/strings/overlay.js");
const words = require("../ui/overlay-words.js");

test("a seat's words say its triples and how many trinkets it holds, no names", () => {
  const seat = {
    seat: 2,
    triples: 2,
    trinkets: [
      { slot: "first", dbf_id: 1, card_id: "BG30_MagicItem_1" },
      { slot: "hero_power", dbf_id: 9, card_id: null },
    ],
  };
  assert.deepEqual(words.seatWords(seat), { triples: 2, trinkets: 2 });
  assert.deepEqual(words.seatWords({ triples: 0, trinkets: [] }), { triples: 0, trinkets: 0 });
  assert.deepEqual(words.seatWords({ triples: null }), { triples: 0, trinkets: 0 }, "a count the log never gave is not shown, so it reads as none");
  assert.deepEqual(words.seatWords(null), { triples: 0, trinkets: 0 });
});

test("the overlay has no keyword letters, trinket names or Advanced-only words", () => {
  for (const name of ["keywordMarks", "keywordLegend", "trinketText", "triplesText", "nextOpponent", "KEYWORDS"]) {
    assert.equal(name in words, false, name);
  }
});

test("the marked opponent is the next one in the shop and the current one during its combat", () => {
  const shop = { turn: { value: 6 }, last_combat: { value: 5 }, is_duos: false };
  const combat = { turn: { value: 6 }, last_combat: { value: 6 }, is_duos: true };
  assert.equal(words.nextLabel(shop), "Next");
  assert.equal(words.nextLabel(combat), "Now");
  assert.equal(words.nextLabel({ turn: { value: null }, last_combat: { value: null } }), "Next");
});

test("in Spanish the labels are Spanish, and English comes back", () => {
  I18n.setLang("es");
  try {
    assert.equal(words.nextLabel({ turn: { value: null }, last_combat: { value: null } }), "Siguiente");
  } finally {
    I18n.setLang("en");
  }
});

// The hover board (T-333): each hero of a Duos team with its own board.
const team = {
  heroes: [{ id: "HERO_A" }, { id: "HERO_B" }],
  seats: [5, 8],
  tiers: [{ value: 4, source: "log" }, { value: null, source: "log" }],
  boards: [{ seat: 8, round: 6, minions: [] }],
  shown: [{ seat: 5, triples: 1, trinkets: [] }, { seat: 8, triples: 0, trinkets: [] }],
  shown_source: "log",
};

test("a team's boards go with their own hero, matched by seat", () => {
  const [a, b] = words.seatViews(team);
  assert.equal(a.hero.id, "HERO_A");
  assert.equal(a.board, null, "the first hero was not met: no board, never the teammate's");
  assert.equal(a.tier, 4);
  assert.equal(a.shown.triples, 1);
  assert.equal(b.hero.id, "HERO_B");
  assert.equal(b.board.round, 6);
  assert.equal(b.tier, null, "unknown, not 0");
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
  assert.equal(words.seenWords(null, 6), null, "no round: nothing to say");
  I18n.setLang("es");
  assert.equal(words.seenWords(4, 7), "Turno 4 · hace 3 turnos");
  I18n.setLang("en");
});

// Session 059 (T-339, T-340): the side panel and the single-hero hover.

const duoTeam = {
  heroes: [{ id: "HERO_A" }, { id: "HERO_B" }],
  seats: [3, 4],
  tiers: [{ value: 4 }, { value: null }],
  health: { value: 22 },
  next: true,
  shown_source: "log",
  shown: [{ seat: 3, triples: 2, trinkets: [] }, { seat: 4, triples: 1, trinkets: [] }],
};

test("an opponent row has one face per hero with its known tier, no names", () => {
  const row = words.opponentRow(duoTeam);
  assert.deepEqual(row.faces, [{ hero: { id: "HERO_A" }, tier: 4 }, { hero: { id: "HERO_B" }, tier: null }]);
  assert.equal(row.health, 22);
  assert.equal(row.triples, 3, "the team's triples together");
  assert.equal(row.next, true);
  assert.equal(row.out, false);
});

test("an opponent row hides what is unknown instead of showing ?", () => {
  const row = words.opponentRow({ heroes: [], seats: [5], tiers: [], health: { value: null }, shown_source: "unknown", shown: [] });
  assert.deepEqual(row.faces, [{ hero: null, tier: null }]);
  assert.equal(row.health, null);
  assert.equal(row.triples, 0, "no triple count: nothing shown");
  assert.equal(row.next, false);
});

test("an opponent row counts triples only from the log and marks a team that is out", () => {
  assert.equal(words.opponentRow({ ...duoTeam, shown_source: "unknown" }).triples, 0);
  assert.equal(words.opponentRow({ ...duoTeam, shown: [{ seat: 3, triples: null }] }).triples, 0);
  assert.equal(words.opponentRow({ ...duoTeam, health: { value: 0 } }).out, true);
  assert.deepEqual(words.opponentRow(null).faces, []);
});

test("Lobby tribes: from the game or entered with no mark, confirmed by the tavern marked inferred", () => {
  assert.deepEqual(words.lobbyTribes({ lobby_tribes: { basis: "game", tribes: ["BEAST", "NAGA"], complete: true, source: "game" } }), {
    tribes: ["BEAST", "NAGA"],
    inferred: false,
    count: null,
  });
  assert.deepEqual(words.lobbyTribes({ lobby_tribes: { basis: "entered", tribes: ["BEAST"], complete: true, source: "entered" } }).inferred, false);
  assert.deepEqual(words.lobbyTribes({ lobby_tribes: { basis: "tavern_confirmed", tribes: ["MECHANICAL", "UNDEAD"], complete: false, source: "inferred" } }), {
    tribes: ["MECHANICAL", "UNDEAD"],
    inferred: true,
    count: 2,
  });
  assert.equal(words.lobbyTribes({ lobby_tribes: { basis: "tavern_confirmed", tribes: ["A", "B", "C", "D", "E"], complete: true, source: "inferred" } }).count, null);
});

test("Lobby tribes: nothing when only the tavern's offers or nothing is known", () => {
  assert.equal(words.lobbyTribes({ lobby_tribes: { basis: "tavern_seen", tribes: ["BEAST"], source: "inferred" } }), null);
  assert.equal(words.lobbyTribes({ lobby_tribes: { basis: "unknown", tribes: [], source: "unknown" } }), null);
  assert.equal(words.lobbyTribes({ lobby_tribes: { basis: "game", tribes: [], source: "game" } }), null);
  assert.equal(words.lobbyTribes(null), null);
});

test("a Duos hover shows only the hovered hero's view, the higher player id on top", () => {
  const views = words.seatViews({ ...duoTeam, boards: [] });
  assert.deepEqual(words.hoveredViews(views, 0).map((v) => v.seat), [4]);
  assert.deepEqual(words.hoveredViews(views, 1).map((v) => v.seat), [3]);
});

test("the top portrait is the higher player id whatever order the seats come in", () => {
  // Synthetic Duos team with each hero's own board, seats listed both ways.
  const boards = [
    { seat: 5, round: 4, minions: [{ id: "MINION_LOW" }] },
    { seat: 8, round: 4, minions: [{ id: "MINION_HIGH" }] },
  ];
  const lowFirst = { heroes: [{ id: "HERO_LOW" }, { id: "HERO_HIGH" }], seats: [5, 8], tiers: [], boards };
  const highFirst = { heroes: [{ id: "HERO_HIGH" }, { id: "HERO_LOW" }], seats: [8, 5], tiers: [], boards };
  for (const team of [lowFirst, highFirst]) {
    const views = words.seatViews(team);
    const [top] = words.hoveredViews(views, 0);
    const [bottom] = words.hoveredViews(views, 1);
    assert.equal(top.hero.id, "HERO_HIGH");
    assert.equal(top.board.minions[0].id, "MINION_HIGH");
    assert.equal(bottom.hero.id, "HERO_LOW");
    assert.equal(bottom.board.minions[0].id, "MINION_LOW");
  }
});

test("the top portrait rule leaves the views it was given unchanged", () => {
  const views = words.seatViews({ ...duoTeam, boards: [] });
  words.hoveredViews(views, 0);
  assert.deepEqual(views.map((v) => v.seat), [3, 4]);
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

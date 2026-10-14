"use strict";
const test = require("node:test");
const assert = require("node:assert/strict");
const I18n = require("../ui/i18n.js");
require("../ui/strings/common.js");
require("../ui/strings/overlay.js");
const { oddsView } = require("../ui/odds-words.js");

const odds = (extra) => ({
  seat: 3,
  gap: null,
  seen_round: 6,
  won: 0.62,
  tied: 0.08,
  lost: 0.3,
  trials: 10000,
  dealt: { low: 4, likely: 6, high: 9 },
  taken: { low: 3, likely: 5, high: 7 },
  approximate: [],
  missing: [],
  start_of_combat: false,
  ...extra,
});

test("the odds are the three shares and the board's age, no more", () => {
  I18n.setLang("en");
  const v = oddsView(odds());
  assert.equal(v.kind, "odds");
  assert.deepEqual(v.shares.map((s) => s.text), ["Win 62%", "Tie 8%", "Lose 30%"]);
  assert.deepEqual(v.shares.map((s) => [s.label, s.value]), [["Win", "62%"], ["Tie", "8%"], ["Lose", "30%"]]);
  assert.equal(v.age, "their board seen turn 6");
  assert.equal("lines" in v, false, "no extra lines: damage and card lists are gone with Advanced");
  assert.equal(v.approx, false);
  assert.match(v.said, /^Combat odds\. Win 62%, Tie 8%, Lose 30%\. their board seen turn 6$/);
});

test("a build the rules were not checked against warns and keeps the odds", () => {
  for (const [lang, text] of [
    ["en", "New game build (999999): odds not checked yet"],
    ["es", "Versión nueva del juego (999999): probabilidades sin comprobar aún"],
  ]) {
    I18n.setLang(lang);
    const v = oddsView(odds({ new_build: 999999 }));
    assert.equal(v.kind, "odds");
    assert.equal(v.warning, text);
    assert.equal(v.shares.length, 3);
    assert.ok(v.said.includes(text));
  }
  I18n.setLang("en");
  assert.equal(oddsView(odds({ new_build: null })).warning, null);
  assert.equal(oddsView(odds()).warning, null);
});

test("a reason replaces the numbers, and some reasons say nothing", () => {
  I18n.setLang("en");
  const gap = (g, extra) => oddsView({ ...odds(), gap: g, won: 0, tied: 0, lost: 0, trials: 0, ...extra });
  assert.equal(gap("no_board_seen").text, "Odds: no board of theirs seen yet");
  assert.deepEqual(gap("no_board_seen").shares, []);
  assert.equal(gap("order").text, "Odds: who fights first, or who is on which team, is not known yet");
  assert.equal(gap("not_modelled", { missing: ["SYN_A"] }).text, "Odds: a card can't be simulated");
  assert.equal(gap("no_card_data", { missing: ["SYN_X"] }).said, gap("no_card_data").text, "the missing cards are not listed");
  assert.equal(gap("no_next_opponent"), null);
  assert.equal(gap("not_in_shop"), null);
  assert.equal(oddsView(null), null);
});

test("a share that can happen is never 0%, one that can fail never 100%", () => {
  I18n.setLang("en");
  const v = oddsView(odds({ won: 0.996, tied: 0, lost: 0.004 }));
  assert.deepEqual(v.shares.map((s) => s.value), [">99%", "0%", "<1%"]);
  const sure = oddsView(odds({ won: 1, tied: 0, lost: 0 }));
  assert.deepEqual(sure.shares.map((s) => s.value), ["100%", "0%", "0%"]);
});

test("shares that are not shares are never drawn", () => {
  assert.equal(oddsView(odds({ won: 2 })), null);
  assert.equal(oddsView(odds({ tied: "0.1" })), null);
});

test("Spanish writes the shares its own way", () => {
  I18n.setLang("es");
  const v = oddsView(odds());
  assert.deepEqual(v.shares.map((s) => s.text.replace(/\s/g, " ")), ["Ganar 62 %", "Empatar 8 %", "Perder 30 %"]);
  assert.equal(v.age, "su tablero visto en el turno 6");
  I18n.setLang("en");
});

const duosBoards = (teammate, opponent, second) => [
  { role: "teammate", seat: 2, round: teammate },
  { role: "opponent", seat: 5, round: opponent },
  { role: "opponent_teammate", seat: 6, round: second },
];

test("Duos odds give the oldest board's age", () => {
  I18n.setLang("en");
  const duo = odds({ seen_round: 4, boards: duosBoards(7, 6, 4) });
  const v = oddsView(duo);
  assert.equal(v.age, "oldest board seen turn 4");
  assert.match(v.about, /your teammate's last board/);
  // Solo keeps its words.
  assert.match(oddsView(odds()).about, /^Simulated from your board now against/);
});

test("Duos names the boards never seen instead of a number", () => {
  I18n.setLang("en");
  const missing = (boards) =>
    oddsView({ ...odds(), gap: "missing_board", won: 0, tied: 0, lost: 0, trials: 0, boards });
  const v = missing(duosBoards(null, 6, null));
  assert.equal(v.kind, "note");
  assert.equal(v.text, "Odds: not seen yet: your teammate's board and their second board");
  assert.deepEqual(v.shares, []);
  assert.equal(v.said, v.text);
  // Nothing named as missing: nothing to say rather than an empty list.
  assert.equal(missing(duosBoards(7, 6, 4)), null);
  assert.equal(missing([{ role: "boss", seat: 1, round: null }]), null);
  I18n.setLang("es");
  assert.equal(
    missing(duosBoards(7, null, 4)).text,
    "Probabilidades: aún no has visto su primer tablero",
  );
  assert.equal(oddsView(odds({ seen_round: 4, boards: duosBoards(7, 6, 4) })).age, "tablero más antiguo visto en el turno 4");
  I18n.setLang("en");
});

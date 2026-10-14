"use strict";
const test = require("node:test");
const assert = require("node:assert/strict");
const I18n = require("../ui/i18n.js");
require("../ui/strings/common.js");
require("../ui/strings/overlay.js");
const { oddsView } = require("../ui/odds-words.js");

const names = { SYN_A: "Made-up Minion" };
const nameOf = (id) => names[id] || null;

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

test("simple shows the three shares and the board's age, no more", () => {
  I18n.setLang("en");
  const v = oddsView(odds(), false, nameOf);
  assert.equal(v.kind, "odds");
  assert.deepEqual(v.shares.map((s) => s.text), ["Win 62%", "Tie 8%", "Lose 30%"]);
  assert.deepEqual(v.shares.map((s) => [s.label, s.value]), [["Win", "62%"], ["Tie", "8%"], ["Lose", "30%"]]);
  assert.equal(v.age, "their board seen turn 6");
  assert.deepEqual(v.lines, []);
  assert.equal(v.approx, false);
  assert.match(v.said, /^Combat odds\. Win 62%, Tie 8%, Lose 30%\. their board seen turn 6$/);
});

test("advanced adds the damage both ways, the cards with a gap and the trials", () => {
  I18n.setLang("en");
  const v = oddsView(odds({ approximate: ["SYN_A", "SYN_B"], start_of_combat: true }), true, nameOf);
  assert.deepEqual(v.lines, [
    "You deal 4–9, most likely 6",
    "You take 3–7, most likely 5",
    "Approximate: Made-up Minion, SYN_B",
    "Start of combat effects are not played, and their board may already carry some",
    "10,000 combats simulated",
  ]);
  assert.equal(v.approx, true);
});

test("a build the rules were not checked against warns in both views and keeps the odds", () => {
  for (const [lang, text] of [
    ["en", "New game build (999999): odds not checked yet"],
    ["es", "Versión nueva del juego (999999): probabilidades sin comprobar aún"],
  ]) {
    I18n.setLang(lang);
    for (const advanced of [false, true]) {
      const v = oddsView(odds({ new_build: 999999 }), advanced, nameOf);
      assert.equal(v.kind, "odds");
      assert.equal(v.warning, text);
      assert.equal(v.shares.length, 3);
      assert.ok(v.said.includes(text));
    }
  }
  I18n.setLang("en");
  assert.equal(oddsView(odds({ new_build: null }), false, nameOf).warning, null);
  assert.equal(oddsView(odds(), true, nameOf).warning, null);
});

test("damage is said only for a result that can happen, and unknown is not 0", () => {
  I18n.setLang("en");
  const sure = oddsView(odds({ won: 1, tied: 0, lost: 0, dealt: { low: 5, likely: 5, high: 5 }, taken: null }), true, nameOf);
  assert.deepEqual(sure.lines.slice(0, 1), ["You deal 5"]);
  assert.ok(!sure.lines.some((l) => l.startsWith("You take")));
  const unknown = oddsView(odds({ dealt: null, taken: null }), true, nameOf);
  assert.deepEqual(unknown.lines.slice(0, 2), [
    "Damage you deal: unknown (a tier is not known)",
    "Damage you take: unknown (a tier is not known)",
  ]);
});

test("a reason replaces the numbers, and some reasons say nothing", () => {
  I18n.setLang("en");
  const gap = (g, extra) => oddsView({ ...odds(), gap: g, won: 0, tied: 0, lost: 0, trials: 0, ...extra }, true, nameOf);
  assert.equal(gap("no_board_seen").text, "Odds: no board of theirs seen yet");
  assert.deepEqual(gap("no_board_seen").shares, []);
  assert.equal(gap("order").text, "Odds: who fights first, or who is on which team, is not known yet");
  const missing = gap("not_modelled", { missing: ["SYN_A"] });
  assert.deepEqual(missing.lines, ["Not simulated: Made-up Minion"]);
  assert.deepEqual(gap("no_card_data", { missing: ["SYN_X"] }).lines, ["No card data for: SYN_X"]);
  assert.equal(gap("no_next_opponent"), null);
  assert.equal(gap("not_in_shop"), null);
  assert.equal(oddsView(null, false, nameOf), null);
});

test("a share that can happen is never 0%, one that can fail never 100%", () => {
  I18n.setLang("en");
  const v = oddsView(odds({ won: 0.996, tied: 0, lost: 0.004 }), false, nameOf);
  assert.deepEqual(v.shares.map((s) => s.value), [">99%", "0%", "<1%"]);
  const sure = oddsView(odds({ won: 1, tied: 0, lost: 0 }), false, nameOf);
  assert.deepEqual(sure.shares.map((s) => s.value), ["100%", "0%", "0%"]);
});

test("shares that are not shares are never drawn", () => {
  assert.equal(oddsView(odds({ won: 2 }), false, nameOf), null);
  assert.equal(oddsView(odds({ tied: "0.1" }), false, nameOf), null);
});

test("Spanish writes the shares its own way", () => {
  I18n.setLang("es");
  const v = oddsView(odds(), true, nameOf);
  assert.deepEqual(v.shares.map((s) => s.text.replace(/\s/g, " ")), ["Ganar 62 %", "Empatar 8 %", "Perder 30 %"]);
  assert.equal(v.age, "su tablero visto en el turno 6");
  assert.equal(v.lines[0], "Infliges 4–9, lo más probable 6");
  I18n.setLang("en");
});

test("a long list of cards is cut", () => {
  I18n.setLang("en");
  const many = ["C1", "C2", "C3", "C4", "C5", "C6", "C7", "C8"];
  const v = oddsView(odds({ approximate: many }), true, nameOf);
  assert.ok(v.lines.includes("Approximate: C1, C2, C3, C4, C5, C6, +2"));
});

const duosBoards = (teammate, opponent, second) => [
  { role: "teammate", seat: 2, round: teammate },
  { role: "opponent", seat: 5, round: opponent },
  { role: "opponent_teammate", seat: 6, round: second },
];

test("Duos odds give the oldest board's age, and each board's in Advanced", () => {
  I18n.setLang("en");
  const duo = odds({ seen_round: 4, boards: duosBoards(7, 6, 4) });
  const simple = oddsView(duo, false, nameOf);
  assert.equal(simple.age, "oldest board seen turn 4");
  assert.deepEqual(simple.lines, []);
  assert.match(simple.about, /your teammate's last board/);
  const advanced = oddsView(duo, true, nameOf);
  assert.deepEqual(advanced.lines.slice(2, 5), [
    "Your teammate's board: seen turn 7",
    "Their first board: seen turn 6",
    "Their second board: seen turn 4",
  ]);
  // Solo keeps its words.
  assert.match(oddsView(odds(), false, nameOf).about, /^Simulated from your board now against/);
});

test("Duos names the boards never seen instead of a number", () => {
  I18n.setLang("en");
  const missing = (boards) =>
    oddsView({ ...odds(), gap: "missing_board", won: 0, tied: 0, lost: 0, trials: 0, boards }, false, nameOf);
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
  assert.equal(oddsView(odds({ seen_round: 4, boards: duosBoards(7, 6, 4) }), false, nameOf).age, "tablero más antiguo visto en el turno 4");
  I18n.setLang("en");
});

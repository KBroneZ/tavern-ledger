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

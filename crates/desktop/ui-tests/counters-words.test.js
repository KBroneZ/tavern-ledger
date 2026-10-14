"use strict";
const test = require("node:test");
const assert = require("node:assert/strict");
const I18n = require("../ui/i18n.js");
require("../ui/strings/common.js");
require("../ui/strings/overlay.js");
const counters = require("../ui/counters-words.js");

const KINDS = [
  { kind: "blood_gem", atk: 3, health: 2 },
  { kind: "tavern_spell", atk: 2, health: 0 },
  { kind: "undead_attack", atk: 4 },
  { kind: "eternal_knights", count: 5 },
  { kind: "beetle", atk: 6, health: 7 },
  { kind: "shop_elemental", atk: 4, health: 4 },
  { kind: "shop_all", atk: 1, health: 1 },
  { kind: "shop_tribes", atk: 2, health: 3 },
  { kind: "free_rolls", count: 2 },
  { kind: "gold_next_turn", count: 3 },
];

test.afterEach(() => I18n.setLang("en"));

test("every kind of the contract has a label, a value, a where line and a sentence", () => {
  const rows = counters.rows(KINDS);
  assert.deepEqual(rows.map((r) => r.kind), KINDS.map((k) => k.kind));
  for (const r of rows) {
    for (const part of ["label", "value", "where", "said"]) assert.ok(r[part].length > 0, `${r.kind} ${part}`);
    assert.ok(r.said.endsWith("."), r.kind);
  }
});

test("Simple shows a short label and value", () => {
  const [gem, spell, undead, knights, , , , , rolls, gold] = counters.rows(KINDS);
  assert.deepEqual([gem.label, gem.value], ["Blood Gem", "+3/+2"]);
  assert.deepEqual([spell.label, spell.value], ["Spells", "+2/+0"]);
  assert.deepEqual([undead.label, undead.value], ["Undead", "+4 Attack"]);
  assert.deepEqual([knights.label, knights.value], ["Eternal Knights fallen", "5"]);
  assert.deepEqual([rolls.label, rolls.value], ["Free rolls", "2"]);
  assert.deepEqual([gold.label, gold.value], ["Next turn", "+3 gold"]);
});

test("Advanced says where the game shows it", () => {
  const [gem] = counters.rows(KINDS);
  assert.equal(gem.where, "from your Blood Gems");
});

test("the screen-reader label is a full sentence with the numbers", () => {
  const [gem, , , knights] = counters.rows(KINDS);
  assert.equal(gem.said, "Your Blood Gems give +3/+2.");
  assert.match(knights.said, /5/);
  assert.ok(knights.said.split(" ").length >= 4);
});

test("Spanish has the same kinds, in words of its own", () => {
  const en = counters.rows(KINDS);
  I18n.setLang("es");
  const es = counters.rows(KINDS);
  assert.equal(es.length, en.length);
  es.forEach((r, i) => {
    for (const part of ["label", "where", "said"]) assert.notEqual(r[part], en[i][part], `${r.kind} ${part}`);
  });
  assert.equal(es[0].value, "+3/+2");
  assert.equal(es[2].value, "+4 de ataque");
});

test("an unknown kind is skipped, the known ones stay", () => {
  const rows = counters.rows([{ kind: "from_the_future", atk: 1 }, { kind: "free_rolls", count: 1 }, null, 7, {}]);
  assert.deepEqual(rows.map((r) => r.kind), ["free_rolls"]);
});

test("null and an empty list both show nothing, and not as an error", () => {
  assert.deepEqual(counters.rows(null), []);
  assert.deepEqual(counters.rows(undefined), []);
  assert.deepEqual(counters.rows([]), []);
  assert.equal(counters.hasRows(null), false);
  assert.equal(counters.hasRows([]), false);
  assert.equal(counters.hasRows([{ kind: "free_rolls", count: 1 }]), true);
});

test("a number that is not a whole number is not shown", () => {
  assert.deepEqual(counters.rows([{ kind: "blood_gem", atk: "3", health: 2 }, { kind: "free_rolls", count: 1.5 }]), []);
});

test("a +0/+5 spell buff is kept", () => {
  const [spell] = counters.rows([{ kind: "tavern_spell", atk: 0, health: 5 }]);
  assert.equal(spell.value, "+0/+5");
});

test("figures use the language's own number format", () => {
  I18n.setLang("es");
  const [knights] = counters.rows([{ kind: "eternal_knights", count: 12345 }]);
  assert.equal(knights.value, I18n.num(12345));
});

test("the out-of-range warning has its Spanish words", () => {
  I18n.setLang("es");
  assert.equal(I18n.backend("counter out of range: blood_gem"), "contador fuera de rango: blood_gem");
  I18n.setLang("en");
  assert.equal(I18n.backend("counter out of range: blood_gem"), "counter out of range: blood_gem");
});

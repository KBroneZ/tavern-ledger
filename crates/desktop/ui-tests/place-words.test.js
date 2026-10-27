"use strict";
const test = require("node:test");
const assert = require("node:assert/strict");
const I18n = require("../ui/i18n.js");
require("../ui/strings/common.js");
require("../ui/strings/window.js");
const { placeView } = require("../ui/place-words.js");

const dashed = (dash) => ({ dash, points: [{ turn: 1, mean: null }, { turn: 2, mean: null }], worst: null });

const shown = (worst) => ({
  dash: null,
  points: [
    { turn: 1, mean: 3.25, low: 2, high: 4 },
    { turn: 2, mean: null },
    { turn: 3, mean: 5.5, low: 4, high: 7 },
  ],
  worst,
});

test("while no model ships the curve is one quiet line, in both languages", () => {
  for (const [lang, text] of [
    ["en", "Expected place appears once enough games train the model."],
    ["es", "El puesto esperado aparece cuando haya partidas suficientes para entrenar el modelo."],
  ]) {
    I18n.setLang(lang);
    assert.deepEqual(placeView(dashed("no_model")), { kind: "dash", text });
  }
});

test("each dash says why, and an unknown why reads as no model", () => {
  I18n.setLang("en");
  assert.match(placeView(dashed("other_mode")).text, /other mode/);
  assert.match(placeView(dashed("other_format")).text, /another version/);
  assert.equal(placeView(dashed("too_few_games")).text, placeView(dashed("no_model")).text);
  assert.equal(placeView(dashed("made_up")).text, placeView(dashed("no_model")).text);
});

test("nothing to show is null, never a made-up curve", () => {
  for (const bad of [null, undefined, "x", {}, { dash: "no_model", points: [] }]) assert.equal(placeView(bad), null);
  const broken = shown(null);
  broken.points[0] = { turn: 1, mean: 9, low: 1, high: 8 };
  assert.equal(placeView(broken), null, "a place past 8 is not a place");
  const flipped = shown(null);
  flipped.points[0] = { turn: 1, mean: 3, low: 5, high: 2 };
  assert.equal(placeView(flipped), null);
  assert.equal(placeView({ dash: null, points: [{ turn: 1, mean: null }], worst: null }), null);
});

test("a shown curve keeps each point, says each turn and names what changed with the worst turn", () => {
  I18n.setLang("en");
  const v = placeView(shown({ index: 2, turn: 3, reason: "opponent_attack_sum" }));
  assert.equal(v.kind, "curve");
  assert.deepEqual(v.points[1], { turn: 2, mean: null });
  assert.equal(v.worst, "Turn 3 cost the most expected place; the opponent's total Attack changed most with it.");
  assert.equal(
    v.said,
    "Expected place by turn: turn 1: 3.3, most games 2 to 4, turn 2: no estimate, turn 3: 5.5, most games 4 to 7",
  );
  I18n.setLang("es");
  assert.match(placeView(shown({ index: 2, turn: 3, reason: "health" })).worst, /^El turno 3 .* tu vida\.$/);
});

test("a worst turn without a known reason is still named; one that does not match its point is dropped", () => {
  I18n.setLang("en");
  assert.equal(placeView(shown({ index: 2, turn: 3, reason: null })).worst, "Turn 3 cost the most expected place.");
  assert.equal(placeView(shown({ index: 2, turn: 3, reason: "solo" })).worst, "Turn 3 cost the most expected place.");
  assert.equal(placeView(shown({ index: 0, turn: 3, reason: null })).worst, null);
  assert.equal(placeView(shown(null)).worst, null);
});

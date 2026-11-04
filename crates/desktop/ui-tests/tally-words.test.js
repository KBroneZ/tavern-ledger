"use strict";
// The four numbers under the tabs say "no games yet" before the first game of
// a mode, and keep "unknown" for a value a game left out (T-360).
const test = require("node:test");
const assert = require("node:assert/strict");
const I18n = require("../ui/i18n.js");
require("../ui/strings/common.js");
require("../ui/strings/window.js");
const words = require("../ui/tally-words.js");

const log = { games: "log", average_place: "log", top_half_share: "log", wins: "log" };
const unknown = { games: "unknown", average_place: "unknown", top_half_share: "unknown", wins: "unknown" };

test.beforeEach(() => I18n.setLang("en"));

test("a mode with no games says so once, under the games count, and marks nothing else", () => {
  const marks = words.marks({ games: 0 }, unknown);
  assert.deepEqual(marks, { games: "nogames", average_place: "none", top_half_share: "none", wins: "none" });
});

test("a mode with games keeps the app's source for each number, unknown included", () => {
  const sources = { ...log, average_place: "unknown", top_half_share: "unknown" };
  assert.deepEqual(words.marks({ games: 3 }, sources), sources);
});

test("a number the app has not sent yet is not 'no games'", () => {
  // The window starts with an empty tally until the app answers: that is not a mode with 0 games.
  assert.deepEqual(words.marks({}, undefined), { games: "unknown", average_place: "unknown", top_half_share: "unknown", wins: "unknown" });
  assert.deepEqual(words.marks(undefined, log), log);
});

test("a source the app does not know still reads as unknown", () => {
  assert.equal(words.marks({ games: 2 }, { ...log, wins: "from-the-future" }).wins, "unknown");
});

test("a hero name from the card data carries no 'unknown' mark; one the log or the id gives keeps its mark", () => {
  // The stats' hero rows: "unknown" meant the log did not print the name, but the name shown
  // is the card database's (D-066), so the mark would say the opposite of what is on screen.
  assert.equal(words.heroNameMark("unknown", true), null);
  assert.equal(words.heroNameMark("log", true), null);
  assert.equal(words.heroNameMark("unknown", false), "unknown");
  assert.equal(words.heroNameMark("log", false), "log");
});

test("the hint about the marks shows until the first game, and only then", () => {
  assert.equal(words.hintShown(0), true);
  assert.equal(words.hintShown(1), false);
  assert.equal(words.hintShown(undefined), false);
});

test("the words exist in both languages", () => {
  for (const lang of ["en", "es"]) {
    I18n.setLang(lang);
    for (const key of ["source.nogames.label", "source.nogames.description", "win.hint.marks"]) {
      assert.notEqual(I18n.t(key), key, `${lang} ${key}`);
    }
  }
  I18n.setLang("en");
  assert.equal(I18n.t("source.nogames.label"), "No games yet");
});

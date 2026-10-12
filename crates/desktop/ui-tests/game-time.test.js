"use strict";
const test = require("node:test");
const assert = require("node:assert/strict");
const I18n = require("../ui/i18n.js");
require("../ui/strings/common.js");
require("../ui/strings/window.js");
const time = require("../ui/game-time.js");

const SESSION = "Hearthstone_2026_10_11_02_56_10";

test.beforeEach(() => I18n.setLang("en"));

test("a game with its own clock times shows its own start and length", () => {
  const started = Math.floor(new Date(2026, 9, 11, 3, 41).getTime() / 1000);
  const game = { session: SESSION, index: 2, started, ended: started + 23 * 60 + 20 };
  assert.equal(time.when(game), I18n.dateTime(new Date(2026, 9, 11, 3, 41)));
  assert.ok(!time.when(game).includes("· 2"), "no session number when the game has its own time");
  assert.equal(time.length(game), "23 min");
  assert.equal(time.tip(game), I18n.t("win.recap.subTipGame"));
});

test("a game without its own times shows the session's start and its number", () => {
  const game = { session: SESSION, index: 2, started: null, ended: null };
  assert.equal(time.when(game), `${I18n.dateTime(new Date(2026, 9, 11, 2, 56))} · 2`);
  assert.equal(time.length(game), null);
  assert.equal(time.tip(game), I18n.t("win.recap.subTip"));
});

test("times that do not make a span are not used", () => {
  const backwards = { session: SESSION, index: 1, started: 2000, ended: 1000 };
  assert.ok(time.when(backwards).endsWith("· 1"));
  assert.equal(time.length(backwards), null);
  const text = { session: SESSION, index: 1, started: "2000", ended: 3000 };
  assert.equal(time.length(text), null);
});

test("an unknown session name is shown as it is", () => {
  assert.equal(time.when({ session: "custom", index: 3 }), "custom · 3");
});

test("a game shorter than a minute still reads one minute, in Spanish too", () => {
  I18n.setLang("es");
  const game = { session: SESSION, index: 1, started: 1000, ended: 1010 };
  assert.equal(time.length(game), "1 min");
});

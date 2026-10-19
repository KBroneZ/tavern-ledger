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

// T-359: a game that crosses midnight. Session "Hearthstone_2026_10_09_23_50_00",
// the game starts at 23:58:30 on 9 October and ends at 00:03:10 on 10 October
// (the PC's local time; the app saves both as UTC seconds). The history row and
// the recap both read `when`, so both show the start's day, 9 October.
test("a game across midnight is dated by its start in the row and in the recap", () => {
  const start = new Date(2026, 9, 9, 23, 58, 30);
  const end = new Date(2026, 9, 10, 0, 3, 10);
  const game = { session: "Hearthstone_2026_10_09_23_50_00", index: 1, started: start.getTime() / 1000, ended: end.getTime() / 1000 };
  const row = time.when(game);
  assert.equal(row, I18n.dateTime(start));
  assert.notEqual(row, I18n.dateTime(end), "the day it started, not the day it ended");
  assert.equal(time.length(game), "5 min");
  // The recap's subtitle is built from the same two functions as the row.
  const recapSub = [time.when(game), time.length(game)].join(" · ");
  assert.ok(recapSub.startsWith(row));
});

test("a game written after midnight in a session that began before it is dated the next day", () => {
  const game = { session: "Hearthstone_2026_10_09_23_50_00", index: 2, started: new Date(2026, 9, 10, 0, 20).getTime() / 1000, ended: new Date(2026, 9, 10, 0, 41).getTime() / 1000 };
  assert.equal(time.when(game), I18n.dateTime(new Date(2026, 9, 10, 0, 20)));
});

test("without its own times a game falls back to the session's day, which can be the day before", () => {
  // The session folder says 9 October; the fallback cannot know the game ran past midnight.
  const game = { session: "Hearthstone_2026_10_09_23_50_00", index: 3, started: null, ended: null };
  assert.equal(time.when(game), `${I18n.dateTime(new Date(2026, 9, 9, 23, 50))} · 3`);
});

// T-359: the recap opens from a history row. Its header used to be built from
// the bare { session, index } and so fell back to the session's start, a day
// and hours away from its own row's date.
test("the recap's target keeps the game's own times, so its date is the row's", () => {
  const row = { session: "Hearthstone_2026_10_01_10_00_00", index: 1, started: new Date(2026, 9, 2, 0, 0, 5).getTime() / 1000, ended: new Date(2026, 9, 2, 0, 20).getTime() / 1000, report: {} };
  const target = time.target(row);
  assert.deepEqual(Object.keys(target).sort(), ["ended", "index", "session", "started"], "no report or other bulk");
  assert.equal(time.when(target), time.when(row));
  assert.equal(time.length(target), time.length(row));
  assert.equal(time.tip(target), time.tip(row));
});

test("a game with no own times has a target with none, and still dates by its session", () => {
  const row = { session: "Hearthstone_2026_10_01_10_00_00", index: 2, report: {} };
  const target = time.target(row);
  assert.equal(target.started, null);
  assert.equal(target.ended, null);
  assert.equal(time.when(target), time.when(row));
});

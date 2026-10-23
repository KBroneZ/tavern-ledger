"use strict";
const test = require("node:test");
const assert = require("node:assert/strict");
const I18n = require("../ui/i18n.js");
require("../ui/strings/common.js");
require("../ui/strings/window.js");
const period = require("../ui/period-words.js");

const SOLO = "GT_BATTLEGROUNDS";
const DUOS = "GT_BATTLEGROUNDS_DUO";
const HOUR = 3600 * 1000;

// A fixed zone with named times, so the tests do not depend on this PC's zone:
// UTC+3 (summer time) until the clock goes back to UTC+2 on Sunday 2026-10-25
// at 04:00 local, as in Bulgaria.
const SWITCH = Date.UTC(2026, 9, 25, 1, 0, 0);
const offsetAt = (utcMs) => (utcMs >= SWITCH ? 2 : 3);
const zone = {
  parts: (now) => {
    const local = new Date(now.getTime() + offsetAt(now.getTime()) * HOUR);
    return { y: local.getUTCFullYear(), m: local.getUTCMonth(), d: local.getUTCDate(), weekday: local.getUTCDay() };
  },
  midnight: (y, m, d) => new Date(Date.UTC(y, m, d) - offsetAt(Date.UTC(y, m, d) - 3 * HOUR) * HOUR),
};
// A named local time of that zone ("2026-10-21 12:00:00" is a Wednesday).
const at = (y, m, d, h = 0, min = 0, s = 0) => new Date(Date.UTC(y, m - 1, d, h, min, s) - offsetAt(Date.UTC(y, m - 1, d, h, min, s) - 3 * HOUR) * HOUR);
const utc = (y, m, d, h = 0, min = 0, s = 0) => Date.UTC(y, m - 1, d, h, min, s) / 1000;

test("a Wednesday: today is its local day and the week runs from Monday to the next Monday", () => {
  const r = period.ranges(at(2026, 10, 21, 12), zone);
  assert.equal(r.todayStart, utc(2026, 10, 20, 21), "Wednesday 00:00 local is Tuesday 21:00 UTC");
  assert.equal(r.todayEnd, utc(2026, 10, 21, 21));
  assert.equal(r.weekStart, utc(2026, 10, 18, 21), "Monday 2026-10-19 00:00 local");
  assert.equal(r.weekEnd, utc(2026, 10, 25, 22), "Monday 2026-10-26 00:00 local, after the clock went back");
  assert.equal(r.weekEnd - r.weekStart, 169 * 3600, "the clock change makes this week an hour longer");
});

test("Monday at midnight starts the week, Sunday just before it still belongs to the old one", () => {
  const monday = period.ranges(at(2026, 10, 19, 0, 0, 0), zone);
  assert.equal(monday.weekStart, monday.todayStart, "Monday 00:00:00 is the first second of the week and of the day");
  assert.equal(monday.weekStart, utc(2026, 10, 18, 21));

  const sunday = period.ranges(at(2026, 10, 18, 23, 59, 59), zone);
  assert.equal(sunday.weekStart, utc(2026, 10, 11, 21), "the week before began Monday 2026-10-12");
  assert.equal(sunday.weekEnd, monday.weekStart, "and ends where this one starts");
  assert.equal(sunday.todayEnd, monday.todayStart);
});

test("the Sunday that ends the week is in it, and the next midnight starts another", () => {
  const sunday = period.ranges(at(2026, 10, 25, 23, 59, 59), zone);
  assert.equal(sunday.weekStart, utc(2026, 10, 18, 21));
  assert.equal(sunday.weekEnd, utc(2026, 10, 25, 22));
  const monday = period.ranges(at(2026, 10, 26, 0, 0, 0), zone);
  assert.equal(monday.weekStart, sunday.weekEnd);
  assert.equal(monday.weekEnd - monday.weekStart, 168 * 3600);
});

test("a week that began in the month before still starts on its Monday", () => {
  // Thursday 2026-10-01: Monday was 2026-09-28.
  const r = period.ranges(at(2026, 10, 1, 9), zone);
  assert.equal(r.weekStart, utc(2026, 9, 27, 21));
  assert.equal(r.weekEnd, utc(2026, 10, 4, 21), "Monday 2026-10-05 00:00 local");
});

test("with no zone given it is the viewer's own: local midnights", () => {
  const now = new Date(2026, 9, 21, 15, 30); // a Wednesday
  const r = period.ranges(now);
  assert.equal(r.todayStart, new Date(2026, 9, 21).getTime() / 1000);
  assert.equal(r.todayEnd, new Date(2026, 9, 22).getTime() / 1000);
  assert.equal(r.weekStart, new Date(2026, 9, 19).getTime() / 1000);
  assert.equal(r.weekEnd, new Date(2026, 9, 26).getTime() / 1000);
  assert.equal(r.weekFrom.getDay(), 1, "a Monday");
});

const nameOf = (hero) => hero.name || hero.id;

function mode(over = {}) {
  return {
    summary: {
      mode: SOLO,
      top_half: 4,
      top_half_games: 2,
      tally: { games: 3, placed: 3, incomplete: 0, unsupported: 0, average_place: 3.0, top_half_share: 2 / 3, wins: 1 },
      ...over.summary,
    },
    net_rating: { state: "known", change: 15 },
    revisit: [],
    ...over,
    ...(over.summary ? { summary: { ...mode().summary, ...over.summary } } : {}),
  };
}

test("no games is said as no games, never a row of zeros", () => {
  assert.deepEqual(period.period([], nameOf), { empty: true, modes: [] });
  assert.deepEqual(period.period(null, nameOf), { empty: true, modes: [] });
  const none = mode({ summary: { tally: { games: 0, placed: 0 } } });
  assert.equal(period.period([none], nameOf).empty, true);
});

test("games, average place and record are worded per mode", () => {
  const duos = mode({
    summary: { mode: DUOS, top_half: 2, top_half_games: 1, tally: { games: 2, placed: 2, average_place: 2.5 } },
  });
  const { modes, empty } = period.period([mode(), duos], nameOf);
  assert.equal(empty, false);
  assert.deepEqual(
    modes.map((m) => [m.label, m.games, m.average, m.record]),
    [
      ["Solo", "3 games", "average place 3.0", "2 in the top 4, 1 in the bottom 4"],
      ["Duos", "2 games", "average place 2.5", "1 in the top 2, 1 in the bottom 2"],
    ],
  );
});

test("one mode only gives one row", () => {
  const only = mode({ summary: { mode: DUOS, top_half: 2, top_half_games: 0, tally: { games: 1, placed: 1, average_place: 4 } } });
  const { modes } = period.period([only], nameOf);
  assert.deepEqual(modes.map((m) => m.label), ["Duos"]);
  assert.equal(modes[0].games, "1 game");
});

test("games with no place say how many have one, and a mode with none says no finished game", () => {
  const some = mode({ summary: { tally: { games: 4, placed: 3, average_place: 3.0 } } });
  assert.equal(period.period([some], nameOf).modes[0].games, "4 games · 3 with a place");
  const none = mode({ summary: { top_half_games: 0, tally: { games: 2, placed: 0, average_place: null } } });
  const row = period.period([none], nameOf).modes[0];
  assert.equal(row.average, "average place: no finished game");
  assert.equal(row.record, "record: no finished game");
});

test("the rating change is off, unknown or a number, and net 0 is a number the app knows", () => {
  const net = (state) => period.period([mode({ net_rating: state })], nameOf).modes[0].net;
  assert.deepEqual(net({ state: "off" }), { state: "off", text: "rating change: not shown, the rating option is off", up: null });
  assert.deepEqual(net({ state: "unknown" }), { state: "unknown", text: "rating change: unknown", up: null });
  assert.deepEqual(net({ state: "known", change: 15 }), { state: "known", text: "rating change +15", up: true });
  assert.deepEqual(net({ state: "known", change: -12 }), { state: "known", text: "rating change −12", up: false });
  assert.deepEqual(net({ state: "known", change: 0 }), { state: "known", text: "rating change: net 0", up: null });
});

test("anything the app does not state plainly is unknown, never zero", () => {
  for (const bad of [undefined, null, {}, { state: "known" }, { state: "known", change: "5" }, { state: "known", change: 1.5 }, { state: "weird" }]) {
    assert.equal(period.net(bad).state, "unknown", JSON.stringify(bad));
    assert.ok(!/ 0\b/.test(period.net(bad).text));
  }
});

test("games worth revisiting carry their hero, place and the rating they moved", () => {
  const revisit = [
    { reason: "best_place", session: "Hearthstone_2026_10_21_20_00_00", index: 4, hero: { id: "H_A", name: "Alpha" }, place: 1, change: 40 },
    { reason: "close_loss", session: "Hearthstone_2026_10_21_20_00_00", index: 3, hero: { id: "H_B", name: null }, place: 5, change: null },
    { reason: "big_swing", session: "Hearthstone_2026_10_21_20_00_00", index: 2, hero: null, place: 8, change: -25 },
  ];
  const rows = period.period([mode({ revisit })], nameOf).modes[0].revisit;
  assert.deepEqual(
    rows.map((r) => [r.label, r.index, r.place, r.change, r.up, r.said]),
    [
      ["Best place", 4, 1, "+40", true, "Best place, Alpha, place 1, +40"],
      ["Close loss", 3, 5, null, null, "Close loss, H_B, place 5"],
      ["Big swing", 2, 8, "−25", false, "Big swing, place 8, −25"],
    ],
  );
  assert.equal(rows[0].session, "Hearthstone_2026_10_21_20_00_00");
});

test("a revisit that is not a game the window can open is left out", () => {
  const good = { reason: "best_place", session: "S", index: 1, hero: null, place: 2, change: null };
  const revisit = [
    good,
    { reason: "streak", session: "S", index: 2, place: 1 },
    { reason: "best_place", session: "S", index: 3 },
    { reason: "best_place", index: 4, place: 3 },
    null,
  ];
  const rows = period.period([mode({ revisit })], nameOf).modes[0].revisit;
  assert.deepEqual(rows.map((r) => r.index), [1]);
});

test("no streaks are worded anywhere in the summary", () => {
  const table = I18n.tables().en;
  Object.entries(table)
    .filter(([key]) => key.startsWith("period."))
    .forEach(([key, text]) => assert.ok(!/streak/i.test(text), `${key}: ${text}`));
});

test("the week note says the week starts on Monday and which Monday", () => {
  const r = period.ranges(at(2026, 10, 21, 12), zone);
  assert.match(period.weekNote(r), /^Weeks start on Monday \(this one on .*19.*\)\./);
});

test("it is worded in Spanish too, with the same states", () => {
  I18n.setLang("es");
  try {
    const row = period.period([mode({ net_rating: { state: "off" } })], nameOf).modes[0];
    assert.equal(row.games, "3 partidas");
    assert.equal(row.average, "puesto medio 3,0");
    assert.equal(row.record, "2 en los 4 primeros, 1 en los 4 últimos");
    assert.equal(period.net({ state: "unknown" }).text, "cambio de rating: desconocido");
    assert.equal(period.net({ state: "known", change: 0 }).text, "cambio de rating: neto 0");
  } finally {
    I18n.setLang("en");
  }
});

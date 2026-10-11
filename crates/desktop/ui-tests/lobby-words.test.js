"use strict";
const test = require("node:test");
const assert = require("node:assert/strict");
const I18n = require("../ui/i18n.js");
require("../ui/strings/common.js");
require("../ui/strings/overlay.js");
const lobby = require("../ui/lobby-words.js");

const SOLO = "GT_BATTLEGROUNDS";
const DUOS = "GT_BATTLEGROUNDS_DUO";

function view(over) {
  return {
    in_lobby: true,
    rating: { solo: 6543, duos: 5040, status: "ok" },
    rating_change: null,
    today: [],
    ...over,
  };
}

test("the day is local midnight to the next, in UTC seconds", () => {
  const { dayStart, dayEnd } = lobby.dayRange(new Date(2026, 9, 11, 15, 30));
  assert.equal(dayStart, new Date(2026, 9, 11).getTime() / 1000);
  assert.equal(dayEnd, new Date(2026, 9, 12).getTime() / 1000);
});

test("ratings show only from a read that passed, one row per mode with a rating", () => {
  const rows = lobby.ratingRows(view());
  assert.deepEqual(
    rows.map((r) => [r.label, r.value, r.change]),
    [
      ["Solo", "6543", null],
      ["Duos", "5040", null],
    ],
  );
  assert.deepEqual(lobby.ratingRows(view({ rating: { solo: 6543, duos: null, status: "ok" } })).map((r) => r.label), ["Solo"]);
  for (const status of ["off", "waiting", "unsupported_version", "not_running", "no_access"]) {
    assert.deepEqual(lobby.ratingRows(view({ rating: { solo: 6543, duos: 5040, status } })), [], status);
  }
  assert.deepEqual(lobby.ratingRows(null), []);
});

test("the last game's change goes on its mode only, with a sign", () => {
  const rows = lobby.ratingRows(view({ rating_change: { duos: true, delta: 40 } }));
  assert.equal(rows[0].change, null);
  assert.equal(rows[1].change, "+40");
  assert.equal(rows[1].up, true);
  const down = lobby.ratingRows(view({ rating_change: { duos: false, delta: -125 } }));
  assert.equal(down[0].change, "−125");
  assert.equal(down[0].up, false);
  assert.equal(lobby.signed(0), null);
  assert.equal(lobby.signed("12"), null);
});

test("a rating that cannot be read says why; off is only explained in Advanced", () => {
  const note = (status, advanced) => lobby.ratingNote(view({ rating: { solo: null, duos: null, status } }), advanced);
  assert.equal(note("unsupported_version", false), "Rating: unsupported game version");
  assert.equal(note("no_access", false), "Rating: no read access to the game");
  assert.equal(note("off", false), null);
  assert.match(note("off", true), /reading from the game/);
  assert.equal(note("waiting", true), null);
  assert.equal(note("ok", true), null);
});

test("the day's summary is one line per mode played", () => {
  const today = [
    { mode: SOLO, top_half: 4, top_half_games: 2, tally: { games: 3, placed: 3, average_place: 4.333, top_half_share: 2 / 3 } },
    { mode: DUOS, top_half: 2, top_half_games: 0, tally: { games: 1, placed: 0, average_place: null, top_half_share: null } },
  ];
  assert.deepEqual(lobby.summaryLines(view({ today })), ["Solo today: 3 games · avg 4.3 · top 4: 2 of 3", "Duos today: 1 game"]);
  // The case reported on 2026-10-11: 10 games, one not finished in the log.
  const duos = [{ mode: DUOS, top_half: 2, top_half_games: 6, tally: { games: 10, placed: 9, average_place: 2.111, top_half_share: 6 / 9 } }];
  assert.deepEqual(lobby.summaryLines(view({ today: duos })), ["Duos today: 10 games · 9 with a place · avg 2.1 · top 2: 6 of 9"]);
  assert.deepEqual(lobby.summaryLines(view({ today: [] })), ["No games yet today"]);
  assert.deepEqual(lobby.summaryLines(view({ today: [{ mode: "OTHER", top_half: 4, tally: { games: 2 } }] })), ["No games yet today"]);
});

test("Spanish words and numbers", () => {
  I18n.setLang("es");
  try {
    const rows = lobby.ratingRows(view({ rating_change: { duos: false, delta: 1200 } }));
    assert.deepEqual(
      rows.map((r) => [r.label, r.value, r.change]),
      [
        ["Solo", "6543", "+1200"],
        ["Dúos", "5040", null],
      ],
    );
    const today = [{ mode: SOLO, top_half: 4, top_half_games: 1, tally: { games: 3, placed: 2, average_place: 3.5, top_half_share: 0.5 } }];
    assert.deepEqual(lobby.summaryLines(view({ today })), ["Solo hoy: 3 partidas · 2 con puesto · media 3,5 · top 4: 1 de 2"]);
    assert.deepEqual(lobby.summaryLines(view()), ["Aún no has jugado hoy"]);
  } finally {
    I18n.setLang("en");
  }
});

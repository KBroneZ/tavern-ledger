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
    today: [],
    recap: { games: 0, starting: { solo: null, duos: null }, tiles: [] },
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
    rows.map((r) => [r.label, r.value, r.net]),
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

test("each row has the rating now, the one the day started with and the difference", () => {
  const recap = { games: 2, starting: { solo: 6500, duos: null }, tiles: [] };
  const rows = lobby.ratingRows(view({ recap }));
  assert.deepEqual(
    rows.map((r) => [r.label, r.value, r.start, r.net, r.up]),
    [
      ["Solo", "6543", "6500", "+43", true],
      ["Duos", "5040", null, null, null],
    ],
  );
  assert.equal(rows[0].said, "Solo: 6543, started the day at 6500, +43 today");
  const down = lobby.ratingRows(view({ recap: { ...recap, starting: { solo: 6600, duos: 5040 } } }));
  assert.deepEqual(down.map((r) => [r.net, r.up]), [["−57", false], [null, null]], "no difference is no line");
  assert.equal(lobby.signed(0), null);
  assert.equal(lobby.signed("12"), null);
});

test("tiles are the day's games, the newest first, with only what the app gave", () => {
  const nameOf = (h) => h.name || h.id;
  const tiles = [
    { hero: { id: "H_A", name: "Hero A" }, duos: false, place: 3, change: 22 },
    { hero: { id: "H_B", name: null }, duos: true, place: 1, change: -15 },
    { hero: null, duos: false, place: null, change: null },
  ];
  const shown = lobby.recapTiles(view({ recap: { games: 3, starting: {}, tiles } }), nameOf);
  assert.deepEqual(shown.map((t) => t.hero && t.hero.id), [null, "H_B", "H_A"]);
  assert.deepEqual(shown.map((t) => [t.place, t.change, t.up]), [[null, null, null], [1, "−15", false], [3, "+22", true]]);
  assert.equal(shown[2].said, "Solo, Hero A, place 3, rating +22");
  assert.equal(shown[1].said, "Duos, H_B, place 1, rating −15");
  assert.equal(shown[0].said, "Solo, no place recorded", "unknown is not zero");
  assert.deepEqual(lobby.recapTiles(view(), nameOf), []);
  assert.deepEqual(lobby.recapTiles(null, nameOf), []);
  assert.deepEqual(lobby.recapTiles({ recap: { tiles: [{ duos: "no" }, null] } }, nameOf), [], "a malformed tile is skipped");
});

test("a rating that cannot be read says why; off says nothing", () => {
  const note = (status) => lobby.ratingNote(view({ rating: { solo: null, duos: null, status } }));
  assert.equal(note("unsupported_version"), "Rating: unsupported game version");
  assert.equal(note("no_access"), "Rating: no read access to the game");
  assert.equal(note("off"), null);
  assert.equal(note("waiting"), null);
  assert.equal(note("ok"), null);
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
    const recap = { games: 1, starting: { solo: 5343, duos: null }, tiles: [{ hero: { id: "H" }, duos: true, place: 2, change: 12 }] };
    const rows = lobby.ratingRows(view({ recap }));
    assert.deepEqual(
      rows.map((r) => [r.label, r.value, r.start, r.net]),
      [
        ["Solo", "6543", "5343", "+1200"],
        ["Dúos", "5040", null, null],
      ],
    );
    assert.equal(rows[0].said, "Solo: 6543, empezó el día en 5343, +1200 hoy");
    assert.equal(lobby.recapTiles(view({ recap }), (h) => h.id)[0].said, "Dúos, H, puesto 2, puntuación +12");
    const today = [{ mode: SOLO, top_half: 4, top_half_games: 1, tally: { games: 3, placed: 2, average_place: 3.5, top_half_share: 0.5 } }];
    assert.deepEqual(lobby.summaryLines(view({ today })), ["Solo hoy: 3 partidas · 2 con puesto · media 3,5 · top 4: 1 de 2"]);
    assert.deepEqual(lobby.summaryLines(view()), ["Aún no has jugado hoy"]);
  } finally {
    I18n.setLang("en");
  }
});

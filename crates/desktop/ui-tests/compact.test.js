"use strict";
const test = require("node:test");
const assert = require("node:assert/strict");
const TLCompact = require("../ui/compact.js");
const I18n = require("../ui/i18n.js");
require("../ui/strings/common.js");
require("../ui/strings/details.js");

const hero = (name, games, place) => ({ name, games, place });
const byGames = { games: (r) => r.games, place: (r) => r.place, name: (r) => r.name };

test("rows rank by games, then better average place, then name", () => {
  const rows = [hero("Cleo", 2, 3.5), hero("Ana", 2, 3.5), hero("Bo", 5, 4), hero("Dee", 2, 1.5), hero("Eve", 1, 1)];
  assert.deepEqual(TLCompact.rank(rows, byGames).map((r) => r.name), ["Bo", "Dee", "Ana", "Cleo", "Eve"]);
});

test("an unknown average place ranks after a known one", () => {
  const rows = [hero("Ana", 2, null), hero("Bo", 2, 4.5)];
  assert.deepEqual(TLCompact.rank(rows, byGames).map((r) => r.name), ["Bo", "Ana"]);
});

test("ranking does not change the list it was given", () => {
  const rows = [hero("Ana", 1, 2), hero("Bo", 3, 2)];
  TLCompact.rank(rows, byGames);
  assert.deepEqual(rows.map((r) => r.name), ["Ana", "Bo"]);
});

test("names are compared without case or accents", () => {
  const rows = [hero("señora", 1, 2), hero("Senna", 1, 2), hero("Sam", 1, 2)];
  assert.deepEqual(TLCompact.rank(rows, byGames).map((r) => r.name), ["Sam", "Senna", "señora"]);
});

const many = (n) => Array.from({ length: n }, (_, i) => hero(`H${String(i).padStart(2, "0")}`, n - i, 2));

test("a long list shows the top rows and a button for the rest", () => {
  const v = TLCompact.view({ rows: many(20), limit: 6, expanded: false });
  assert.equal(v.rows.length, 6);
  assert.equal(v.rows[0].name, "H00");
  assert.deepEqual(v.button, { text: "Show all (20)", expanded: false });
});

test("expanded, every row shows and the button folds them back", () => {
  const v = TLCompact.view({ rows: many(20), limit: 6, expanded: true });
  assert.equal(v.rows.length, 20);
  assert.deepEqual(v.button, { text: "Show top 6", expanded: true });
});

test("a short list needs no button", () => {
  const v = TLCompact.view({ rows: many(6), limit: 6, expanded: false });
  assert.equal(v.rows.length, 6);
  assert.equal(v.button, null);
});

test("one row over the limit still gets the button (no surprise rows)", () => {
  const v = TLCompact.view({ rows: many(7), limit: 6, expanded: false });
  assert.equal(v.rows.length, 6);
  assert.equal(v.button.text, "Show all (7)");
});

const columns = [
  { label: "Hero", sort: (r) => r.name, text: true },
  { label: "Games", sort: (r) => r.games },
  { label: "Avg place", sort: (r) => r.place, low: true },
];

test("sorting by a column happens before the top rows are cut", () => {
  const rows = [hero("Ana", 9, 4.5), hero("Bo", 1, 1.0), ...many(10)];
  const v = TLCompact.view({ rows, limit: 3, expanded: false, columns, sort: { column: 2, dir: "best" } });
  assert.equal(v.rows[0].name, "Bo", "best average place first");
  assert.equal(v.rows.length, 3);
});

test("a number column's best is high, a place's best is low; unknown values go last", () => {
  const rows = [hero("Ana", 3, null), hero("Bo", 1, 2), hero("Cy", 2, 3)];
  const games = TLCompact.view({ rows, limit: 8, columns, sort: { column: 1, dir: "best" } });
  assert.deepEqual(games.rows.map((r) => r.name), ["Ana", "Cy", "Bo"]);
  const place = TLCompact.view({ rows, limit: 8, columns, sort: { column: 2, dir: "best" } });
  assert.deepEqual(place.rows.map((r) => r.name), ["Bo", "Cy", "Ana"]);
  const worst = TLCompact.view({ rows, limit: 8, columns, sort: { column: 2, dir: "worst" } });
  assert.deepEqual(worst.rows.map((r) => r.name), ["Cy", "Bo", "Ana"]);
});

test("text columns sort A to Z first", () => {
  const rows = [hero("Cy", 1, 1), hero("ana", 1, 1), hero("Bo", 1, 1)];
  const v = TLCompact.view({ rows, limit: 8, columns, sort: { column: 0, dir: "best" } });
  assert.deepEqual(v.rows.map((r) => r.name), ["ana", "Bo", "Cy"]);
});

test("each sortable heading says how it is sorted, for aria-sort", () => {
  const v = TLCompact.view({ rows: many(3), limit: 8, columns, sort: { column: 2, dir: "best" } });
  assert.deepEqual(v.headings.map((h) => h.ariaSort), ["none", "none", "ascending"]);
  const games = TLCompact.view({ rows: many(3), limit: 8, columns, sort: { column: 1, dir: "best" } });
  assert.equal(games.headings[1].ariaSort, "descending");
  const none = TLCompact.view({ rows: many(3), limit: 8, columns });
  assert.deepEqual(none.headings.map((h) => h.ariaSort), ["none", "none", "none"]);
});

test("pressing a heading sorts best first, then worst first, then back to the default", () => {
  assert.deepEqual(TLCompact.nextSort(null, 1), { column: 1, dir: "best" });
  assert.deepEqual(TLCompact.nextSort({ column: 1, dir: "best" }, 1), { column: 1, dir: "worst" });
  assert.equal(TLCompact.nextSort({ column: 1, dir: "worst" }, 1), null);
  assert.deepEqual(TLCompact.nextSort({ column: 1, dir: "worst" }, 2), { column: 2, dir: "best" });
});

test("the state of each list is kept by its key across redraws", () => {
  const state = TLCompact.state("heroes:GT_BATTLEGROUNDS");
  state.expanded = true;
  assert.equal(TLCompact.state("heroes:GT_BATTLEGROUNDS").expanded, true);
  assert.equal(TLCompact.state("heroes:GT_BATTLEGROUNDS_DUO").expanded, false);
});

test("in Spanish the button and the sort order follow the language", () => {
  I18n.setLang("es");
  try {
    assert.equal(TLCompact.view({ rows: many(20), limit: 6 }).button.text, "Mostrar todo (20)");
    assert.equal(TLCompact.view({ rows: many(20), limit: 6, expanded: true }).button.text, "Mostrar las 6 primeras");
    const names = [hero("Zoe", 1, 2), hero("Ñandú", 1, 2), hero("Nora", 1, 2), hero("Óscar", 1, 2)];
    assert.deepEqual(TLCompact.rank(names, byGames).map((r) => r.name), ["Nora", "Ñandú", "Óscar", "Zoe"]);
  } finally {
    I18n.setLang("en");
  }
  assert.equal(TLCompact.view({ rows: many(20), limit: 6 }).button.text, "Show all (20)");
});

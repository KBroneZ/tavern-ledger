// The games list's filters (T-503): the address is the state; anything not
// valid is left out and named, and the conditions match the view's columns.
import { test } from "node:test";
import assert from "node:assert/strict";
import { conditions, DEFAULT_FILTERS, filtersQuery, pageCount, pageRange, parseFilters } from "../../src/lib/data/filters.ts";

const ID = "0f8fad5b-d9cb-469f-a165-70867728950e";

test("no parameters: every game, newest first, page 1", () => {
  assert.deepEqual(parseFilters(""), { filters: DEFAULT_FILTERS, invalid: [] });
  assert.deepEqual(conditions(DEFAULT_FILTERS), []);
});

test("every filter read from the address", () => {
  const { filters, invalid } = parseFilters("?mode=duos&hero=BG20_HERO_202&from=2026-10-01&to=2026-10-31&place=top&page=3");
  assert.deepEqual(invalid, []);
  assert.deepEqual(filters, { mode: "duos", hero: "BG20_HERO_202", from: "2026-10-01", to: "2026-10-31", place: "top", page: 3 });
  assert.deepEqual(conditions(filters), [
    { column: "mode", op: "eq", value: "duos" },
    { column: "hero", op: "eq", value: "BG20_HERO_202" },
    { column: "played_on", op: "gte", value: "2026-10-01" },
    { column: "played_on", op: "lte", value: "2026-10-31" },
    { column: "top_half", op: "eq", value: true },
  ]);
});

test("the place filters: 1st, top half, bottom half and no place", () => {
  const of = (place: string) => conditions(parseFilters(`?place=${place}`).filters);
  assert.deepEqual(of("first"), [{ column: "counted_place", op: "eq", value: 1 }]);
  assert.deepEqual(of("bottom"), [{ column: "top_half", op: "eq", value: false }]);
  assert.deepEqual(of("none"), [{ column: "counted_place", op: "is", value: null }]);
  assert.deepEqual(of("any"), []);
});

test("values that are not valid are left out and named, never guessed", () => {
  const { filters, invalid } = parseFilters("?mode=arena&hero=<b>&from=2026-13-01&place=2&page=0");
  assert.deepEqual(filters, DEFAULT_FILTERS);
  assert.deepEqual(invalid, ["mode", "hero", "from", "place", "page"]);
  assert.deepEqual(parseFilters("?page=401").invalid, ["page"]);
  assert.deepEqual(parseFilters("?page=1e3").invalid, ["page"]);
});

test("a date range that ends before it starts is left out", () => {
  const { filters, invalid } = parseFilters("?from=2026-10-10&to=2026-10-01");
  assert.equal(filters.from, null);
  assert.equal(filters.to, null);
  assert.deepEqual(invalid, ["from", "to"]);
});

test("empty form fields count as not set (a GET form sends them all)", () => {
  assert.deepEqual(parseFilters("?mode=all&hero=&from=&to=&place=any"), { filters: DEFAULT_FILTERS, invalid: [] });
});

test("filtersQuery writes only what differs from the defaults, with the profile id first", () => {
  assert.equal(filtersQuery(DEFAULT_FILTERS), "");
  assert.equal(filtersQuery({ ...DEFAULT_FILTERS, mode: "solo", page: 2 }, ID), `?id=${ID}&mode=solo&page=2`);
  const round = parseFilters(filtersQuery({ mode: "duos", hero: "X_1", from: "2026-01-01", to: "2026-02-01", place: "none", page: 4 }));
  assert.deepEqual(round, { filters: { mode: "duos", hero: "X_1", from: "2026-01-01", to: "2026-02-01", place: "none", page: 4 }, invalid: [] });
});

test("pages of 25 games", () => {
  assert.deepEqual(pageRange(1), { from: 0, to: 24 });
  assert.deepEqual(pageRange(3), { from: 50, to: 74 });
  assert.equal(pageCount(0), 1);
  assert.equal(pageCount(25), 1);
  assert.equal(pageCount(26), 2);
});

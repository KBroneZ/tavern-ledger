"use strict";
const test = require("node:test");
const assert = require("node:assert/strict");
const path = require("node:path");
const { overlap, findOverlaps, collect } = require(path.join(__dirname, "..", "..", "..", "tools", "overlay-dev", "overlaps.js"));

const r = (name, x, y, w, h) => ({ name, x, y, w, h });

test("rectangles that share an area overlap, and the shared size is reported", () => {
  assert.deepEqual(overlap(r("a", 0, 0, 100, 100), r("b", 90, 80, 100, 100)), { w: 10, h: 20 });
});

test("rectangles that only touch, or are apart, do not overlap", () => {
  assert.equal(overlap(r("a", 0, 0, 100, 100), r("b", 100, 0, 50, 50)), null);
  assert.equal(overlap(r("a", 0, 0, 100, 100), r("b", 0, 100, 50, 50)), null);
  assert.equal(overlap(r("a", 0, 0, 10, 10), r("b", 500, 500, 10, 10)), null);
});

test("a hair of rounding is not an overlap", () => {
  assert.equal(overlap(r("a", 0, 0, 100.3, 100), r("b", 100, 0, 50, 50)), null);
});

test("every pair is checked and named", () => {
  const found = findOverlaps([r("odds", 690, 11, 540, 52), r("hover-card", 501, 8, 918, 273), r("tribes", 1719, 96, 193, 54)]);
  assert.deepEqual(found, [{ a: "odds", b: "hover-card", w: 540, h: 52 }]);
  assert.deepEqual(findOverlaps([]), []);
});

test("collect reads the visible slots, the hover card and the marks, not the hidden ones", () => {
  const box = (left, top, width, height) => ({ getBoundingClientRect: () => ({ left, top, width, height }) });
  const slot = (panel, hidden, rect) => ({ ...rect, hidden, dataset: { panel } });
  const nodes = {
    ".slot": [slot("odds", false, box(690, 11, 540, 52)), slot("tribes", true, box(1719, 96, 193, 54)), slot("legend", false, box(0, 0, 0, 0))],
    ".opp-mark": [box(179, 207, 63, 24), box(224, 849, 18, 24)],
  };
  const doc = {
    querySelectorAll: (selector) => nodes[selector],
    getElementById: () => ({ ...box(501, 8, 918, 273), hidden: true }),
  };
  const names = collect(doc).map((x) => x.name);
  assert.deepEqual(names, ["odds", "mark 1", "mark 2"]);
});

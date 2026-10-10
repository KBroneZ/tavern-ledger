"use strict";
const test = require("node:test");
const assert = require("node:assert/strict");
const TLScrollCue = require("../ui/scroll-cue.js");

test("at the top of a long page there is more below, nothing above", () => {
  assert.deepEqual(TLScrollCue.cue(0, 700, 3000), { above: false, below: true });
});

test("in the middle there is more on both sides", () => {
  assert.deepEqual(TLScrollCue.cue(900, 700, 3000), { above: true, below: true });
});

test("at the bottom (within a pixel or two of rounding) nothing is below", () => {
  assert.deepEqual(TLScrollCue.cue(2299, 700, 3000), { above: true, below: false });
  assert.deepEqual(TLScrollCue.cue(2300, 700, 3000), { above: true, below: false });
});

test("a page that fits the window has no cue at all", () => {
  assert.deepEqual(TLScrollCue.cue(0, 700, 700), { above: false, below: false });
  assert.deepEqual(TLScrollCue.cue(0, 700, 650), { above: false, below: false });
});

test("the scrollbar shows near the window's right edge only", () => {
  assert.equal(TLScrollCue.nearEdge(1270, 1280), true);
  assert.equal(TLScrollCue.nearEdge(1200, 1280), false);
});

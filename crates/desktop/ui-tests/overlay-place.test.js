"use strict";
const test = require("node:test");
const assert = require("node:assert/strict");
const { overlaps, spot, padSpot, MARGIN } = require("../ui/overlay-place.js");

const screen = { w: 1920, h: 1080 };
const toolbar = { w: 960, h: 150 };
const hint = { w: 520, h: 34 };
const at = (p, size) => ({ ...p, ...size });
const inside = (r) => r.x >= 0 && r.y >= 0 && r.x + r.w <= screen.w && r.y + r.h <= screen.h;

// A panel against each edge of the screen, in the middle of that edge.
const EDGES = {
  top: { x: 790, y: 0, w: 340, h: 260 },
  bottom: { x: 790, y: 820, w: 340, h: 260 },
  left: { x: 0, y: 300, w: 340, h: 480 },
  right: { x: 1580, y: 300, w: 340, h: 480 },
  "top left": { x: 0, y: 0, w: 340, h: 200 },
  "top right": { x: 1580, y: 0, w: 340, h: 200 },
  "bottom left": { x: 0, y: 880, w: 340, h: 200 },
  "bottom right": { x: 1580, y: 880, w: 340, h: 200 },
};

test("overlap is strict: touching edges do not overlap", () => {
  const a = { x: 0, y: 0, w: 10, h: 10 };
  assert.ok(overlaps(a, { x: 5, y: 5, w: 10, h: 10 }));
  assert.ok(!overlaps(a, { x: 10, y: 0, w: 10, h: 10 }));
  assert.ok(!overlaps(a, { x: 0, y: 10, w: 10, h: 10 }));
});

test("with nothing in the way the toolbar sits at the top centre and the hint at the bottom centre", () => {
  assert.deepEqual(spot(toolbar, screen, [], "top"), { x: 480, y: MARGIN });
  assert.deepEqual(spot(hint, screen, [], "bottom"), { x: 700, y: 1080 - 34 - MARGIN });
});

for (const [edge, panel] of Object.entries(EDGES)) {
  test(`the toolbar and the hint never cover a panel at the ${edge} edge`, () => {
    const bar = at(spot(toolbar, screen, [panel], "top"), toolbar);
    assert.ok(!overlaps(bar, panel) && inside(bar), JSON.stringify(bar));
    const line = at(spot(hint, screen, [panel, bar], "bottom"), hint);
    assert.ok(!overlaps(line, panel) && !overlaps(line, bar) && inside(line), JSON.stringify(line));
  });
}

test("with a panel at every edge at once both still find room", () => {
  const panels = [EDGES.top, EDGES.bottom, EDGES.left, EDGES.right];
  const bar = at(spot(toolbar, screen, panels, "top"), toolbar);
  for (const p of panels) assert.ok(!overlaps(bar, p));
  const line = at(spot(hint, screen, [...panels, bar], "bottom"), hint);
  for (const p of [...panels, bar]) assert.ok(!overlaps(line, p));
});

test("when no spot is free the one covering least is used, still on screen", () => {
  const wall = [{ x: 0, y: 0, w: 1920, h: 1000 }];
  const bar = at(spot(toolbar, screen, wall, "top"), toolbar);
  assert.ok(inside(bar));
  assert.equal(bar.y, 1080 - 150 - MARGIN, "the bottom strip is the least covered");
});

test("a toolbar wider than the screen starts at the left margin", () => {
  assert.equal(spot({ w: 2000, h: 100 }, screen, [], "top").x, MARGIN);
});

test("the button pad sits next to its panel, never on it", () => {
  const pad = { w: 200, h: 70 };
  for (const [edge, panel] of Object.entries(EDGES)) {
    const p = at(padSpot(pad, panel, screen), pad);
    assert.ok(!overlaps(p, panel) && inside(p), `${edge}: ${JSON.stringify(p)}`);
  }
  const below = padSpot(pad, { x: 500, y: 100, w: 300, h: 200 }, screen);
  assert.deepEqual(below, { x: 500, y: 300 + MARGIN }, "under the panel when there is room");
});

test("a panel as big as the screen gets the pad at its bottom, inside the screen", () => {
  const pad = { w: 200, h: 70 };
  const p = at(padSpot(pad, { x: 0, y: 0, w: 1920, h: 1080 }, screen), pad);
  assert.ok(inside(p));
});

test("the button pad also keeps off the other panels when there is room", () => {
  const pad = { w: 300, h: 90 };
  // The default stack at the right edge: the selected panel has one above and one below.
  const status = { x: 1572, y: 96, w: 340, h: 60 };
  const tribes = { x: 1572, y: 162, w: 340, h: 120 };
  const opponents = { x: 1572, y: 288, w: 340, h: 300 };
  const p = at(padSpot(pad, tribes, screen, [status, opponents]), pad);
  for (const r of [status, tribes, opponents]) assert.ok(!overlaps(p, r), JSON.stringify(p));
  assert.ok(inside(p));
  assert.equal(p.x + p.w + MARGIN, tribes.x, "to its left, the only side with room");
});

test("with no free side the pad still keeps off its own panel", () => {
  const pad = { w: 300, h: 90 };
  const target = { x: 600, y: 400, w: 300, h: 200 };
  const walls = [{ x: 0, y: 0, w: 1920, h: 400 }, { x: 0, y: 600, w: 1920, h: 480 }, { x: 0, y: 400, w: 600, h: 200 }, { x: 900, y: 400, w: 1020, h: 200 }];
  const p = at(padSpot(pad, target, screen, walls), pad);
  assert.ok(!overlaps(p, target) && inside(p));
});

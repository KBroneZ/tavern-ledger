"use strict";
const test = require("node:test");
const assert = require("node:assert/strict");
const { rootPx, cardPx, turnsAgo, playHeight, MIN_PX } = require("../ui/overlay-scale.js");

// The two screens the user asked about (T-334): the overlay window covers the
// game's picture, here full screen, so the monitor's work area (taskbar left
// out), in its own pixels.
const SCREENS = [
  { name: "3840x2160 at 150%", height: 2160, scale: 1.5, work: 2112 / 1.5 },
  { name: "1920x1080 at 100%", height: 1080, scale: 1, work: 1040 },
];
// The board's width: 85% of the game's height (hover.rs, CARD_SHARE), the
// game full screen.
const boardWidth = (s) => (0.85 * s.height) / s.scale;

for (const s of SCREENS) {
  test(`at ${s.name} body text is at least 1.25% of the screen's height`, () => {
    const physical = rootPx(s.work) * s.scale;
    assert.ok(physical / s.height >= 0.0125, `${physical} physical px`);
    assert.ok(rootPx(s.work) >= MIN_PX);
  });

  test(`at ${s.name} the hover board's text is bigger than the body text`, () => {
    const physical = cardPx(boardWidth(s)) * s.scale;
    assert.ok(physical / s.height >= 0.0135, `${physical} physical px`);
    assert.ok(cardPx(boardWidth(s)) >= rootPx(s.work));
  });
}

test("the same share of the screen at both, within a pixel", () => {
  const [uhd, fhd] = SCREENS;
  const share = (s) => (rootPx(s.work) * s.scale) / s.height;
  assert.ok(Math.abs(share(uhd) - share(fhd)) * 1080 < 1);
});

test("a tiny or broken size never goes under the minimum", () => {
  for (const bad of [0, -5, NaN, Infinity, undefined, null, "900"]) {
    assert.equal(rootPx(bad), MIN_PX);
    assert.equal(cardPx(bad), MIN_PX);
  }
  assert.equal(rootPx(400), MIN_PX);
});

test("turns ago: from the turn now and the combat the board was seen in", () => {
  assert.equal(turnsAgo(7, 7), 0);
  assert.equal(turnsAgo(9, 6), 3);
  assert.equal(turnsAgo(null, 6), null);
  assert.equal(turnsAgo(5, null), null);
  assert.equal(turnsAgo(5, 6), null, "never a negative number");
});

// The game's picture is 16:9; in a wider or taller window it is centred with bars.
test("the play height is the window's own when it is 16:9 or wider than that", () => {
  assert.equal(playHeight(1920, 1080), 1080);
  assert.equal(playHeight(3440, 1440), 1440);
});

test("in a window taller than 16:9 the play height is the picture's, cut by its width", () => {
  assert.equal(playHeight(1024, 768), 576);
  assert.equal(playHeight(1440, 1080), 810);
});

test("a window with no usable size has no play height of its own", () => {
  for (const bad of [[0, 0], [NaN, 1080], [1920, -1], [undefined, undefined], ["1920", "1080"]]) {
    assert.equal(playHeight(...bad), null);
  }
});

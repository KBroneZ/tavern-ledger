// The default places never overlap: by their columns, so it
// holds whatever the panels hold. From the left: the marks and the game's
// leaderboard, the lobby panel (lobby screen only), the hover card and the odds
// bar at the centre, the default stack at the right. The shares come from the
// app's Rust source (through default-layout.js) and from overlay.css; the
// rendered rectangles are checked in the workbench ("Check overlaps").
"use strict";
const test = require("node:test");
const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const { rootPx, playHeight } = require("../ui/overlay-scale.js");
const { clearOf, BAR_GAP_REM } = require("../ui/overlay-place.js");
const defaults = require("../../../tools/overlay-gallery/default-layout.js");

const css = fs.readFileSync(path.join(__dirname, "..", "ui", "overlay.css"), "utf8");
function cssNumber(pattern, name) {
  const found = css.match(pattern);
  assert.ok(found, `overlay.css has no ${name} in the form this test reads`);
  return Number(found[1]);
}
const share = (name) => cssNumber(new RegExp(name + String.raw`: calc\([^;]*\* ([0-9.]+)\);`), name);
const rem = (name) => cssNumber(new RegExp(`${name}: ([0-9.]+)rem;`), name);
const STACK = { right: share("--stack-right"), width: rem("--stack-width") };
const LOBBY = { left: share("--lobby-left"), width: rem("--lobby-width") };

// The overlay window sizes the task names, the game's 16:9 picture centred in
// the wider ones, and a 4:3 window.
const SIZES = [[1920, 1080], [2560, 1440], [1280, 720], [3440, 1440], [1024, 768]];
// What the card keeps from the bar, as the page does.
const gapOf = (px) => px * BAR_GAP_REM;

function places(w, h) {
  const play = defaults.playArea(w, h);
  const px = rootPx(playHeight(w, h));
  const card = defaults.hoverCard(w, h);
  const board = defaults.leaderboardArea(w, h);
  const stackRight = w - (play.x + STACK.right * play.h);
  return {
    play,
    px,
    card: { x: card.x, y: card.y, w: card.width },
    odds: defaults.oddsRect(w, h),
    board,
    stack: { left: stackRight - STACK.width * px, right: stackRight },
    lobby: { left: play.x + LOBBY.left * play.h, right: play.x + LOBBY.left * play.h + LOBBY.width * px },
  };
}

for (const [w, h] of SIZES) {
  test(`${w}x${h}: the hover card is clear of the leaderboard on its left and the stack on its right`, () => {
    const p = places(w, h);
    assert.ok(p.card.x >= p.board.x + p.board.w, `card ${p.card.x} over the leaderboard ending ${p.board.x + p.board.w}`);
    assert.ok(p.card.x + p.card.w <= p.stack.left, `card ending ${p.card.x + p.card.w} over the stack at ${p.stack.left}`);
  });

  test(`${w}x${h}: the odds bar is clear of the leaderboard and the stack`, () => {
    const p = places(w, h);
    assert.ok(p.odds.x >= p.board.x + p.board.w);
    assert.ok(p.odds.x + p.odds.w <= p.stack.left);
  });

  test(`${w}x${h}: the lobby panel's corner is clear of the hover card and the odds bar`, () => {
    const p = places(w, h);
    assert.ok(p.lobby.right <= p.card.x, `lobby ending ${p.lobby.right}, card at ${p.card.x}`);
    assert.ok(p.lobby.right <= p.odds.x);
  });

  test(`${w}x${h}: the hover card goes just below the odds bar, whatever its height`, () => {
    const p = places(w, h);
    for (const barHeight of [0.03, 0.05, 0.09].map((share) => share * p.play.h)) {
      const bar = { x: p.odds.x, y: p.odds.y, w: p.odds.w, h: barHeight };
      const top = clearOf(bar, p.card, gapOf(p.px));
      assert.ok(top >= bar.y + bar.h, `card at ${top} over a bar ending ${bar.y + bar.h}`);
      assert.ok(top - (bar.y + bar.h) <= gapOf(p.px) + 1e-9, "just below, not far below");
    }
  });
}

test("the default stack and lobby shares are read from overlay.css", () => {
  assert.ok(STACK.right > 0 && STACK.right < 0.05);
  assert.ok(STACK.width > 8 && STACK.width < 20);
  assert.ok(LOBBY.left > 0 && LOBBY.width > 8);
});

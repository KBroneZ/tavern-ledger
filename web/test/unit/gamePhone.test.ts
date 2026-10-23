// The game page and its combat replay on a phone (T-551): the stylesheets
// give touch-size controls on a narrow or touch screen, keep the plaque of a
// minion on the page, and leave a desktop screen as it was. The rules are
// read from the source; the real widths are checked in a browser.
import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { join } from "node:path";

const read = (...parts: string[]) => readFileSync(join(import.meta.dirname, "..", "..", "src", ...parts), "utf8").replace(/\r\n/g, "\n");
const REPLAY = read("components", "game", "replay.css");
const GAME = read("components", "game", "game.css");
const TIP = read("styles", "tip.css");

/** The text between the braces of the block that starts at `open` (nested braces counted). */
function block(css: string, open: string): string {
  const at = css.indexOf(open);
  assert.ok(at >= 0, `missing: ${open}`);
  let depth = 0;
  for (let i = css.indexOf("{", at); i < css.length; i++) {
    if (css[i] === "{") depth++;
    if (css[i] === "}" && --depth === 0) return css.slice(css.indexOf("{", at) + 1, i);
  }
  throw new Error(`unclosed: ${open}`);
}

const TOUCH = "@media (max-width: 720px), (pointer: coarse) {";

/** The declarations of `selector`'s rule inside the touch block. */
function rule(css: string, selector: string): string {
  const touch = block(css, TOUCH);
  const at = touch.indexOf(`${selector} {`);
  assert.ok(at >= 0, `the touch block has no ${selector}`);
  return block(touch, `${selector} {`);
}

test("replay: every control and step is at least 44px high on a phone or a touch screen", () => {
  for (const selector of [".rp-chip", ".rp-speed select", ".rp .rp-step", ".rp-steps > summary"]) {
    assert.match(rule(REPLAY, selector), /min-height: 44px/, selector);
  }
});

test("replay: the player's buttons sit in one row with room for the stepper, and Play is not as wide as on a desktop", () => {
  assert.match(block(block(REPLAY, "@media (max-width: 480px) {"), ".rp-btn.rp-play {"), /min-width: 6\.5rem/);
  assert.match(rule(REPLAY, ".rp-controls"), /gap: /);
});

test("game page: the turn toggle, the Simple/Advanced switch and the page's own links are 44px high on a phone", () => {
  for (const selector of [".gv-toggle", ".gv-head .seg button", ".gv-link a", ".gv .fold > summary"]) {
    assert.match(rule(GAME, selector), /min-height: 44px/, selector);
  }
});

test("game page: a fold's long hint drops under its title on a phone instead of widening the page", () => {
  const summary = rule(GAME, ".gv .fold > summary");
  assert.match(summary, /flex-wrap: wrap/);
  assert.match(rule(GAME, ".gv .fold-hint"), /flex-basis: 100%/);
});

test("the plaque of a minion moves by --tip-dx and its gift lines show where a title cannot", () => {
  assert.match(block(TIP, ".minion-tip {"), /transform: translateX\(var\(--tip-dx, 0px\)\)/);
  assert.match(block(TIP, ".tip-gift {"), /display: none/);
  assert.match(block(TIP, "@media (max-width: 720px), (hover: none) {"), /\.tip-gift \{\n\s+display: block/);
});

test("a desktop screen keeps the player's chip size: the 44px rule is only in the touch block", () => {
  const outside = REPLAY.replace(block(REPLAY, TOUCH), "");
  assert.doesNotMatch(outside, /\.rp-chip \{[^}]*min-height: 44px/);
  assert.match(outside, /\.rp-chip \{[^}]*min-height: 36px/);
});

test("while one minion shows its plaque the others' plaques take no room, so they cannot widen the page", () => {
  const rest = block(TIP, ".rp-stage:has(.rp-minion:is(:hover, :focus-visible, :focus-within)) .rp-minion:not(:hover, :focus-visible, :focus-within) .minion-tip {");
  assert.match(rest, /display: none/);
});

test("on a screen that cannot hover, a plaque lets a tap through to the minion under it", () => {
  assert.match(block(block(TIP, "@media (hover: none) {"), ".minion-tip {"), /pointer-events: none/);
});

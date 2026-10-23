// The masthead and the footer on a phone or a touch screen (T-551): their
// links are 44px targets, and a desktop screen keeps its sizes. The rules
// are read from the source; the real widths are checked in a browser.
import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { join } from "node:path";

const NAV = readFileSync(join(import.meta.dirname, "..", "..", "src", "styles", "nav.css"), "utf8").replace(/\r\n/g, "\n");

const TOUCH = "@media (max-width: 720px), (pointer: coarse) {";

/** The text between the braces of the block that starts at `open` (nested braces counted). */
function block(css: string, open: string): string {
  const at = css.indexOf(open);
  assert.ok(at >= 0, `missing: ${open}`);
  const start = css.indexOf("{", at);
  let depth = 0;
  for (let i = start; i < css.length; i++) {
    if (css[i] === "{") depth++;
    if (css[i] === "}" && --depth === 0) return css.slice(start + 1, i);
  }
  throw new Error(`unclosed: ${open}`);
}

const touch = block(NAV, TOUCH);
const desktop = NAV.replace(touch, "");

test("masthead: the brand link and the menu links are at least 44px high on a phone or a touch screen", () => {
  for (const selector of [".brand", ".site-nav a"]) {
    assert.match(block(touch, `${selector} {`), /min-height: 44px/, selector);
  }
});

test("masthead: a short menu link (Inicio) is at least 44px wide where the menu does not fold", () => {
  assert.match(block(touch, ".site-nav a {"), /min-width: 44px/);
});

test("footer: each link stands on a line of at least 44px on a phone or a touch screen", () => {
  const rule = block(touch, ".colophon a {");
  assert.match(rule, /display: inline-flex/);
  assert.match(rule, /align-items: center/);
  assert.match(rule, /min-height: 44px/);
  assert.match(rule, /min-width: 44px/);
});

test("the base rules, outside the touch block, set no 44px minimum (a desktop screen keeps its sizes)", () => {
  for (const selector of [".brand {", ".site-nav a {", ".colophon a {"]) {
    assert.doesNotMatch(block(desktop, selector), /min-(height|width): 44px/, selector);
  }
});

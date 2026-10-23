"use strict";
// The overlay's default stack (T-614, layout B): status, then the opponents
// with the lobby's tribes as their header in one box, then the counters under
// them, so a counter that appears never moves the opponents list.
const test = require("node:test");
const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");

const UI = path.join(__dirname, "..", "ui");
const read = (f) => fs.readFileSync(path.join(UI, f), "utf8").replace(/\r\n/g, "\n");
const HTML = read("overlay.html");
const PAGE = read("overlay.js");
const LAYOUT = read("overlay-layout.js");
const CSS = read("overlay.css");

// The body of a function in overlay.js, up to the next top-level one.
function body(name) {
  const start = PAGE.indexOf(`function ${name}(`);
  assert.ok(start >= 0, `${name} exists`);
  const next = PAGE.indexOf("\nfunction ", start + 1);
  return PAGE.slice(start, next < 0 ? undefined : next);
}

test("the stack's slots, top to bottom: status, heroes, opponents, counters, then the rest", () => {
  const stack = HTML.slice(HTML.indexOf('<div id="stack">'), HTML.indexOf("</div>\n  <script"));
  const order = [...stack.matchAll(/data-panel="([a-z]+)"/g)].map((m) => m[1]);
  assert.deepEqual(order, ["status", "heroes", "opponents", "counters", "odds", "legend"]);
});

test("the tribes have no panel of their own any more", () => {
  assert.doesNotMatch(HTML, /data-panel="tribes"|id="tribes"/);
  assert.doesNotMatch(PAGE, /getElementById\("tribes"\)|renderTribes|"tribes"/);
  assert.doesNotMatch(LAYOUT, /"tribes"|\btribes:/);
});

test("the opponents panel draws the tribes first, then the damage cap, then the rows", () => {
  const opponents = body("renderOpponents");
  assert.match(opponents, /tribeRow\(g\)/);
  assert.match(opponents, /replaceChildren\(\.\.\.\[head, capLine, list\]\.filter\(Boolean\)\)/);
  // Its header is never empty, so the panel never is during a game.
  assert.doesNotMatch(opponents, /classList\.toggle\("empty"/);
});

test("the tribes header ends with a thin rule above the rows", () => {
  assert.match(CSS, /\.opp-tribes \{[^}]*border-bottom: 1px solid/);
});

test("the tribe chips are a named group for screen readers", () => {
  assert.match(body("tribeRow"), /row\.setAttribute\("role", "group"\);\s*row\.setAttribute\("aria-label", TLI18n\.t\("ov\.slot\.tribes"\)\);/);
});

test("the hero select panel keeps its own tribes row", () => {
  assert.match(body("renderHeroes"), /tribeRow\(example \? EXAMPLE_GAME : g\)/);
});

test("the layout editor offers the opponents and the counters, not the tribes", () => {
  for (const name of ["PANEL_IDS", "TOGGLE_IDS"]) {
    const list = LAYOUT.match(new RegExp(String.raw`${name} = window\.TLCounters\.panelIds\(\[([^\]]*)\]`));
    assert.ok(list, name);
    const ids = list[1].split(",").map((s) => s.trim().replaceAll('"', ""));
    assert.ok(ids.includes("opponents") && ids.includes("counters"), name);
    assert.ok(!ids.includes("tribes"), name);
  }
});

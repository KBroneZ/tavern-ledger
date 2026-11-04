"use strict";
// The counters panel is on (T-614, D-139): shown under the opponents list
// whenever the player has a running total, offered in the layout editor like
// any other panel.
const test = require("node:test");
const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const counters = require("../ui/counters-words.js");

const UI = path.join(__dirname, "..", "ui");
const read = (f) => fs.readFileSync(path.join(UI, f), "utf8");
const IDS = ["status", "opponents", "counters", "legend", "lobby"];

test("the panel is on by default", () => {
  assert.equal(counters.ENABLED, true);
});

test("while on, the counters panel is in every list the page builds", () => {
  assert.deepEqual(counters.panelIds(IDS), IDS);
});

test("the layout script builds its panel list and toolbar toggles through the switch", () => {
  const layout = read("overlay-layout.js");
  assert.match(layout, /PANEL_IDS = window\.TLCounters\.panelIds\(/);
  assert.match(layout, /TOGGLE_IDS = window\.TLCounters\.panelIds\(/);
});

test("the page's counters slot is shown exactly while the switch is on", () => {
  const slot = read("overlay.html").match(/<div class="slot"([^>]*)data-panel="counters"[^>]*>/);
  assert.ok(slot, "the slot exists");
  assert.equal(/\bhidden\b/.test(slot[0]), !counters.ENABLED);
  // The panel itself starts hidden: it shows only once there are rows.
  assert.match(read("overlay.html"), /<section id="counters"[^>]* hidden>/);
});

test("the panel stays hidden when there is nothing to show: unknown and none alike", () => {
  assert.deepEqual(counters.rows(null), []);
  assert.deepEqual(counters.rows([]), []);
  assert.equal(counters.hasRows(null), false);
  assert.equal(counters.hasRows([]), false);
  assert.match(read("overlay.js"), /panel\.hidden = !rows\.length;/);
});

test("the words and their rows work", () => {
  assert.equal(counters.rows([{ kind: "free_rolls", count: 2 }]).length, 1);
});

"use strict";
// The counters panel is off until the overlay rework (D-139): the data and
// the words stay, the panel is not shown and not offered.
const test = require("node:test");
const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const counters = require("../ui/counters-words.js");

const UI = path.join(__dirname, "..", "ui");
const read = (f) => fs.readFileSync(path.join(UI, f), "utf8");
const IDS = ["status", "counters", "tribes", "opponents", "legend", "lobby"];

test("the panel is off by default", () => {
  assert.equal(counters.ENABLED, false);
});

test("while off, the counters panel is in no list the page builds", () => {
  assert.deepEqual(counters.panelIds(IDS), ["status", "tribes", "opponents", "legend", "lobby"]);
  assert.deepEqual(counters.panelIds(["status", "tribes"]), ["status", "tribes"]);
});

test("the layout script builds its panel list and toolbar toggles through the switch", () => {
  const layout = read("overlay-layout.js");
  assert.match(layout, /PANEL_IDS = window\.TLCounters\.panelIds\(/);
  assert.match(layout, /TOGGLE_IDS = window\.TLCounters\.panelIds\(/);
});

test("the page's counters slot is hidden exactly while the switch is off", () => {
  const slot = read("overlay.html").match(/<div class="slot"([^>]*)data-panel="counters"[^>]*>/);
  assert.ok(slot, "the slot exists, so the page and its checks keep their shape");
  assert.equal(/\bhidden\b/.test(slot[0]), !counters.ENABLED);
  assert.match(read("overlay.html"), /<section id="counters"[^>]* hidden>/);
});

test("the page never draws the panel while off, whatever the data says", () => {
  assert.match(read("overlay.js"), /if \(!TLCounters\.ENABLED\) \{\s*panel\.hidden = true;\s*return;/);
});

test("the words and their rows still work for the rework", () => {
  assert.equal(counters.rows([{ kind: "free_rolls", count: 2 }]).length, 1);
});

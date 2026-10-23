"use strict";
// The in-app preview (T-614): the tray shows the overlay with its example
// data over the desktop. The app owns the state (overlay_settings `preview`)
// and closes it when a game starts; the page shows the example while it is on
// and says how to close it.
const test = require("node:test");
const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const I18n = require("../ui/i18n.js");
require("../ui/strings/overlay.js");

const UI = path.join(__dirname, "..", "ui");
const PAGE = fs.readFileSync(path.join(UI, "overlay.js"), "utf8").replace(/\r\n/g, "\n");

test("the page follows the app's preview state on every refresh", () => {
  assert.match(PAGE, /previewing = Boolean\(settings && settings\.preview\);/);
});

test("the example game fills the page while arranging or previewing, and only with no game on", () => {
  assert.match(PAGE, /function showsExample\(\) \{\s*return editing \|\| previewing;\s*\}/);
  assert.match(PAGE, /return game \|\| \(showsExample\(\) \? EXAMPLE_GAME : null\);/);
  assert.match(PAGE, /rows = TLCounters\.rows\(EXAMPLE_GAME\.counters\)/);
  assert.match(PAGE, /if \(!rows\.length && showsExample\(\)\)/);
  assert.match(PAGE, /if \(!v && showsExample\(\)\) v = TLOdds\.oddsView\(EXAMPLE_GAME\.odds\);/);
});

test("the preview shows the overlay as during a game: no hero select or lobby example, no lobby panel", () => {
  assert.match(PAGE, /const example = !view && editing;/);
  assert.match(PAGE, /editing \? EXAMPLE_LOBBY : previewing \? null :/);
});

test("the preview says it is one and how to close it, in both languages", () => {
  assert.match(PAGE, /previewing && !editing \? "ov\.status\.preview" : "ov\.status\.example"/);
  const en = I18n.tables().en["ov.status.preview"];
  const es = I18n.tables().es["ov.status.preview"];
  assert.match(en, /tray/);
  assert.match(es, /bandeja/);
});

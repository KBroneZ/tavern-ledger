"use strict";
// In a dialog the way out ("Close", "Cancel") is the quiet button, so the
// action ("Save") is the one that stands out, also while it waits to be
// enabled: a dimmed Save beside a bright Close read as "the wrong one is
// active" (T-359).
const test = require("node:test");
const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");

const UI = path.join(__dirname, "..", "ui");
const html = fs.readFileSync(path.join(UI, "index.html"), "utf8");
const css = fs.readFileSync(path.join(UI, "style.css"), "utf8");

const actionBars = [...html.matchAll(/<div class="dialog-actions">([\s\S]*?)<\/div>/g)].map((m) => m[1]);
const buttons = (bar) => [...bar.matchAll(/<button\b([^>]*)>/g)].map((m) => m[1]);

test("every dialog has an action bar with buttons", () => {
  assert.ok(actionBars.length >= 2);
  actionBars.forEach((bar) => assert.ok(buttons(bar).length >= 2));
});

test("Close and Cancel buttons in a dialog are quiet; no other button is", () => {
  for (const bar of actionBars) {
    for (const attrs of buttons(bar)) {
      const leaves = /data-i18n="win\.(close|cancel)"/.test(attrs);
      const quiet = /class="[^"]*\bquiet\b/.test(attrs);
      assert.equal(quiet, leaves, attrs);
    }
  }
});

test("the quiet button has its own look", () => {
  assert.match(css, /\.dialog-actions button\.quiet\s*\{/);
});

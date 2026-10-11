"use strict";
// Simple or Advanced for the main window (T-344, D-072).
const test = require("node:test");
const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const detail = require("../ui/detail-switch.js");

function fakeDoc() {
  const button = (pick) => {
    const attrs = {};
    return { dataset: { detailPick: pick }, attrs, setAttribute: (k, v) => { attrs[k] = v; } };
  };
  const buttons = [button("simple"), button("advanced"), button("simple"), button("advanced")];
  const select = { value: "" };
  return {
    body: { dataset: {} },
    buttons,
    select,
    querySelectorAll: (q) => (q === "[data-detail-pick]" ? buttons : []),
    getElementById: (id) => (id === "setting-window-detail" ? select : null),
  };
}

test("the page shows the chosen detail on the body, both switches and Settings", () => {
  const doc = fakeDoc();
  assert.equal(detail.apply(doc, "advanced"), "advanced");
  assert.equal(doc.body.dataset.detail, "advanced");
  assert.deepEqual(doc.buttons.map((b) => b.attrs["aria-pressed"]), ["false", "true", "false", "true"]);
  assert.equal(doc.select.value, "advanced");
});

test("anything but a known detail is Simple, the default", () => {
  for (const value of [undefined, null, "", "pro", 1]) {
    const doc = fakeDoc();
    assert.equal(detail.apply(doc, value), "simple", String(value));
    assert.equal(doc.body.dataset.detail, "simple");
  }
});

test("Simple leaves the detail parts to Advanced and keeps the key numbers", () => {
  const html = fs.readFileSync(path.join(__dirname, "../ui/index.html"), "utf8");
  const classOf = (needle) => {
    const at = html.indexOf(needle);
    assert.ok(at >= 0, needle);
    const tag = html.slice(html.lastIndexOf("<", at), html.indexOf(">", at));
    return (tag.match(/class="([^"]*)"/) || [, ""])[1].split(/\s+/);
  };
  for (const id of ['aria-labelledby="lobby-title"', 'aria-labelledby="tribes-title"', 'id="entered-tribes-section"', 'aria-labelledby="hero-picks-title"', 'aria-labelledby="choice-stats-title"', 'id="recap-tribes"']) {
    assert.ok(classOf(id).includes("adv-only"), id);
  }
  for (const id of ['aria-labelledby="ledger-title"', 'aria-labelledby="heroes-title"', 'aria-labelledby="shop-stats-title"', 'class="tally"']) {
    assert.ok(!classOf(id).includes("adv-only"), id);
  }
  const css = fs.readFileSync(path.join(__dirname, "../ui/style.css"), "utf8");
  assert.match(css, /body\[data-detail="simple"\] \.adv-only \{ display: none !important; \}/);
});

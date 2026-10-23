"use strict";
// The legend is out of the default overlay (T-614): a "?" in the unlocked
// toolbar shows it, a second click or Esc hides it, and nothing is saved.
const test = require("node:test");
const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const { FakeDocument } = require("./fake-dom.js");
const TLLegend = require("../ui/legend-toggle.js");

const UI = path.join(__dirname, "..", "ui");
const read = (f) => fs.readFileSync(path.join(UI, f), "utf8").replace(/\r\n/g, "\n");

function page() {
  const doc = new FakeDocument();
  const button = doc.make("button", { id: "legend-toggle", "aria-expanded": "false", "aria-controls": "legend" });
  const legend = doc.make("p", { id: "legend" });
  legend.hidden = true;
  const toggle = TLLegend.bind(doc, button, legend);
  return { doc, button, legend, toggle };
}

const esc = (doc) => {
  const event = { type: "keydown", key: "Escape", defaultPrevented: false, preventDefault() { this.defaultPrevented = true; } };
  (doc.listeners.keydown || []).forEach((fn) => fn(event));
  return event;
};

test("the legend starts hidden", () => {
  const { button, legend, toggle } = page();
  assert.equal(legend.hidden, true);
  assert.equal(button.getAttribute("aria-expanded"), "false");
  assert.equal(toggle.isOpen(), false);
});

test("a click on the ? shows the legend and a second click hides it", () => {
  const { button, legend } = page();
  button.dispatch("click");
  assert.equal(legend.hidden, false);
  assert.equal(button.getAttribute("aria-expanded"), "true");
  button.dispatch("click");
  assert.equal(legend.hidden, true);
  assert.equal(button.getAttribute("aria-expanded"), "false");
});

test("Esc hides an open legend and gives focus back to the ?", () => {
  const { doc, button, legend } = page();
  button.dispatch("click");
  const event = esc(doc);
  assert.equal(legend.hidden, true);
  assert.equal(button.getAttribute("aria-expanded"), "false");
  assert.equal(doc.activeElement, button);
  assert.equal(event.defaultPrevented, true);
});

test("Esc with the legend hidden does nothing", () => {
  const { doc, legend } = page();
  const event = esc(doc);
  assert.equal(legend.hidden, true);
  assert.equal(event.defaultPrevented, false);
});

test("close() hides it, as when the overlay is locked", () => {
  const { button, legend, toggle } = page();
  button.dispatch("click");
  toggle.close();
  assert.equal(legend.hidden, true);
  assert.equal(button.getAttribute("aria-expanded"), "false");
});

test("the page: no legend panel in the stack, the ? and the legend in the toolbar", () => {
  const html = read("overlay.html");
  assert.doesNotMatch(html, /data-panel="legend"/);
  const toolbar = html.slice(html.indexOf('<div id="toolbar"'), html.indexOf("<!-- The lobby panel"));
  assert.match(toolbar, /<button type="button" id="legend-toggle" aria-expanded="false" aria-controls="legend"[^>]*data-i18n-attr="aria-label:ov\.toolbar\.legend"[^>]*>\?<\/button>/);
  assert.match(toolbar, /<p id="legend" class="legend" data-i18n="ov\.legend" hidden>/);
  assert.ok(html.indexOf("legend-toggle.js") < html.indexOf("overlay-layout.js"), "loaded before the layout script");
});

test("the overlay scripts no longer treat the legend as a panel", () => {
  assert.doesNotMatch(read("overlay.js"), /getElementById\("legend"\)|"legend"/);
  const layout = read("overlay-layout.js");
  assert.doesNotMatch(layout.match(/const PANEL_IDS = [^\n]*/)[0], /"legend"/);
  assert.doesNotMatch(layout, /^\s*legend: \(\) =>/m);
  assert.match(read("overlay-layout.js"), /TLLegend\.bind\(/);
  // Locked, the toolbar goes and the legend with it: it is never kept open.
  assert.match(read("overlay-layout.js"), /if \(!v\.unlocked\) legendToggle\.close\(\);/);
});

test("the ? looks pressed while its legend is open", () => {
  assert.match(read("overlay.css"), /\.toolbar button\[aria-expanded="true"\] \{[^}]*border-color: var\(--accent\)/);
});

test("the ? has a name in both languages", () => {
  const strings = read("strings/overlay.js");
  assert.equal((strings.match(/"ov\.toolbar\.legend": "[^"]+"/g) || []).length, 2);
});

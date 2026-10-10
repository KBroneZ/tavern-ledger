"use strict";
const test = require("node:test");
const assert = require("node:assert/strict");
const { FakeDocument } = require("./fake-dom.js");
const TLTabs = require("../ui/tabs.js");

// The window's two tabs and their panels, as index.html has them.
function page() {
  const doc = new FakeDocument();
  const list = doc.make("div", { id: "view-tabs", role: "tablist" });
  const tab = (id, panel, selected) =>
    doc.make("button", { id, role: "tab", "aria-controls": panel, "aria-selected": String(selected) }, list);
  const tabs = [tab("tab-games", "panel-games", true), tab("tab-curious", "panel-curious", false)];
  const panels = [doc.make("div", { id: "panel-games", role: "tabpanel" }), doc.make("section", { id: "panel-curious", role: "tabpanel" })];
  return { doc, list, tabs, panels };
}

const state = ({ tabs, panels }) =>
  tabs.map((t, i) => [t.getAttribute("aria-selected"), t.getAttribute("tabindex"), panels[i].hidden]);

test("arrow keys wrap around, Home and End go to the ends, other keys are not ours", () => {
  assert.equal(TLTabs.nextIndex("ArrowRight", 1, 2), 0);
  assert.equal(TLTabs.nextIndex("ArrowLeft", 0, 2), 1);
  assert.equal(TLTabs.nextIndex("Home", 1, 3), 0);
  assert.equal(TLTabs.nextIndex("End", 0, 3), 2);
  assert.equal(TLTabs.nextIndex("Enter", 0, 2), null);
  assert.equal(TLTabs.nextIndex("ArrowRight", 0, 0), null);
});

test("on start only the selected tab is in the Tab order and only its panel shows", () => {
  const p = page();
  TLTabs.setup(p.doc, "view-tabs");
  assert.deepEqual(state(p), [["true", "0", false], ["false", "-1", true]]);
});

test("a click shows the curious stats tab and hides the games", () => {
  const p = page();
  const changes = [];
  TLTabs.setup(p.doc, "view-tabs", (i) => changes.push(i));
  p.tabs[1].dispatch("click");
  assert.deepEqual(state(p), [["false", "-1", true], ["true", "0", false]]);
  assert.deepEqual(changes, [1]);
});

test("an arrow key moves focus to the next tab and shows it, and is not passed on", () => {
  const p = page();
  TLTabs.setup(p.doc, "view-tabs");
  p.tabs[0].focus();
  const event = p.tabs[0].dispatch("keydown", { key: "ArrowRight" });
  assert.equal(p.doc.activeElement, p.tabs[1]);
  assert.equal(p.panels[1].hidden, false);
  assert.ok(event.defaultPrevented, "the page does not scroll");
  const other = p.tabs[1].dispatch("keydown", { key: "a" });
  assert.equal(other.defaultPrevented, false, "other keys are left alone");
  assert.equal(p.doc.activeElement, p.tabs[1]);
});

"use strict";
// The opponent's name in a turn row ends in an ellipsis when the row is too
// narrow (a flex box does not clip text on its own, so the name has its own
// element) and the full name is on hover (T-359).
const test = require("node:test");
const assert = require("node:assert/strict");
const TLTurns = require("../ui/turns.js");
const I18n = require("../ui/i18n.js");
require("../ui/strings/common.js");
require("../ui/strings/details.js");
require("../ui/strings/replay.js");

// Just enough of a page: an element is a record with children.
const ui = {
  el(tag, className, text) {
    const node = { tag, className: className || "", text: text === undefined ? "" : String(text), title: "", children: [] };
    node.append = (...kids) => { node.children.push(...kids); return node; };
    node.prepend = (...kids) => { node.children.unshift(...kids); return node; };
    return node;
  },
  addTip() {},
};
const art = (id, className) => ui.el("img", className);
const heroText = (h) => h.name || h.id;

const combat = (opponents) => ({ outcome: "won", source: "log", reason: null, opponents });

test("the opponent's name sits in its own element, after the portraits", () => {
  const box = TLTurns.result(ui, combat([{ id: "A", name: "Tavern Keeper Extraordinaire" }]), heroText, art);
  const who = box.children.find((c) => c.className === "turn-opponent");
  assert.ok(who, "the opponent element");
  assert.deepEqual(who.children.map((c) => c.className), ["portrait", "turn-opponent-name"]);
  assert.equal(who.children[1].text, "Tavern Keeper Extraordinaire");
});

test("a team of opponents is one name with every hero, and the whole name is the tooltip", () => {
  const box = TLTurns.result(ui, combat([{ id: "A", name: "Alpha" }, { id: "B", name: "Beta" }]), heroText, art);
  const name = box.children.find((c) => c.className === "turn-opponent").children.find((c) => c.className === "turn-opponent-name");
  assert.equal(name.text, "Alpha + Beta");
  assert.equal(name.title, "Alpha + Beta");
});

test("an unknown opponent says so in the same element", () => {
  I18n.setLang("en");
  const box = TLTurns.result(ui, combat([]), heroText, art);
  const name = box.children.find((c) => c.className === "turn-opponent").children[0];
  assert.equal(name.className, "turn-opponent-name");
  assert.equal(name.text, "opponent unknown");
});

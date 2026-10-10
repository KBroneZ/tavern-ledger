"use strict";
const test = require("node:test");
const assert = require("node:assert/strict");
const { FakeDocument, FakeNode } = require("./fake-dom.js");
const TLFocus = require("../ui/focus-keeper.js");

// A history table with three buttons per game, like the app's.
function history(doc, games) {
  const tbody = doc.make("tbody", { id: "games", "data-focus-scope": "" });
  const draw = (list) =>
    tbody.replaceChildren(
      ...list.flatMap((g) =>
        ["recap", "tribes", "report"].map((kind) => new FakeNode(doc, "button", { "data-focus": `${kind}:${g}` })),
      ),
    );
  draw(games);
  return { tbody, draw };
}

const keyOf = (doc) => doc.activeElement.getAttribute("data-focus") || doc.activeElement.id;
const button = (tbody, key) => tbody.children.find((b) => b.getAttribute("data-focus") === key);

test("focus stays on the same button when the history is redrawn", () => {
  const doc = new FakeDocument();
  const { tbody, draw } = history(doc, ["a", "b"]);
  button(tbody, "tribes:b").focus();
  TLFocus.keep(doc, () => draw(["a", "b"]));
  assert.equal(keyOf(doc), "tribes:b");
  assert.ok(doc.activeElement.isConnected, "it is the new button, not the old one");
});

test("focus stays put when a new game is added above", () => {
  const doc = new FakeDocument();
  const { tbody, draw } = history(doc, ["a", "b"]);
  button(tbody, "recap:a").focus();
  TLFocus.keep(doc, () => draw(["new", "a", "b"]));
  assert.equal(keyOf(doc), "recap:a");
});

test("a removed game hands focus to the control in the same place", () => {
  const doc = new FakeDocument();
  const { tbody, draw } = history(doc, ["a", "b", "c"]);
  button(tbody, "report:b").focus(); // the sixth button
  TLFocus.keep(doc, () => draw(["a", "c"]));
  assert.equal(keyOf(doc), "report:c", "the sixth of six buttons is the report button of the game that took its place");
});

test("when the list shrinks a lot, focus goes to the new last control", () => {
  const doc = new FakeDocument();
  const { tbody, draw } = history(doc, ["a", "b", "c"]);
  tbody.children[8].focus();
  TLFocus.keep(doc, () => draw(["a"]));
  assert.equal(keyOf(doc), "report:a");
});

test("a control that hides itself passes focus to the one named in data-focus-next", () => {
  const doc = new FakeDocument();
  const enter = doc.make("button", { id: "lobby-enter" });
  const clear = doc.make("button", { id: "lobby-clear", "data-focus-next": "lobby-enter" });
  clear.focus();
  TLFocus.keep(doc, () => {
    clear.hidden = true;
  });
  assert.equal(doc.activeElement, enter);
});

test("a disabled candidate is skipped, the next one gets focus", () => {
  const doc = new FakeDocument();
  doc.make("button", { id: "save" }).disabled = true;
  const close = doc.make("button", { id: "close" });
  const gone = doc.make("button", { id: "gone", "data-focus-next": "save close" });
  gone.focus();
  TLFocus.keep(doc, () => {
    gone.hidden = true;
  });
  assert.equal(doc.activeElement, close);
});

test("with nothing to go to, nothing is moved and the answer says so", () => {
  const doc = new FakeDocument();
  const only = doc.make("button", { id: "only" });
  only.focus();
  const saved = TLFocus.note(doc);
  only.hidden = true;
  assert.equal(TLFocus.restore(doc, saved), false);
});

test("nothing focused: nothing is moved", () => {
  const doc = new FakeDocument();
  doc.make("button", { id: "a" });
  TLFocus.keep(doc, () => {});
  assert.equal(doc.focusCalls, 0);
  assert.equal(doc.activeElement, doc.body);
});

test("focus is restored even if the redraw throws", () => {
  const doc = new FakeDocument();
  doc.make("button", { id: "a" }).focus();
  assert.throws(() =>
    TLFocus.keep(doc, () => {
      doc.body.replaceChildren(new FakeNode(doc, "button", { id: "a" }));
      throw new Error("boom");
    }),
  );
  assert.equal(doc.activeElement.id, "a");
  assert.ok(doc.activeElement.isConnected);
});

test("a dialog opener is found again by its key after the history was redrawn", () => {
  const doc = new FakeDocument();
  const { tbody, draw } = history(doc, ["a", "b"]);
  button(tbody, "tribes:a").focus();
  const opener = TLFocus.note(doc);
  draw(["a", "b"]); // the dialog's save redrew the table; focus fell to the page
  assert.equal(doc.activeElement, doc.body);
  assert.equal(TLFocus.restore(doc, opener), true);
  assert.equal(keyOf(doc), "tribes:a");
});

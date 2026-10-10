"use strict";
const test = require("node:test");
const assert = require("node:assert/strict");
const { FakeDocument } = require("./fake-dom.js");
const TLFocus = require("../ui/focus-keeper.js");
const TLRecapView = require("../ui/recap-view.js");

// A window that remembers where the page was scrolled.
function fakeWindow() {
  return {
    scrollY: 0,
    scrollTo(x, y) { this.scrollY = y; },
  };
}

function page() {
  const doc = new FakeDocument();
  const win = fakeWindow();
  const masthead = doc.make("header", { id: "masthead" });
  const main = doc.make("main", { id: "main" });
  const games = doc.make("tbody", { id: "games", "data-focus-scope": "" }, main);
  const opener = doc.make("button", { "data-focus": "recap:s:2" }, games);
  const view = doc.make("section", { id: "recap-view" });
  view.hidden = true;
  const heading = doc.make("h2", { id: "recap-title", tabindex: "-1" }, view);
  const close = doc.make("button", { id: "recap-close" }, view);
  let blocked = false;
  let closed = 0;
  const recap = TLRecapView.create({
    doc, win, focus: TLFocus, view, heading, hide: [masthead, main],
    blocked: () => blocked, onClose: () => { closed += 1; },
  });
  return { doc, win, masthead, main, games, opener, view, heading, close, recap, block: (b) => { blocked = b; }, closed: () => closed };
}

test("opening fills the window: the rest of the page is hidden and the title takes focus", () => {
  const p = page();
  p.opener.focus();
  p.win.scrollY = 840;
  p.recap.open();
  assert.equal(p.view.hidden, false);
  assert.equal(p.main.hidden, true);
  assert.equal(p.masthead.hidden, true);
  assert.equal(p.doc.activeElement, p.heading);
  assert.equal(p.win.scrollY, 0, "the recap starts at its top");
  assert.equal(p.recap.isOpen(), true);
});

test("closing goes back to where you were: same scroll, focus on the game you opened", () => {
  const p = page();
  p.opener.focus();
  p.win.scrollY = 840;
  p.recap.open();
  p.win.scrollY = 1200; // scrolled inside the recap
  p.recap.close();
  assert.equal(p.view.hidden, true);
  assert.equal(p.main.hidden, false);
  assert.equal(p.masthead.hidden, false);
  assert.equal(p.win.scrollY, 840);
  assert.equal(p.doc.activeElement, p.opener);
  assert.equal(p.closed(), 1);
});

test("the list was redrawn while the recap was open: focus finds the new button by its key", () => {
  const p = page();
  p.opener.focus();
  p.recap.open();
  const fresh = p.doc.make("button", { "data-focus": "recap:s:2" });
  p.games.replaceChildren(fresh);
  p.recap.close();
  assert.equal(p.doc.activeElement, fresh);
});

test("opening again while open keeps the first place to go back to", () => {
  const p = page();
  p.opener.focus();
  p.win.scrollY = 300;
  p.recap.open();
  p.close.focus();
  p.recap.open(); // e.g. the recap reloads after a change
  p.recap.close();
  assert.equal(p.win.scrollY, 300);
  assert.equal(p.doc.activeElement, p.opener);
});

test("a reload while the recap is open leaves focus where the user is", () => {
  const p = page();
  p.recap.open();
  p.close.focus();
  p.recap.open();
  assert.equal(p.doc.activeElement, p.close);
});

test("Escape closes the recap", () => {
  const p = page();
  p.opener.focus();
  p.recap.open();
  const event = p.close.dispatch("keydown", { key: "Escape" });
  assert.equal(p.view.hidden, true);
  assert.equal(event.defaultPrevented, true);
  assert.equal(p.doc.activeElement, p.opener);
});

test("Escape inside a dialog over the recap closes only the dialog", () => {
  const p = page();
  p.recap.open();
  p.block(true);
  p.close.dispatch("keydown", { key: "Escape" });
  assert.equal(p.view.hidden, false);
});

test("other keys and a closed recap leave everything alone", () => {
  const p = page();
  p.recap.open();
  p.close.dispatch("keydown", { key: "Enter" });
  assert.equal(p.view.hidden, false);
  p.recap.close();
  const event = p.opener.dispatch("keydown", { key: "Escape" });
  assert.equal(event.defaultPrevented, false);
  assert.equal(p.closed(), 1);
});

test("a recap that opened by itself gives focus back to its game's button", () => {
  const p = page();
  p.recap.open("recap:s:2");
  p.recap.close();
  assert.equal(p.doc.activeElement, p.opener);
});

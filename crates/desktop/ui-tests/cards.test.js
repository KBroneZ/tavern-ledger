"use strict";
// cards.js is a page script: it is loaded here with just enough of a page and
// of Tauri to test how a card picture is made and what replaces a broken one.
const test = require("node:test");
const assert = require("node:assert/strict");
const path = require("node:path");

class Node {
  constructor(tag) {
    this.tag = tag;
    this.attrs = {};
    this.listeners = {};
    this.replacedBy = null;
    this.removed = false;
  }
  setAttribute(name, value) { this.attrs[name] = String(value); }
  addEventListener(type, fn) { (this.listeners[type] = this.listeners[type] || []).push(fn); }
  fire(type) { (this.listeners[type] || []).forEach((fn) => fn({ type })); }
  replaceWith(node) { this.replacedBy = node; }
  remove() { this.removed = true; }
}

global.window = { __TAURI__: { core: { invoke: async () => ({}) }, event: { listen: () => {} } } };
global.location = { protocol: "http:" };
global.document = { createElement: (tag) => new Node(tag) };
require(path.join(__dirname, "..", "ui", "cards.js"));
const { TLCards } = global.window;

test("a picture is an image of the card's art, with no text for screen readers", () => {
  const img = TLCards.art("BG26_174", "shop-art");
  assert.equal(img.tag, "img");
  assert.equal(img.src, "http://cardart.localhost/BG26_174");
  assert.equal(img.className, "shop-art");
  assert.equal(img.alt, "");
});

test("an id that is not a card id makes no picture", () => {
  assert.equal(TLCards.art("../etc", "shop-art"), null);
  assert.equal(TLCards.art(null, "shop-art"), null);
});

test("by default a picture that fails is removed (the overlay's behaviour)", () => {
  const img = TLCards.art("BG26_174", "portrait");
  img.fire("error");
  assert.equal(img.removed, true);
  assert.equal(img.replacedBy, null);
  assert.equal(img.loading, undefined);
});

test("in the main window a picture loads lazily and a failed one becomes a same-size placeholder", () => {
  const img = TLCards.art("BG26_174", "portrait", { lazy: true, placeholder: true });
  assert.equal(img.loading, "lazy");
  img.fire("error");
  assert.equal(img.removed, false);
  const spot = img.replacedBy;
  assert.equal(spot.tag, "span");
  assert.equal(spot.className, "portrait art-missing", "keeps the picture's class, so its size and shape");
  assert.equal(spot.attrs["aria-hidden"], "true");
  assert.match(spot.title, /No picture/);
});

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

const I18n = require("../ui/i18n.js");
require("../ui/strings/common.js");
require("../ui/strings/details.js");
require("../ui/strings/gifts.js");

// The app's answer to card_names: Spanish names when asked in Spanish.
const asked = [];
const answer = async (cmd, args) => {
  if (cmd !== "card_names") return {};
  asked.push(args.lang);
  const names = args.lang === "es" ? { BG_KNOWN: "Nombre español" } : { BG_KNOWN: "English name" };
  return Object.fromEntries(args.ids.map((id) => [id, names[id] || null]));
};
global.window = { __TAURI__: { core: { invoke: answer }, event: { listen: () => {} } }, TLI18n: I18n };
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

test("the placeholder's tooltip is in the language in use", () => {
  I18n.setLang("es");
  try {
    const img = TLCards.art("BG26_174", "portrait", { placeholder: true });
    img.fire("error");
    assert.match(img.replacedBy.title, /^Sin imagen: la base de cartas/);
  } finally {
    I18n.setLang("en");
  }
});

test("names are asked in the app's language; the log's name and the id are the fallbacks", async () => {
  I18n.setLang("es");
  try {
    assert.equal(TLCards.display("BG_KNOWN", "Nombre del registro"), "Nombre del registro", "not answered yet: the log's");
    assert.equal(TLCards.display("BG_UNKNOWN", null), "BG_UNKNOWN");
    await new Promise((resolve) => setTimeout(resolve, 10));
    assert.deepEqual([...new Set(asked)], ["es"]);
    assert.equal(TLCards.display("BG_KNOWN", "Nombre del registro"), "Nombre español", "the card data's, in Spanish");
    assert.equal(TLCards.display("BG_UNKNOWN", "Del registro"), "Del registro", "the card data lacks it: the log's");
    assert.equal(TLCards.display("BG_UNKNOWN", null), "BG_UNKNOWN", "then the id");
    assert.equal(TLCards.display(null, null), null);
  } finally {
    I18n.setLang("en");
  }
});

test("each Dark Gift is a mark with its name and summary, in the minion's order", () => {
  I18n.setLang("en");
  const marks = TLCards.gifts({ gifts: ["BG36_MidGameEffect_000t9", "BG36_MidGameEffect_000t13"] });
  assert.deepEqual(marks.map((m) => m.name), ["Admiration", "Harpy's Talons"]);
  assert.equal(marks[0].title, "Dark Gift: Admiration: Start of Combat: gain the Attack of the minion to its left.");
  I18n.setLang("es");
  assert.equal(TLCards.gifts({ gifts: ["BG36_MidGameEffect_000t13"] })[0].title, "Don siniestro: Garfas de arpía: Escudo divino y Viento furioso.");
  I18n.setLang("en");
});

test("a minion with no gift has no mark, and an unknown gift says so", () => {
  assert.deepEqual(TLCards.gifts({}), []);
  assert.deepEqual(TLCards.gifts({ gifts: [] }), []);
  assert.deepEqual(TLCards.gifts(null), []);
  const unknown = TLCards.gifts({ gift_unknown: true });
  assert.equal(unknown.length, 1);
  assert.equal(unknown[0].name, "Dark Gift (unknown)");
  // A gift the table does not know: its id, no summary; ids that are not card ids are dropped.
  const other = TLCards.gifts({ gifts: ["BG99_New_000t1", "<b>", 7] });
  assert.deepEqual(other.map((m) => [m.id, m.title]), [["BG99_New_000t1", "Dark Gift: BG99_New_000t1"]]);
});

"use strict";
// The hide-all toggle on the page (T-614): everything the overlay draws goes
// at once, a short note with the key shows for about 3 seconds, and showing
// again brings back what was there. The app owns the state (overlay_settings
// `all_hidden`); the page only follows it.
const test = require("node:test");
const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const { FakeDocument } = require("./fake-dom.js");
const I18n = require("../ui/i18n.js");
const TLHideAll = require("../ui/hide-all.js");

const UI = path.join(__dirname, "..", "ui");
const read = (f) => fs.readFileSync(path.join(UI, f), "utf8").replace(/\r\n/g, "\n");
require("../ui/strings/overlay.js");

// A page with the note, and a clock the test moves by hand.
function page() {
  const doc = new FakeDocument();
  doc.body.classList = new Set();
  doc.body.classList.toggle = function (name, on) {
    if (on) this.add(name);
    else this.delete(name);
  };
  const note = doc.make("p", { id: "hidden-note" });
  note.hidden = true;
  const timers = [];
  const clock = {
    setTimeout: (fn, ms) => timers.push({ fn, ms, live: true }) - 1,
    clearTimeout: (id) => {
      if (timers[id]) timers[id].live = false;
    },
    fire: () => timers.filter((t) => t.live).forEach((t) => ((t.live = false), t.fn())),
    pending: () => timers.filter((t) => t.live),
  };
  const hide = TLHideAll.create(doc, note, clock);
  return { doc, note, clock, hide };
}

const view = (allHidden, keyTaken = false) => ({ all_hidden: allHidden, hide_key_taken: keyTaken });

test("shown at start: no class, no note", () => {
  const { doc, note, clock, hide } = page();
  hide.apply(view(false));
  assert.equal(doc.body.classList.has("all-hidden"), false);
  assert.equal(note.hidden, true);
  assert.equal(clock.pending().length, 0);
});

test("hiding hides everything and shows the note with the key for about 3 seconds", () => {
  const { doc, note, clock, hide } = page();
  hide.apply(view(false));
  hide.apply(view(true));
  assert.equal(doc.body.classList.has("all-hidden"), true);
  assert.equal(note.hidden, false);
  assert.match(note.textContent, /Ctrl\+Shift\+H/);
  assert.equal(clock.pending().length, 1);
  assert.equal(clock.pending()[0].ms, TLHideAll.NOTE_MS);
  assert.ok(TLHideAll.NOTE_MS >= 2500 && TLHideAll.NOTE_MS <= 3500);
  clock.fire();
  assert.equal(note.hidden, true, "then nothing is drawn at all");
  assert.equal(doc.body.classList.has("all-hidden"), true);
});

test("redraws while hidden (new game events, settings) keep it hidden and do not bring the note back", () => {
  const { doc, note, clock, hide } = page();
  hide.apply(view(true));
  clock.fire();
  hide.apply(view(true));
  hide.apply(view(true));
  assert.equal(doc.body.classList.has("all-hidden"), true);
  assert.equal(note.hidden, true);
  assert.equal(clock.pending().length, 0);
});

test("showing again removes the class and the note at once", () => {
  const { doc, note, clock, hide } = page();
  hide.apply(view(true));
  hide.apply(view(false));
  assert.equal(doc.body.classList.has("all-hidden"), false);
  assert.equal(note.hidden, true);
  assert.equal(clock.pending().length, 0, "the note's timer is dropped");
});

test("a page that opens while hidden draws nothing, not even the note", () => {
  const { doc, note, hide } = page();
  hide.apply(view(true), { first: true });
  assert.equal(doc.body.classList.has("all-hidden"), true);
  assert.equal(note.hidden, true);
});

test("never hidden while the layout is unlocked, whatever the app's flags say", () => {
  const { doc, note, hide } = page();
  hide.apply({ all_hidden: true, unlocked: true });
  assert.equal(doc.body.classList.has("all-hidden"), false);
  assert.equal(note.hidden, true);
});

test("with the key taken by another program, the note points at the tray menu", () => {
  const { note, hide } = page();
  hide.apply(view(true, true));
  assert.doesNotMatch(note.textContent, /Ctrl/);
  assert.match(note.textContent, /tray/);
});

test("the note's words in both languages", () => {
  const en = I18n.tables().en;
  const es = I18n.tables().es;
  for (const key of ["ov.hidden.note", "ov.hidden.noteNoKey", "ov.hidden.key"]) {
    assert.ok(en[key] && es[key] && en[key] !== es[key] || key === "ov.hidden.key", key);
  }
  assert.equal(en["ov.hidden.key"], "Ctrl+Shift+H");
  assert.equal(es["ov.hidden.key"], "Ctrl+Mayús+H");
  assert.match(en["ov.hidden.note"], /\{key\}/);
  assert.match(es["ov.hidden.note"], /\{key\}/);
});

test("with the key taken, the status box says so during a game, in both languages", () => {
  const page = read("overlay.js");
  assert.match(page, /keyTaken = Boolean\(settings && settings\.hide_key_taken\);/);
  assert.match(page, /if \(keyTaken && g !== EXAMPLE_GAME\) notes\.push\(TLI18n\.t\("ov\.status\.keyTaken"\)\);/);
  assert.match(I18n.tables().en["ov.status.keyTaken"], /Ctrl\+Shift\+H.*tray/);
  assert.match(I18n.tables().es["ov.status.keyTaken"], /Ctrl\+Mayús\+H.*bandeja/);
});

test("the page: the note exists, takes no clicks or focus, and the layout script follows the app's state", () => {
  const html = read("overlay.html");
  assert.match(html, /<p id="hidden-note" class="hidden-note" role="status" hidden><\/p>/);
  assert.ok(html.indexOf("hide-all.js") < html.indexOf("overlay-layout.js"));
  const css = read("overlay.css");
  assert.match(css, /\.hidden-note \{[^}]*pointer-events: none/);
  // Everything the overlay draws: panels in the stack and placed, the marks,
  // the lobby, the hover board, the odds bar (a panel), the layout editor.
  for (const part of ["#stack", "#marks", "#lobby-dock", "body > .slot", "#hover-card", "#board-box", "#toolbar", "#scrim", "#arrange-pad", "#key-hint"]) {
    assert.ok(css.includes(`body.all-hidden ${part === "body > .slot" ? "> .slot" : part}`), part);
  }
  assert.match(read("overlay-layout.js"), /hideAll\.apply\(v/);
});

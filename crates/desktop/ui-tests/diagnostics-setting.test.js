"use strict";
// "Copy diagnostics" (T-626): the app makes the text, the window shows it, and
// "Copy" puts exactly the shown text on the clipboard, never before it was shown.
const test = require("node:test");
const assert = require("node:assert/strict");
const I18n = require("../ui/i18n.js");
require("../ui/strings/common.js");
require("../ui/strings/window.js");
const Diagnostics = require("../ui/diagnostics-setting.js");
const { FakeDocument } = require("./fake-dom.js");

const t = I18n.t;
const tick = () => new Promise((resolve) => setImmediate(resolve));

function page() {
  const doc = new FakeDocument();
  doc.make("button", { id: "diag-open" });
  doc.make("p", { id: "diag-problem" });
  const dialog = doc.make("dialog", { id: "diag-dialog" });
  dialog.open = false;
  dialog.showModal = () => { dialog.open = true; };
  // A real browser queues the close event; closing by Escape and by the button both fire it.
  dialog.close = () => { if (dialog.open) { dialog.open = false; dialog.dispatch("close"); } };
  doc.make("pre", { id: "diag-text" });
  doc.make("p", { id: "diag-status" });
  doc.make("button", { id: "diag-copy" });
  doc.make("button", { id: "diag-close" });
  return doc;
}

const TEXT = "Tavern Ledger diagnostics (format 1)\nApp version: 0.2.5\n  trailing spaces kept  \n\nGame build: 254477\n";

function setup(call, writeText = async () => {}) {
  const doc = page();
  Diagnostics.start(doc, call, writeText, t, String);
  const el = (id) => doc.getElementById(id);
  return { doc, el };
}

test.beforeEach(() => I18n.setLang("en"));

test("the button asks the app for the text, sends nothing, and shows it unchanged in a dialog", async () => {
  const calls = [];
  const { el } = setup((...args) => { calls.push(args); return Promise.resolve(TEXT); });
  el("diag-open").dispatch("click");
  await tick();
  assert.deepEqual(calls, [["diagnostics_preview"]], "no argument: the window sends nothing");
  assert.equal(el("diag-text").textContent, TEXT, "shown as it came, whitespace and all");
  assert.equal(el("diag-dialog").open, true);
  assert.equal(el("diag-copy").getAttribute("aria-disabled"), null, "Copy can be used now");
});

test("a second click while the app is answering asks nothing more", async () => {
  let calls = 0;
  const { el } = setup(() => { calls += 1; return new Promise(() => {}); });
  el("diag-open").dispatch("click");
  el("diag-open").dispatch("click");
  await tick();
  assert.equal(calls, 1);
});

test("Copy writes exactly the shown text, once per click, and says so", async () => {
  const written = [];
  const { el } = setup(() => Promise.resolve(TEXT), async (text) => { written.push(text); });
  el("diag-open").dispatch("click");
  await tick();
  el("diag-copy").dispatch("click");
  await tick();
  assert.deepEqual(written, [TEXT]);
  assert.equal(written[0], el("diag-text").textContent, "what is copied is what was previewed");
  assert.equal(el("diag-status").textContent, "Copied to the clipboard.");
});

test("nothing is copied before the text has been shown", async () => {
  const written = [];
  const { el } = setup(() => new Promise(() => {}), async (text) => { written.push(text); });
  assert.equal(el("diag-copy").getAttribute("aria-disabled"), "true");
  el("diag-copy").dispatch("click");
  el("diag-open").dispatch("click");
  el("diag-copy").dispatch("click");
  await tick();
  assert.deepEqual(written, []);
  assert.equal(el("diag-dialog").open, false);
});

test("a clipboard that refuses is said, with the way out", async () => {
  const { el } = setup(() => Promise.resolve(TEXT), async () => { throw new Error("denied"); });
  el("diag-open").dispatch("click");
  await tick();
  el("diag-copy").dispatch("click");
  await tick();
  assert.equal(el("diag-status").textContent, "Could not copy. Select the text above and copy it by hand.");
  assert.equal(el("diag-text").textContent, TEXT, "the text stays to be copied by hand");
});

test("when the app cannot make the text, the problem is said under the button and no dialog opens", async () => {
  for (const [lang, start] of [["en", "Couldn't make the diagnostics"], ["es", "No se ha podido crear el diagnóstico"]]) {
    I18n.setLang(lang);
    const { el } = setup(() => Promise.reject("No diagnostics were made: the text held something that must not leave the app (a drive path)."));
    el("diag-open").dispatch("click");
    await tick();
    assert.ok(el("diag-problem").textContent.startsWith(start), el("diag-problem").textContent);
    assert.equal(el("diag-dialog").open, false);
    assert.equal(el("diag-text").textContent, "");
  }
});

test("an empty or odd answer is a failure said in the player's language, not an empty dialog", async () => {
  for (const [lang, said] of [["en", "(the app returned nothing)"], ["es", "(la app no ha devuelto nada)"]]) {
    I18n.setLang(lang);
    for (const answer of ["", null, 42, { text: "x" }]) {
      const { el } = setup(() => Promise.resolve(answer));
      el("diag-open").dispatch("click");
      await tick();
      assert.equal(el("diag-dialog").open, false, JSON.stringify(answer));
      assert.ok(el("diag-problem").textContent.endsWith(`${said}.`), el("diag-problem").textContent);
    }
  }
});

test("a failed open after a good one leaves no text to copy", async () => {
  const written = [];
  let good = true;
  const { el } = setup(() => (good ? Promise.resolve(TEXT) : Promise.reject("nope")), async (text) => { written.push(text); });
  el("diag-open").dispatch("click");
  await tick();
  el("diag-dialog").close();
  good = false;
  el("diag-open").dispatch("click");
  await tick();
  el("diag-copy").dispatch("click");
  await tick();
  assert.deepEqual(written, []);
  assert.equal(el("diag-text").textContent, "");
});

test("closing (the button or Escape) clears the text and gives the focus back to the setting", async () => {
  const written = [];
  let n = 0;
  const { doc, el } = setup(() => Promise.resolve(`${TEXT}#${++n}`), async (text) => { written.push(text); });
  el("diag-open").dispatch("click");
  await tick();
  el("diag-close").dispatch("click");
  assert.equal(el("diag-dialog").open, false);
  assert.equal(el("diag-text").textContent, "");
  assert.equal(el("diag-copy").getAttribute("aria-disabled"), "true");
  assert.equal(doc.activeElement, el("diag-open"), "back on the button that opened it");
  el("diag-copy").dispatch("click");
  await tick();
  assert.deepEqual(written, [], "nothing to copy once closed");
  // Escape: the browser closes the dialog itself and fires `close`.
  el("diag-open").dispatch("click");
  await tick();
  el("diag-dialog").close();
  assert.equal(el("diag-text").textContent, "");
  el("diag-open").dispatch("click");
  await tick();
  el("diag-copy").dispatch("click");
  await tick();
  assert.deepEqual(written, [`${TEXT}#3`]);
});

test("a copy that finishes after the dialog was closed and opened again does not speak for the new text", async () => {
  let release;
  const slow = new Promise((resolve) => { release = resolve; });
  let n = 0;
  const { el } = setup(() => Promise.resolve(`${TEXT}#${++n}`), () => slow);
  el("diag-open").dispatch("click");
  await tick();
  el("diag-copy").dispatch("click"); // pending
  el("diag-dialog").close();
  el("diag-open").dispatch("click");
  await tick();
  release();
  await tick();
  assert.equal(el("diag-status").textContent, "", "the old copy says nothing about the new preview");
});

test("the page's clipboard: the async API; else, or when it refuses, a textarea inside the dialog and the old command", async () => {
  const doc = new FakeDocument();
  const done = [];
  await Diagnostics.pageClipboard(doc, { clipboard: { writeText: async (text) => { done.push(text); } } })("abc");
  assert.deepEqual(done, ["abc"]);

  const dialog = doc.make("dialog", { id: "into" });
  const button = doc.make("button", { id: "was-focused" });
  doc.activeElement = button;
  let boxes = [];
  doc.createElement = () => {
    const box = new (doc.body.constructor)(doc, "textarea", {});
    box.style = {};
    box.remove = () => { box.parent.children = box.parent.children.filter((c) => c !== box); };
    box.select = () => { boxes.push({ value: box.value, parent: box.parent, readonly: box.getAttribute("readonly") }); doc.activeElement = box; };
    return box;
  };
  doc.execCommand = (name) => name === "copy";

  // No async API.
  await Diagnostics.pageClipboard(doc, {}, dialog)("xyz");
  assert.equal(boxes.length, 1);
  assert.deepEqual(boxes[0], { value: "xyz", parent: dialog, readonly: "" });
  assert.equal(dialog.children.length, 0, "the textarea is removed");
  assert.equal(doc.activeElement, button, "the focus goes back");

  // The async API refuses (no focus, a policy): the old command still gets its chance.
  boxes = [];
  const refusing = { clipboard: { writeText: async () => { throw new Error("NotAllowedError"); } } };
  await Diagnostics.pageClipboard(doc, refusing, dialog)("after a refusal");
  assert.equal(boxes[0].value, "after a refusal");

  // Both fail: the player is told.
  doc.execCommand = () => false;
  await assert.rejects(Diagnostics.pageClipboard(doc, refusing, dialog)("no"));
  assert.equal(doc.activeElement, button);
});

test("the words exist in both languages", () => {
  for (const lang of ["en", "es"]) {
    I18n.setLang(lang);
    for (const key of ["settings.diag.open", "settings.diag.note", "settings.diag.title", "settings.diag.preview", "settings.diag.copy", "settings.diag.copied", "settings.diag.copyFailed", "settings.diag.failed", "settings.diag.empty"]) {
      assert.notEqual(t(key, { error: "x" }), key, `${lang} ${key}`);
    }
  }
});

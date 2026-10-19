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
  dialog.close = () => { dialog.open = false; dialog.dispatch("close"); };
  doc.make("pre", { id: "diag-text" });
  doc.make("p", { id: "diag-status" });
  const copy = doc.make("button", { id: "diag-copy" });
  copy.disabled = true;
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
  assert.equal(el("diag-open").disabled, true, "one click at a time");
  await tick();
  assert.deepEqual(calls, [["diagnostics_preview"]], "no argument: the window sends nothing");
  assert.equal(el("diag-text").textContent, TEXT, "shown as it came, whitespace and all");
  assert.equal(el("diag-dialog").open, true);
  assert.equal(el("diag-copy").disabled, false);
  assert.equal(el("diag-open").disabled, false);
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
  el("diag-copy").dispatch("click"); // disabled in a real page; the handler must refuse too
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
    assert.equal(el("diag-open").disabled, false);
  }
});

test("an empty or odd answer is a failure, not an empty dialog", async () => {
  for (const answer of ["", null, 42, { text: "x" }]) {
    const { el } = setup(() => Promise.resolve(answer));
    el("diag-open").dispatch("click");
    await tick();
    assert.equal(el("diag-dialog").open, false, JSON.stringify(answer));
    assert.notEqual(el("diag-problem").textContent, "");
  }
});

test("closing clears the text, so the next copy is of a fresh preview", async () => {
  const written = [];
  let n = 0;
  const { el } = setup(() => Promise.resolve(`${TEXT}#${++n}`), async (text) => { written.push(text); });
  el("diag-open").dispatch("click");
  await tick();
  el("diag-close").dispatch("click");
  assert.equal(el("diag-dialog").open, false);
  assert.equal(el("diag-text").textContent, "");
  assert.equal(el("diag-copy").disabled, true);
  el("diag-copy").dispatch("click");
  await tick();
  assert.deepEqual(written, [], "nothing to copy once closed");
  el("diag-open").dispatch("click");
  await tick();
  el("diag-copy").dispatch("click");
  await tick();
  assert.deepEqual(written, [`${TEXT}#2`]);
});

test("the page's clipboard uses the async API, else a hidden textarea and the old command", async () => {
  const doc = new FakeDocument();
  const done = [];
  await Diagnostics.pageClipboard(doc, { clipboard: { writeText: async (text) => { done.push(text); } } })("abc");
  assert.deepEqual(done, ["abc"]);

  const boxes = [];
  doc.createElement = () => {
    const box = doc.make("textarea", {});
    box.style = {};
    box.select = () => boxes.push(box.value);
    box.remove = () => {};
    return box;
  };
  doc.execCommand = (name) => name === "copy";
  await Diagnostics.pageClipboard(doc, {})("xyz");
  assert.deepEqual(boxes, ["xyz"]);
  doc.execCommand = () => false;
  await assert.rejects(Diagnostics.pageClipboard(doc, {})("no"));
});

test("the words exist in both languages", () => {
  for (const lang of ["en", "es"]) {
    I18n.setLang(lang);
    for (const key of ["settings.diag.open", "settings.diag.note", "settings.diag.title", "settings.diag.preview", "settings.diag.copy", "settings.diag.copied", "settings.diag.copyFailed", "settings.diag.failed"]) {
      assert.notEqual(t(key, { error: "x" }), key, `${lang} ${key}`);
    }
  }
});

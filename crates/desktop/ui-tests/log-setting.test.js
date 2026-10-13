"use strict";
const test = require("node:test");
const assert = require("node:assert/strict");
const I18n = require("../ui/i18n.js");
require("../ui/strings/common.js");
require("../ui/strings/window.js");
const LogSetting = require("../ui/log-setting.js");
const { FakeDocument } = require("./fake-dom.js");

const t = I18n.t;

function page() {
  const doc = new FakeDocument();
  doc.make("button", { id: "log-open" });
  doc.make("p", { id: "log-problem" });
  return doc;
}

const tick = () => new Promise((resolve) => setImmediate(resolve));

test("the button asks the app to open its log folder and sends nothing else", async () => {
  const doc = page();
  const calls = [];
  LogSetting.start(doc, (...args) => { calls.push(args); return Promise.resolve(); }, t, String);
  const button = doc.getElementById("log-open");
  button.dispatch("click");
  assert.equal(button.disabled, true, "one click at a time");
  await tick();
  assert.deepEqual(calls, [["open_log_folder"]]);
  assert.equal(button.disabled, false);
  assert.equal(doc.getElementById("log-problem").textContent, "");
});

test("a failure is said under the setting, in the player's language", async () => {
  for (const [lang, start] of [["en", "Couldn't open the log folder"], ["es", "No se ha podido abrir la carpeta del registro"]]) {
    I18n.setLang(lang);
    const doc = page();
    LogSetting.start(doc, () => Promise.reject("Could not open the log folder (NotFound)."), t, String);
    doc.getElementById("log-open").dispatch("click");
    await tick();
    assert.ok(doc.getElementById("log-problem").textContent.startsWith(start), doc.getElementById("log-problem").textContent);
    assert.equal(doc.getElementById("log-open").disabled, false);
  }
  I18n.setLang("en");
});

test("every word of the line is in both languages", () => {
  for (const lang of ["en", "es"]) {
    I18n.setLang(lang);
    for (const key of ["settings.log.title", "settings.log.open", "settings.log.note", "settings.log.failed"]) {
      const text = t(key, { error: "x" });
      assert.ok(text && text !== key, `${lang}: ${key}`);
    }
  }
  I18n.setLang("en");
});

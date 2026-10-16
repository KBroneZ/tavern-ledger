"use strict";
const test = require("node:test");
const assert = require("node:assert/strict");
const I18n = require("../ui/i18n.js");
require("../ui/strings/common.js");
require("../ui/strings/window.js");
const TLSetupFix = require("../ui/setup-fix.js");
const TLGameFolder = require("../ui/game-folder.js");
const { FakeDocument } = require("./fake-dom.js");

const t = I18n.t;
const IDS = [
  "setup", "setup-list", "setup-state", "setup-fix", "setup-fix-optional", "setup-fix-problem", "setup-result",
  "setup-result-log", "setup-result-client", "setup-result-restart", "setup-result-close", "setup-folder",
  "setup-choose", "setup-settings", "setup-folder-problem",
];

function page() {
  const doc = new FakeDocument();
  IDS.forEach((id) => doc.make("button", { id }));
  return doc;
}

const text = (doc, id) => doc.getElementById(id).textContent;
const hidden = (doc, id) => doc.getElementById(id).hidden;
const LINE = "client.config (D:\\Hearthstone\\client.config) needs these lines under [Log]: FileSizeLimit.Int=-1.";

const needsFix = { state: "needs_fix", can_fix: true, can_fix_optional: false, fix: null };
const quiet = (fix = null) => ({ state: "ok", can_fix: false, can_fix_optional: false, fix });
const file = (name, result, extra = {}) => ({ file: name, result, reason: null, backup: false, ...extra });

test("a problem that can be fixed shows the Fix button and what it does", () => {
  I18n.setLang("en");
  const doc = page();
  TLSetupFix.render(doc, { setup: [LINE], setup_view: needsFix }, t);
  assert.equal(hidden(doc, "setup"), false);
  assert.equal(hidden(doc, "setup-fix"), false);
  assert.equal(text(doc, "setup-fix"), "Fix");
  assert.equal(doc.getElementById("setup-fix").disabled, false);
  assert.equal(hidden(doc, "setup-fix-optional"), true);
  assert.match(text(doc, "setup-state"), /can fix this for you/);
});

test("the box states: ok hides it, cannot check says so and offers no Fix, fixing disables the button", () => {
  I18n.setLang("en");
  const doc = page();
  TLSetupFix.render(doc, { setup: [], setup_view: quiet() }, t);
  assert.equal(hidden(doc, "setup"), true);

  const cannot = { ...quiet(), state: "cannot_check" };
  TLSetupFix.render(doc, { setup: ["client.config could not be checked: no Hearthstone folder is in use."], setup_view: cannot }, t);
  assert.equal(hidden(doc, "setup"), false);
  assert.equal(hidden(doc, "setup-fix"), true);
  assert.match(text(doc, "setup-state"), /cannot check this right now/);

  TLSetupFix.render(doc, { setup: [LINE], setup_view: { ...needsFix, state: "fixing" } }, t);
  assert.equal(doc.getElementById("setup-fix").disabled, true);
  assert.equal(text(doc, "setup-fix"), "Fixing…");

  TLSetupFix.render(doc, { setup: [LINE], setup_view: { ...needsFix, state: "error" } }, t);
  assert.match(text(doc, "setup-state"), /did not work/);

  // A status from before the first check says nothing and shows nothing.
  TLSetupFix.render(doc, {}, t);
  assert.equal(hidden(doc, "setup"), true);
  assert.equal(hidden(doc, "setup-fix"), true);
});

test("only the optional lines missing offers the optional button, never beside Fix", () => {
  I18n.setLang("en");
  const doc = page();
  const optionalOnly = { ...quiet(), can_fix_optional: true };
  TLSetupFix.render(doc, { setup: ["Optional: log.config can have these lines under [LoadingScreen]."], setup_view: optionalOnly }, t);
  assert.equal(hidden(doc, "setup-fix"), true);
  assert.equal(hidden(doc, "setup-fix-optional"), false);
  assert.equal(text(doc, "setup-fix-optional"), "Add the optional lines");
  TLSetupFix.render(doc, { setup: [LINE], setup_view: { ...needsFix, can_fix_optional: true } }, t);
  assert.equal(hidden(doc, "setup-fix"), false);
  assert.equal(hidden(doc, "setup-fix-optional"), true);
});

test("the result tells each file, the backup and that the game must be restarted", () => {
  I18n.setLang("en");
  const doc = page();
  const fix = { id: 1, game_running: true, files: [file("log.config", "created"), file("client.config", "fixed", { backup: true })] };
  TLSetupFix.render(doc, { setup: [], setup_view: quiet(fix) }, t);
  assert.equal(hidden(doc, "setup"), false, "a result keeps the box open though nothing is wrong now");
  assert.equal(text(doc, "setup-result-log"), "log.config created.");
  assert.equal(text(doc, "setup-result-client"), "client.config fixed. The old file was copied to a .bak file beside it first.");
  assert.equal(text(doc, "setup-result-restart"), "Hearthstone is running. Restart it for the new logging settings to take effect.");
  fix.game_running = false;
  TLSetupFix.render(doc, { setup: [], setup_view: quiet(fix) }, t);
  assert.match(text(doc, "setup-result-restart"), /If it is open now, restart it\.$/);
});

test("one file can work while the other does not, each said in its own line", () => {
  I18n.setLang("en");
  const doc = page();
  const fix = { id: 2, game_running: false, files: [file("log.config", "fixed"), file("client.config", "failed", { reason: "access_denied" })] };
  TLSetupFix.render(doc, { setup: [LINE], setup_view: { ...needsFix, state: "error", fix } }, t);
  assert.equal(text(doc, "setup-result-log"), "log.config fixed.");
  assert.equal(text(doc, "setup-result-client"), "client.config was not changed: Windows would not let Tavern Ledger write there. Use the steps above to do it by hand.");
  // A file that was already fine is not mentioned; nothing worked: no restart note.
  const none = { id: 3, game_running: true, files: [file("log.config", "already_ok"), file("client.config", "refused", { reason: "no_game_exe" })] };
  TLSetupFix.render(doc, { setup: [LINE], setup_view: { ...needsFix, state: "error", fix: none } }, t);
  assert.equal(hidden(doc, "setup-result-log"), true);
  assert.equal(hidden(doc, "setup-result-restart"), true);
  assert.match(text(doc, "setup-result-client"), /no Hearthstone\.exe/);
});

test("every reason has its words in both languages", () => {
  for (const lang of ["en", "es"]) {
    I18n.setLang(lang);
    for (const reason of ["link", "not_a_file", "not_plain_text", "folder_missing", "no_game_exe", "no_path", "access_denied", "read_only", "path_too_long", "locked", "changed", "disk_full", "too_large", "too_many_backups", "other"]) {
      const line = TLSetupFix.resultLine({ file: "client.config", result: "failed", reason }, t);
      assert.ok(!line.includes("setup.fix"), `${lang} ${reason}: ${line}`);
    }
    // A reason the page does not know falls back to the plain one.
    assert.equal(TLSetupFix.resultLine({ file: "log.config", result: "failed", reason: "new" }, t),
      TLSetupFix.resultLine({ file: "log.config", result: "failed", reason: "other" }, t));
  }
  I18n.setLang("es");
  assert.equal(t("setup.fix"), "Arreglar");
  I18n.setLang("en");
});

test("the folder button goes away once the folder is usable", () => {
  I18n.setLang("en");
  const doc = page();
  ["folder-current", "folder-reset", "folder-offer", "folder-offer-text"].forEach((id) => doc.make("p", { id }));
  TLGameFolder.render(doc, { folder: { path: "D:\\H", how: "chosen", chosen: true, usable: true } }, t);
  assert.equal(hidden(doc, "setup-choose"), true);
  TLGameFolder.render(doc, { folder: { path: "D:\\H", how: "chosen", chosen: true, usable: false } }, t);
  assert.equal(hidden(doc, "setup-choose"), false);
  TLGameFolder.render(doc, { folder: { path: null, how: null, chosen: false, usable: false } }, t);
  assert.equal(hidden(doc, "setup-choose"), false);
});

function fakeBackend(answers) {
  const calls = [];
  const call = async (name, args) => {
    calls.push([name, args]);
    const answer = await (typeof answers[name] === "function" ? answers[name](args) : answers[name]);
    if (answer instanceof Error) throw answer;
    return answer;
  };
  const handlers = {};
  const listen = async (name, fn) => { handlers[name] = fn; return () => {}; };
  return { call, calls, listen, handlers };
}

const settle = () => new Promise((resolve) => setImmediate(resolve));
const fixCalls = (backend) => backend.calls.filter(([name]) => name === "setup_fix");

test("Fix asks the app once, sends no path, and the box follows the new status", async () => {
  I18n.setLang("en");
  const doc = page();
  const done = (id) => ({ setup: [], setup_view: quiet({ id, game_running: false, files: [file("client.config", "fixed")] }) });
  let status = { setup: [LINE], setup_view: needsFix };
  const backend = fakeBackend({
    status: () => status,
    setup_fix: () => { status = done(1); },
  });
  await TLSetupFix.start(doc, backend.call, backend.listen, t, I18n.backend, null);
  await settle();
  doc.getElementById("setup-fix").dispatch("click");
  await settle();
  assert.deepEqual(fixCalls(backend), [["setup_fix", { optional: false }]]);
  assert.equal(text(doc, "setup-result-client"), "client.config fixed.");
  assert.equal(text(doc, "setup-fix-problem"), "");

  // Closing the result hides it; the next click brings a new one.
  doc.getElementById("setup-result-close").dispatch("click");
  assert.equal(hidden(doc, "setup-result"), true);
  assert.equal(hidden(doc, "setup"), true);
  backend.handlers["status-changed"]({ payload: done(1) });
  assert.equal(hidden(doc, "setup-result"), true, "the same result stays closed");
  backend.handlers["status-changed"]({ payload: done(2) });
  assert.equal(hidden(doc, "setup-result"), false);
});

test("the optional button asks for the optional lines", async () => {
  const doc = page();
  const status = { setup: ["Optional: x"], setup_view: { ...quiet(), can_fix_optional: true } };
  const backend = fakeBackend({ status, setup_fix: undefined });
  await TLSetupFix.start(doc, backend.call, backend.listen, t, I18n.backend, null);
  await settle();
  doc.getElementById("setup-fix-optional").dispatch("click");
  await settle();
  assert.deepEqual(fixCalls(backend), [["setup_fix", { optional: true }]]);
});

test("a click while the app is still working is ignored, and an error is said plainly", async () => {
  I18n.setLang("es");
  const doc = page();
  let release;
  const gate = new Promise((resolve) => { release = resolve; });
  const failure = new Error("x");
  failure.toString = () => "A fix is already running.";
  const backend = fakeBackend({ status: { setup: [LINE], setup_view: needsFix }, setup_fix: () => gate.then(() => failure) });
  await TLSetupFix.start(doc, backend.call, backend.listen, t, I18n.backend, null);
  await settle();
  const button = doc.getElementById("setup-fix");
  button.dispatch("click");
  button.dispatch("click");
  await settle();
  assert.equal(button.disabled, true);
  assert.equal(fixCalls(backend).length, 1);
  release();
  await settle();
  assert.match(text(doc, "setup-fix-problem"), /^No se ha podido arreglar: /);
  I18n.setLang("en");
});

test("coming back to the window checks the files again", async () => {
  const doc = page();
  const listeners = {};
  const win = { addEventListener: (type, fn) => { listeners[type] = fn; } };
  const backend = fakeBackend({ status: { setup: [], setup_view: {} }, setup_recheck: undefined });
  await TLSetupFix.start(doc, backend.call, backend.listen, t, I18n.backend, win);
  listeners.focus();
  await settle();
  assert.equal(backend.calls.some(([name]) => name === "setup_recheck"), true);
});

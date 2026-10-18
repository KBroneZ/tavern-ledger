"use strict";
// What the Start with Windows switch shows for each answer of the app
// (src/startup.rs), in both languages.
const test = require("node:test");
const assert = require("node:assert/strict");
const I18n = require("../ui/i18n.js");
require("../ui/strings/common.js");
const words = require("../ui/autostart-words.js");

const view = (extra = {}) => ({ available: true, enabled: false, state: "off", blocked: null, ...extra });

test.beforeEach(() => I18n.setLang("en"));

test("a working switch shows its state and no note", () => {
  assert.deepEqual(words.show(view()), { checked: false, disabled: false, note: "" });
  assert.deepEqual(words.show(view({ enabled: true, state: "on" })), { checked: true, disabled: false, note: "" });
});

test("a start-up the player turned off in Task Manager stays off and says how to turn it on", () => {
  const shown = words.show(view({ state: "off_by_user", blocked: "Windows has Tavern Ledger's start-up turned off. Turn it on in Task Manager > Startup apps; the app cannot do it for you." }));
  assert.equal(shown.checked, false);
  assert.equal(shown.disabled, true);
  assert.match(shown.note, /Task Manager/);
});

test("a policy locks the switch either way", () => {
  const blocked = "Your administrator or a policy decides whether Tavern Ledger starts with Windows.";
  const on = words.show(view({ enabled: true, state: "on_by_policy", blocked }));
  assert.equal(on.checked, true);
  assert.equal(on.disabled, true);
  assert.equal(on.note, blocked);
  assert.equal(words.show(view({ state: "off_by_policy", blocked })).disabled, true);
});

test("without the option the switch is off and the note says it is not available", () => {
  const shown = words.show(view({ available: false }));
  assert.equal(shown.disabled, true);
  assert.match(shown.note, /Not available/);
});

test("the notes are in Spanish too", () => {
  I18n.setLang("es");
  const shown = words.show(view({ state: "off_by_user", blocked: "Windows has Tavern Ledger's start-up turned off. Turn it on in Task Manager > Startup apps; the app cannot do it for you." }));
  assert.match(shown.note, /Administrador de tareas/);
  const policy = words.show(view({ state: "off_by_policy", blocked: "Your administrator or a policy decides whether Tavern Ledger starts with Windows." }));
  assert.match(policy.note, /administrador/);
});

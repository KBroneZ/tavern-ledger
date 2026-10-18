"use strict";
// What the updates setting and the update notice say for each state the app
// sends (src/updates.rs), in both languages.
const test = require("node:test");
const assert = require("node:assert/strict");
const I18n = require("../ui/i18n.js");
require("../ui/strings/common.js");
require("../ui/strings/updates.js");
const words = require("../ui/update-words.js");

const view = (state, extra = {}) => ({ current: "0.1.0", auto_check: true, state, checked_at: null, notice: null, warning: null, ...extra });
const at = Math.floor(new Date(2026, 9, 13, 14, 5).getTime() / 1000);

test.beforeEach(() => I18n.setLang("en"));

test("a build without a key says so and offers no check", () => {
  assert.match(words.line(view("off")), /aren't set up/);
  assert.equal(words.canCheck(view("off")), false);
  assert.equal(words.notice(view("off")), null);
});

test("each state has its own line, with the version or the time of the check", () => {
  assert.equal(words.line(view("idle")), "Not checked yet.");
  assert.equal(words.line(view("checking")), "Checking for updates…");
  assert.equal(words.line(view("downloading", { version: "0.2.0" })), "Downloading version 0.2.0…");
  assert.equal(words.line(view("ready", { version: "0.2.0" })), "Version 0.2.0 is ready. Restart to update.");
  assert.equal(words.line(view("latest", { checked_at: at })), `You have the latest version (checked ${I18n.dateTime(new Date(at * 1000))}).`);
  assert.match(words.line(view("installing", { version: "0.2.0" })), /closes and opens again/);
});

test("a refused or failed check says why, and an unknown reason is not shown as a key", () => {
  assert.match(words.line(view("failed", { reason: "signature", checked_at: at })), /isn't signed by the project's key/);
  assert.match(words.line(view("failed", { reason: "network", checked_at: at })), /Couldn't reach GitHub/);
  assert.match(words.line(view("failed", { reason: "not_found" })), /No update information/);
  assert.match(words.line(view("failed", { reason: "install" })), /couldn't be installed/);
  assert.match(words.line(view("failed", { reason: "something new" })), /didn't look right/);
});

test("the notice shows only what the app offers, and check now waits while one runs", () => {
  assert.equal(words.notice(view("ready", { version: "0.2.0", notice: "0.2.0" })), "Version 0.2.0 is ready. Restart to update.");
  assert.equal(words.notice(view("ready", { version: "0.2.0", notice: null })), null, "after Later");
  assert.match(words.notice(view("failed", { reason: "install" })), /couldn't be installed/, "after Restart now failed");
  assert.equal(words.notice(view("failed", { reason: "network" })), null, "a failed check stays in Settings");
  for (const state of ["idle", "latest", "failed"]) assert.equal(words.canCheck(view(state)), true, state);
  for (const state of ["checking", "downloading", "ready", "installing"]) assert.equal(words.canCheck(view(state)), false, state);
});

test("the words are in Spanish too, and the app's refusals are translated", () => {
  I18n.setLang("es");
  assert.equal(words.line(view("ready", { version: "0.2.0" })), "La versión 0.2.0 está lista. Reinicia para actualizar.");
  assert.equal(I18n.backend("Finish your game first: updating restarts the app."), "Termina la partida primero: actualizar reinicia la app.");
});

test("the Store edition shows no updates setting and no notice", () => {
  assert.equal(words.shown(view("store")), false);
  assert.equal(words.notice(view("store")), null);
  for (const state of ["off", "idle", "ready", "failed"]) assert.equal(words.shown(view(state)), true, state);
});

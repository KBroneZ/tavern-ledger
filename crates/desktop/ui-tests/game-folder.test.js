"use strict";
const test = require("node:test");
const assert = require("node:assert/strict");
const I18n = require("../ui/i18n.js");
require("../ui/strings/common.js");
require("../ui/strings/window.js");
const TLGameFolder = require("../ui/game-folder.js");
const { FakeDocument } = require("./fake-dom.js");

const t = I18n.t;
const IDS = [
  "folder-current", "folder-reset", "folder-choose", "folder-problem", "setup-folder", "setup-choose",
  "setup-settings", "setup-folder-problem", "folder-offer", "folder-offer-text", "folder-offer-yes",
  "folder-offer-no", "tab-settings",
];

function page() {
  const doc = new FakeDocument();
  IDS.forEach((id) => doc.make("button", { id }));
  const tab = doc.getElementById("tab-settings");
  tab.clicked = 0;
  tab.click = () => { tab.clicked += 1; };
  return doc;
}

const text = (doc, id) => doc.getElementById(id).textContent;
const chosen = { path: "D:\Hearthstone", how: "chosen", chosen: true };
const automatic = { path: "D:\Hearthstone", how: "registry", chosen: false };

test("the setting shows the folder in use and how it was decided", () => {
  I18n.setLang("en");
  const doc = page();
  TLGameFolder.render(doc, { folder: automatic }, t);
  assert.equal(text(doc, "folder-current"), "D:\Hearthstone (found automatically, from Windows' install record)");
  assert.equal(doc.getElementById("folder-reset").hidden, true, "nothing to go back from");
  TLGameFolder.render(doc, { folder: chosen }, t);
  assert.equal(text(doc, "folder-current"), "D:\Hearthstone (chosen by you)");
  assert.equal(doc.getElementById("folder-reset").hidden, false);
});

test("every way a folder is found has its words in both languages", () => {
  for (const lang of ["en", "es"]) {
    I18n.setLang(lang);
    const words = ["chosen", "running", "registry", "battlenet", "usual", "scan"].map((how) => TLGameFolder.howText(how, t));
    assert.equal(new Set(words).size, 6, lang);
    words.forEach((w) => assert.ok(w && !w.startsWith("folder."), `${lang}: ${w}`));
  }
  assert.equal(TLGameFolder.howText("unknown", t), "");
  I18n.setLang("en");
});

test("with no folder the setting says so and the banner asks to choose one", () => {
  I18n.setLang("en");
  const doc = page();
  TLGameFolder.render(doc, { folder: { path: null, how: null, chosen: false } }, t);
  assert.equal(text(doc, "folder-current"), "Not found yet");
  assert.match(text(doc, "setup-folder"), /^No Hearthstone folder is in use yet\./);
  assert.equal(doc.getElementById("folder-reset").hidden, true);
  // A status from before the first look has no folder at all.
  TLGameFolder.render(doc, {}, t);
  assert.equal(text(doc, "folder-current"), "Not found yet");
});

test("the setup box names the folder it checked, chosen or found", () => {
  I18n.setLang("en");
  const doc = page();
  TLGameFolder.render(doc, { folder: chosen }, t);
  assert.equal(text(doc, "setup-folder"), "Checked in the Hearthstone folder D:\Hearthstone (chosen by you). Wrong folder? Choose the right one.");
  TLGameFolder.render(doc, { folder: automatic }, t);
  assert.match(text(doc, "setup-folder"), /^Checked in the Hearthstone folder D:\Hearthstone \(found automatically, from Windows' install record\)\./);
  I18n.setLang("es");
  TLGameFolder.render(doc, { folder: chosen }, t);
  assert.equal(text(doc, "setup-folder"), "Comprobado en la carpeta de Hearthstone D:\Hearthstone (elegida por ti). ¿No es la carpeta correcta? Elige la correcta.");
  I18n.setLang("en");
});

test("the running game's folder is offered only while the app says so", () => {
  I18n.setLang("en");
  const doc = page();
  TLGameFolder.render(doc, { folder: chosen, folder_offer: "E:\Games\Hearthstone" }, t);
  assert.equal(doc.getElementById("folder-offer").hidden, false);
  assert.equal(text(doc, "folder-offer-text"), "Hearthstone is running from E:\Games\Hearthstone. Use this folder?");
  TLGameFolder.render(doc, { folder: chosen, folder_offer: null }, t);
  assert.equal(doc.getElementById("folder-offer").hidden, true);
  assert.equal(text(doc, "folder-offer-text"), "");
});

function fakeBackend(answers) {
  const calls = [];
  const call = async (name) => {
    calls.push(name);
    const answer = answers[name];
    if (answer instanceof Error) throw answer;
    return answer;
  };
  const handlers = {};
  const listen = async (name, fn) => { handlers[name] = fn; return () => {}; };
  return { call, calls, listen, handlers };
}

const settle = () => new Promise((resolve) => setImmediate(resolve));

test("both Choose buttons open the app's picker and send no path", async () => {
  I18n.setLang("en");
  const doc = page();
  const backend = fakeBackend({ status: { folder: automatic }, game_folder_choose: true });
  await TLGameFolder.start(doc, backend.call, backend.listen, t, I18n.backend);
  await settle();
  doc.getElementById("folder-choose").dispatch("click");
  await settle();
  doc.getElementById("setup-choose").dispatch("click");
  await settle();
  assert.equal(backend.calls.filter((c) => c === "game_folder_choose").length, 2);
  assert.equal(text(doc, "folder-problem"), "");
  assert.equal(doc.getElementById("folder-choose").disabled, false, "buttons come back");
});

test("a pick that is not the game is said where the player looks, in both languages", async () => {
  const refusal = new Error("x");
  refusal.toString = () => "Hearthstone.exe is not in that folder, and it has no Logs folder with game sessions.";
  const doc = page();
  const backend = fakeBackend({ status: { folder: automatic }, game_folder_choose: refusal });
  I18n.setLang("es");
  await TLGameFolder.start(doc, backend.call, backend.listen, t, I18n.backend);
  doc.getElementById("setup-choose").dispatch("click");
  await settle();
  const said = "No se ha cambiado: Hearthstone.exe no está en esa carpeta y no tiene una carpeta Logs con partidas.";
  assert.equal(text(doc, "folder-problem"), said);
  assert.equal(text(doc, "setup-folder-problem"), said);
  // The next try starts clean.
  backend.calls.length = 0;
  doc.getElementById("folder-choose").dispatch("click");
  await settle();
  assert.equal(backend.calls.includes("game_folder_choose"), true);
  I18n.setLang("en");
});

test("reset and the notice's two answers are single commands, and the status follows", async () => {
  I18n.setLang("en");
  const doc = page();
  const backend = fakeBackend({ status: { folder: chosen }, game_folder_reset: null, game_folder_use_running: null, game_folder_decline_running: null });
  await TLGameFolder.start(doc, backend.call, backend.listen, t, I18n.backend);
  for (const id of ["folder-reset", "folder-offer-yes", "folder-offer-no"]) {
    doc.getElementById(id).dispatch("click");
    await settle();
  }
  assert.deepEqual(backend.calls.filter((c) => c.startsWith("game_folder")), ["game_folder_reset", "game_folder_use_running", "game_folder_decline_running"]);
  backend.handlers["status-changed"]({ payload: { folder: automatic } });
  assert.match(text(doc, "folder-current"), /found automatically/);
});

test("the link in the setup box opens the Settings tab and focuses Choose", async () => {
  I18n.setLang("en");
  const doc = page();
  const backend = fakeBackend({ status: { folder: automatic } });
  await TLGameFolder.start(doc, backend.call, backend.listen, t, I18n.backend);
  doc.getElementById("setup-settings").dispatch("click");
  assert.equal(doc.getElementById("tab-settings").clicked, 1);
  assert.equal(doc.activeElement, doc.getElementById("folder-choose"));
});

"use strict";
// The app's languages (T-325): every key in every language, the same
// placeholders, every key the pages and scripts use defined, and the
// formatting of numbers and dates per language.
const test = require("node:test");
const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const I18n = require("../ui/i18n.js");

const UI = path.join(__dirname, "..", "ui");
const STRINGS = path.join(UI, "strings");
const files = fs.readdirSync(STRINGS).filter((f) => f.endsWith(".js"));
files.forEach((f) => require(path.join(STRINGS, f)));
const tables = I18n.tables();

const placeholders = (text) => [...String(text).matchAll(/\{(\w+)\}/g)].map((m) => m[1]).sort();
const read = (file) => fs.readFileSync(file, "utf8");
const scripts = () => fs.readdirSync(UI).filter((f) => f.endsWith(".js")).map((f) => [f, read(path.join(UI, f))]);
const pages = () => fs.readdirSync(UI).filter((f) => f.endsWith(".html")).map((f) => [f, read(path.join(UI, f))]);

test("every string table loads and has both languages", () => {
  assert.ok(files.includes("common.js"));
  files.forEach((f) => {
    const table = require(path.join(STRINGS, f));
    assert.ok(table.en && table.es, f);
  });
});

test("no key is missing in Spanish, and Spanish has no key English lacks", () => {
  const en = Object.keys(tables.en);
  const es = Object.keys(tables.es);
  assert.deepEqual(en.filter((k) => !(k in tables.es)), [], "missing in Spanish");
  assert.deepEqual(es.filter((k) => !(k in tables.en)), [], "only in Spanish");
});

test("no string is empty and each translation has the same placeholders", () => {
  for (const [key, text] of Object.entries(tables.en)) {
    assert.ok(String(text).trim(), `en ${key} is empty`);
    assert.ok(String(tables.es[key]).trim(), `es ${key} is empty`);
    assert.deepEqual(placeholders(tables.es[key]), placeholders(text), key);
  }
});

test("plural keys come in pairs", () => {
  const keys = Object.keys(tables.en);
  keys.filter((k) => k.endsWith(".one")).forEach((k) => assert.ok(keys.includes(k.replace(/\.one$/, ".other")), k));
  keys.filter((k) => k.endsWith(".other")).forEach((k) => assert.ok(keys.includes(k.replace(/\.other$/, ".one")), k));
});

test("every key the scripts and pages name is defined", () => {
  const missing = [];
  for (const [name, text] of scripts()) {
    for (const m of text.matchAll(/\bt\(\s*"([^"]+)"/g)) if (!(m[1] in tables.en)) missing.push(`${name}: ${m[1]}`);
    for (const m of text.matchAll(/\bplural\(\s*"([^"]+)"/g)) {
      if (!(`${m[1]}.one` in tables.en)) missing.push(`${name}: ${m[1]}.one`);
    }
  }
  for (const [name, text] of pages()) {
    for (const m of text.matchAll(/data-i18n="([^"]+)"/g)) if (!(m[1] in tables.en)) missing.push(`${name}: ${m[1]}`);
    for (const m of text.matchAll(/data-i18n-attr="([^"]+)"/g)) {
      m[1].split(";").forEach((pair) => {
        const key = pair.split(":")[1];
        if (key && !(key.trim() in tables.en)) missing.push(`${name}: ${key}`);
      });
    }
  }
  assert.deepEqual(missing, []);
});

test("both pages load the language before any script that draws words", () => {
  for (const [name, text] of pages()) {
    const i18n = text.indexOf('<script src="i18n.js">');
    assert.ok(i18n > 0, name);
    const first = text.indexOf("<script src=");
    assert.equal(first, i18n, `${name}: i18n.js is the first script`);
    const strings = [...text.matchAll(/<script src="strings\/([^"]+)">/g)].map((m) => m[1]);
    assert.ok(strings.includes("common.js"), name);
    strings.forEach((f) => assert.ok(files.includes(f), `${name} loads strings/${f}, which does not exist`));
  }
});

test("every tribe, source, keyword, mode and result has its words", () => {
  const tribes = ["ABERRATION", "BEAST", "DEMON", "DRAGON", "ELEMENTAL", "MECHANICAL", "MURLOC", "NAGA", "PIRATE", "QUILBOAR", "UNDEAD", "NEUTRAL", "ALL"];
  tribes.forEach((t) => assert.ok(`tribe.${t}` in tables.en, t));
  ["log", "inferred", "entered", "leaderboard", "card_data", "possible", "unknown"].forEach((s) => {
    assert.ok(`source.${s}.label` in tables.en && `source.${s}.description` in tables.en, s);
  });
  ["taunt", "divine_shield", "reborn", "venomous", "windfury", "stealth", "deathrattle", "rally"].forEach((k) => {
    assert.ok(`keyword.${k}` in tables.en && `keyword.${k}.short` in tables.en, k);
  });
  ["won", "lost", "tie", "unknown"].forEach((o) => assert.ok(`outcome.${o}` in tables.en, o));
  ["GT_BATTLEGROUNDS", "GT_BATTLEGROUNDS_DUO", "OTHER"].forEach((m) => assert.ok(`mode.${m}` in tables.en, m));
});

test("the language is the saved choice, else the system's, else English", () => {
  assert.equal(I18n.pick("es", ["en-US"]), "es");
  assert.equal(I18n.pick("en", ["es-ES"]), "en");
  assert.equal(I18n.pick(null, ["es-MX", "en-US"]), "es");
  assert.equal(I18n.pick(null, ["fr-FR", "es-ES"]), "es");
  assert.equal(I18n.pick(undefined, ["fr-FR"]), "en");
  assert.equal(I18n.pick("fr", []), "en");
  assert.equal(I18n.pick(null, undefined), "en");
});

test("a string fills its placeholders and a missing key shows the key", () => {
  I18n.setLang("es");
  assert.equal(I18n.t("settings.language.system", { name: "Español" }), "Idioma de Windows (Español)");
  assert.equal(I18n.t("no.such.key"), "no.such.key");
  // A placeholder without a value stays visible rather than turning into "undefined".
  assert.equal(I18n.t("settings.language.system"), "Idioma de Windows ({name})");
  I18n.setLang("en");
  assert.equal(I18n.t("settings.language.system", { name: "English" }), "Windows language (English)");
});

test("numbers, shares and dates are written the language's way", () => {
  I18n.setLang("en");
  assert.equal(I18n.num(2.5, 2), "2.50");
  assert.equal(I18n.num(1234), "1234");
  assert.equal(I18n.percent(0.5), "50%");
  const d = new Date(2026, 9, 9, 15, 26);
  assert.equal(I18n.date(d), "09/10/2026");
  I18n.setLang("es");
  assert.equal(I18n.num(2.5, 2), "2,50");
  assert.equal(I18n.num(12345.5, 1), "12.345,5");
  assert.match(I18n.percent(0.5), /^50\s%$/);
  assert.equal(I18n.date(d), "09/10/2026");
  assert.match(I18n.dateTime(d), /09\/10\/2026,? 15:26/);
  assert.equal(I18n.list(["a", "b", "c"]), "a, b y c");
  I18n.setLang("en");
  assert.equal(I18n.list(["a", "b", "c"]), "a, b and c");
});

test("a fixed sentence from the app is translated, any other is shown as written", () => {
  I18n.setLang("es");
  assert.equal(I18n.backend("In the main menu"), "En el menú principal");
  assert.equal(I18n.backend("Could not save the report (NotFound)."), "Could not save the report (NotFound).");
  assert.equal(I18n.backend(null), null);
  I18n.setLang("en");
  assert.equal(I18n.backend("In the main menu"), "In the main menu");
});

test("plural forms follow the language", () => {
  I18n.define({ en: { "test.games.one": "{n} game", "test.games.other": "{n} games" }, es: { "test.games.one": "{n} partida", "test.games.other": "{n} partidas" } });
  I18n.setLang("en");
  assert.equal(I18n.plural("test.games", 1), "1 game");
  assert.equal(I18n.plural("test.games", 0), "0 games");
  I18n.setLang("es");
  assert.equal(I18n.plural("test.games", 1), "1 partida");
  assert.equal(I18n.plural("test.games", 2), "2 partidas");
  I18n.setLang("en");
});

test("the markup's words and attributes are set from the table", () => {
  const nodes = [
    { attrs: { "data-i18n": "settings.title" }, textContent: "x" },
    { attrs: { "data-i18n-attr": "aria-label:settings.tab; title:settings.tab" }, textContent: "" },
  ];
  nodes.forEach((n) => {
    n.getAttribute = (a) => n.attrs[a];
    n.setAttribute = (a, v) => { n.attrs[a] = v; };
  });
  const root = { querySelectorAll: (sel) => nodes.filter((n) => n.attrs[sel.slice(1, -1)] !== undefined) };
  I18n.setLang("es");
  I18n.apply(root);
  assert.equal(nodes[0].textContent, "Ajustes");
  assert.equal(nodes[1].attrs["aria-label"], "Ajustes");
  assert.equal(nodes[1].attrs.title, "Ajustes");
  I18n.setLang("en");
});

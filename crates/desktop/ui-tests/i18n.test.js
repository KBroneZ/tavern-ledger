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
  ["log", "inferred", "entered", "leaderboard", "card_data", "possible", "game", "unknown"].forEach((s) => {
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

test("the setup box's lines are translated with their paths and settings kept", () => {
  I18n.setLang("es");
  // As tracker::setup writes them.
  const missing = "client.config not found (D:/Games/client.config). Without it the game stops writing Power.log at about 10 MB and later games are lost. Create it with [Log] and these lines: FileSizeLimit.Int=-1. Then restart the game.";
  assert.equal(I18n.backend(missing), "No se encuentra client.config (D:/Games/client.config). Sin él, el juego deja de escribir Power.log al llegar a unos 10 MB y se pierden las partidas siguientes. Créalo con [Log] y estas líneas: FileSizeLimit.Int=-1. Después reinicia el juego.");
  const lines = "log.config (D:/a b/log.config) needs these lines under [Power]: LogLevel=1, FilePrinting=true. Without the Power log Tavern Ledger sees no games. Restart the game afterwards.";
  assert.equal(I18n.backend(lines), "log.config (D:/a b/log.config) necesita estas líneas en [Power]: LogLevel=1, FilePrinting=true. Sin el registro Power, Tavern Ledger no ve ninguna partida. Después reinicia el juego.");
  const locked = "log.config could not be read (PermissionDenied): D:/x/log.config. Check that the file is not locked or protected.";
  assert.match(I18n.backend(locked), /^No se puede leer log\.config \(PermissionDenied\): D:\/x\/log\.config\./);
  const optional = "Optional: log.config (D:/x/log.config) can have these lines under [LoadingScreen]: LogLevel=1, FilePrinting=true. Without them the app reads Power.log only and works as before; with them it shows when you are in the Battlegrounds lobby or a game is loading, and the overlay ends a game you leave before it is over. Restart the game afterwards.";
  assert.match(I18n.backend(optional), /^Opcional: log\.config \(D:\/x\/log\.config\) puede tener estas líneas en \[LoadingScreen\]: LogLevel=1, FilePrinting=true\. Sin ellas/);
  // A file the app cannot find a place for is "cannot check", never "missing" (T-376).
  assert.equal(I18n.backend("client.config could not be checked: no Hearthstone folder is in use."), "No se puede comprobar client.config: no se está usando ninguna carpeta de Hearthstone.");
  assert.equal(I18n.backend("log.config could not be checked: this PC's local data folder is not known."), "No se puede comprobar log.config: no se conoce la carpeta de datos locales de este PC.");
  I18n.setLang("en");
  assert.equal(I18n.backend(missing), missing);
});

test("the folder lines from the app are translated with the path kept", () => {
  I18n.setLang("es");
  const none = "Tavern Ledger could not find Hearthstone on this PC. Choose the folder where the game is installed, or start the game once.";
  assert.match(I18n.backend(none), /^Tavern Ledger no ha encontrado Hearthstone en este PC\./);
  // As tracker::setup::folder_messages writes it, the reason a sentence of its own.
  const gone = "The Hearthstone folder you chose (E:\Program Files (x86)\Hearthstone) cannot be used. That folder was not found. Choose it again, or go back to finding it automatically.";
  assert.equal(I18n.backend(gone), "La carpeta de Hearthstone que elegiste (E:\Program Files (x86)\Hearthstone) no se puede usar. No se ha encontrado esa carpeta. Elígela de nuevo o vuelve a la búsqueda automática.");
  const refused = "Hearthstone.exe is not in that folder, and it has no Logs folder with game sessions.";
  assert.equal(I18n.backend(refused), "Hearthstone.exe no está en esa carpeta y no tiene una carpeta Logs con partidas.");
  I18n.setLang("en");
  assert.equal(I18n.backend(gone), gone);
});

test("English dates follow the PC's own English, en-GB when it has none", () => {
  const d = new Date(2026, 9, 9);
  I18n.setLang("en");
  I18n.englishFrom(["en-US", "es-ES"]);
  assert.equal(I18n.date(d), "10/09/2026");
  I18n.englishFrom(["es-ES"]);
  assert.equal(I18n.date(d), "09/10/2026");
  I18n.englishFrom(["en"]);
  assert.equal(I18n.date(d), "09/10/2026");
});

test("the warning about another upload server is translated with its address kept", () => {
  I18n.setLang("es");
  const warning = "Uploads go to https://other.example, not to Tavern Ledger's own server.";
  assert.equal(I18n.backend(warning), "Las subidas van a https://other.example, no al servidor de Tavern Ledger.");
  I18n.setLang("en");
  assert.equal(I18n.backend(warning), warning);
});

test("why a game's lines could not be read is translated with what to do", () => {
  I18n.setLang("es");
  assert.equal(
    I18n.backend("the game log has no times (restart Hearthstone and play a new game)"),
    "el registro del juego no tiene horas (reinicia Hearthstone y juega una partida nueva)"
  );
  assert.equal(
    I18n.backend("the game log has too many unreadable lines (restart Hearthstone and play a new game)"),
    "el registro del juego tiene demasiadas líneas ilegibles (reinicia Hearthstone y juega una partida nueva)"
  );
  assert.equal(
    I18n.backend("the log was cut in the middle of a line (without client.config the game stops writing Power.log at 10 MB)"),
    "El registro se cortó a mitad de una línea (sin client.config el juego deja de escribir Power.log al llegar a 10 MB)"
  );
  assert.equal(I18n.backend("skipped 1 unreadable log line"), "Se ha omitido 1 línea ilegible del registro");
  assert.equal(I18n.backend("skipped 7 unreadable log lines"), "Se han omitido 7 líneas ilegibles del registro");
  I18n.setLang("en");
  assert.equal(I18n.backend("skipped 7 unreadable log lines"), "skipped 7 unreadable log lines");
});

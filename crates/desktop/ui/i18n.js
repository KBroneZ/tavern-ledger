// The app's languages (T-325, D-065): English is the source, Spanish (Spain)
// is complete. Every string the window and the overlay show goes through
// `t(key)`; the tables are in ui/strings/, one file per part of the app, and
// a test (ui-tests/i18n.test.js) fails when a key is missing in a language.
// Numbers and dates are formatted for the language with Intl. Card and hero
// names come from the card data in the language (cards.js), else the log's.
//
// The language is the one chosen in Settings, else Windows' own (read from
// the web view, which follows it), else English. A change in Settings reloads
// both windows, so every string is drawn again in the new language.
"use strict";

(() => {
  const LANGS = ["en", "es"];
  // The locale used for numbers and dates in each language. Spanish is
  // Spain's; English follows the PC's own English (en-US writes dates
  // month first), en-GB when the PC has none.
  const LOCALES = { en: "en-GB", es: "es-ES" };
  function englishFrom(systemLangs) {
    const tag = (systemLangs || []).map(String).find((l) => /^en-[a-z]{2}$/i.test(l));
    LOCALES.en = tag || "en-GB";
  }
  const tables = { en: {}, es: {} };
  // Fixed sentences the app's Rust side writes in English, by their English
  // text: the ones with a translation are shown translated, any other is shown
  // as written (a parser warning, an error with a file name in it).
  const backendTables = { es: {} };
  // Sentence shapes from the Rust side with values in them (a path, a list
  // of settings): [whole-sentence regex source, text with $1, $2…]. Each
  // value goes through the exact table first, so a fixed sentence inside one
  // is translated too and a path stays as it is.
  const backendPatterns = { es: [] };
  let lang = "en";

  // Adds a table: { en: { key: text }, es: { key: text }, backend: { es: { english: text } } }.
  // A key defined twice is a mistake in the tables, not something to merge.
  function define(table) {
    LANGS.forEach((l) => {
      Object.entries(table[l] || {}).forEach(([key, text]) => {
        if (Object.prototype.hasOwnProperty.call(tables[l], key)) throw new Error(`i18n: ${key} is defined twice`);
        tables[l][key] = text;
      });
    });
    Object.assign(backendTables.es, (table.backend && table.backend.es) || {});
    ((table.backendPatterns && table.backendPatterns.es) || []).forEach(([source, text]) => {
      backendPatterns.es.push([new RegExp(`^${source}$`), text]);
    });
  }

  // The language to use: the saved choice when it is one we have, else the
  // first of the system's languages we have, else English.
  function pick(saved, systemLangs) {
    if (LANGS.includes(saved)) return saved;
    for (const tag of systemLangs || []) {
      const base = String(tag).toLowerCase().split("-")[0];
      if (LANGS.includes(base)) return base;
    }
    return "en";
  }

  const fill = (text, params) =>
    String(text).replace(/\{(\w+)\}/g, (whole, name) => (params && params[name] !== undefined && params[name] !== null ? String(params[name]) : whole));

  // The text of `key` in the current language, English when the language
  // lacks it (the test makes sure none does), the key itself when English
  // lacks it too, so a missing string shows instead of an empty space.
  function t(key, params) {
    const own = tables[lang][key];
    const text = own !== undefined ? own : tables.en[key] !== undefined ? tables.en[key] : key;
    return fill(text, params);
  }

  // A count with its words: `${key}.one` or `${key}.other` by the language's
  // plural rules; {n} is the number, formatted.
  function plural(key, n, params) {
    const form = new Intl.PluralRules(LOCALES[lang]).select(n) === "one" ? "one" : "other";
    return t(`${key}.${form}`, { ...params, n: num(n) });
  }

  // A number for the language: 2.50 in English, 2,50 in Spanish. `digits`
  // fixes the decimals (like toFixed); without it the number is shown as is.
  // No grouping below 10,000, as both languages write years and scores.
  function num(value, digits) {
    if (typeof value !== "number" || !Number.isFinite(value)) return String(value);
    const options = digits === undefined ? { maximumFractionDigits: 20 } : { minimumFractionDigits: digits, maximumFractionDigits: digits };
    if (Math.abs(value) < 10000) options.useGrouping = false;
    return new Intl.NumberFormat(LOCALES[lang], options).format(value);
  }

  // A minion's Attack or Health the way the game writes a big one (T-611):
  // from 1,000 up it is 1K, 1M, 1B with one decimal that is cut, not rounded
  // (1,999 is 1.9K, 999,999 is 999.9K), and no ".0". The website has the same
  // rule in lib/stat.ts and both run the same table of cases. An unknown
  // value is "?". `exact` has every digit, for the tooltip.
  const STAT_UNITS = [[1e9, "B"], [1e6, "M"], [1e3, "K"]];
  const isKnown = (value) => typeof value === "number" && Number.isFinite(value);
  function stat(value) {
    if (!isKnown(value)) return "?";
    const whole = Math.trunc(value);
    const sign = whole < 0 ? "-" : "";
    const size = Math.abs(whole);
    const unit = STAT_UNITS.find(([from]) => size >= from);
    if (!unit) return `${sign}${size}`;
    const tenths = Math.floor(size / (unit[0] / 10));
    const decimal = tenths % 10;
    const point = lang === "es" ? "," : ".";
    return `${sign}${Math.floor(tenths / 10)}${decimal === 0 ? "" : point + decimal}${unit[1]}`;
  }
  const exact = (value) => (isKnown(value) ? new Intl.NumberFormat(LOCALES[lang], { maximumFractionDigits: 0 }).format(Math.trunc(value)) : "?");

  // A share from 0 to 1 as a whole percentage: 50% / 50 %.
  const percent = (share) => new Intl.NumberFormat(LOCALES[lang], { style: "percent", maximumFractionDigits: 0 }).format(share);

  // A date (and time) for the language. `date` is a Date.
  const date = (d) => new Intl.DateTimeFormat(LOCALES[lang], { year: "numeric", month: "2-digit", day: "2-digit" }).format(d);
  const dateTime = (d) =>
    new Intl.DateTimeFormat(LOCALES[lang], { year: "numeric", month: "2-digit", day: "2-digit", hour: "2-digit", minute: "2-digit", hour12: false }).format(d);

  // A list joined the language's way: "a, b and c" / "a, b y c".
  const list = (items) => new Intl.ListFormat(LOCALES[lang], { style: "long", type: "conjunction" }).format(items.map(String));

  // A sentence from the Rust side, translated when it is a known fixed one
  // or has a known shape; any other is returned as written.
  function backend(text) {
    if (typeof text !== "string" || lang === "en") return text;
    const exact = (part) => {
      const own = backendTables[lang] && backendTables[lang][part];
      return own !== undefined ? own : part;
    };
    if (exact(text) !== text) return exact(text);
    for (const [regex, out] of backendPatterns[lang] || []) {
      const m = regex.exec(text);
      if (m) return out.replace(/\$(\d)/g, (whole, n) => (m[Number(n)] !== undefined ? exact(m[Number(n)]) : whole));
    }
    return text;
  }

  // The markup's own words: `data-i18n="key"` sets the element's text and
  // `data-i18n-attr="aria-label:key;title:key"` its attributes. Elements with
  // `data-i18n` hold text only (a label with an input wraps its words in a span).
  function apply(root) {
    root.querySelectorAll("[data-i18n]").forEach((node) => {
      node.textContent = t(node.getAttribute("data-i18n"));
    });
    root.querySelectorAll("[data-i18n-attr]").forEach((node) => {
      node.getAttribute("data-i18n-attr").split(";").forEach((pair) => {
        const [attr, key] = pair.split(":").map((s) => s.trim());
        if (attr && key) node.setAttribute(attr, t(key));
      });
    });
  }

  function setLang(next) {
    lang = LANGS.includes(next) ? next : "en";
  }

  // In a window: read the saved choice, apply it to the page, and reload when
  // it changes. Resolves once the page is in its language; never rejects (a
  // setting that cannot be read leaves the system language).
  let ready = Promise.resolve();
  if (typeof window !== "undefined" && window.__TAURI__) {
    const { invoke } = window.__TAURI__.core;
    const { listen } = window.__TAURI__.event;
    const system = () => (navigator.languages && navigator.languages.length ? navigator.languages : [navigator.language]);
    ready = invoke("app_settings")
      .then((s) => s && s.language, () => null)
      .then((saved) => {
        englishFrom(system());
        setLang(pick(saved, system()));
        document.documentElement.lang = lang;
        // The tray menu follows the system language while none is chosen.
        invoke("app_set_system_language", { language: pick(null, system()) }).catch(() => {});
        apply(document);
        listen("app-settings-changed", (e) => {
          const next = pick(e.payload && e.payload.language, system());
          if (next !== lang) window.location.reload();
        });
      });
  }

  const api = {
    LANGS, define, pick, englishFrom, t, plural, num, stat, exact, percent, date, dateTime, list, backend, apply, setLang,
    lang: () => lang,
    locale: () => LOCALES[lang],
    ready: () => ready,
    // For the tests: the tables as defined.
    tables: () => tables,
    backendTables: () => backendTables,
    backendPatterns: () => backendPatterns,
  };
  if (typeof module !== "undefined" && module.exports) module.exports = api;
  else window.TLI18n = api;
})();

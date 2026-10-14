// Words for the overlay's counters panel (T-612): each kind of
// LiveGame.counters as a short label and value (Simple), a "where the game
// shows it" line (Advanced) and a full sentence for screen readers. Pure:
// plain data in, plain data out, so it is tested without a page
// (ui-tests/counters-words.test.js). The page draws the words with
// textContent only.
"use strict";

(() => {
  const I18n = typeof module !== "undefined" && module.exports ? require("./i18n.js") : window.TLI18n;

  // The kinds the panel knows, in the parser's order, with the numbers each
  // one carries: "stats" is +atk/+health, "attack" is +atk, "count" and
  // "gold" are one whole number. A kind not listed here (a newer parser) is skipped.
  const SHAPES = {
    blood_gem: "stats",
    tavern_spell: "stats",
    undead_attack: "attack",
    eternal_knights: "count",
    beetle: "stats",
    shop_elemental: "stats",
    shop_all: "stats",
    shop_tribes: "stats",
    free_rolls: "count",
    gold_next_turn: "gold",
  };
  const FIELDS = { stats: ["atk", "health"], attack: ["atk"], count: ["count"], gold: ["count"] };

  const isWhole = (n) => Number.isInteger(n) && n >= 0;

  function rowOf(entry) {
    if (!entry || typeof entry !== "object" || !Object.hasOwn(SHAPES, entry.kind)) return null;
    const shape = SHAPES[entry.kind];
    const numbers = FIELDS[shape].map((field) => entry[field]);
    if (!numbers.every(isWhole)) return null;
    // Figures follow the language's number format; the words use {atk}, {health} and {n}.
    const [first, second] = numbers.map((n) => I18n.num(n));
    const params = shape === "stats" ? { atk: first, health: second } : shape === "attack" ? { atk: first } : { n: first };
    const key = `ov.counters.${entry.kind}`;
    return {
      kind: entry.kind,
      label: I18n.t(`${key}.label`),
      value: I18n.t(`ov.counters.value.${shape}`, params),
      where: I18n.t(`${key}.where`),
      said: I18n.t(`${key}.said`, params),
    };
  }

  // The panel's rows. `null` (unknown) and `[]` (nothing applies) both give
  // no rows: the panel is hidden, the status panel already says why.
  function rows(counters) {
    if (!Array.isArray(counters)) return [];
    return counters.map(rowOf).filter(Boolean);
  }

  const hasRows = (counters) => rows(counters).length > 0;

  const api = { rows, hasRows };
  if (typeof module !== "undefined" && module.exports) module.exports = api;
  else window.TLCounters = api;
})();

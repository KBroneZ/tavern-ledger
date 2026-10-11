// The "Curious stats" tab (T-216): fun totals over every game, each with its
// source and how many games back it. The app works out every value, source
// and reason (tracker::stats::curious); this file only words and draws them.
// `rows` is plain data in, plain data out (ui-tests/curious.test.js).
"use strict";

(() => {
  const I18n = typeof module !== "undefined" && module.exports ? require("./i18n.js") : window.TLI18n;
  const t = I18n.t;
  const known = (v) => v !== null && v !== undefined;

  // "in 14 of 29 games"; the whole history when it is every game. A sum over
  // shop turns adds how many of those games' turns have it when some lack it.
  function coverage(games, total, turns) {
    if (!total) return t("curious.none_yet");
    if (!games) return I18n.plural("curious.in_none", total);
    const base = games === total
      ? I18n.plural("curious.in_all", total)
      : t("curious.in_some", { games: I18n.num(games), total: I18n.num(total) });
    if (!turns || turns.known >= turns.of) return base;
    return `${base} ${t("curious.turns", { turns: I18n.num(turns.known), of: I18n.num(turns.of) })}`;
  }

  // A count with the language's thousands separator (1,234 / 1234 in Spanish).
  const grouped = (value) => new Intl.NumberFormat(I18n.locale()).format(value);

  // One row per stat. `card` is the card id to name (the most bought minion).
  function rows(curious) {
    if (!curious || !Array.isArray(curious.stats)) return [];
    return curious.stats.map((s) => ({
      key: s.key,
      label: I18n.backend(s.label),
      value: known(s.value) ? grouped(s.value) : t("curious.no_data"),
      times: s.card && known(s.value) ? I18n.plural("curious.bought", s.value) : null,
      card: s.card || null,
      source: known(s.value) ? s.source : "unknown",
      coverage: coverage(s.games, curious.games, s.turns),
      why: known(s.value) ? null : I18n.backend(s.missing),
      basis: I18n.backend(s.basis),
    }));
  }

  function dropped(curious) {
    return curious && Array.isArray(curious.dropped) ? curious.dropped.map((d) => `${I18n.backend(d.label)}: ${I18n.backend(d.why)}`) : [];
  }

  // --- Drawing (the page only).

  // A card's name from the card data when it is known, else its id.
  function cardName(id) {
    const cards = typeof window !== "undefined" && window.TLCards;
    const name = cards ? cards.name(id) : null;
    return name || id;
  }

  function tile(ui, r) {
    const item = ui.el("li", `curious-stat curious-${r.key}`);
    item.setAttribute("aria-labelledby", `curious-${r.key}-label`);
    const label = ui.el("h3", "curious-label", r.label);
    label.id = `curious-${r.key}-label`;
    const value = ui.el("p", known(r.why) ? "curious-value none" : "curious-value");
    if (r.card) {
      const art = ui.art ? ui.art(r.card, "curious-art") : null;
      if (art) value.append(art);
      const name = ui.el("span", "curious-card", cardName(r.card));
      name.title = r.card;
      ui.addTip(name, "card_data");
      value.append(name, ui.el("span", "curious-times", r.times));
    } else {
      value.append(ui.el("span", "curious-number", r.value));
    }
    ui.withSource(value, r.source);
    item.append(label, value, ui.el("p", "curious-coverage", r.coverage));
    if (r.why) item.append(ui.el("p", "curious-why", r.why));
    // The long definition waits behind a fold (T-323).
    if (r.basis) item.append(window.TLCompact.about(ui, `curious:${r.key}`, r.basis, t("curious.how")));
    return item;
  }

  function render(curious, ui, box) {
    const list = rows(curious);
    const parts = [];
    if (!list.length) {
      parts.push(ui.el("p", "empty", t("curious.unavailable")));
    } else {
      const grid = ui.el("ul", "curious-grid");
      grid.append(...list.map((r) => tile(ui, r)));
      parts.push(grid);
    }
    const left = dropped(curious);
    if (left.length) {
      const notes = ui.el("ul", "messages");
      notes.append(...left.map((line) => ui.el("li", null, line)));
      parts.push(ui.el("h3", null, t("curious.left_out")), notes);
    }
    box.replaceChildren(...parts);
  }

  const api = { coverage, rows, dropped, render };
  if (typeof module !== "undefined" && module.exports) module.exports = api;
  else window.TLCurious = api;
})();

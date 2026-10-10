// The "Curious stats" tab (T-216): fun totals over every game, each with its
// source and how many games back it. The app works out every value, source
// and reason (tracker::stats::curious); this file only words and draws them.
// `rows` is plain data in, plain data out (ui-tests/curious.test.js).
"use strict";

(() => {
  const known = (v) => v !== null && v !== undefined;

  // "in 14 of 29 games"; the whole history when it is every game.
  function coverage(games, total) {
    if (!total) return "No games yet.";
    if (!games) return `In none of your ${total} game${total === 1 ? "" : "s"}.`;
    if (games === total) return `In all ${total} game${total === 1 ? "" : "s"}.`;
    return `In ${games} of ${total} games.`;
  }

  // One row per stat. `card` is the card id to name (the most bought minion).
  function rows(curious) {
    if (!curious || !Array.isArray(curious.stats)) return [];
    return curious.stats.map((s) => ({
      key: s.key,
      label: s.label,
      value: known(s.value) ? s.value.toLocaleString("en-US") : "No data",
      times: s.card && known(s.value) ? `bought ${s.value} time${s.value === 1 ? "" : "s"}` : null,
      card: s.card || null,
      source: known(s.value) ? s.source : "unknown",
      coverage: coverage(s.games, curious.games),
      why: known(s.value) ? null : s.missing,
      basis: s.basis,
    }));
  }

  function dropped(curious) {
    return curious && Array.isArray(curious.dropped) ? curious.dropped.map((d) => `${d.label}: ${d.why}`) : [];
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
      const art = typeof window !== "undefined" && window.TLCards ? window.TLCards.art(r.card, "curious-art") : null;
      if (art) {
        art.title = "Picture: card database. © Blizzard Entertainment.";
        value.append(art);
      }
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
    item.append(ui.el("p", "note curious-basis", r.basis));
    return item;
  }

  function render(curious, ui, box) {
    const list = rows(curious);
    const parts = [];
    if (!list.length) {
      parts.push(ui.el("p", "empty", "Curious stats are not available yet."));
    } else {
      const grid = ui.el("ul", "curious-grid");
      grid.append(...list.map((r) => tile(ui, r)));
      parts.push(grid);
    }
    const left = dropped(curious);
    if (left.length) {
      const notes = ui.el("ul", "messages");
      notes.append(...left.map((t) => ui.el("li", null, t)));
      parts.push(ui.el("h3", null, "Left out"), notes);
    }
    box.replaceChildren(...parts);
  }

  const api = { coverage, rows, dropped, render };
  if (typeof module !== "undefined" && module.exports) module.exports = api;
  else window.TLCurious = api;
})();

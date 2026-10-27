// The game review in the recap (T-658, D-176): a button starts it, after the
// game only; then the choices that cost the most in the next fight with
// their boards and plain reasons, and every turn behind a fold. The words
// come from review-words.js; this file only draws them, with textContent
// only (ui-tests/review.test.js).
"use strict";

(() => {
  const node = typeof module !== "undefined" && module.exports;
  const I18n = node ? require("./i18n.js") : window.TLI18n;
  const W = node ? require("./review-words.js") : window.TLReviewWords;
  const t = I18n.t;

  const note = (ui, text) => ui.el("p", "note", text);
  const stats = (m) => `${m.atk === null ? "?" : I18n.stat(m.atk)}/${m.health === null ? "?" : I18n.stat(m.health)}`;

  function startButton(ui, h, label) {
    const button = ui.el("button", "report-btn review-start", label);
    button.type = "button";
    button.addEventListener("click", () => h.onStart());
    return button;
  }

  function boardRow(ui, h, label, minions) {
    const row = ui.el("div", "shop-row");
    row.append(ui.el("span", "shop-row-label", label));
    const cards = ui.el("div", "shop-cards");
    cards.append(...minions.map((m) => {
      const box = ui.el("span", "review-minion");
      box.append(h.card(ui, m.card), ui.el("span", "review-stats", stats(m)));
      return box;
    }));
    row.append(cards);
    return row;
  }

  function costly(ui, h, view, c) {
    const d = c.decision;
    const li = ui.el("li", "review-costly");
    li.append(ui.el("p", "review-reason", W.reasonLine(d, c.alt, h.cardName)));
    [W.tieLine(d.rebuilt, c.alt), W.placeLine(d.rebuilt, c.alt)].filter(Boolean).forEach((line) => li.append(note(ui, line)));
    const b = W.boards(h.report, d, c.alt, view.stats);
    if (b && b.trinket) li.append(note(ui, t("review.trinket", { alt: h.cardName(c.alt.card), taken: h.cardName(d.taken) })));
    else if (b) {
      const boards = ui.el("div", "review-boards");
      const after = d.kind === "sell" ? t("review.afterKept", { card: h.cardName(c.alt.card) }) : t("review.after", { card: h.cardName(c.alt.card) });
      boards.append(boardRow(ui, h, t("review.before"), b.before), boardRow(ui, h, after, b.after));
      li.append(boards);
    }
    const more = W.notes(d, c.alt, h.cardName);
    if (more.length) {
      const items = ui.el("ul", "review-notes");
      items.append(...more.map((m) => ui.el("li", "note", m)));
      li.append(items);
    }
    return li;
  }

  function everyTurn(ui, h, view) {
    const box = h.fold(ui, `${h.game}:review:all`, t("review.all"));
    const items = ui.el("ul", "review-all");
    items.append(...view.decisions.map((d) => {
      const li = ui.el("li");
      li.append(ui.el("p", "review-line", W.decisionLine(d, h.cardName)));
      if (!d.rebuilt) li.append(note(ui, W.notLine(d)));
      else {
        const alts = ui.el("ul", "review-alts");
        alts.append(...d.rebuilt.alternatives.map((a) => {
          const item = ui.el("li", null, W.altLine(d, a, h.cardName));
          W.altNotes(a, h.cardName).forEach((n) => item.append(note(ui, n)));
          return item;
        }));
        li.append(alts);
        const unknown = W.unknownLine(d.rebuilt);
        if (unknown) li.append(note(ui, unknown));
      }
      return li;
    }));
    box.append(items);
    return box;
  }

  function done(ui, h, part, view) {
    const s = W.summary(view);
    if (s.banner) part.append(ui.el("p", "review-banner", s.banner));
    part.append(note(ui, s.count));
    const top = W.top(view.decisions);
    if (top.length) {
      part.append(ui.el("h4", null, t("review.topTitle")));
      const items = ui.el("ol", "review-top");
      items.append(...top.map((c) => costly(ui, h, view, c)));
      part.append(items);
    } else part.append(note(ui, s.empty));
    if (view.decisions.length) part.append(everyTurn(ui, h, view));
  }

  // The recap's section. `state`: "idle", "reviewing", "failed", or the
  // answer as review-words.js reads it.
  function section(ui, state, h) {
    const part = ui.el("section", "recap-part review");
    part.append(ui.el("h3", null, t("review.title")), note(ui, t("review.measures")));
    if (state === "idle") part.append(note(ui, t("review.startNote")), startButton(ui, h, t("review.start")));
    else if (state === "reviewing") part.append(ui.el("p", "note review-busy", t("review.reviewing")));
    else if (state && state.state === "done") done(ui, h, part, state);
    else if (state && state.state === "game_on") part.append(note(ui, t("review.gameOn")), startButton(ui, h, t("review.again")));
    else if (state && state.state === "busy") part.append(note(ui, t("review.busy")), startButton(ui, h, t("review.again")));
    else if (state && state.state === "no_review") part.append(note(ui, t(`review.noReview.${state.why}`) === `review.noReview.${state.why}` ? t("review.failed") : t(`review.noReview.${state.why}`)));
    else part.append(note(ui, t("review.failed")), startButton(ui, h, t("review.again")));
    return part;
  }

  const api = { section };
  if (node) module.exports = api;
  else window.TLReview = api;
})();

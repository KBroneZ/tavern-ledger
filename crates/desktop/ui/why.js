// "Why the result differed" in the recap (T-346, D-127): per combat, the
// odds the simulator gives on the boards as they fought, the result and,
// when the likeliest result did not happen, the cause in plain words, with
// the evidence folded. The app works it out (`game_why`, the same engine,
// rules and JSON as the website's game page); this file only reads the
// answer and draws it. `lines`, `causeLine` and `evidence` are plain data in,
// plain data out (ui-tests/why.test.js).
"use strict";

(() => {
  const node = typeof module !== "undefined" && module.exports;
  const I18n = node ? require("./i18n.js") : window.TLI18n;
  const t = I18n.t;
  const list = (v) => (Array.isArray(v) ? v : []);
  const RESULTS = ["won", "lost", "tie"];
  const CARD_ID = /^[A-Za-z0-9_]{1,64}$/;
  const card = (v) => (typeof v === "string" && CARD_ID.test(v) ? v : null);
  const share = (v) => (typeof v === "number" && v >= 0 && v <= 1 ? v : null);

  // The app's answer, read defensively: a combat out of shape is left out;
  // an answer that is not a list is null (shown as a failure).
  function lines(answer) {
    if (!Array.isArray(answer)) return null;
    return answer.filter((c) => c && Number.isInteger(c.round) && RESULTS.includes(c.result)).map((c) => {
      const o = c.odds && typeof c.odds === "object" ? c.odds : null;
      const odds = o && [o.won, o.tied, o.lost].every((x) => share(x) !== null) ? { won: o.won, tied: o.tied, lost: o.lost } : null;
      const cause = c.cause && typeof c.cause === "object" && typeof c.cause.id === "string" ? c.cause : null;
      return {
        round: c.round,
        result: c.result,
        odds,
        noOdds: typeof c.no_odds === "string" ? c.no_odds : null,
        cards: list(c.cards).map(card).filter(Boolean),
        cause: cause && {
          id: cause.id,
          attacks: Number.isInteger(cause.attacks) ? cause.attacks : null,
          attack: Number.isInteger(cause.attack) ? cause.attack : null,
          rule: typeof cause.rule === "string" && cause.rule.startsWith("replay_") ? cause.rule : null,
          card: card(cause.card),
          inputs: list(cause.inputs).filter((i) => i && typeof i.what === "string" && card(i.card)),
          cards: list(cause.cards).map(card).filter(Boolean),
          replay: typeof cause.replay === "string" ? cause.replay : null,
        },
      };
    });
  }

  // A key's words, or null when the table has none (an id this build does not know).
  const words = (key, values) => {
    const text = t(key, values);
    return text === key ? null : text;
  };

  function oddsText(c) {
    if (!c.odds) return t("why.noOddsShort");
    return t("why.odds", { won: I18n.percent(c.odds.won), tied: I18n.percent(c.odds.tied), lost: I18n.percent(c.odds.lost) });
  }

  // The one short line: the cause in plain words.
  function causeLine(c, nameOf) {
    const names = (ids) => I18n.list(ids.map(nameOf));
    if (c.noOdds) return words(`why.noOdds.${c.noOdds}`, { cards: names(c.cards) }) || t("why.noOdds.bad_board");
    const cause = c.cause;
    if (!cause) return t("why.noOdds.bad_board");
    if (cause.id === "matched") return t("why.matched");
    if (cause.id === "chance") return t("why.chance");
    if (cause.id === "missing_inputs") {
      const what = [...new Set(cause.inputs.map((i) => words(`why.unseen.${i.what}`, { card: nameOf(i.card) })).filter(Boolean))];
      return t("why.missing", { what: what.join("; ") });
    }
    if (cause.id === "approximate_card") return t("why.approximate", { cards: names(cause.cards) });
    if (cause.replay) return words(`why.replay.${cause.replay}`) || t("why.replay.not_saved");
    return cause.card ? t("why.rule", { attack: cause.attack || 0, card: nameOf(cause.card) }) : t("why.ruleNoCard", { attack: cause.attack || 0 });
  }

  // The evidence behind the line; empty when the line says it all.
  function evidence(c, nameOf) {
    const cause = c.cause;
    if (!cause || cause.id === "matched") return [];
    const out = [];
    if (cause.id === "chance") out.push(t("why.ev.chance", { attacks: cause.attacks || 0 }));
    if (cause.attack !== null && cause.rule) {
      const where = cause.card ? t("why.ev.partsAtCard", { card: nameOf(cause.card) }) : "";
      out.push(t("why.ev.parts", { attack: cause.attack, rule: words(`why.rule.${cause.rule}`) || cause.rule, where }));
    }
    if (cause.id === "missing_inputs") out.push(t("why.ev.missing"));
    if (cause.id === "approximate_card") out.push(t("why.ev.approximate"));
    if (cause.replay === "not_saved") out.push(t("why.ev.notSaved"));
    return out;
  }

  function row(ui, c, nameOf, h) {
    const li = ui.el("li");
    li.dataset.round = String(c.round);
    const line = ui.el("p", "why-line");
    const result = ui.el("span", `chip chip-${c.result}`, t(`outcome.${c.result}`));
    line.append(ui.el("span", "why-turn", t("why.turn", { turn: c.round })), " ", result, " ", ui.el("span", "why-odds", oddsText(c)), " · ", ui.el("span", "why-cause", causeLine(c, nameOf)));
    li.append(line);
    const more = evidence(c, nameOf);
    if (more.length) {
      const box = h.fold(ui, `${h.game}:why:${c.round}`, t("why.more"));
      const items = ui.el("ul");
      items.append(...more.map((m) => ui.el("li", null, m)));
      box.append(items);
      li.append(box);
    }
    return li;
  }

  // The recap's section. `state`: "loading", "failed", or the combats (`lines`).
  function section(ui, state, h) {
    const part = ui.el("section", "recap-part why");
    part.append(ui.el("h3", null, t("why.title")));
    if (state === "loading") part.append(ui.el("p", "note", t("why.loading")));
    else if (state === "failed" || !Array.isArray(state)) part.append(ui.el("p", "note", t("why.failed")));
    else if (!state.length) part.append(ui.el("p", "note", t("why.none")));
    else {
      const missed = state.filter((c) => c.cause && c.cause.id !== "matched").length;
      part.append(ui.el("p", "note", t("why.count", { missed: I18n.num(missed), total: I18n.num(state.length) })));
      const items = ui.el("ul", "why-list");
      items.append(...state.map((c) => row(ui, c, h.cardName, h)));
      part.append(items);
    }
    part.append(ui.el("p", "note adv-only", t("why.about")));
    return part;
  }

  const api = { lines, causeLine, evidence, section };
  if (node) module.exports = api;
  else window.TLWhy = api;
})();

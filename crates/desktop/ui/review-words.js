// The game review's words (T-658, D-176): the app plays each choice's next
// fight again with the other cards on offer (`game_review`, the
// counterfactual engine); this file reads its answer, picks the choices that
// cost the most and says each result in plain words. It measures the next
// fight only: a difference is never called the cost in final place, and the
// expected place is shown only when a model gives one. A difference the
// engine does not call clear is never ranked as costly. Plain data in, plain
// data out (ui-tests/review-words.test.js); review.js draws it.
"use strict";

(() => {
  const node = typeof module !== "undefined" && module.exports;
  const I18n = node ? require("./i18n.js") : window.TLI18n;
  const t = I18n.t;
  const CARD_ID = /^[A-Za-z0-9_]{1,64}$/;
  const CODE = /^[a-z_]{1,64}$/;
  const STATES = ["game_on", "busy"];
  const KINDS = ["roll", "buy", "buy_spell", "sell", "freeze", "unfreeze", "tier_up", "choose"];
  const TOP = 3;
  // Ties below this share are not worth a line.
  const TIE_SHOWN = 0.005;

  const isObject = (v) => v !== null && typeof v === "object" && !Array.isArray(v);
  const card = (v) => (typeof v === "string" && CARD_ID.test(v) ? v : null);
  const code = (v) => (typeof v === "string" && CODE.test(v) ? v : null);
  const share = (v) => typeof v === "number" && v >= 0 && v <= 1;
  const finite = (v) => typeof v === "number" && Number.isFinite(v);
  // A serde enum: "name" for a unit variant, { name: value } for the others.
  const variant = (v) => code(v) || (isObject(v) && Object.keys(v).length === 1 ? code(Object.keys(v)[0]) : null);
  // A serde Result: { Ok } or { Err }.
  const result = (v) => (isObject(v) && "Ok" in v ? { ok: v.Ok } : isObject(v) && "Err" in v ? { err: v.Err } : null);

  // Throws on anything out of shape: the whole answer is then a failure.
  class Unreadable extends Error {}
  const need = (ok) => {
    if (!ok) throw new Unreadable();
  };

  function readFight(f) {
    need(isObject(f) && [f.won, f.tied, f.lost].every(share) && Number.isInteger(f.trials));
    const expected = isObject(f.place) && finite(f.place.expected) ? f.place.expected : null;
    return {
      won: f.won,
      tied: f.tied,
      lost: f.lost,
      trials: f.trials,
      place: expected,
      approximate: (Array.isArray(f.approximate) ? f.approximate : []).map(card).filter(Boolean),
    };
  }

  function readAlternative(a) {
    need(isObject(a) && card(a.card));
    const fight = result(a.fight);
    need(fight);
    const d = a.delta;
    const delta = isObject(d) && finite(d.score) && typeof d.clear === "boolean"
      ? { score: d.score, clear: d.clear, place: finite(d.place) ? d.place : null }
      : null;
    return {
      card: a.card,
      approx: (Array.isArray(a.approx) ? a.approx : []).map(variant).filter(Boolean),
      fight: "ok" in fight ? readFight(fight.ok) : null,
      gap: "err" in fight ? variant(fight.err) || "fallback" : null,
      delta: "ok" in fight ? delta : null,
      stop: code(a.stop),
    };
  }

  // "actual.<gap>" for the actual board's gap, else the reason's code.
  function notRebuilt(err) {
    if (isObject(err) && "actual" in err) return `actual.${variant(err.actual) || "fallback"}`;
    return variant(err) || "fallback";
  }

  function readDecision(r) {
    need(isObject(r) && isObject(r.decision));
    const d = r.decision;
    need(Number.isInteger(d.turn) && typeof d.kind === "string");
    const rebuilt = result(r.rebuilt);
    need(rebuilt);
    let built = null;
    if ("ok" in rebuilt) {
      const b = rebuilt.ok;
      need(isObject(b) && Number.isInteger(b.round) && Array.isArray(b.alternatives));
      built = {
        round: b.round,
        actual: readFight(b.actual),
        alternatives: b.alternatives.map(readAlternative),
        unknownOffers: Number.isInteger(b.unknown_offers) && b.unknown_offers > 0 ? b.unknown_offers : 0,
      };
    }
    return {
      turn: d.turn,
      kind: KINDS.includes(d.kind) ? d.kind : "fallback",
      taken: card(d.taken),
      tier: Number.isInteger(d.tier) ? d.tier : null,
      rebuilt: built,
      notRebuilt: built ? null : notRebuilt(rebuilt.err),
    };
  }

  // Card data stats of the cards put in: { id: [attack, health] }.
  function readStats(stats) {
    const out = {};
    if (!isObject(stats)) return out;
    Object.entries(stats).forEach(([id, s]) => {
      if (card(id) && Array.isArray(s) && s.length === 2 && s.every(Number.isInteger)) out[id] = { atk: s[0], health: s[1] };
    });
    return out;
  }

  // The app's answer, or null when it is out of shape (shown as a failure,
  // never as a review that found nothing).
  function read(answer) {
    if (!isObject(answer)) return null;
    if (STATES.includes(answer.state)) return { state: answer.state };
    if (answer.state === "no_review") return { state: "no_review", why: variant(answer.why) || "fallback" };
    if (answer.state !== "done" || !isObject(answer.review) || !Array.isArray(answer.review.decisions)) return null;
    try {
      return {
        state: "done",
        decisions: answer.review.decisions.map(readDecision),
        outOfTime: answer.review.out_of_time === true,
        stoppedForGame: answer.stopped_for_game === true,
        stats: readStats(answer.stats),
      };
    } catch (e) {
      if (e instanceof Unreadable) return null;
      throw e;
    }
  }

  // The alternative that did clearly better in the next fight, by the most;
  // null when none did.
  function costOf(d) {
    if (!d.rebuilt) return null;
    let best = null;
    d.rebuilt.alternatives.forEach((alt) => {
      const delta = alt.delta;
      if (!delta || !delta.clear || !(delta.score > 0)) return;
      if (!best || delta.score > best.score) best = { alt, score: delta.score };
    });
    return best;
  }

  // The choices that cost the most in the next fight, the earlier first on a tie.
  function top(decisions, n = TOP) {
    return decisions
      .map((decision) => ({ decision, cost: costOf(decision) }))
      .filter((c) => c.cost)
      .sort((a, b) => b.cost.score - a.cost.score || a.decision.turn - b.decision.turn)
      .slice(0, n)
      .map(({ decision, cost }) => ({ decision, alt: cost.alt, score: cost.score }));
  }

  const stat = (v) => (Number.isInteger(v) ? v : null);
  const minionOf = (m) => ({ card: card(m && m.card_id), atk: stat(m && m.atk), health: stat(m && m.health), golden: Boolean(m && m.golden) });

  // The player's board at combat `round`, as saved: the local player's own
  // entry first, as the engine reads it.
  function ownBoard(report, round) {
    const r = (report && Array.isArray(report.rounds) ? report.rounds : []).find((x) => x && x.number === round);
    const entries = r && Array.isArray(r.entries) ? r.entries.filter((e) => e && e.side === "own" && Array.isArray(e.board)) : [];
    const mine = entries.find((e) => e.player_id === report.local_player_id) || entries[0];
    return mine ? mine.board : null;
  }

  // The board of the last combat before `round` that has one.
  function boardBefore(report, round) {
    const numbers = (report && Array.isArray(report.rounds) ? report.rounds : [])
      .map((r) => r && r.number)
      .filter((n) => Number.isInteger(n) && n < round)
      .sort((a, b) => b - a);
    for (const n of numbers) {
      const b = ownBoard(report, n);
      if (b) return b;
    }
    return [];
  }

  // The board as it fought and the board with the alternative: the card
  // taken swapped for the alternative at its card data's stats, or the
  // minion sold put back where it was with its stats then. A trinket leaves
  // the board as it is. Null when the record has no board.
  function boards(report, d, alt, stats) {
    if (!d.rebuilt) return null;
    const saved = ownBoard(report, d.rebuilt.round);
    if (!saved) return null;
    const before = saved.map(minionOf);
    if (d.kind === "sell") {
      const earlier = boardBefore(report, d.rebuilt.round);
      const at = earlier.findIndex((m) => m && m.card_id === alt.card);
      if (at < 0) return null;
      const after = before.slice();
      after.splice(Math.min(at, after.length), 0, minionOf(earlier[at]));
      return { before, after };
    }
    const at = before.findIndex((m) => m.card === d.taken);
    if (at < 0) return { trinket: true, before };
    const s = (stats && stats[alt.card]) || {};
    const after = before.map((m, i) => (i === at ? { card: alt.card, atk: stat(s.atk), health: stat(s.health), golden: false } : m));
    return { before, after };
  }

  // A key's words, or null when the table has none (a code this build does not know).
  const words = (key, values) => {
    const text = t(key, values);
    return text === key ? null : text;
  };
  const named = (id, nameOf) => (id ? nameOf(id) || id : t("review.unnamed"));
  const turn = (d) => t("review.turn", { turn: d.turn });

  function did(d, nameOf) {
    if (d.kind === "tier_up") return d.tier ? t("review.did.tier_up", { tier: d.tier }) : t("review.did.tier_upNoTier");
    return t(`review.did.${d.kind}`, { card: named(d.taken, nameOf) });
  }

  // "Turn 5: you bought X."
  const decisionLine = (d, nameOf) => t("review.line", { turn: turn(d), did: did(d, nameOf) });

  // Why a costly choice cost the next fight, in one sentence.
  function reasonLine(d, alt, nameOf) {
    const kind = d.kind === "sell" ? "sell" : d.kind === "choose" ? "choose" : "buy";
    return t(`review.why.${kind}`, {
      turn: turn(d),
      taken: named(d.taken, nameOf),
      alt: named(alt.card, nameOf),
      altWon: I18n.percent(alt.fight.won),
      won: I18n.percent(d.rebuilt.actual.won),
    });
  }

  function tieLine(rebuilt, alt) {
    const [a, b] = [alt.fight.tied, rebuilt.actual.tied];
    if (a < TIE_SHOWN && b < TIE_SHOWN) return null;
    return t("review.ties", { alt: I18n.percent(a), actual: I18n.percent(b) });
  }

  // Only when a model gives both places.
  function placeLine(rebuilt, alt) {
    if (!alt.delta || alt.delta.place === null || alt.fight.place === null || rebuilt.actual.place === null) return null;
    return t("review.place", { alt: I18n.num(alt.fight.place, 1), actual: I18n.num(rebuilt.actual.place, 1) });
  }

  // What an alternative takes as given, each once.
  function notes(d, alt, nameOf) {
    const values = { alt: named(alt.card, nameOf), taken: named(d.taken, nameOf) };
    const lines = alt.approx.map((a) => words(`review.approx.${a}`, values) || t("review.approx.fallback"));
    return [...new Set(lines.concat(altNotes(alt, nameOf)))];
  }

  // The cards the simulator plays with a known gap in an alternative's fight.
  function altNotes(alt, nameOf) {
    if (!alt.fight || !alt.fight.approximate.length) return [];
    return [t("review.approximate", { cards: I18n.list(alt.fight.approximate.map((id) => named(id, nameOf))) })];
  }

  function unknownLine(rebuilt) {
    return rebuilt.unknownOffers ? I18n.plural("review.unknownOffers", rebuilt.unknownOffers) : null;
  }

  // One alternative in the full list.
  function altLine(d, alt, nameOf) {
    const name = named(alt.card, nameOf);
    const label = d.kind === "sell" ? t("review.keep", { card: name }) : name;
    if (!alt.fight) return t("review.alt.gap", { card: label, why: words(`review.gap.${alt.gap}`) || t("review.gap.fallback") });
    const values = { card: label, altWon: I18n.percent(alt.fight.won), won: I18n.percent(d.rebuilt.actual.won), trials: I18n.num(alt.fight.trials) };
    const delta = alt.delta;
    if (delta && delta.clear) return t(delta.score > 0 ? "review.alt.better" : "review.alt.worse", values);
    if (alt.stop === "close") return t("review.alt.close", values);
    return t("review.alt.unclear", values);
  }

  // Why a choice was not simulated again.
  function notLine(d) {
    const why = d.notRebuilt;
    if (!why) return null;
    if (why === "actual.out_of_time") return t("review.not.out_of_time");
    if (why.startsWith("actual.")) return t("review.not.actual", { why: words(`review.gap.${why.slice(7)}`) || t("review.gap.fallback") });
    return words(`review.not.${why}`) || t("review.not.fallback");
  }

  // The lines above the costly choices: how many were simulated, a banner
  // when the review stopped partway, and what to say when none cost a fight.
  function summary(view) {
    const total = view.decisions.length;
    const rebuilt = view.decisions.filter((d) => d.rebuilt).length;
    const partial = view.outOfTime || view.stoppedForGame;
    const banner = view.stoppedForGame ? t("review.stoppedForGame") : view.outOfTime ? t("review.outOfTime") : null;
    let empty = null;
    if (!top(view.decisions).length) {
      if (!rebuilt && !partial) empty = t("review.noneRebuilt");
      else empty = partial ? t("review.nonePartial") : t("review.none");
    }
    return { count: t("review.count", { rebuilt: I18n.num(rebuilt), total: I18n.num(total) }), banner, empty };
  }

  const api = { read, costOf, top, boards, minionOf, decisionLine, reasonLine, tieLine, placeLine, notes, altNotes, unknownLine, altLine, notLine, summary };
  if (node) module.exports = api;
  else window.TLReviewWords = api;
})();

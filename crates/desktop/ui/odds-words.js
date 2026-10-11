// Words for the combat odds (T-605): the shares to win, tie or lose against
// the next opponent, how old their board is, and in Advanced the damage
// spread and the cards played with a gap or not at all. Pure: plain data in,
// plain data out (ui-tests/odds-words.test.js). The app works the odds out
// and says why when there are none; this file never makes up a number.
"use strict";

(() => {
  const I18n = typeof module !== "undefined" && module.exports ? require("./i18n.js") : window.TLI18n;

  // Reasons the overlay says in words; the others show nothing (no next
  // opponent yet, or a combat the shop's odds are not for).
  const SAID_GAPS = ["duos", "no_board_seen", "not_modelled", "no_card_data", "bad_board"];
  const MAX_CARDS = 6;

  const isShare = (x) => typeof x === "number" && Number.isFinite(x) && x >= 0 && x <= 1;
  const isSpread = (s) => s && ["low", "likely", "high"].every((k) => Number.isInteger(s[k]));

  // Card names, at most a few, then "+N".
  function cardList(ids, nameOf) {
    const list = Array.isArray(ids) ? ids.filter((id) => typeof id === "string") : [];
    const names = list.slice(0, MAX_CARDS).map((id) => nameOf(id) || id);
    if (list.length > MAX_CARDS) names.push(`+${list.length - MAX_CARDS}`);
    return names.join(", ");
  }

  function spreadText(key, s) {
    if (s.low === s.high) return I18n.t(`${key}Exact`, { value: I18n.num(s.likely) });
    return I18n.t(key, { low: I18n.num(s.low), likely: I18n.num(s.likely), high: I18n.num(s.high) });
  }

  // `odds` as the app sends it (LiveOdds), `advanced` for the extra lines,
  // `nameOf(id)` a card's name or null. Returns null when there is nothing
  // to say, else { kind: "odds" | "note", shares, age, approx, lines, said }.
  function oddsView(odds, advanced, nameOf) {
    if (!odds || typeof odds !== "object") return null;
    const name = typeof nameOf === "function" ? nameOf : () => null;
    if (odds.gap) {
      if (!SAID_GAPS.includes(odds.gap)) return null;
      const lines = [];
      if (advanced && Array.isArray(odds.missing) && odds.missing.length) {
        const key = odds.gap === "no_card_data" ? "ov.odds.noData" : "ov.odds.missing";
        lines.push(I18n.t(key, { cards: cardList(odds.missing, name) }));
      }
      const text = I18n.t(`ov.odds.gap.${odds.gap}`);
      return { kind: "note", text, shares: [], age: null, approx: false, lines, said: [text, ...lines].join(". ") };
    }
    if (![odds.won, odds.tied, odds.lost].every(isShare)) return null;
    // Label and number apart, so a narrow column can stack them.
    const share = (key, value) => ({
      key,
      label: I18n.t(`ov.odds.${key}Label`),
      value: I18n.percent(value),
      text: I18n.t(`ov.odds.${key}`, { value: I18n.percent(value) }),
    });
    const shares = [share("win", odds.won), share("tie", odds.tied), share("lose", odds.lost)];
    const age = Number.isInteger(odds.seen_round) ? I18n.t("ov.odds.seen", { turn: odds.seen_round }) : null;
    const approximate = Array.isArray(odds.approximate) ? odds.approximate : [];
    const approx = approximate.length > 0 || Boolean(odds.start_of_combat);
    const lines = [];
    if (advanced) {
      const damage = [];
      if (odds.won > 0) damage.push(isSpread(odds.dealt) ? spreadText("ov.odds.dealt", odds.dealt) : I18n.t("ov.odds.dealtUnknown"));
      if (odds.lost > 0) damage.push(isSpread(odds.taken) ? spreadText("ov.odds.taken", odds.taken) : I18n.t("ov.odds.takenUnknown"));
      lines.push(...damage);
      if (approximate.length) lines.push(I18n.t("ov.odds.approximate", { cards: cardList(approximate, name) }));
      if (odds.start_of_combat) lines.push(I18n.t("ov.odds.soc"));
      if (Number.isInteger(odds.trials) && odds.trials > 0) lines.push(I18n.plural("ov.odds.trials", odds.trials));
    }
    const said = [
      I18n.t("ov.odds.label"),
      shares.map((s) => s.text).join(", "),
      age,
      approx ? I18n.t("ov.odds.approx") : null,
      ...lines,
    ].filter(Boolean).join(". ");
    return { kind: "odds", text: null, shares, age, approx, lines, said };
  }

  const api = { oddsView, SAID_GAPS };
  if (typeof module !== "undefined" && module.exports) module.exports = api;
  else window.TLOdds = api;
})();

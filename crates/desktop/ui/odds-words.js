// Words for the combat odds (T-605, D-152): the shares to win, tie or lose
// in the combat the game is playing, from the boards that entered play in it.
// In Duos (T-608) the odds use four boards, and the boards not in play yet
// are named when there is no number. Pure: plain data in, plain data out
// (ui-tests/odds-words.test.js). The app works the odds out and says why
// when there are none; this file never makes up a number.
"use strict";

(() => {
  const I18n = typeof module !== "undefined" && module.exports ? require("./i18n.js") : window.TLI18n;

  // Reasons the overlay says in words; the others show nothing (not in a
  // combat: there are no odds in a shop turn).
  const SAID_GAPS = ["missing_board", "no_board_seen", "not_modelled", "no_card_data", "bad_board"];
  // Duos boards, in the order the app sends them.
  const ROLES = ["own", "teammate", "opponent", "opponent_teammate"];

  // The Duos boards the app sent, kept only when well formed.
  function duosBoards(odds) {
    const list = Array.isArray(odds.boards) ? odds.boards : [];
    return list.filter((b) => b && ROLES.includes(b.role) && (b.round === null || Number.isInteger(b.round)));
  }

  const NO_RANGES = { win: null, lose: null };
  const isShare = (x) => typeof x === "number" && Number.isFinite(x) && x >= 0 && x <= 1;

  // A whole percentage, except that a share that can happen is never "0%"
  // and one that can fail is never "100%".
  function shareText(x) {
    if (x > 0 && x < 0.005) return `<${I18n.percent(0.01)}`;
    if (x < 1 && x > 0.995) return `>${I18n.percent(0.99)}`;
    return I18n.percent(x);
  }

  const isDamage = (x) => Number.isInteger(x) && x >= 0;

  // A damage spread (after the damage cap) as the bar's range and its words:
  // "5–12", or the one value when both ends are equal. Null when the app has
  // none or it is not well formed: never a guess.
  function rangeView(spread, key) {
    if (!spread || typeof spread !== "object") return null;
    const { low, high } = spread;
    if (!isDamage(low) || !isDamage(high) || low > high) return null;
    const one = low === high;
    const said = one
      ? I18n.t(`ov.odds.range${key}.single`, { value: I18n.num(low) })
      : I18n.t(`ov.odds.range${key}.span`, { low: I18n.num(low), high: I18n.num(high) });
    return { text: one ? I18n.num(low) : `${I18n.num(low)}–${I18n.num(high)}`, said };
  }

  // `odds` as the app sends it (LiveOdds). Returns null when there is
  // nothing to say, else { kind: "odds" | "note", shares, ranges, approx, said }.
  function oddsView(odds) {
    if (!odds || typeof odds !== "object") return null;
    const boards = duosBoards(odds);
    const duos = boards.length > 0;
    const about = I18n.t(duos ? "ov.odds.aboutDuos" : "ov.odds.about");
    if (odds.gap) {
      if (!SAID_GAPS.includes(odds.gap)) return null;
      if (odds.gap === "missing_board") {
        const unseen = boards.filter((b) => b.round === null);
        if (!unseen.length) return null;
        const who = I18n.list(unseen.map((b) => I18n.t(`ov.odds.unseen.${b.role}`)));
        const text = I18n.t("ov.odds.gap.missing_board", { who });
        return { kind: "note", text, shares: [], ranges: NO_RANGES, approx: false, about, said: text };
      }
      const text = I18n.t(`ov.odds.gap.${odds.gap}`);
      return { kind: "note", text, shares: [], ranges: NO_RANGES, approx: false, about, said: text };
    }
    if (![odds.won, odds.tied, odds.lost].every(isShare)) return null;
    // Label and number apart, so a narrow column can stack them.
    const share = (key, value) => ({
      key,
      label: I18n.t(`ov.odds.${key}Label`),
      value: shareText(value),
      text: I18n.t(`ov.odds.${key}`, { value: shareText(value) }),
    });
    const shares = [share("win", odds.won), share("tie", odds.tied), share("lose", odds.lost)];
    const approximate = Array.isArray(odds.approximate) ? odds.approximate : [];
    const approx = approximate.length > 0 || Boolean(odds.start_of_combat);
    // The odds come back for a game build the rules were not checked against,
    // with this warning (D-145).
    const warning = Number.isInteger(odds.new_build) && odds.new_build > 0
      ? I18n.t("ov.odds.newBuild", { build: odds.new_build })
      : null;
    const ranges = { win: rangeView(odds.dealt, "Win"), lose: rangeView(odds.taken, "Lose") };
    const warningShort = warning ? I18n.t("ov.odds.warningShort", { build: odds.new_build }) : null;
    const said = [
      I18n.t("ov.odds.label"),
      shares.map((s) => s.text).join(", "),
      ranges.win && ranges.win.said,
      ranges.lose && ranges.lose.said,
      warning,
      approx ? I18n.t("ov.odds.approx") : null,
    ].filter(Boolean).join(". ");
    return { kind: "odds", text: null, shares, ranges, approx, warning, warningShort, about, said };
  }

  const api = { oddsView, SAID_GAPS };
  if (typeof module !== "undefined" && module.exports) module.exports = api;
  else window.TLOdds = api;
})();

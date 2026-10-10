// Words for the overlay's new data (T-208's overlay side, T-312, T-313):
// short keyword marks with their full names, the leaderboard's triple count
// and trinkets, and the next opponent. Pure: plain data in, plain data out,
// so it is tested without a page (ui-tests/overlay-words.test.js). The page
// draws them (overlay.js, overlay-hover.js) with textContent only.
"use strict";

(() => {
  const I18n = typeof module !== "undefined" && module.exports ? require("./i18n.js") : window.TLI18n;

  // Short marks fit the hover card's minion chips; the legend spells them
  // out, and each mark carries its full name for screen readers. Letters,
  // never colour alone. The marks and names are in the language's table
  // (strings/common.js: keyword.<key>.short and keyword.<key>), read when
  // they are used, so a language change is followed.
  const KEYWORDS = ["taunt", "divine_shield", "reborn", "venomous", "windfury", "stealth", "deathrattle", "rally"];
  const MAX_KEYWORDS = 8;
  const MAX_TRINKETS = 3;

  const titleCase = (key) =>
    String(key).split("_").filter(Boolean).map((w) => w.charAt(0).toUpperCase() + w.slice(1)).join(" ");

  // The mark and the full name of a keyword; one this version does not know
  // shows as its own name.
  function marksOf(key) {
    if (KEYWORDS.includes(key)) return [I18n.t(`keyword.${key}.short`), I18n.t(`keyword.${key}`)];
    return [titleCase(key), titleCase(key)];
  }

  // A minion's keywords as marks; null when the record did not keep them
  // (older than parser revision 5): "not recorded", not "none".
  function keywordMarks(keywords) {
    if (!Array.isArray(keywords)) return null;
    return keywords
      .filter((k) => typeof k === "string" && k)
      .slice(0, MAX_KEYWORDS)
      .map((k) => {
        const [short, full] = marksOf(k);
        return { key: k, short, full };
      });
  }

  // The legend's key to the marks, in the order the parser writes them.
  function keywordLegend() {
    return KEYWORDS.map((key) => marksOf(key).join(" ")).join(" · ");
  }

  // The leaderboard's triple count for one hero. null: the log never gave
  // one for this hero, which is said in words, never shown as 0.
  function triplesText(triples) {
    if (!Number.isInteger(triples) || triples < 0) return I18n.t("ov.triples.unknown");
    return I18n.plural("ov.triples", triples);
  }

  // One trinket as text: the card data's name, else its card id, else the
  // log's database id. `named` says whether the card data named it.
  function trinketText(trinket, nameOf) {
    const id = trinket && typeof trinket.card_id === "string" ? trinket.card_id : null;
    const name = id ? nameOf(id) : null;
    if (typeof name === "string" && name) return { text: name, named: true, known: true };
    if (id) return { text: id, named: false, known: name !== null };
    const dbf = trinket && Number.isInteger(trinket.dbf_id) ? trinket.dbf_id : "?";
    return { text: I18n.t("ov.trinket.unknown", { id: dbf }), named: false, known: false };
  }

  // What the hover card says about one hero's leaderboard values.
  function seatWords(seat, nameOf) {
    const trinkets = (Array.isArray(seat && seat.trinkets) ? seat.trinkets : [])
      .slice(0, MAX_TRINKETS)
      .map((t) => ({ ...trinketText(t, nameOf), hero_power: t.slot === "hero_power" }));
    return {
      triples: triplesText(seat ? seat.triples : null),
      trinkets,
      named: trinkets.some((t) => t.named),
      unknown: trinkets.some((t) => !t.known),
    };
  }

  // The opponent the game marks as next (T-313), if any.
  function nextOpponent(opponents) {
    return (Array.isArray(opponents) ? opponents : []).find((o) => o && o.next === true) || null;
  }

  // The game keeps its mark on the opponent being fought until the next
  // shop turn: while combat N is under way (the log's last combat is the
  // current turn) the marked opponent is the current one, not the next.
  function nextLabels(game) {
    const value = (s) => (s && Number.isInteger(s.value) ? s.value : null);
    const turn = value(game && game.turn);
    const fighting = turn !== null && value(game && game.last_combat) === turn;
    const team = Boolean(game && game.is_duos);
    if (fighting) return { badge: I18n.t("ov.now.badge"), heading: team ? I18n.t("ov.now.headingTeam") : I18n.t("ov.now.heading") };
    return { badge: I18n.t("ov.next.badge"), heading: team ? I18n.t("ov.next.headingTeam") : I18n.t("ov.next.heading") };
  }

  const api = { KEYWORDS, keywordMarks, keywordLegend, triplesText, trinketText, seatWords, nextOpponent, nextLabels };
  if (typeof module !== "undefined" && module.exports) module.exports = api;
  else window.TLWords = api;
})();

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

  // The hover board (T-333): one entry per hero of the opponent (two in a
  // Duos team), in the order of its seats, each with its hero, tier, the
  // leaderboard's values and the last board met. A board is matched by its
  // seat, never by position, so a team's two boards never swap; a Solo board
  // without a seat (older records) belongs to the only hero.
  function seatViews(opponent) {
    const o = opponent || {};
    const seats = Array.isArray(o.seats) ? o.seats : [];
    const heroes = Array.isArray(o.heroes) && o.heroes.length === seats.length ? o.heroes : [];
    const tiers = Array.isArray(o.tiers) ? o.tiers : [];
    const boards = Array.isArray(o.boards) ? o.boards : [];
    const shown = o.shown_source === "log" && Array.isArray(o.shown) ? o.shown : [];
    const possible = Array.isArray(o.possible) ? o.possible : [];
    return seats.map((seat, i) => {
      const bySeat = boards.find((b) => b && b.seat === seat);
      const lone = seats.length === 1 && boards.length === 1 && !Number.isInteger(boards[0].seat) ? boards[0] : null;
      const tier = tiers[i] && Number.isInteger(tiers[i].value) ? tiers[i].value : null;
      return {
        seat,
        hero: heroes[i] || null,
        tier,
        shown: shown.find((s) => s && s.seat === seat) || null,
        board: bySeat || lone || null,
        possible: possible.filter((p) => p && (p.seat === seat || (seats.length === 1 && !Number.isInteger(p.seat)))),
      };
    });
  }

  // When a board was seen, in words: "Turn 6 · this turn" or "Turn 6 · 2
  // turns ago"; the turn alone when the turn now is unknown.
  function seenWords(round, turnNow) {
    if (!Number.isInteger(round)) return I18n.t("ov.board.noneYet");
    const turn = I18n.t("ov.board.turn", { turn: round });
    if (!Number.isInteger(turnNow) || round > turnNow) return turn;
    const ago = turnNow - round;
    return `${turn} · ${ago === 0 ? I18n.t("ov.board.thisTurn") : I18n.plural("ov.board.ago", ago)}`;
  }

  // The hovered hero of a Duos team (T-340): the app says which of the
  // team's two portraits the mouse is on (hover.rs, 0 the top one). The
  // game puts the higher player id on top (T-343, D-080): the user saw the
  // lower id's board under an opposing team's top portrait, and their own
  // hero is the top one of their own team, where they hold the higher id in
  // 33 of their 39 Duos games. The log does not say it; hero entity ids
  // agree with the lower-id order in 13 of 21 teams, so they do not fit.
  // With no portrait known (an older app or Solo), the whole team stays.
  function hoveredViews(views, member) {
    const list = Array.isArray(views) ? views : [];
    if (list.length < 2 || !Number.isInteger(member) || member < 0 || member >= list.length) return list;
    const topFirst = [...list].sort((a, b) => b.seat - a.seat);
    return [topFirst[member]];
  }

  // One opponent in the Simple side panel (T-339): a face per hero with
  // their tier when known, the health, the triples the leaderboard shows (0
  // when it shows none or did not say), the game's next mark. Unknown
  // values are null and left out of the panel, never shown as "?".
  function simpleRow(opponent) {
    const o = opponent || {};
    const seats = Array.isArray(o.seats) ? o.seats : [];
    const heroes = Array.isArray(o.heroes) && o.heroes.length === seats.length ? o.heroes : [];
    const tiers = Array.isArray(o.tiers) ? o.tiers : [];
    const shown = o.shown_source === "log" && Array.isArray(o.shown) ? o.shown : [];
    const health = o.health && Number.isInteger(o.health.value) ? o.health.value : null;
    const triples = shown.reduce((sum, s) => sum + (s && Number.isInteger(s.triples) && s.triples > 0 ? s.triples : 0), 0);
    return {
      faces: seats.map((_, i) => ({
        hero: heroes[i] || null,
        tier: tiers[i] && Number.isInteger(tiers[i].value) ? tiers[i].value : null,
      })),
      health,
      triples,
      next: o.next === true,
      out: health !== null && health <= 0,
    };
  }

  // The lobby's tribes for the Simple panel: read from the game (D-063) or
  // entered by the user, else the ones the tavern confirmed, marked inferred
  // with how many of the five when not all. Offers seen in the tavern alone
  // are not the lobby: Simple leaves them to Advanced.
  function simpleTribes(game) {
    const lobby = game && game.lobby_tribes;
    if (!lobby || !Array.isArray(lobby.tribes) || !lobby.tribes.length) return null;
    if (lobby.basis === "game" || lobby.basis === "entered") return { tribes: lobby.tribes, inferred: false, count: null };
    if (lobby.basis === "tavern_confirmed") {
      return { tribes: lobby.tribes, inferred: true, count: lobby.complete ? null : lobby.tribes.length };
    }
    return null;
  }

  // The combat damage cap the game shows (T-315): "Max damage 15", the
  // game's own words, in Simple and Advanced; null when no cap applies or
  // the log has none (never a guess).
  function damageCap(game) {
    const cap = game ? game.damage_cap : null;
    if (!Number.isInteger(cap) || cap <= 0) return null;
    return { text: I18n.t("ov.cap", { value: I18n.num(cap) }), about: I18n.t("ov.cap.about") };
  }

  const api = {
    KEYWORDS,
    keywordMarks,
    keywordLegend,
    triplesText,
    trinketText,
    seatWords,
    nextOpponent,
    nextLabels,
    seatViews,
    seenWords,
    hoveredViews,
    simpleRow,
    simpleTribes,
    damageCap,
  };
  if (typeof module !== "undefined" && module.exports) module.exports = api;
  else window.TLWords = api;
})();

// Words for the overlay's data (T-312, T-313): the leaderboard's triple
// count and trinkets, and the next opponent. Pure: plain data in, plain data out,
// so it is tested without a page (ui-tests/overlay-words.test.js). The page
// draws them (overlay.js, overlay-hover.js) with textContent only.
"use strict";

(() => {
  const I18n = typeof module !== "undefined" && module.exports ? require("./i18n.js") : window.TLI18n;

  const MAX_TRINKETS = 3;

  // What the hover card says about one hero's leaderboard values: the
  // triples (0 when the log gave none) and how many trinkets it holds.
  function seatWords(seat) {
    const triples = seat && Number.isInteger(seat.triples) && seat.triples > 0 ? seat.triples : 0;
    const trinkets = Array.isArray(seat && seat.trinkets) ? Math.min(seat.trinkets.length, MAX_TRINKETS) : 0;
    return { triples, trinkets };
  }

  // The game keeps its mark on the opponent being fought until the next
  // shop turn: while combat N is under way (the log's last combat is the
  // current turn) the marked opponent is the current one, not the next.
  function nextLabel(game) {
    const value = (s) => (s && Number.isInteger(s.value) ? s.value : null);
    const turn = value(game && game.turn);
    const fighting = turn !== null && value(game && game.last_combat) === turn;
    return I18n.t(fighting ? "ov.now.badge" : "ov.next.badge");
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
      };
    });
  }

  // When a board was seen, in words: "Turn 6 · this turn" or "Turn 6 · 2
  // turns ago"; the turn alone when the turn now is unknown; null when the
  // round is not known.
  function seenWords(round, turnNow) {
    if (!Number.isInteger(round)) return null;
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

  // One opponent in the side panel (T-339): a face per hero with
  // their tier when known, the health, the triples the leaderboard shows (0
  // when it shows none or did not say), the game's next mark. Unknown
  // values are null and left out of the panel, never shown as "?".
  function opponentRow(opponent) {
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

  // The lobby's tribes: read from the game (D-063) or entered by the user,
  // else the ones the tavern confirmed, marked inferred with how many of the
  // five when not all. Offers seen in the tavern alone are not the lobby.
  function lobbyTribes(game) {
    const lobby = game && game.lobby_tribes;
    if (!lobby || !Array.isArray(lobby.tribes) || !lobby.tribes.length) return null;
    if (lobby.basis === "game" || lobby.basis === "entered") return { tribes: lobby.tribes, inferred: false, count: null };
    if (lobby.basis === "tavern_confirmed") {
      return { tribes: lobby.tribes, inferred: true, count: lobby.complete ? null : lobby.tribes.length };
    }
    return null;
  }

  // The combat damage cap the game shows (T-315): "Max damage 15", the
  // game's own words; null when no cap applies or
  // the log has none (never a guess).
  function damageCap(game) {
    const cap = game ? game.damage_cap : null;
    if (!Number.isInteger(cap) || cap <= 0) return null;
    return { text: I18n.t("ov.cap", { value: I18n.num(cap) }), about: I18n.t("ov.cap.about") };
  }

  const api = {
    seatWords,
    nextLabel,
    seatViews,
    seenWords,
    hoveredViews,
    opponentRow,
    lobbyTribes,
    damageCap,
  };
  if (typeof module !== "undefined" && module.exports) module.exports = api;
  else window.TLWords = api;
})();

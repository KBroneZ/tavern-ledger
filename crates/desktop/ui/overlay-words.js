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
  // team's two portraits the mouse is on (hover.rs, 0 the top one). On an
  // opposing team the game puts the lower player id on top: in two
  // screenshots of the game's leaderboard, read against the Power.log of the
  // same game, the upper portrait was the lower id both times (3 over 4, 6
  // over 7); the opposite rule showed the other hero's board. The log does
  // not say the order itself. With no portrait known (an older app or Solo),
  // the whole team stays.
  function hoveredViews(views, member) {
    const list = Array.isArray(views) ? views : [];
    if (list.length < 2 || !Number.isInteger(member) || member < 0 || member >= list.length) return list;
    const topFirst = [...list].sort((a, b) => a.seat - b.seat);
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
    // The armor shield is shown only when the game shows one: none is not 0.
    const armor = o.armor && Number.isInteger(o.armor.value) && o.armor.value > 0 ? o.armor.value : null;
    const triples = shown.reduce((sum, s) => sum + (s && Number.isInteger(s.triples) && s.triples > 0 ? s.triples : 0), 0);
    return {
      faces: seats.map((_, i) => ({
        hero: heroes[i] || null,
        tier: tiers[i] && Number.isInteger(tiers[i].value) ? tiers[i].value : null,
      })),
      health,
      armor,
      triples,
      next: o.next === true,
      out: health !== null && health <= 0,
    };
  }

  // The lobby's tribes the overlay shows (D-148): only the ones read from
  // the game (D-063). Entered tribes and the tavern's deductions stay in the
  // main window and the saved games; never here.
  function lobbyTribes(game) {
    const lobby = game && game.lobby_tribes;
    if (!lobby || lobby.basis !== "game" || !Array.isArray(lobby.tribes) || !lobby.tribes.length) return null;
    return { tribes: lobby.tribes };
  }

  // Why the tribes are not shown, from the reader's status (D-148).
  const REASONS = {
    off: "off",
    unsupported_version: "unsupported",
    no_lobby: "no_lobby",
    waiting: "waiting",
    not_running: "failed",
    no_access: "failed",
  };

  // The tribes panel: the tribes read from the game, or none and one line
  // that says why. A status that reads "ok" with no tribes (the tavern
  // contradicted them, or they did not pass a check) is a failed read.
  function tribesPanel(game) {
    const known = lobbyTribes(game);
    if (known) return { tribes: known.tribes, reason: null };
    const status = game && game.game_tribes && game.game_tribes.status;
    const key = Object.prototype.hasOwnProperty.call(REASONS, status) ? REASONS[status] : "failed";
    return { tribes: [], reason: I18n.t(`ov.tribes.reason.${key}`) };
  }

  // The marks over the game's leaderboard (T-368): one per opponent whose
  // place is a row of the board, with its health and the triples the log
  // gave. A place the board does not have, or a board size not known, gives
  // no mark: the position is never a guess.
  function opponentMarks(game) {
    const slots = game && game.leaderboard_slots;
    if (!Number.isInteger(slots) || slots < 1) return [];
    const list = Array.isArray(game.opponents) ? game.opponents : [];
    return list.flatMap((o) => {
      const place = o && o.place && o.place.value;
      if (!Number.isInteger(place) || place < 1 || place > slots) return [];
      const row = opponentRow(o);
      return [{ place, slots, health: row.health, armor: row.armor, triples: row.triples, out: row.out }];
    });
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
    tribesPanel,
    opponentMarks,
    damageCap,
  };
  if (typeof module !== "undefined" && module.exports) module.exports = api;
  else window.TLWords = api;
})();

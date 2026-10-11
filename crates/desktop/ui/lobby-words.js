// Words for the overlay's lobby panel (T-338): the own rating (from the game,
// D-063), the change the last game made and the day's summary per mode. Pure:
// plain data in, plain data out, so it is tested without a page
// (ui-tests/lobby-words.test.js). The page draws them (overlay.js) with
// textContent only. The app decides every value; this file only words them.
"use strict";

(() => {
  const I18n = typeof module !== "undefined" && module.exports ? require("./i18n.js") : window.TLI18n;

  const SOLO = "GT_BATTLEGROUNDS";
  const DUOS = "GT_BATTLEGROUNDS_DUO";
  const MINUS = "−";

  // The viewer's day, as the app wants it: local midnight to the next one,
  // in UTC seconds (a clock change makes the day 23 or 25 hours long).
  function dayRange(now) {
    const start = new Date(now.getFullYear(), now.getMonth(), now.getDate());
    const end = new Date(now.getFullYear(), now.getMonth(), now.getDate() + 1);
    return { dayStart: Math.floor(start.getTime() / 1000), dayEnd: Math.floor(end.getTime() / 1000) };
  }

  const isRating = (v) => Number.isInteger(v) && v > 0;

  // "+40" / "−25"; null for anything that is not a change.
  function signed(delta) {
    if (!Number.isInteger(delta) || delta === 0) return null;
    return `${delta > 0 ? "+" : MINUS}${I18n.num(Math.abs(delta))}`;
  }

  // The rating rows: one per mode the game shows a rating for, with the
  // last game's change on its mode. Only a read that passed (status "ok")
  // gives numbers; never a guess.
  function ratingRows(view) {
    const rating = (view && view.rating) || {};
    if (rating.status !== "ok") return [];
    const change = view.rating_change && typeof view.rating_change.duos === "boolean" ? view.rating_change : null;
    return [
      ["solo", rating.solo, false],
      ["duos", rating.duos, true],
    ]
      .filter(([, value]) => isRating(value))
      .map(([mode, value, duos]) => {
        const delta = change && change.duos === duos ? signed(change.delta) : null;
        return {
          mode,
          label: I18n.t(duos ? "mode.GT_BATTLEGROUNDS_DUO" : "mode.GT_BATTLEGROUNDS"),
          value: I18n.num(value),
          change: delta,
          up: delta ? change.delta > 0 : null,
        };
      });
  }

  // A line under the ratings when there are none to show and the reason is
  // worth saying. Advanced also says how to get them; a failure always shows.
  function ratingNote(view, advanced) {
    const status = view && view.rating ? view.rating.status : null;
    if (status === "unsupported_version") return I18n.t("lobby.rating.unsupported");
    if (status === "no_access") return I18n.t("lobby.rating.noAccess");
    if (status === "off" && advanced) return I18n.t("lobby.rating.off");
    return null;
  }

  // One line per mode played today: games, average place, top-half games.
  // Every number says how many games are behind it (T-342): a game with no
  // place (not finished in the log) counts in the games but not in the
  // average or the top half, so those say "9 with a place" and "6 of 9".
  function summaryLines(view) {
    const today = Array.isArray(view && view.today) ? view.today : [];
    const lines = today
      .filter((m) => (m.mode === SOLO || m.mode === DUOS) && m.tally && m.tally.games > 0)
      .map((m) => {
        const t = m.tally;
        const placed = Number.isInteger(t.placed) ? t.placed : 0;
        const parts = [I18n.plural("lobby.games", t.games)];
        if (placed > 0 && placed < t.games) parts.push(I18n.t("lobby.placed", { n: I18n.num(placed) }));
        if (placed > 0 && typeof t.average_place === "number") {
          parts.push(I18n.t("lobby.avg", { value: I18n.num(t.average_place, 1) }));
        }
        if (placed > 0 && Number.isInteger(m.top_half_games)) {
          parts.push(I18n.t("lobby.top", { top: m.top_half, count: I18n.num(m.top_half_games), of: I18n.num(placed) }));
        }
        const mode = I18n.t(`mode.${m.mode}`);
        return I18n.t("lobby.today", { mode, list: parts.join(" · ") });
      });
    return lines.length ? lines : [I18n.t("lobby.today.none")];
  }

  const api = { dayRange, signed, ratingRows, ratingNote, summaryLines };
  if (typeof module !== "undefined" && module.exports) module.exports = api;
  else window.TLLobby = api;
})();

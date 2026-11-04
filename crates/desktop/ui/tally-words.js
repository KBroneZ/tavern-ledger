// Which mark each of the four numbers under the tabs carries (T-360). The app
// sends a source per number; a mode that has no game yet has none for every
// number, and "unknown" there reads like an error. This file turns that case
// into "No games yet" once, under the games count, and leaves "unknown" for a
// number a game left out. Pure: plain data in, plain data out, so it is tested
// without a page (ui-tests/tally-words.test.js).
"use strict";

(() => {
  const KEYS = ["games", "average_place", "top_half_share", "wins"];
  const KNOWN = ["log", "inferred", "entered", "leaderboard", "card_data", "possible", "game", "unknown"];

  // `totals.games === 0` is a mode with no game; a tally the app has not sent
  // yet has no `games` at all, which is not the same thing.
  const hasNoGames = (totals) => Boolean(totals) && totals.games === 0;

  // "nogames": the pill that says so; "none": no pill and no source tip.
  function marks(totals, sources) {
    if (hasNoGames(totals)) {
      return { games: "nogames", average_place: "none", top_half_share: "none", wins: "none" };
    }
    const given = sources || {};
    return Object.fromEntries(KEYS.map((key) => [key, KNOWN.includes(given[key]) ? given[key] : "unknown"]));
  }

  // The one line about the marks shows until the first game is in the history.
  const hintShown = (games) => games === 0;

  // The mark on a hero's name in the stats: the name shown is the card
  // database's whenever it has one (D-066), so the log's "unknown" (it did not
  // print the name) is left out then; with no card name the log's source stays.
  const heroNameMark = (source, fromCardData) => (fromCardData ? null : source);

  const api = { marks, hintShown, heroNameMark };
  if (typeof module !== "undefined" && module.exports) module.exports = api;
  else window.TLTallyWords = api;
})();

// When each game was played (T-540): the game's own start and length from the
// log's clock (`started` and `ended`, UTC seconds the app saved with the
// game), else the log session's start and the game's number in it, as before.
// Pure: plain data in, plain words out, tested without a page
// (ui-tests/game-time.test.js).
"use strict";

(() => {
  const I18n = typeof module !== "undefined" && module.exports ? require("./i18n.js") : window.TLI18n;

  // "Hearthstone_2026_10_09_15_26_25": the local time the game client started.
  const SESSION = /^Hearthstone_(\d{4})_(\d{2})_(\d{2})_(\d{2})_(\d{2})_\d{2}$/;

  const own = (game) => Number.isInteger(game.started) && Number.isInteger(game.ended) && game.ended >= game.started;

  // The game's own start ("11/10/2026, 02:56"), else the session's start and
  // the game's number ("11/10/2026, 02:56 · 2").
  function when(game) {
    if (own(game)) return I18n.dateTime(new Date(game.started * 1000));
    const m = SESSION.exec(game.session);
    const start = m ? I18n.dateTime(new Date(Number(m[1]), Number(m[2]) - 1, Number(m[3]), Number(m[4]), Number(m[5]))) : game.session;
    return `${start} · ${game.index}`;
  }

  // How long the game took ("23 min"); null when the log's clock does not say.
  function length(game) {
    if (!own(game)) return null;
    return I18n.t("win.recap.length", { n: I18n.num(Math.max(1, Math.round((game.ended - game.started) / 60))) });
  }

  // The tooltip that says where the time comes from.
  const tip = (game) => I18n.t(own(game) ? "win.recap.subTipGame" : "win.recap.subTip");

  const api = { when, length, tip };
  if (typeof module !== "undefined" && module.exports) module.exports = api;
  else window.TLGameTime = api;
})();

// Where the app puts things by default, copied from the Rust source so the
// workbench shows them at any window size: the game's 16:9 picture
// (screen_fit.rs `play_area`), the combat odds bar (overlay_layout.rs
// `with_default_odds`), the hover card (hover.rs, screen_fit.rs `place_card`)
// and the leaderboard (leaderboard.rs `default_area`).
// ui-tests/overlay-gallery.test.js fails when a number here differs from the
// Rust source. Sizes are the overlay window's, in its own pixels.
"use strict";

(() => {
  const ODDS_WIDTH = 0.5;
  const ODDS_TOP = 0.01;
  const MIN_WIDTH = 160;
  const MAX_WIDTH = 800;
  const CARD_SHARE = 0.85;
  const GAP = 8;
  const MIN_CARD_WIDTH = 120;
  const LEADERBOARD = { leftFromCentre: 0.665, width: 0.085, top: 0.16, height: 0.68 };

  // The largest 16:9 rectangle in the window, centred.
  function playArea(w, h) {
    const pw = w * 9 <= h * 16 ? w : (h * 16) / 9;
    const ph = (pw * 9) / 16;
    return { x: (w - pw) / 2, y: (h - ph) / 2, w: pw, h: ph };
  }

  function oddsRect(w, h) {
    const play = playArea(w, h);
    const width = Math.min(Math.max(ODDS_WIDTH * play.h, Math.min(MIN_WIDTH, w)), Math.min(MAX_WIDTH, w));
    return { x: play.x + (play.w - width) / 2, y: play.y + ODDS_TOP * play.h, w: width, h: null };
  }

  // The hover card: as wide as the game's own board, centred on the picture,
  // at its top.
  function hoverCard(w, h) {
    const play = playArea(w, h);
    const room = Math.min(play.w - 2 * GAP, w - 2 * GAP);
    const width = Math.min(Math.max(CARD_SHARE * play.h, MIN_CARD_WIDTH), Math.max(room, MIN_CARD_WIDTH));
    const x = Math.min(Math.max(w / 2 - width / 2, 0), Math.max(w - width, 0));
    const y = Math.min(Math.max(play.y + GAP, GAP), Math.max(h - GAP, GAP));
    return { x, y, width, max_height: Math.max(0, h - GAP - y) };
  }

  // The measured leaderboard of a game window of this size.
  function leaderboardArea(w, h) {
    const play = playArea(w, h);
    return {
      x: play.x + play.w / 2 - LEADERBOARD.leftFromCentre * play.h,
      y: play.y + LEADERBOARD.top * play.h,
      w: LEADERBOARD.width * play.h,
      h: LEADERBOARD.height * play.h,
    };
  }

  const api = { ODDS_WIDTH, ODDS_TOP, CARD_SHARE, LEADERBOARD, playArea, oddsRect, hoverCard, leaderboardArea };
  if (typeof module !== "undefined" && module.exports) module.exports = api;
  else window.TLGalleryDefaults = api;
})();

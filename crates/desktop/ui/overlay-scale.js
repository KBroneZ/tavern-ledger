// Text sizes of the overlay (T-334): they follow the screen, not fixed pixels,
// so the text reads at a glance at 3840x2160 at 150% as at 1920x1080 at 100%.
// The overlay window covers the game's monitor in its own (logical) pixels,
// so a share of its height is the same share of the screen at any scaling.
// The hover board's text follows the board's width, which the app sizes to
// the game window (hover.rs). Pure, so it is tested without a page
// (ui-tests/overlay-scale.test.js).
"use strict";

(() => {
  // Body text: 1.32% of the window's height (about 28 physical pixels on a
  // 2160-pixel screen, 14 on a 1080 one), never under 13 px.
  const ROOT_SHARE = 0.0132;
  // The board's text: 1.65% of its width (seven minions across).
  const CARD_SHARE = 0.0165;
  const MIN_PX = 13;
  const MAX_PX = 44;

  const round = (n) => Math.round(n * 10) / 10;
  const clamp = (n) => Math.min(Math.max(n, MIN_PX), MAX_PX);
  const usable = (n) => typeof n === "number" && Number.isFinite(n) && n > 0;

  // The page's base text size for a window this tall (its own pixels).
  function rootPx(windowHeight) {
    return usable(windowHeight) ? round(clamp(windowHeight * ROOT_SHARE)) : MIN_PX;
  }

  // The hover board's base text size for a board this wide.
  function cardPx(cardWidth) {
    return usable(cardWidth) ? round(clamp(cardWidth * CARD_SHARE)) : MIN_PX;
  }

  // How many turns ago a board was seen: combat N is in game turn N (D-049).
  // null when either is unknown, never a guess.
  function turnsAgo(turnNow, seenIn) {
    if (!Number.isInteger(turnNow) || !Number.isInteger(seenIn) || seenIn > turnNow) return null;
    return turnNow - seenIn;
  }

  const api = { ROOT_SHARE, CARD_SHARE, MIN_PX, rootPx, cardPx, turnsAgo };
  if (typeof module !== "undefined" && module.exports) module.exports = api;
  else window.TLScale = api;
})();

// Arranging the overlay by keyboard (T-317): what an arrow key does to a
// rectangle. Pure, so it is tested without a screen. The caller draws the
// result and saves it like the end of a drag (overlay-layout.js); the app
// still enforces the real limits when it saves.
//
//   Arrow            move by STEP pixels (Shift: BIG_STEP)
//   Alt + arrow      make bigger or smaller by the same steps
"use strict";

(() => {
  const STEP = 10;
  const BIG_STEP = 50;

  const DIRECTIONS = {
    ArrowLeft: [-1, 0],
    ArrowRight: [1, 0],
    ArrowUp: [0, -1],
    ArrowDown: [0, 1],
  };

  const clamp = (n, lo, hi) => Math.min(Math.max(n, lo), Math.max(lo, hi));

  // The same limits as dragging, so a key can never reach what a drag cannot.
  // `limits` is { minW, maxW, minH, grabH } for a panel; for the leaderboard
  // box `keepInside` also stops it at the bottom edge. `screen` is { w, h }.
  function nudge(rect, event, screen, limits) {
    const direction = DIRECTIONS[event.key];
    if (!direction) return null;
    const step = event.shiftKey ? BIG_STEP : STEP;
    const dx = direction[0] * step;
    const dy = direction[1] * step;
    const next = { ...rect };
    if (event.altKey) {
      next.w = clamp(rect.w + dx, limits.minW, Math.min(limits.maxW, screen.w - rect.x));
      next.h = clamp(rect.h + dy, limits.minH, screen.h - rect.y);
    } else {
      next.x = clamp(rect.x + dx, 0, screen.w - rect.w);
      next.y = clamp(rect.y + dy, 0, limits.keepInside ? screen.h - rect.h : screen.h - limits.grabH);
    }
    const changed = next.x !== rect.x || next.y !== rect.y || next.w !== rect.w || next.h !== rect.h;
    return changed ? next : rect;
  }

  const api = { STEP, BIG_STEP, nudge };
  if (typeof module !== "undefined" && module.exports) module.exports = api;
  else window.TLKeys = api;
})();

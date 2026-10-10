// Arranging the overlay by keyboard (T-317): what an arrow key does to a
// rectangle. Pure, so it is tested without a screen. The caller draws the
// result and saves it like the end of a drag (overlay-layout.js); the app
// still enforces the real limits when it saves.
//
//   Arrow            move by STEP pixels (Shift: BIG_STEP)
//   Alt + arrow      make bigger or smaller by the same steps
//
// The on-screen buttons (T-319, WCAG 2.5.7) are the same keys for a single
// click: `press(action, big)` gives the key press a button stands for.
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

  // Each on-screen button and the key it stands for (`resize`: with Alt);
  // `label` is the English text, `i18n` the key of its words (strings/overlay.js).
  const PAD = [
    { action: "left", label: "Move left", i18n: "ov.pad.left", key: "ArrowLeft", resize: false },
    { action: "up", label: "Move up", i18n: "ov.pad.up", key: "ArrowUp", resize: false },
    { action: "down", label: "Move down", i18n: "ov.pad.down", key: "ArrowDown", resize: false },
    { action: "right", label: "Move right", i18n: "ov.pad.right", key: "ArrowRight", resize: false },
    { action: "narrower", label: "Narrower", i18n: "ov.pad.narrower", key: "ArrowLeft", resize: true },
    { action: "wider", label: "Wider", i18n: "ov.pad.wider", key: "ArrowRight", resize: true },
    { action: "shorter", label: "Shorter", i18n: "ov.pad.shorter", key: "ArrowUp", resize: true },
    { action: "taller", label: "Taller", i18n: "ov.pad.taller", key: "ArrowDown", resize: true },
  ];

  function press(action, big) {
    const button = PAD.find((b) => b.action === action);
    return button ? { key: button.key, shiftKey: Boolean(big), altKey: button.resize } : null;
  }

  const api = { STEP, BIG_STEP, PAD, nudge, press };
  if (typeof module !== "undefined" && module.exports) module.exports = api;
  else window.TLKeys = api;
})();

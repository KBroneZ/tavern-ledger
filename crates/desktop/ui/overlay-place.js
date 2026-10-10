// Where the overlay's own controls go while unlocked (T-319): the toolbar, the
// key hint and the move-and-size buttons must never cover a panel, wherever
// the user put it. Pure, so it is tested without a screen. Rectangles are
// { x, y, w, h } in the window's own pixels; sizes are { w, h }.
"use strict";

(() => {
  // Space kept from the screen's edges and between a panel and its buttons.
  const MARGIN = 8;

  const clamp = (n, lo, hi) => Math.min(Math.max(n, lo), Math.max(lo, hi));

  // Strict: rectangles that only touch do not overlap.
  function overlaps(a, b) {
    return a.x < b.x + b.w && b.x < a.x + a.w && a.y < b.y + b.h && b.y < a.y + a.h;
  }

  function covered(a, b) {
    const w = Math.min(a.x + a.w, b.x + b.w) - Math.max(a.x, b.x);
    const h = Math.min(a.y + a.h, b.y + b.h) - Math.max(a.y, b.y);
    return w > 0 && h > 0 ? w * h : 0;
  }

  // The spots tried, in order: the preferred edge's centre first, then the
  // other edge, the middle, and the corners and sides.
  function candidates(size, screen, prefer) {
    const xs = {
      left: MARGIN,
      centre: Math.round((screen.w - size.w) / 2),
      right: screen.w - size.w - MARGIN,
    };
    const ys = {
      top: MARGIN,
      middle: Math.round((screen.h - size.h) / 2),
      bottom: screen.h - size.h - MARGIN,
    };
    const first = prefer === "bottom" ? "bottom" : "top";
    const second = first === "top" ? "bottom" : "top";
    const order = [
      ["centre", first], ["centre", second], ["centre", "middle"],
      ["left", first], ["right", first], ["left", second], ["right", second],
      ["left", "middle"], ["right", "middle"],
    ];
    return order.map(([x, y]) => ({
      x: clamp(xs[x], MARGIN, screen.w - size.w - MARGIN),
      y: clamp(ys[y], MARGIN, screen.h - size.h - MARGIN),
    }));
  }

  // The first spot that covers none of `obstacles`; when every spot covers
  // something, the one that covers least. `prefer` is "top" or "bottom".
  function spot(size, screen, obstacles, prefer) {
    let best = null;
    let least = Infinity;
    for (const p of candidates(size, screen, prefer)) {
      const r = { ...p, ...size };
      const area = obstacles.reduce((sum, o) => sum + covered(r, o), 0);
      if (area === 0) return p;
      if (area < least) {
        best = p;
        least = area;
      }
    }
    return best;
  }

  // The move-and-size buttons: under the panel, else above it, to its right
  // or to its left, first where they cover no other panel either (`others`);
  // a panel with no room around it gets them inside, at its foot.
  function padSpot(size, target, screen, others = []) {
    const x = clamp(target.x, MARGIN, screen.w - size.w - MARGIN);
    const y = clamp(target.y, MARGIN, screen.h - size.h - MARGIN);
    const tries = [
      { x, y: target.y + target.h + MARGIN },
      { x, y: target.y - size.h - MARGIN },
      { x: target.x + target.w + MARGIN, y },
      { x: target.x - size.w - MARGIN, y },
    ];
    const fits = (p) => p.x >= 0 && p.y >= 0 && p.x + size.w <= screen.w && p.y + size.h <= screen.h;
    const clear = (p, rects) => fits(p) && rects.every((r) => !overlaps({ ...p, ...size }, r));
    const free = tries.find((p) => clear(p, [target, ...others])) || tries.find((p) => clear(p, [target]));
    if (free) return free;
    return {
      x: clamp(target.x + MARGIN, MARGIN, screen.w - size.w - MARGIN),
      y: clamp(target.y + target.h - size.h - MARGIN, MARGIN, screen.h - size.h - MARGIN),
    };
  }

  const api = { MARGIN, overlaps, spot, padSpot };
  if (typeof module !== "undefined" && module.exports) module.exports = api;
  else window.TLPlace = api;
})();

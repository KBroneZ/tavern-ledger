// The overlap check of the workbench: for each scenario at a window size, the
// rendered rectangles of everything the overlay shows (panels, the hover card,
// the marks beside the leaderboard) must share no area. The rectangles are
// read from the page with getBoundingClientRect, so real content (long names,
// eight players, Duos rows) counts, not only the computed boxes.
"use strict";

(() => {
  // Rectangles that only touch, or overlap by less than this, are not overlapping.
  const TOLERANCE = 0.5;

  function overlap(a, b) {
    const w = Math.min(a.x + a.w, b.x + b.w) - Math.max(a.x, b.x);
    const h = Math.min(a.y + a.h, b.y + b.h) - Math.max(a.y, b.y);
    return w > TOLERANCE && h > TOLERANCE ? { w: Math.round(w), h: Math.round(h) } : null;
  }

  // Every pair of named rectangles that share an area.
  function findOverlaps(rects) {
    const found = [];
    for (let i = 0; i < rects.length; i += 1) {
      for (let j = i + 1; j < rects.length; j += 1) {
        const shared = overlap(rects[i], rects[j]);
        if (shared) found.push({ a: rects[i].name, b: rects[j].name, ...shared });
      }
    }
    return found;
  }

  // What the page shows right now, as { name, x, y, w, h }.
  function collect(doc) {
    const out = [];
    const add = (name, node) => {
      const r = node.getBoundingClientRect();
      if (r.width > 0 && r.height > 0) out.push({ name, x: r.left, y: r.top, w: r.width, h: r.height });
    };
    for (const slot of doc.querySelectorAll(".slot")) if (!slot.hidden) add(slot.dataset.panel, slot);
    const card = doc.getElementById("hover-card");
    if (card && !card.hidden) add("hover-card", card);
    doc.querySelectorAll(".opp-mark").forEach((mark, i) => add(`mark ${i + 1}`, mark));
    return out;
  }

  const api = { overlap, findOverlaps, collect };
  if (typeof module !== "undefined" && module.exports) module.exports = api;
  else window.TLOverlaps = api;
})();

// The page's scrollbar and its cue of more content (the user's notes, T-323):
// no wide native scrollbar at the side. The bar stays thin and clear until
// the page scrolls or the pointer comes near the right edge (style.css); the
// wheel, the touchpad and the keyboard scroll as always. A soft fade at the
// bottom (and a shadow under the top bar) says there is more to scroll.
// `cue` and `nearEdge` are plain numbers in, plain data out
// (ui-tests/scroll-cue.test.js).
"use strict";

(() => {
  const SLACK = 2; // px of rounding at the ends
  const EDGE = 18; // px from the right edge where the bar shows
  const SHOW_MS = 900; // how long the bar stays after the last scroll

  function cue(top, view, total) {
    if (total - view <= SLACK) return { above: false, below: false };
    return { above: top > SLACK, below: top + view < total - SLACK };
  }

  const nearEdge = (x, width) => width - x <= EDGE;

  // Sets data-more-above / data-more-below and the "scroll-show" class on <html>.
  function start(win, doc) {
    const root = doc.documentElement;
    let hide = null;
    let near = false;
    const update = () => {
      const c = cue(win.scrollY, win.innerHeight, root.scrollHeight);
      root.toggleAttribute("data-more-above", c.above);
      root.toggleAttribute("data-more-below", c.below);
    };
    const show = () => {
      root.classList.add("scroll-show");
      clearTimeout(hide);
      hide = setTimeout(() => { if (!near) root.classList.remove("scroll-show"); }, SHOW_MS);
    };
    win.addEventListener("scroll", () => { update(); show(); }, { passive: true });
    win.addEventListener("resize", update);
    doc.addEventListener("mousemove", (e) => {
      // clientWidth stops where the bar starts, so a pointer on the bar counts too.
      const now = nearEdge(e.clientX, root.clientWidth);
      if (now === near) return;
      near = now;
      if (near) root.classList.add("scroll-show");
      else show();
    }, { passive: true });
    // Content grows and shrinks as folds open and lists expand.
    if (typeof win.ResizeObserver === "function") new win.ResizeObserver(update).observe(doc.body);
    update();
    return update;
  }

  const api = { cue, nearEdge, start };
  if (typeof module !== "undefined" && module.exports) module.exports = api;
  else window.TLScrollCue = api;
})();

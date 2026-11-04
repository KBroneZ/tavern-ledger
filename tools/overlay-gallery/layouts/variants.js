// Layout variants for the overlay's layout round: the real overlay page with
// the counters above, below or beside the opponents list. Variant B was
// chosen and is now the page itself (the tribes head the opponents panel and
// the counters are on), so a variant only moves the counters. It all happens
// in the gallery's frame, before the page's scripts run: the shipped page and
// its files are never changed. A frame with no `layout` shows the page as it
// ships.
"use strict";

(() => {
  // The stack's order, top to bottom, for each place of the counters; the
  // panels not named keep the page's own order after these.
  const ORDERS = {
    above: ["status", "heroes", "counters", "opponents"],
    below: ["status", "heroes", "opponents", "counters"],
    beside: ["status", "heroes", "counters", "opponents"],
  };
  const PLACES = Object.keys(ORDERS);

  // The key that hides and shows the whole overlay, and the app's own note.
  const HIDE_KEY = { en: "Ctrl+Shift+H", es: "Ctrl+Mayús+H" };
  const HIDDEN_NOTE = {
    en: (key) => `Overlay hidden. ${key} shows it again.`,
    es: (key) => `Overlay oculto. ${key} lo vuelve a mostrar.`,
  };

  // Counters in a column of their own, to the left of the opponents list,
  // both from the stack's top: neither moves the other.
  const BESIDE_CSS = `
#stack { display: grid; grid-template-columns: var(--stack-width) var(--stack-width); column-gap: 6px; row-gap: 6px; align-content: start; align-items: start; width: calc(2 * var(--stack-width) + 6px); }
#stack > .slot { grid-column: 2; }
#stack > .slot[data-panel="counters"] { grid-column: 1; grid-row: 1 / span 4; }`;

  // Everything the overlay draws goes; a short note says how to bring it back.
  const HIDE_ALL_CSS = `
#stack, #marks, #lobby-dock, body > .slot, #hover-card { visibility: hidden !important; }
.gallery-hidden-note { position: absolute; right: var(--stack-right); top: var(--stack-top); max-width: var(--stack-width); padding: 5px 9px 6px; border: 1px solid color-mix(in srgb, var(--border) 70%, transparent); border-radius: 10px; background: var(--panel-bg); color: var(--text); font: 600 0.9rem/1.35 var(--font-body); box-shadow: 0 6px 18px rgba(0, 0, 0, 0.45); }`;

  // The page's panel list in the variant's order, the counters panel included.
  function ordered(ids, order) {
    return [...order.filter((id) => ids.includes(id)), ...ids.filter((id) => !order.includes(id))];
  }

  function css(layout) {
    return [layout.place === "beside" ? BESIDE_CSS : "", layout.hideAll ? HIDE_ALL_CSS : ""].join("\n");
  }

  // Runs in the frame before the page's scripts. The counters panel's switch
  // (counters-words.js `ENABLED`) is turned on as the page defines it, and
  // the panel list comes back in the variant's order; the counters slot is
  // shown and the variant's styles are added after the page's own.
  function apply(win, layout, lang) {
    if (!layout) return;
    if (!PLACES.includes(layout.place)) throw new Error(`unknown counters place: ${layout.place}`);
    const doc = win.document;
    const order = ORDERS[layout.place];
    let api = null;
    Object.defineProperty(win, "TLCounters", {
      configurable: true,
      get: () => api,
      set(value) {
        api = { ...value, ENABLED: true, panelIds: (ids) => ordered(ids, order) };
      },
    });
    // Without the slot every variant would show no counters under a caption that says they are there.
    const slot = doc.querySelector('.slot[data-panel="counters"]');
    if (!slot) throw new Error("the overlay page has no counters slot");
    slot.removeAttribute("hidden");
    const style = doc.createElement("style");
    style.textContent = css(layout);
    doc.head.append(style);
    if (layout.hideAll) {
      const words = lang === "es" ? "es" : "en";
      const note = doc.createElement("p");
      note.className = "gallery-hidden-note";
      note.textContent = HIDDEN_NOTE[words](HIDE_KEY[words]);
      doc.body.append(note);
    }
  }

  const api = { ORDERS, PLACES, HIDE_KEY, ordered, css, apply };
  if (typeof module !== "undefined" && module.exports) module.exports = api;
  else window.TLGalleryLayouts = api;
})();

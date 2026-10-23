// The overlay's legend behind a "?" in the unlocked toolbar (T-614): a click
// shows it, a second click or Esc hides it. Nothing is saved, so it starts
// hidden every time. Plain DOM in, no app calls, so it is tested with a fake
// page (ui-tests/legend-toggle.test.js).
"use strict";

(() => {
  function bind(doc, button, legend) {
    const isOpen = () => !legend.hidden;
    const set = (open) => {
      legend.hidden = !open;
      button.setAttribute("aria-expanded", String(open));
    };
    button.addEventListener("click", () => set(!isOpen()));
    doc.addEventListener("keydown", (event) => {
      if (event.key !== "Escape" || !isOpen()) return;
      set(false);
      button.focus();
      event.preventDefault();
    });
    return { isOpen, close: () => set(false) };
  }

  const api = { bind };
  if (typeof module !== "undefined" && module.exports) module.exports = api;
  else window.TLLegend = api;
})();

// The overlay's legend behind a "?" in the unlocked toolbar (T-614): a click
// shows it, a second click or Esc hides it. Nothing is saved, so it starts
// hidden every time. Plain DOM in, no app calls, so it is tested with a fake
// page (ui-tests/legend-toggle.test.js).
"use strict";

(() => {
  // `onChange` runs after the legend opens or closes: the toolbar changed
  // height, so the page places it again.
  function bind(doc, button, legend, onChange = () => {}) {
    const isOpen = () => !legend.hidden;
    const set = (open) => {
      legend.hidden = !open;
      button.setAttribute("aria-expanded", String(open));
      onChange();
    };
    button.addEventListener("click", () => set(!isOpen()));
    doc.addEventListener("keydown", (event) => {
      if (event.key !== "Escape" || !isOpen()) return;
      // Focus goes back to the "?" only from the toolbar: a panel being
      // arranged by keyboard keeps it.
      const from = doc.activeElement;
      const inToolbar = !from || from === doc.body || from === button || from.closest("#toolbar");
      set(false);
      if (inToolbar) button.focus();
      event.preventDefault();
    });
    return {
      isOpen,
      close: () => {
        if (isOpen()) set(false);
      },
    };
  }

  const api = { bind };
  if (typeof module !== "undefined" && module.exports) module.exports = api;
  else window.TLLegend = api;
})();

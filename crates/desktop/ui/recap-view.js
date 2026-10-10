// The recap in the full window (T-322): opening a game's recap replaces the
// page (not a fold, not a new window); the close button (X) or Escape goes
// back to where you were, with the page scrolled as it was and focus on the
// button that opened it (found again by its key if the list was redrawn,
// see focus-keeper.js). Tested with a fake page (ui-tests/recap-view.test.js).
"use strict";

(() => {
  // `hide`: the parts of the page the recap replaces. `blocked()`: true while
  // a dialog is open over the recap, so Escape only closes that dialog.
  function create({ doc, win, focus, view, heading, hide, blocked, onClose }) {
    let back = null; // where to go back to: { scrollY, focus }

    const isOpen = () => !view.hidden;

    // `fallbackKey`: the data-focus key to go back to when nothing had focus
    // (a recap that opened by itself when a game ended).
    function open(fallbackKey) {
      if (!isOpen()) {
        const noted = focus.note(doc);
        const fallback = fallbackKey ? { id: null, key: fallbackKey, next: [], scopeId: null, index: -1 } : null;
        back = { scrollY: win.scrollY, focus: noted || fallback };
        hide.forEach((node) => { node.hidden = true; });
        view.hidden = false;
        win.scrollTo(0, 0);
        // Only on opening: a reload while it is open leaves focus where it is.
        heading.focus({ preventScroll: true });
      }
    }

    function close() {
      if (!isOpen()) return;
      view.hidden = true;
      hide.forEach((node) => { node.hidden = false; });
      const saved = back;
      back = null;
      if (saved) {
        win.scrollTo(0, saved.scrollY);
        focus.restore(doc, saved.focus);
      }
      if (onClose) onClose();
    }

    doc.addEventListener("keydown", (e) => {
      if (e.key !== "Escape" || !isOpen() || e.defaultPrevented || blocked()) return;
      e.preventDefault();
      close();
    });

    return { open, close, isOpen };
  }

  const api = { create };
  if (typeof module !== "undefined" && module.exports) module.exports = api;
  else window.TLRecapView = api;
})();

// The hide-all toggle on the overlay page (T-614). The app owns the state
// (the hide key or the tray menu; overlay_settings `all_hidden`, never saved);
// the page only follows it: one class on the body hides everything the overlay
// draws (overlay.css), and a short note says how to bring it back for about
// 3 seconds. The note is text in a click-through window: it takes no focus and
// no clicks. Plain DOM and a clock in, so it is tested with a fake page
// (ui-tests/hide-all.test.js).
"use strict";

(() => {
  const I18n = typeof module !== "undefined" && module.exports ? require("./i18n.js") : window.TLI18n;

  // How long the note stays after hiding.
  const NOTE_MS = 3000;

  function create(doc, note, clock) {
    let hidden = false;
    let timer = null;

    const dropNote = () => {
      if (timer !== null) clock.clearTimeout(timer);
      timer = null;
      note.hidden = true;
    };

    // `first`: the page's first draw. A page that opens while hidden shows no note.
    function apply(view, { first = false } = {}) {
      // Never hidden while unlocked: the layout editor takes clicks, so it must be seen.
      const next = Boolean(view && view.all_hidden && !view.unlocked);
      doc.body.classList.toggle("all-hidden", next);
      if (next === hidden && !first) return;
      hidden = next;
      dropNote();
      if (!next || first) return;
      note.textContent = view.hide_key_taken
        ? I18n.t("ov.hidden.noteNoKey")
        : I18n.t("ov.hidden.note", { key: I18n.t("ov.hidden.key") });
      note.hidden = false;
      timer = clock.setTimeout(dropNote, NOTE_MS);
    }

    return { apply };
  }

  const api = { create, NOTE_MS };
  if (typeof module !== "undefined" && module.exports) module.exports = api;
  else window.TLHideAll = api;
})();

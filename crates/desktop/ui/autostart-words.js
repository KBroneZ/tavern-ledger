// What the "Start with Windows" switch shows for the app's answer
// (src/startup.rs): checked or not, whether the player can flip it, and the
// note under it. A start-up task the player turned off in Task Manager, or a
// policy, cannot be flipped from here, and the note says why. Pure: plain data
// in, plain data out, tested without a page (ui-tests/autostart-words.test.js).
"use strict";

(() => {
  const I18n = typeof module !== "undefined" && module.exports ? require("./i18n.js") : window.TLI18n;

  // `view` is {available, enabled, state, blocked}; `blocked` is the app's own
  // sentence (translated by I18n.backend), or null when the switch works.
  function show(view, t = I18n.t, backend = I18n.backend) {
    const available = Boolean(view.available);
    const blocked = typeof view.blocked === "string" && view.blocked ? view.blocked : null;
    let note = "";
    if (!available) note = t("settings.autostart.unavailable");
    else if (blocked) note = backend(blocked);
    return { checked: Boolean(view.enabled), disabled: !available || blocked !== null, note };
  }

  const api = { show };
  if (typeof module !== "undefined" && module.exports) module.exports = api;
  else window.TLAutostartWords = api;
})();

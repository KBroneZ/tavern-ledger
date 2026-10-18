// What the updates setting and the main window's notice say (T-105b), from
// the app's update view (src/updates.rs). Pure: plain data in, plain words
// out, tested without a page (ui-tests/update-words.test.js).
"use strict";

(() => {
  const I18n = typeof module !== "undefined" && module.exports ? require("./i18n.js") : window.TLI18n;
  const { t } = I18n;

  const REASONS = ["network", "not_found", "signature", "invalid", "install"];

  const when = (seconds) => (Number.isInteger(seconds) ? I18n.dateTime(new Date(seconds * 1000)) : "—");

  // The status line under the switch.
  function line(view) {
    const time = when(view.checked_at);
    switch (view.state) {
      case "off": return t("updates.off");
      case "checking": return t("updates.checking");
      case "downloading": return t("updates.downloading", { version: view.version });
      case "ready": return t("updates.ready", { version: view.version });
      case "latest": return t("updates.latest", { time });
      case "installing": return t("updates.installing", { version: view.version });
      case "failed": return t(`updates.failed.${REASONS.includes(view.reason) ? view.reason : "invalid"}`, { time });
      default: return t("updates.idle");
    }
  }

  // The main window's notice, or null when there is nothing to say there: a
  // ready version, or an installer that did not start after "Restart now".
  function notice(view) {
    if (view.notice) return t("updates.ready", { version: view.notice });
    if (view.state === "failed" && view.reason === "install") return t("updates.failed.install");
    return null;
  }

  // Whether "Check now" can do anything right now.
  const canCheck = (view) => ["idle", "latest", "failed"].includes(view.state);

  // Whether the Updates setting and the notice exist at all: the Microsoft
  // Store edition says nothing about updates, the Store delivers them.
  const shown = (view) => view.state !== "store";

  const api = { line, notice, canCheck, shown };
  if (typeof module !== "undefined" && module.exports) module.exports = api;
  else window.TLUpdateWords = api;
})();

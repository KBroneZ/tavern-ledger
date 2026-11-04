// The switch for reading two values from the game (T-327, D-063) on the
// Settings page: on by default (D-149), with what it reads and its risk written
// above it. The app's answer is always shown as it is (game_memory_status):
// the switch, the reader's status and the own rating read, if any. Own scope.
"use strict";

(() => {
  const { invoke: call } = window.__TAURI__.core;
  const I18n = window.TLI18n;
  const { t } = I18n;
  const $ = (id) => document.getElementById(id);
  const box = $("memory-enabled");
  const STATUSES = ["off", "ok", "waiting", "unsupported_version", "no_lobby", "not_running", "no_access"];
  // The status is asked again while the page is open, so it follows the game.
  const REFRESH_MS = 5000;

  const value = (v) => (Number.isInteger(v) && v > 0 ? I18n.num(v) : t("settings.memory.rating.none"));

  function show(view) {
    box.checked = Boolean(view.enabled);
    box.disabled = false;
    const status = STATUSES.includes(view.status) ? view.status : "unsupported_version";
    $("memory-status").textContent = t(`settings.memory.status.${view.enabled ? status : "off"}`);
    // The rating line: the values when a read passed; "not supported" when
    // the reader does not know this game version (never a value then).
    const rating = view.rating || {};
    const line = $("memory-rating");
    let text = "";
    if (view.enabled && rating.status === "ok") {
      text = t("settings.memory.rating", { solo: value(rating.solo), duos: value(rating.duos) });
    } else if (view.enabled && rating.status === "unsupported_version") {
      text = t("settings.memory.rating.unsupported");
    }
    line.textContent = text;
    line.hidden = !text;
    $("memory-problem").textContent = view.warning ? I18n.backend(view.warning) : "";
  }

  function fail(error) {
    $("memory-problem").textContent = t("settings.memory.failed", { error: I18n.backend(String(error)) });
  }

  const refresh = () => call("game_memory_status").then(show).catch(fail);

  box.addEventListener("change", () => {
    box.disabled = true;
    call("game_memory_set_enabled", { enabled: box.checked })
      .then(show)
      .catch((error) => {
        fail(error);
        refresh();
      });
  });

  I18n.ready().then(() => {
    refresh();
    setInterval(() => {
      if (!document.hidden) refresh();
    }, REFRESH_MS);
  });
})();

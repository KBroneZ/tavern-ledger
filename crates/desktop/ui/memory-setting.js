// The switch for reading two values from the game (T-327, D-063) on the
// Settings page: off by default, with what it reads and its risk written
// above it. The app's answer is always shown as it is (game_memory_status):
// the switch, the reader's status and the own rating read, if any. Own scope.
"use strict";

(() => {
  const { invoke: call } = window.__TAURI__.core;
  const I18n = window.TLI18n;
  const { t } = I18n;
  const $ = (id) => document.getElementById(id);
  const box = $("memory-enabled");
  const STATUSES = ["off", "ok", "waiting", "unsupported_version", "not_running", "no_access"];
  // The status is asked again while the page is open, so it follows the game.
  const REFRESH_MS = 5000;

  const value = (v) => (Number.isInteger(v) && v > 0 ? I18n.num(v) : t("settings.memory.rating.none"));

  function show(view) {
    box.checked = Boolean(view.enabled);
    box.disabled = false;
    const status = STATUSES.includes(view.status) ? view.status : "unsupported_version";
    $("memory-status").textContent = t(`settings.memory.status.${view.enabled ? status : "off"}`);
    const rating = view.rating || {};
    const shown = view.enabled && rating.status === "ok";
    const line = $("memory-rating");
    line.textContent = shown ? t("settings.memory.rating", { solo: value(rating.solo), duos: value(rating.duos) }) : "";
    line.hidden = !shown;
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

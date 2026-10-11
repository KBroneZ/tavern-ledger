// The overlay on/off switch in the main window (T-308), the same setting as
// the tray's "Show overlay during a game" (saved in overlay.json); both show
// the real state after any change. The overlay theme (T-324) is on the same
// page, and the lobby panel's switch (T-338: a panel of the overlay, shown
// or hidden like the others, on by default) and the combat odds' (T-605,
// on by default). Own scope.
"use strict";

(() => {
  const { invoke: call } = window.__TAURI__.core;
  const { listen: onEvent } = window.__TAURI__.event;
  const { t, backend } = window.TLI18n;
  const box = document.getElementById("overlay-enabled");
  const lobby = document.getElementById("overlay-lobby");
  const odds = document.getElementById("overlay-odds");
  const arrange = document.getElementById("overlay-arrange");
  const theme = document.getElementById("overlay-theme");
  const problem = document.getElementById("overlay-switch-problem");
  // The overlay's Simple or Advanced (D-067), also in its own toolbar.
  const detail = document.getElementById("overlay-detail");

  // The themes by `theme.<id>`; one the tables do not know keeps the app's own label.
  function themeText(item) {
    const key = `theme.${item.id}`;
    return key in window.TLI18n.tables().en ? t(key) : item.label;
  }

  function showThemes(view) {
    const themes = Array.isArray(view.themes) ? view.themes : [];
    theme.replaceChildren(...themes.map((item) => {
      const option = document.createElement("option");
      option.value = item.id;
      option.textContent = themeText(item);
      option.selected = item.id === view.theme;
      return option;
    }));
    theme.disabled = false;
  }

  // The button says the state in words: "Arrange overlay" or "Lock overlay"
  // (win.overlay.arrange, win.overlay.lock).
  function show(view) {
    box.checked = Boolean(view.enabled);
    box.disabled = false;
    lobby.checked = !(Array.isArray(view.hidden) && view.hidden.includes("lobby"));
    lobby.disabled = false;
    odds.checked = view.odds !== false;
    odds.disabled = false;
    // The label says the state; a pressed state on top would say it twice, differently.
    arrange.dataset.unlocked = String(Boolean(view.unlocked));
    arrange.textContent = view.unlocked ? t("win.overlay.lock") : t("win.overlay.arrange");
    showThemes(view);
    if (detail) {
      detail.value = view.detail === "advanced" ? "advanced" : "simple";
      detail.disabled = false;
    }
    problem.hidden = true;
  }

  function fail(error) {
    problem.hidden = false;
    problem.textContent = t("win.overlay.failed", { error: backend(String(error)) });
  }

  async function refresh() {
    show(await call("overlay_settings"));
  }

  box.addEventListener("change", () => {
    box.disabled = true;
    call("overlay_set_enabled", { enabled: box.checked })
      .then(show)
      .catch((error) => {
        fail(error);
        refresh().catch(fail);
      });
  });

  if (detail) {
    detail.addEventListener("change", () => {
      detail.disabled = true;
      call("overlay_set_detail", { detail: detail.value })
        .then(show)
        .catch((error) => {
          fail(error);
          refresh().catch(fail);
        });
    });
  }

  lobby.addEventListener("change", () => {
    lobby.disabled = true;
    call("overlay_set_shown", { panel: "lobby", shown: lobby.checked })
      .then(show)
      .catch((error) => {
        fail(error);
        refresh().catch(fail);
      });
  });

  odds.addEventListener("change", () => {
    odds.disabled = true;
    call("overlay_set_odds", { shown: odds.checked })
      .then(show)
      .catch((error) => {
        fail(error);
        refresh().catch(fail);
      });
  });

  arrange.addEventListener("click", () => {
    const unlocked = arrange.dataset.unlocked === "true";
    call("overlay_set_unlocked", { unlocked: !unlocked }).then(show).catch(fail);
  });

  theme.addEventListener("change", () => {
    theme.disabled = true;
    call("overlay_set_theme", { theme: theme.value })
      .then(show)
      .catch((error) => {
        fail(error);
        refresh().catch(fail);
      });
  });

  // The words are drawn once the language is known, so the first draw is right.
  window.TLI18n.ready()
    .then(() => onEvent("overlay-settings-changed", () => refresh().catch(fail)))
    .then(refresh)
    .catch(fail);
})();

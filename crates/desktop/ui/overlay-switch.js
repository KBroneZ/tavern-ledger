// The overlay on/off switch in the main window (T-308), the same setting as
// the tray's "Show overlay during a game" (saved in overlay.json); both show
// the real state after any change. Own scope.
"use strict";

(() => {
  const { invoke: call } = window.__TAURI__.core;
  const { listen: onEvent } = window.__TAURI__.event;
  const box = document.getElementById("overlay-enabled");
  const arrange = document.getElementById("overlay-arrange");
  const problem = document.getElementById("overlay-switch-problem");

  function show(view) {
    box.checked = Boolean(view.enabled);
    box.disabled = false;
    arrange.setAttribute("aria-pressed", String(Boolean(view.unlocked)));
    arrange.textContent = view.unlocked ? "Lock overlay" : "Arrange overlay";
    problem.hidden = true;
  }

  function fail(error) {
    problem.hidden = false;
    problem.textContent = `The overlay setting could not be read or changed (${error}).`;
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

  arrange.addEventListener("click", () => {
    const unlocked = arrange.getAttribute("aria-pressed") === "true";
    call("overlay_set_unlocked", { unlocked: !unlocked }).then(show).catch(fail);
  });

  onEvent("overlay-settings-changed", () => refresh().catch(fail))
    .then(refresh)
    .catch(fail);
})();

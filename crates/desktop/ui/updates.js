// The updates setting and the main window's update notice (T-105b). The app
// checks, downloads and verifies on its own (src/updates.rs); this page only
// shows where that stands and passes on the user's choices. Installing asks
// the app, which refuses during a game; the app never restarts by itself.
"use strict";

(() => {
  const { invoke: call } = window.__TAURI__.core;
  const { listen: onEvent } = window.__TAURI__.event;
  const I18n = window.TLI18n;
  const { t } = I18n;
  const Words = window.TLUpdateWords;
  const $ = (id) => document.getElementById(id);

  // Live regions: they stay in the page, empty when there is nothing to say.
  function problem(text) {
    $("update-problem").textContent = text || "";
    $("update-notice-problem").textContent = text || "";
  }

  const failed = (error) => problem(t("updates.problem", { error: I18n.backend(String(error)) }));

  function render(view) {
    const off = view.state === "off";
    $("update-current").textContent = t("updates.current", { version: view.current });
    const auto = $("update-auto");
    auto.checked = Boolean(view.auto_check);
    auto.disabled = off;
    $("update-check").disabled = off || !Words.canCheck(view);
    $("update-line").textContent = Words.line(view);
    $("update-warning").textContent = view.warning ? I18n.backend(view.warning) : "";
    const notice = Words.notice(view);
    $("update-notice").hidden = !notice;
    $("update-notice-text").textContent = notice || "";
    $("update-restart").disabled = view.state !== "ready";
  }

  $("update-auto").addEventListener("change", (e) => {
    problem("");
    call("update_set_auto", { enabled: e.target.checked }).then(render, failed);
  });
  $("update-check").addEventListener("click", () => {
    problem("");
    call("update_check_now").then(render, failed);
  });
  $("update-restart").addEventListener("click", () => {
    problem("");
    call("update_install").catch(failed);
  });
  $("update-later").addEventListener("click", () => {
    problem("");
    call("update_later").then(render, failed);
  });

  I18n.ready()
    .then(async () => {
      await onEvent("update-changed", (e) => render(e.payload));
      render(await call("update_status"));
    })
    .catch(failed);
})();

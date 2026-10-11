// Simple or Advanced for the main window (T-344, D-072), the same idea as
// the overlay's switch (D-067) but its own setting (settings.json): Simple
// shows the key numbers (the totals, the ledger, the recap's facts, health,
// shop summary, record and messages, the heroes and the shop stats) and
// leaves the rest (turn by turn, hero picks, trinkets and Dark Gifts, the
// tribe lists, the lobby tribes you enter) to Advanced; nothing is lost,
// one click away. Elements marked `adv-only` are hidden in Simple
// (style.css). The switch is in the view bar, the recap bar and Settings.
"use strict";

(() => {
  const DETAILS = ["simple", "advanced"];
  // Anything else (an older app, a broken answer) is Simple, the default.
  const normal = (detail) => (DETAILS.includes(detail) ? detail : "simple");

  // Shows `detail` on the page: the body's mark, the pressed button of each
  // switch and the Settings list.
  function apply(doc, detail) {
    const d = normal(detail);
    doc.body.dataset.detail = d;
    doc.querySelectorAll("[data-detail-pick]").forEach((b) => b.setAttribute("aria-pressed", String(b.dataset.detailPick === d)));
    const select = doc.getElementById("setting-window-detail");
    if (select) select.value = d;
    return d;
  }

  const api = { DETAILS, normal, apply };
  if (typeof module !== "undefined" && module.exports) {
    module.exports = api;
    return;
  }
  window.TLDetail = api;

  const { invoke: call } = window.__TAURI__.core;
  const { listen: onEvent } = window.__TAURI__.event;
  const problem = document.getElementById("window-detail-problem");
  let current = apply(document, "simple");

  function say(text) {
    if (problem) problem.textContent = text || "";
  }

  function choose(detail) {
    const d = normal(detail);
    if (d === current) return;
    current = apply(document, d);
    say("");
    call("app_set_window_detail", { detail: d }).catch((error) => {
      // It holds for this run; the words say it will not survive a restart.
      say(window.TLI18n.t("settings.detail.failed", { error: window.TLI18n.backend(String(error)) }));
    });
  }

  document.querySelectorAll("[data-detail-pick]").forEach((b) => b.addEventListener("click", () => choose(b.dataset.detailPick)));
  const select = document.getElementById("setting-window-detail");
  if (select) select.addEventListener("change", (e) => choose(e.target.value));

  window.TLI18n.ready().then(async () => {
    const view = await call("app_settings").catch(() => null);
    current = apply(document, view && view.window_detail);
    await onEvent("app-settings-changed", (e) => {
      current = apply(document, e.payload && e.payload.window_detail);
    });
  });
})();

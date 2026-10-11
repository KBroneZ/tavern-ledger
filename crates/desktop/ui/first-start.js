// The first-start question (T-531, D-106): asked once, while no overlay
// choice exists, whether the player wants the in-game overlay or only the
// stats. The answer is the same setting as the tray's check and the switch
// in Settings (overlay.json), so it can be changed there later. Closing the
// dialog without an answer leaves the overlay off and asks again at the next
// start. `ask` and `answer` take the backend call and the dialog, so
// ui-tests/first-start.test.js runs them without a page.
"use strict";

(() => {
  // Opens the dialog when the app says no overlay choice exists yet. When the
  // app cannot say, nothing is asked: the overlay stays as it is.
  async function ask(call, dialog) {
    const asks = await call("overlay_asks_first").catch(() => false);
    if (asks !== true) return false;
    if (!dialog.open) dialog.showModal();
    return true;
  }

  // Saves the answer and closes the dialog; null when done, else the error,
  // and the dialog stays open to try again.
  async function answer(call, dialog, enabled) {
    try {
      await call("overlay_set_enabled", { enabled });
    } catch (error) {
      return String(error);
    }
    dialog.close();
    return null;
  }

  function start(doc, call, t, backend) {
    const dialog = doc.getElementById("first-start");
    const problem = doc.getElementById("first-start-problem");
    if (!dialog) return;
    doc.querySelectorAll("[data-overlay-answer]").forEach((button) => {
      button.addEventListener("click", async () => {
        const failed = await answer(call, dialog, button.dataset.overlayAnswer === "on");
        problem.textContent = failed ? t("win.first.failed", { error: backend(failed) }) : "";
        problem.hidden = !failed;
      });
    });
    ask(call, dialog);
  }

  const api = { ask, answer, start };
  if (typeof module !== "undefined" && module.exports) module.exports = api;
  else {
    window.TLFirstStart = api;
    window.TLI18n.ready().then(() => {
      const { invoke } = window.__TAURI__.core;
      start(document, invoke, window.TLI18n.t, window.TLI18n.backend);
    });
  }
})();

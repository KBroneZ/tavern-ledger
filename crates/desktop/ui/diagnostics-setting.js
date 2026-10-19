// The Settings button "Copy diagnostics" (T-626): the app builds a short text
// for a tester to send (versions, the Power lines of log.config, the app's
// known errors), the window shows it, and only after the player has seen it,
// "Copy" puts exactly that text on the clipboard. The window never builds or
// edits the text: it asks the app (`diagnostics_preview`) and keeps what came
// back as one value, shown and copied unchanged. `start` takes the page, the
// backend call, the clipboard writer and the words, so
// ui-tests/diagnostics-setting.test.js runs it without a browser.
"use strict";

(() => {
  function start(doc, call, writeText, t, backend) {
    const open = doc.getElementById("diag-open");
    const problem = doc.getElementById("diag-problem");
    const dialog = doc.getElementById("diag-dialog");
    const pre = doc.getElementById("diag-text");
    const status = doc.getElementById("diag-status");
    const copy = doc.getElementById("diag-copy");
    const close = doc.getElementById("diag-close");
    if (![open, problem, dialog, pre, status, copy, close].every(Boolean)) return;

    // The text the app made, as it came: what is on screen and what is copied.
    let shown = null;
    // Bumped whenever the text changes or goes away, so a copy that finishes
    // late cannot say "Copied" about a text that is no longer the one shown.
    let generation = 0;
    let asking = false;

    const reset = () => {
      generation += 1;
      shown = null;
      pre.textContent = "";
      status.textContent = "";
      copy.setAttribute("aria-disabled", "true");
    };
    reset();

    open.addEventListener("click", async () => {
      if (asking) return;
      asking = true;
      problem.textContent = "";
      try {
        const text = await call("diagnostics_preview");
        if (typeof text !== "string" || !text) throw new Error(t("settings.diag.empty"));
        reset();
        shown = text;
        pre.textContent = text;
        copy.removeAttribute("aria-disabled");
        dialog.showModal();
      } catch (error) {
        reset();
        const said = error instanceof Error ? error.message : backend(String(error));
        problem.textContent = t("settings.diag.failed", { error: said });
      } finally {
        asking = false;
      }
    });

    // The button stays enabled (it keeps the keyboard focus); with no text it does nothing.
    copy.addEventListener("click", async () => {
      if (shown === null) return;
      const text = shown;
      const mine = generation;
      let said;
      try {
        await writeText(text);
        said = t("settings.diag.copied");
      } catch (_) {
        said = t("settings.diag.copyFailed");
      }
      if (mine === generation) status.textContent = said;
    });

    close.addEventListener("click", () => dialog.close());
    // Closing, by the button or Escape: forget the text and give the focus back.
    dialog.addEventListener("close", () => {
      reset();
      open.focus();
    });
  }

  // The page's clipboard: the async API, and when it refuses (a window where
  // it is not allowed) a hidden textarea and the old command. The textarea goes
  // inside `into` (the open dialog: the rest of the page is inert while it is
  // modal) and the focus goes back where it was.
  function pageClipboard(doc, nav, into) {
    return async (text) => {
      if (nav.clipboard && nav.clipboard.writeText) {
        try {
          await nav.clipboard.writeText(text);
          return;
        } catch (_) {
          // fall through to the old command
        }
      }
      const before = doc.activeElement;
      const box = doc.createElement("textarea");
      box.value = text;
      box.setAttribute("readonly", "");
      box.style.position = "fixed";
      box.style.opacity = "0";
      (into || doc.body).append(box);
      box.select();
      let done = false;
      try {
        done = doc.execCommand("copy");
      } finally {
        box.remove();
        if (before && before.focus) before.focus();
      }
      if (!done) throw new Error("copy refused");
    };
  }

  const api = { start, pageClipboard };
  if (typeof module !== "undefined" && module.exports) module.exports = api;
  else {
    window.TLDiagnostics = api;
    window.TLI18n.ready().then(() => {
      start(document, window.__TAURI__.core.invoke, pageClipboard(document, navigator, document.getElementById("diag-dialog")), window.TLI18n.t, window.TLI18n.backend);
    });
  }
})();

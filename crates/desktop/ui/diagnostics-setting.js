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

    const reset = () => {
      shown = null;
      pre.textContent = "";
      status.textContent = "";
      copy.disabled = true;
    };

    open.addEventListener("click", async () => {
      open.disabled = true;
      problem.textContent = "";
      try {
        const text = await call("diagnostics_preview");
        if (typeof text !== "string" || !text) throw new Error("empty");
        shown = text;
        pre.textContent = text;
        status.textContent = "";
        copy.disabled = false;
        dialog.showModal();
      } catch (error) {
        reset();
        problem.textContent = t("settings.diag.failed", { error: backend(String(error)) });
      } finally {
        open.disabled = false;
      }
    });

    copy.addEventListener("click", async () => {
      if (shown === null) return;
      copy.disabled = true;
      try {
        await writeText(shown);
        status.textContent = t("settings.diag.copied");
      } catch (_) {
        status.textContent = t("settings.diag.copyFailed");
      } finally {
        copy.disabled = shown === null;
      }
    });

    close.addEventListener("click", () => {
      dialog.close();
      reset();
    });
    // Escape closes the dialog without the button.
    dialog.addEventListener("close", reset);
  }

  // The page's clipboard: the async API, else a hidden textarea and the old
  // command (a window where the async one is not allowed).
  function pageClipboard(doc, nav) {
    return async (text) => {
      if (nav.clipboard && nav.clipboard.writeText) return nav.clipboard.writeText(text);
      const box = doc.createElement("textarea");
      box.value = text;
      box.setAttribute("readonly", "");
      box.style.position = "fixed";
      box.style.opacity = "0";
      doc.body.append(box);
      box.select();
      const done = doc.execCommand("copy");
      box.remove();
      if (!done) throw new Error("copy refused");
      return undefined;
    };
  }

  const api = { start, pageClipboard };
  if (typeof module !== "undefined" && module.exports) module.exports = api;
  else {
    window.TLDiagnostics = api;
    window.TLI18n.ready().then(() => {
      start(document, window.__TAURI__.core.invoke, pageClipboard(document, navigator), window.TLI18n.t, window.TLI18n.backend);
    });
  }
})();

// The Settings line "Error log" (T-580): "Open log folder" asks the app to show
// the folder in the file browser. The window never sends a path; the app knows
// where its own log is. `start` takes the page, the backend call and the
// words, so ui-tests/log-setting.test.js runs it without a browser.
"use strict";

(() => {
  function start(doc, call, t, backend) {
    const button = doc.getElementById("log-open");
    const problem = doc.getElementById("log-problem");
    if (!button || !problem) return;
    button.addEventListener("click", async () => {
      button.disabled = true;
      problem.textContent = "";
      try {
        await call("open_log_folder");
      } catch (error) {
        problem.textContent = t("settings.log.failed", { error: backend(String(error)) });
      } finally {
        button.disabled = false;
      }
    });
  }

  const api = { start };
  if (typeof module !== "undefined" && module.exports) module.exports = api;
  else {
    window.TLLogSetting = api;
    window.TLI18n.ready().then(() => {
      start(document, window.__TAURI__.core.invoke, window.TLI18n.t, window.TLI18n.backend);
    });
  }
})();

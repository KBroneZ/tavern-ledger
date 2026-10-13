// Errors the page itself raises (a script error, a promise nobody handled) go
// to the app's error log (T-580). Both windows load this first. The app does
// the rest: it trims, redacts and rate-limits what arrives, and it alone
// writes the file. Sending can never raise an error of its own.
"use strict";

(() => {
  const MAX_MESSAGE = 300;
  const MAX_PLACE = 120;

  // By characters, not UTF-16 units: a cut must not split a surrogate pair.
  const clip = (text, max) => Array.from(String(text)).slice(0, max).join("");

  // "app.js:12:5" from a script's address: the file name, not the full URL.
  function placeOf(filename, line, column) {
    if (!filename) return null;
    const file = String(filename).split(/[?#]/)[0].split("/").pop() || "script";
    return clip(`${file}:${line || 0}:${column || 0}`, MAX_PLACE);
  }

  function reasonText(reason) {
    if (reason && typeof reason === "object" && "message" in reason) return reason.message;
    return reason === undefined ? "unknown reason" : reason;
  }

  // `target` is the window; `call` is the app's invoke.
  function install(target, call) {
    const report = (kind, message, place) => {
      try {
        const sent = call("log_ui_error", { kind, message: clip(message, MAX_MESSAGE), place });
        if (sent && typeof sent.catch === "function") sent.catch(() => {});
      } catch (_) {
        // Nothing to do: the log is a help, never a reason to fail again.
      }
    };
    target.addEventListener("error", (e) => {
      report("error", e.message || (e.error && e.error.message) || "error", placeOf(e.filename, e.lineno, e.colno));
    });
    target.addEventListener("unhandledrejection", (e) => {
      report("rejection", reasonText(e.reason), null);
    });
  }

  const api = { install, placeOf };
  if (typeof module !== "undefined" && module.exports) module.exports = api;
  else if (window.__TAURI__ && window.__TAURI__.core) install(window, window.__TAURI__.core.invoke);
})();

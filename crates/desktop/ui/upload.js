// Upload panel (T-104d): sign-in state, the "Upload games" switch (off until
// the user turns it on), what is waiting and the last result. It only renders
// what the app sends ("upload-changed") and asks the app to act; values are
// set with textContent only.
"use strict";

(() => {
  const { invoke } = window.__TAURI__.core;
  const { listen } = window.__TAURI__.event;
  const $ = (id) => document.getElementById(id);

  const plural = (n, one, many) => `${n} ${n === 1 ? one : many}`;

  function queueText(c) {
    if (!c) return "";
    const parts = [
      `${plural(c.waiting, "game", "games")} waiting to upload`,
      c.uploaded && `${c.uploaded} uploaded`,
      c.rejected && `${c.rejected} refused by the server`,
      c.not_uploadable && `${c.not_uploadable} stay on this PC (mode not uploaded)`,
      c.dev_reconnect && `${c.dev_reconnect} stay on this PC (played with the reconnect dev tool)`,
    ].filter(Boolean);
    return parts.join(" · ");
  }

  function when(secs) {
    if (!Number.isInteger(secs)) return "";
    const d = new Date(secs * 1000);
    return ` (${d.toLocaleDateString()} ${d.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" })})`;
  }

  function summary(v) {
    if (!v.running) return "not running";
    if (!v.available) return "not available yet";
    if (!v.signed_in) return v.signing_in ? "signing in…" : "signed out";
    if (!v.enabled) return "off";
    if (v.problem) return "paused";
    return v.counts && v.counts.waiting ? `${v.counts.waiting} waiting` : "on, up to date";
  }

  function show(id, text) {
    const node = $(id);
    node.textContent = text || "";
    node.hidden = !text;
  }

  function render(v) {
    $("upload-summary").textContent = summary(v);
    const unavailable = !v.running
      ? "Upload is not running in this window: it starts once the app follows your games (if another Tavern Ledger is open, use that one)."
      : !v.available ? `Upload is not available: ${v.unavailable_reason || "no server"}.` : "";
    show("upload-unavailable", unavailable);
    const usable = v.running && v.available;
    $("upload-signed-out").hidden = !usable || v.signed_in;
    $("upload-signed-in").hidden = !usable || !v.signed_in;
    $("upload-sign-in").hidden = Boolean(v.signing_in);
    show("upload-waiting-text", v.signing_in
      ? (v.link_sent
        ? `If ${v.signing_in} has an account, a sign-in link is on its way. Open it on this PC.`
        : `Asking for a sign-in link for ${v.signing_in}…`)
      : "");
    $("upload-waiting").hidden = !v.signing_in;
    $("upload-email-shown").textContent = v.email || "your account";
    const toggle = $("upload-enabled");
    toggle.checked = Boolean(v.enabled);
    toggle.disabled = !v.signed_in;
    show("upload-queue", usable ? queueText(v.counts) : "");
    show("upload-last", v.last_result ? `Last: ${v.last_result}${when(v.last_at)}` : "");
    show("upload-problem", v.problem || "");
    show("upload-notice", v.notice || "");
  }

  const report = (err) => show("upload-notice", String(err));

  $("upload-sign-in").addEventListener("submit", (e) => {
    e.preventDefault();
    invoke("upload_sign_in", { email: $("upload-email").value }).catch(report);
  });
  $("upload-cancel").addEventListener("click", () => invoke("upload_cancel_sign_in").catch(report));
  $("upload-sign-out").addEventListener("click", () => invoke("upload_sign_out").catch(report));
  $("upload-enabled").addEventListener("change", (e) => {
    invoke("upload_set_enabled", { on: e.target.checked }).catch(report);
  });

  (async () => {
    await listen("upload-changed", (e) => render(e.payload));
    render(await invoke("upload_status"));
  })().catch(report);
})();

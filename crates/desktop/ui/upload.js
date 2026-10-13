// Upload panel (T-104d): sign-in state, the "Upload games" switch (off until
// the user turns it on), what is waiting and the last result. It only renders
// what the app sends ("upload-changed") and asks the app to act; values are
// set with textContent only.
"use strict";

(() => {
  const { invoke } = window.__TAURI__.core;
  const { listen } = window.__TAURI__.event;
  const { t, plural, backend, num, dateTime } = window.TLI18n;
  const $ = (id) => document.getElementById(id);

  function queueText(c) {
    if (!c) return "";
    const parts = [
      plural("win.upload.waiting", c.waiting),
      c.uploaded && t("win.upload.uploaded", { n: num(c.uploaded) }),
      c.rejected && t("win.upload.rejected", { n: num(c.rejected) }),
      c.not_uploadable && t("win.upload.notUploadable", { n: num(c.not_uploadable) }),
      c.dev_reconnect && t("win.upload.devReconnect", { n: num(c.dev_reconnect) }),
      c.held_dev_file && t("win.upload.heldDev", { n: num(c.held_dev_file) }),
    ].filter(Boolean);
    return parts.join(" · ");
  }

  function when(secs) {
    if (!Number.isInteger(secs)) return "";
    return ` (${dateTime(new Date(secs * 1000))})`;
  }

  function summary(v) {
    if (!v.running) return t("win.upload.sum.notRunning");
    if (!v.available) return t("win.upload.sum.notAvailable");
    if (!v.signed_in) return v.signing_in ? t("win.upload.sum.signingIn") : t("win.upload.sum.signedOut");
    if (!v.enabled) return t("win.upload.sum.off");
    if (v.problem) return t("win.upload.sum.paused");
    return v.counts && v.counts.waiting ? t("win.upload.sum.waiting", { n: num(v.counts.waiting) }) : t("win.upload.sum.upToDate");
  }

  function show(id, text) {
    const node = $(id);
    node.textContent = text || "";
    node.hidden = !text;
  }

  function render(v) {
    $("upload-summary").textContent = summary(v);
    const unavailable = !v.running
      ? t("win.upload.notRunning")
      : !v.available ? t("win.upload.unavailable", { reason: backend(v.unavailable_reason) || t("win.upload.noServer") }) : "";
    show("upload-unavailable", unavailable);
    show("upload-server-warning", v.running ? backend(v.server_warning || "") : "");
    const usable = v.running && v.available;
    $("upload-signed-out").hidden = !usable || v.signed_in;
    $("upload-signed-in").hidden = !usable || !v.signed_in;
    $("upload-sign-in").hidden = Boolean(v.signing_in);
    show("upload-waiting-text", v.signing_in
      ? (v.link_sent
        ? t("win.upload.linkSent", { email: v.signing_in })
        : t("win.upload.linkAsking", { email: v.signing_in }))
      : "");
    $("upload-waiting").hidden = !v.signing_in;
    $("upload-email-shown").textContent = v.email || t("win.upload.yourAccount");
    const toggle = $("upload-enabled");
    toggle.checked = Boolean(v.enabled);
    toggle.disabled = !v.signed_in;
    show("upload-queue", usable ? queueText(v.counts) : "");
    show("upload-last", v.last_result ? t("win.upload.last", { result: backend(v.last_result), when: when(v.last_at) }) : "");
    show("upload-problem", backend(v.problem || ""));
    show("upload-notice", backend(v.notice || ""));
  }

  const report = (err) => show("upload-notice", backend(String(err)));

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
    await window.TLI18n.ready();
    await listen("upload-changed", (e) => render(e.payload));
    render(await invoke("upload_status"));
  })().catch(report);
})();

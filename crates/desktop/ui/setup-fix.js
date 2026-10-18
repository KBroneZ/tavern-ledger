// The setup box's Fix button (T-376, D-154): what the box says about the two
// logging config files, the button that creates or completes them on a click,
// and the result of the last click. The window never sends a path, "Fix" only asks
// the app, which works out the two files itself. `render` and `start` take the
// page, the backend call and the words, so ui-tests/setup-fix.test.js runs
// them without a browser.
"use strict";

(() => {
  // The words for each state of the box; `ok` and `unknown` say nothing.
  const STATE_KEYS = {
    needs_fix: "setup.state.needsFix",
    cannot_check: "setup.state.cannotCheck",
    fixing: "setup.state.fixing",
    error: "setup.state.error",
    manual: "setup.state.manual",
  };
  const REASONS = [
    "link", "not_a_file", "not_plain_text", "folder_missing", "no_game_exe", "no_path", "access_denied",
    "read_only", "path_too_long", "locked", "changed", "disk_full", "too_large", "too_many_backups", "virtualized", "other",
  ];
  const RESULT_IDS = { "log.config": "setup-result-log", "client.config": "setup-result-client" };

  const viewOf = (status) => (status && status.setup_view) || {};
  const worked = (file) => file.result === "fixed" || file.result === "created";
  const wentWrong = (file) => file.result === "failed" || file.result === "refused";

  // The files of the last click that the box tells about: what changed and
  // what did not work; a file that was already fine is not mentioned.
  function shownFiles(view) {
    const files = view.fix && Array.isArray(view.fix.files) ? view.fix.files : [];
    return files.filter((file) => worked(file) || wentWrong(file));
  }

  function resultLine(file, t) {
    if (wentWrong(file)) {
      const reason = REASONS.includes(file.reason) ? file.reason : "other";
      // "other" would read as a plural form in the table, so its key is "unknown".
      const key = reason === "other" ? "unknown" : reason;
      return t("setup.fix.not", { file: file.file, reason: t(`setup.fix.reason.${key}`) });
    }
    const done = t(file.result === "created" ? "setup.fix.created" : "setup.fix.fixed", { file: file.file });
    return file.backup ? `${done} ${t("setup.fix.backup", { file: file.file })}` : done;
  }

  // `closedId` is the id of the result the player closed, so it stays closed
  // until the next click makes a new one.
  function render(doc, status, t, closedId = null) {
    const s = status || {};
    const view = viewOf(s);
    const lines = Array.isArray(s.setup) ? s.setup : [];
    const set = (id, text) => {
      const node = doc.getElementById(id);
      if (node) node.textContent = text;
    };
    const show = (id, visible) => {
      const node = doc.getElementById(id);
      if (node) node.hidden = !visible;
    };
    const fixing = view.state === "fixing";
    const files = view.fix && view.fix.id !== closedId ? shownFiles(view) : [];
    // What went wrong stays on screen only while the problem does.
    set("setup-state", STATE_KEYS[view.state] ? t(STATE_KEYS[view.state]) : "");
    show("setup-state", Boolean(STATE_KEYS[view.state]));
    show("setup-fix", Boolean(view.can_fix));
    // When the needed lines are missing too, Fix comes first.
    show("setup-fix-optional", Boolean(view.can_fix_optional) && !view.can_fix);
    set("setup-fix", t(fixing ? "setup.fix.running" : "setup.fix"));
    set("setup-fix-optional", t("setup.fixOptional"));
    ["setup-fix", "setup-fix-optional"].forEach((id) => {
      const node = doc.getElementById(id);
      if (node) node.disabled = fixing;
    });
    Object.entries(RESULT_IDS).forEach(([name, id]) => {
      const file = files.find((f) => f.file === name);
      set(id, file ? resultLine(file, t) : "");
      show(id, Boolean(file));
    });
    const restart = files.some(worked)
      ? t(view.fix.game_running ? "setup.fix.restartNow" : "setup.fix.restartIfOpen")
      : "";
    set("setup-result-restart", restart);
    show("setup-result-restart", Boolean(restart));
    show("setup-result", files.length > 0);
    // The box shows while there is something to say: a line, or a result.
    show("setup", lines.length > 0 || files.length > 0);
  }

  function start(doc, call, listen, t, backend, win) {
    let closedId = null;
    let last = null;
    const draw = (status) => {
      last = status;
      render(doc, status, t, closedId);
    };
    const say = (text) => {
      const node = doc.getElementById("setup-fix-problem");
      if (node) node.textContent = text;
    };
    // Runs once at a time: the buttons are off while the app works, and the
    // app refuses a second click too.
    let running = false;
    async function fix(optional) {
      if (running) return;
      running = true;
      say("");
      ["setup-fix", "setup-fix-optional"].forEach((id) => {
        const node = doc.getElementById(id);
        if (node) node.disabled = true;
      });
      try {
        await call("setup_fix", { optional });
      } catch (error) {
        say(t("setup.fix.failed", { error: backend(String(error)) }));
      } finally {
        running = false;
        call("status").then(draw, () => {});
      }
    }
    const on = (id, fn) => {
      const node = doc.getElementById(id);
      if (node) node.addEventListener("click", fn);
    };
    on("setup-fix", () => fix(false));
    on("setup-fix-optional", () => fix(true));
    on("setup-result-close", () => {
      const view = viewOf(last);
      if (view.fix) closedId = view.fix.id;
      draw(last);
    });
    // The player may fix the files by hand in another window: look again
    // when this one comes to the front.
    if (win && win.addEventListener) {
      win.addEventListener("focus", () => {
        call("setup_recheck").catch(() => {});
      });
    }
    call("status").then(draw, () => {});
    return listen("status-changed", (e) => draw(e.payload));
  }

  const api = { render, start, resultLine, shownFiles };
  if (typeof module !== "undefined" && module.exports) module.exports = api;
  else {
    window.TLSetupFix = api;
    window.TLI18n.ready().then(() => {
      const { invoke } = window.__TAURI__.core;
      const { listen } = window.__TAURI__.event;
      start(document, invoke, listen, window.TLI18n.t, window.TLI18n.backend, window);
    });
  }
})();

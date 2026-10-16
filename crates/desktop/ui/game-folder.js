// The Hearthstone folder (T-579): which folder the app reads the game's logs
// from, shown on the Settings page and in the setup box, with "Choose…" and
// "Back to automatic", and the notice that the running game is somewhere
// else. The window never sends a path: the app opens the system's folder
// picker itself and takes the running game's folder from its own reading.
// `render` and `start` take the page, the backend call and the words, so
// ui-tests/game-folder.test.js runs them without a browser.
"use strict";

(() => {
  // How the folder was decided, as the app says it.
  const HOW_KEYS = {
    chosen: "folder.how.chosen",
    running: "folder.how.running",
    registry: "folder.how.registry",
    battlenet: "folder.how.battlenet",
    usual: "folder.how.usual",
    scan: "folder.how.scan",
  };

  const howText = (how, t) => (HOW_KEYS[how] ? t(HOW_KEYS[how]) : "");
  const hasFolder = (folder) => Boolean(folder && folder.path);

  // The Settings line: the folder and how it was decided.
  function currentLine(folder, t) {
    if (!hasFolder(folder)) return t("folder.none");
    return t("folder.inUse", { path: folder.path, how: howText(folder.how, t) });
  }

  // The setup box's line: the folder that was checked.
  function checkedLine(folder, t) {
    if (!hasFolder(folder)) return t("folder.checked.none");
    return t("folder.checked", { path: folder.path, how: howText(folder.how, t) });
  }

  function render(doc, status, t) {
    const s = status || {};
    const folder = s.folder || null;
    const set = (id, text) => {
      const node = doc.getElementById(id);
      if (node) node.textContent = text;
    };
    const show = (id, visible) => {
      const node = doc.getElementById(id);
      if (node) node.hidden = !visible;
    };
    set("folder-current", currentLine(folder, t));
    // Back to automatic is for a folder the player chose, also an unusable one.
    show("folder-reset", Boolean(folder && folder.chosen));
    set("setup-folder", checkedLine(folder, t));
    // A folder the app can use needs no "Choose": the files inside it are what the box is about.
    show("setup-choose", !(folder && folder.usable === true));
    const offer = typeof s.folder_offer === "string" && s.folder_offer ? s.folder_offer : "";
    set("folder-offer-text", offer ? t("folder.offer", { path: offer }) : "");
    show("folder-offer", Boolean(offer));
  }

  // Each button asks the app and shows an error where the player looks:
  // under the setting and in the setup box.
  function start(doc, call, listen, t, backend) {
    const problems = ["folder-problem", "setup-folder-problem"];
    const say = (text) =>
      problems.forEach((id) => {
        const node = doc.getElementById(id);
        if (node) node.textContent = text;
      });
    const buttons = ["folder-choose", "setup-choose", "folder-reset", "folder-offer-yes", "folder-offer-no"];
    const busy = (on) =>
      buttons.forEach((id) => {
        const node = doc.getElementById(id);
        if (node) node.disabled = on;
      });
    const refresh = () => call("status").then((s) => render(doc, s, t), () => {});

    // Runs one command; the new folder arrives with the status event.
    async function run(command) {
      busy(true);
      say("");
      try {
        await command();
      } catch (error) {
        say(t("folder.failed", { error: backend(String(error)) }));
      } finally {
        busy(false);
      }
      refresh();
    }
    const on = (id, fn) => {
      const node = doc.getElementById(id);
      if (node) node.addEventListener("click", fn);
    };
    const choose = () => run(() => call("game_folder_choose"));
    on("folder-choose", choose);
    on("setup-choose", choose);
    on("folder-reset", () => run(() => call("game_folder_reset")));
    on("folder-offer-yes", () => run(() => call("game_folder_use_running")));
    on("folder-offer-no", () => run(() => call("game_folder_decline_running")));
    // From the setup box to the setting.
    on("setup-settings", () => {
      const tab = doc.getElementById("tab-settings");
      if (tab) tab.click();
      const choose = doc.getElementById("folder-choose");
      if (choose) choose.focus();
    });
    refresh();
    return listen("status-changed", (e) => render(doc, e.payload, t));
  }

  const api = { render, start, currentLine, checkedLine, howText };
  if (typeof module !== "undefined" && module.exports) module.exports = api;
  else {
    window.TLGameFolder = api;
    window.TLI18n.ready().then(() => {
      const { invoke } = window.__TAURI__.core;
      const { listen } = window.__TAURI__.event;
      start(document, invoke, listen, window.TLI18n.t, window.TLI18n.backend);
    });
  }
})();

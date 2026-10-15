// Stand-in for the app's bridge (window.__TAURI__) so the real overlay page
// runs in a plain browser: every command and event the page uses is answered
// from one scenario of example data. Settings changes are no-ops (the frame
// stays as the scenario says); card art becomes a plain placeholder. The
// game's minion textures are the one thing fetched: from the public host the
// app uses (the set's manifest, layout and images), so a minion looks as it
// does in the app; with no network the page draws plain card art, as the app
// does. A scenario's `textures` says "none" (no set), or { bad: [names] }
// (those textures do not load), and `noWebgl` turns WebGL off.
// ui-tests/overlay-gallery.test.js checks that every name the page's scripts
// use is listed here.
"use strict";

(() => {
  const themes = typeof module !== "undefined" && module.exports ? require("./theme-vars.js") : window.TLGalleryThemes;

  // The game window the frames show: the overlay covers its picture.
  const SCREEN = [1920, 1080];

  // Where the game's own leaderboard is in a 1920x1080 window (the app's
  // measured default, leaderboard.rs): the opponent marks follow it.
  const LEADERBOARD = { area: { x: 241.8, y: 172.8, w: 91.8, h: 734.4 }, custom: false };

  // The view the app's overlay_settings gives (overlay.rs `View`).
  function settingsView(scenario) {
    const s = (scenario.state && scenario.state.settings) || {};
    const theme = s.theme === "contrast" ? "contrast" : "glass";
    return {
      enabled: true,
      unlocked: false,
      theme,
      themes: themes.THEMES,
      opacity: 0.9,
      min_opacity: 0.8,
      max_opacity: 1,
      hidden: s.hidden || [],
      // The odds bar's default place (overlay_layout.rs): top centre.
      layout: { panels: { odds: { x: 690, y: 10.8, w: 540, h: null } } },
      screen: SCREEN,
      leaderboard: LEADERBOARD,
      vars: themes.THEME_VARS[theme],
      warning: null,
      odds: true,
    };
  }

  // Commands that only change something in the app: the answer is the view.
  const SETTERS = [
    "overlay_save_layout",
    "overlay_set_theme",
    "overlay_set_opacity",
    "overlay_set_shown",
    "overlay_set_odds",
    "overlay_set_unlocked",
    "overlay_set_enabled",
    "overlay_reset_layout",
    "overlay_save_leaderboard",
    "overlay_reset_leaderboard",
  ];

  const READS = ["overlay_state", "overlay_scene", "overlay_lobby", "overlay_settings"];
  const CARDS = ["card_names", "card_data_status", "texture_set"];
  const APP = ["app_settings", "app_set_system_language", "log_ui_error"];
  const COMMANDS = [...READS, ...SETTERS, ...CARDS, ...APP];
  const EVENTS = [
    "live-changed",
    "lobby-tribes-changed",
    "cards-changed",
    "textures-changed",
    "overlay-settings-changed",
    "overlay-hover",
    "app-settings-changed",
  ];

  // The game's texture set as the app's texture_set answers it, read from the
  // public host once per page. `files` maps a role to its file; `base` is
  // where they are (the app's own protocol is not there in a browser).
  const TEXTURES_BASE = "https://assets.tavernledger.net/v2/";
  let liveSet = null;
  const getJson = (url) => fetch(url).then((r) => (r.ok ? r.json() : Promise.reject(new Error(String(r.status)))));
  function fetchSet(fetchJson) {
    if (!liveSet) {
      liveSet = Promise.all([fetchJson(`${TEXTURES_BASE}manifest.json`), fetchJson(`${TEXTURES_BASE}layout.json`)])
        .then(([manifest, layout]) => ({
          version: manifest.version,
          layout,
          base: TEXTURES_BASE,
          files: Object.fromEntries(manifest.files.filter((f) => f.name !== "layout").map((f) => [f.name, f.file])),
        }))
        .catch(() => null);
    }
    return liveSet;
  }
  async function textureSet(spec, fetchJson) {
    if (spec === "none") return null;
    const set = await fetchSet(fetchJson);
    const bad = spec && Array.isArray(spec.bad) ? spec.bad : [];
    if (!set || !bad.length) return set;
    return { ...set, files: { ...set.files, ...Object.fromEntries(bad.map((name) => [name, `missing-${name}.webp`])) } };
  }

  // The commands, each as a function of (scenario, language, args).
  function handlers(scenario, lang, fetchJson = getJson) {
    const state = scenario.state;
    const view = () => settingsView(scenario);
    const table = {
      overlay_state: () => state.game || null,
      overlay_scene: () => state.scene || null,
      overlay_lobby: () => state.lobby || null,
      overlay_settings: view,
      // Every id the scenario names has a name; any other is "not in the card data".
      card_names: (args) => {
        const out = {};
        for (const id of (args && args.ids) || []) {
          const names = state.names && state.names[id];
          out[id] = names ? names[lang] : null;
        }
        return out;
      },
      card_data_status: () => ({ ready: true }),
      texture_set: () => textureSet(state.textures, fetchJson),
      app_settings: () => ({ language: lang }),
      app_set_system_language: () => null,
      log_ui_error: () => null,
    };
    for (const name of SETTERS) table[name] = view;
    return table;
  }

  // A bridge for one scenario: { core: { invoke }, event: { listen }, emit }.
  // `gate` holds every answer back until it resolves: in the app an answer
  // takes a moment, so the page's scripts are all loaded before the first one
  // comes; in a single page of inline scripts it would come in between.
  function createTauri(scenario, lang, gate = Promise.resolve(), fetchJson = getJson) {
    const table = handlers(scenario, lang, fetchJson);
    const listeners = new Map();
    const invoke = async (command, args) => {
      await gate;
      if (!Object.prototype.hasOwnProperty.call(table, command)) {
        throw new Error(`the gallery does not answer ${command}`);
      }
      return table[command](args);
    };
    const emit = (name, payload) => {
      for (const callback of listeners.get(name) || []) callback({ event: name, payload });
    };
    const listen = async (name, callback) => {
      if (!EVENTS.includes(name)) throw new Error(`the gallery does not know the event ${name}`);
      listeners.set(name, [...(listeners.get(name) || []), callback]);
      // The pointer is on a hero from the start: say so once the page listens.
      if (name === "overlay-hover" && scenario.state.hover) {
        setTimeout(() => callback({ event: name, payload: scenario.state.hover }), 0);
      }
      return () => listeners.set(name, (listeners.get(name) || []).filter((f) => f !== callback));
    };
    return { core: { invoke }, event: { listen }, emit };
  }

  // A plain shape for a card's picture, a colour from its id: no network.
  function placeholder(id) {
    let hash = 0;
    for (const ch of id) hash = (hash * 31 + ch.charCodeAt(0)) % 360;
    const svg =
      `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 64 64"><rect width="64" height="64" rx="10" fill="hsl(${hash} 28% 34%)"/>` +
      `<circle cx="32" cy="26" r="11" fill="hsl(${hash} 30% 56%)"/><rect x="14" y="42" width="36" height="14" rx="7" fill="hsl(${hash} 30% 46%)"/></svg>`;
    return `data:image/svg+xml;charset=utf-8,${encodeURIComponent(svg)}`;
  }

  // In the browser: defines window.__TAURI__ and keeps card art off the network.
  function install(win, scenario, lang) {
    const loaded = new Promise((resolve) => {
      if (win.document.readyState !== "loading") resolve();
      else win.document.addEventListener("DOMContentLoaded", () => resolve());
    });
    win.__TAURI__ = createTauri(scenario, lang, loaded);
    if (scenario.state.noWebgl) {
      const proto = win.HTMLCanvasElement.prototype;
      const getContext = proto.getContext;
      proto.getContext = function (kind, ...rest) {
        return /webgl/i.test(String(kind)) ? null : getContext.call(this, kind, ...rest);
      };
    }
    const proto = win.HTMLImageElement.prototype;
    const original = Object.getOwnPropertyDescriptor(proto, "src");
    const CARD_ART = /^https?:\/\/cardart\.localhost\/(.*)$/;
    Object.defineProperty(proto, "src", {
      configurable: true,
      enumerable: original.enumerable,
      get: original.get,
      set(value) {
        const m = CARD_ART.exec(String(value));
        original.set.call(this, m ? placeholder(m[1]) : value);
      },
    });
  }

  const api = { COMMANDS, EVENTS, SCREEN, settingsView, createTauri, install };
  if (typeof module !== "undefined" && module.exports) module.exports = api;
  else window.TLGalleryBridge = api;
})();

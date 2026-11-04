// Stand-in for the app's bridge (window.__TAURI__) so the real overlay page
// runs in a plain browser: every command and event the page uses is answered
// from one scenario of example data. Settings changes are no-ops (the frame
// stays as the scenario says); card art becomes a plain placeholder. The
// game's minion textures are the one thing fetched: from the public host the
// app uses (the set's manifest, layout and images), so a minion looks as it
// does in the app; with no network the page draws plain card art, as the app
// does. A scenario's `textures` says "none" (no set), or { bad: [names] }
// (those textures do not load), and `noWebgl` turns WebGL off. Settings may
// say `unlocked` (the layout editor), `preview` (the tray's preview) or
// `all_hidden`; a scenario's `hideCycle` hides and shows the overlay over and
// over, as the hide key does, and `click` names a button pressed once the page
// is up (the legend's "?").
// ui-tests/overlay-gallery.test.js checks that every name the page's scripts
// use is listed here.
"use strict";

(() => {
  const node = typeof module !== "undefined" && module.exports;
  const themes = node ? require("./theme-vars.js") : window.TLGalleryThemes;
  const defaults = node ? require("./default-layout.js") : window.TLGalleryDefaults;

  // The game window the frames show: the overlay covers its picture. In a
  // browser it is the page's own window (the workbench sizes the frame); the
  // tests and the gallery's pictures use this size.
  const SCREEN = [1920, 1080];
  const screenSize = () => (typeof window !== "undefined" && window.innerWidth > 0 && window.innerHeight > 0 ? [window.innerWidth, window.innerHeight] : SCREEN);

  // The view the app's overlay_settings gives (overlay.rs `View`).
  function settingsView(scenario) {
    const s = (scenario.state && scenario.state.settings) || {};
    const theme = s.theme === "contrast" ? "contrast" : "glass";
    return {
      enabled: true,
      unlocked: Boolean(s.unlocked),
      theme,
      themes: themes.THEMES,
      opacity: 0.9,
      min_opacity: 0.8,
      max_opacity: 1,
      hidden: s.hidden || [],
      // The odds bar's default place (overlay_layout.rs): top centre of the picture.
      layout: { panels: { odds: defaults.oddsRect(...screenSize()) } },
      screen: screenSize(),
      // The app's measured default (leaderboard.rs) for the window's size.
      leaderboard: { area: defaults.leaderboardArea(...screenSize()), custom: false },
      vars: themes.THEME_VARS[theme],
      warning: null,
      odds: true,
      all_hidden: Boolean(s.all_hidden),
      hide_key_taken: false,
      preview: Boolean(s.preview),
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
  const CARDS = ["card_names", "hero_groups", "card_data_status", "texture_set"];
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
      // The overlay names heroes as the game shows them: every id is its own group.
      hero_groups: (args) => Object.fromEntries(((args && args.ids) || []).map((id) => [id, id])),
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
    // Its own copy of the settings, so a scenario that changes them over time
    // (`hideCycle`) leaves the shared example data as it is.
    const live = { ...scenario, state: { ...scenario.state, settings: { ...(scenario.state.settings || {}) } } };
    const table = handlers(live, lang, fetchJson);
    const listeners = new Map();
    let cycling = false;
    const invoke = async (command, args) => {
      await gate;
      if (!Object.prototype.hasOwnProperty.call(table, command)) {
        throw new Error(`the gallery does not answer ${command}`);
      }
      return table[command](args);
    };
    // The card is where the app puts it for this window's size.
    const sized = (hover) => (hover ? { ...hover, card: defaults.hoverCard(...screenSize()) } : hover);
    const emit = (name, payload) => {
      const sent = name === "overlay-hover" ? sized(payload) : payload;
      for (const callback of listeners.get(name) || []) callback({ event: name, payload: sent });
    };
    const listen = async (name, callback) => {
      if (!EVENTS.includes(name)) throw new Error(`the gallery does not know the event ${name}`);
      listeners.set(name, [...(listeners.get(name) || []), callback]);
      // The pointer is on a hero from the start: say so once the page listens.
      if (name === "overlay-hover" && scenario.state.hover) {
        setTimeout(() => callback({ event: name, payload: sized(scenario.state.hover) }), 0);
      }
      // Shown, then hidden with its note for a few seconds, then shown again, over and over.
      if (name === "overlay-settings-changed" && scenario.state.hideCycle && !cycling) {
        cycling = true;
        const step = (i) => {
          const timer = setTimeout(() => {
            live.state.settings.all_hidden = !live.state.settings.all_hidden;
            emit("overlay-settings-changed", null);
            step(i + 1);
          }, i % 2 ? HIDDEN_MS : SHOWN_MS);
          // In node (tests) the cycle never keeps the process alive.
          if (timer && typeof timer.unref === "function") timer.unref();
        };
        step(0);
      }
      return () => listeners.set(name, (listeners.get(name) || []).filter((f) => f !== callback));
    };
    return { core: { invoke }, event: { listen }, emit };
  }

  // The hide-all frame's rhythm: shown this long, then hidden (the note shows
  // for its own 3 seconds) this long.
  const SHOWN_MS = 2500;
  const HIDDEN_MS = 5000;
  // Long enough for the layout editor's first draw.
  const CLICK_AFTER_MS = 400;

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
    // A button pressed once the page is drawn (the legend's "?" in the toolbar).
    if (scenario.state.click) {
      loaded.then(() =>
        setTimeout(() => {
          const button = win.document.getElementById(scenario.state.click);
          if (button) button.click();
        }, CLICK_AFTER_MS),
      );
    }
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

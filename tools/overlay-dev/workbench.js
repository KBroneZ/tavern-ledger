// The workbench page: the overlay in a frame at a game-window size over a
// backdrop, and the controls. Every choice is kept in the address
// (?scenario=…&lang=…) so a view can be shared and reloaded.
"use strict";

(() => {
  const DEFAULTS = { scenario: "", lang: "en", theme: "", size: "1920x1080", bg: "dark", zoom: "fit", view: "close" };
  // The gallery's focus regions are in pixels of a 1920x1080 window.
  const FOCUS_BASE = { w: 1920, h: 1080 };
  const MAX_CLOSE_UP = 3;
  const PRESETS = ["1920x1080", "2560x1440", "1280x720", "3440x1440"];
  // The sizes the overlap check goes through: the presets and a 4:3 window (the game's picture is centred in it).
  const CHECK_SIZES = [...PRESETS, "1024x768"];
  // Time the page gets to draw its panels and the hover card, and to load at all.
  const LOAD_WAIT_MS = 800;
  const LOAD_TIMEOUT_MS = 15000;
  let checking = false;
  const MIN = { w: 320, h: 240 };
  const MAX = { w: 7680, h: 4320 };
  // The events the overlay page listens to; `payload` is looked up when sent.
  const EVENTS = [
    { name: "live-changed", label: "live-changed" },
    { name: "overlay-hover", label: "overlay-hover (on)", payload: "hover" },
    { name: "overlay-hover", label: "overlay-hover (off)", payload: "none" },
    { name: "lobby-tribes-changed", label: "lobby-tribes-changed" },
    { name: "cards-changed", label: "cards-changed" },
    { name: "overlay-settings-changed", label: "overlay-settings-changed" },
    { name: "app-settings-changed", label: "app-settings-changed", payload: "language" },
  ];

  const $ = (id) => document.getElementById(id);
  const params = new URLSearchParams(window.location.search);
  const choice = { ...DEFAULTS };
  for (const key of Object.keys(DEFAULTS)) if (params.has(key)) choice[key] = params.get(key);

  let scenarios = [];
  const frame = $("frame");

  function parseSize(text) {
    const m = /^(\d{3,4})x(\d{3,4})$/.exec(text);
    if (!m) return { w: 1920, h: 1080 };
    return {
      w: Math.min(Math.max(Number(m[1]), MIN.w), MAX.w),
      h: Math.min(Math.max(Number(m[2]), MIN.h), MAX.h),
    };
  }

  // The address keeps only what differs from the defaults.
  function writeAddress() {
    const next = new URLSearchParams();
    for (const key of Object.keys(DEFAULTS)) if (choice[key] !== DEFAULTS[key] && choice[key] !== "") next.set(key, choice[key]);
    const query = next.toString();
    window.history.replaceState(null, "", `${window.location.pathname}${query ? `?${query}` : ""}`);
    return next;
  }

  function overlayQuery() {
    const q = new URLSearchParams();
    q.set("scenario", choice.scenario);
    q.set("lang", choice.lang);
    if (choice.theme) q.set("theme", choice.theme);
    return q;
  }

  function loadFrame() {
    const q = overlayQuery();
    frame.src = `/ui/overlay.html?${q}`;
    $("bare").href = `/bare?${q}`;
    const scenario = scenarios.find((s) => s.id === choice.scenario);
    const caption = scenario && scenario.caption[choice.lang];
    $("caption").replaceChildren();
    if (caption) {
      const title = document.createElement("strong");
      title.textContent = caption.title;
      $("caption").append(title, document.createTextNode(` — ${caption.sees || ""}`));
    }
  }

  // The game window's size and the frame's scale: only styles, no reload.
  function layout() {
    // The overlap check sets the frame's size itself.
    if (checking) return;
    const { w, h } = parseSize(choice.size);
    const stage = $("stage");
    const room = { w: stage.clientWidth - 32, h: window.innerHeight - stage.getBoundingClientRect().top - 32 };
    const scenario = scenarios.find((s) => s.id === choice.scenario);
    const focus = choice.view === "close" && scenario && scenario.focus;
    const game = $("game");
    frame.style.width = `${w}px`;
    frame.style.height = `${h}px`;
    if (focus) {
      // The part of the window the scenario is about, enlarged to the stage.
      const region = {
        x: (focus.x / FOCUS_BASE.w) * w, y: (focus.y / FOCUS_BASE.h) * h,
        w: (focus.w / FOCUS_BASE.w) * w, h: (focus.h / FOCUS_BASE.h) * h,
      };
      const scale = Math.min(MAX_CLOSE_UP, room.w / region.w, Math.max(room.h, 200) / region.h);
      game.style.width = `${Math.round(region.w * scale)}px`;
      game.style.height = `${Math.round(region.h * scale)}px`;
      frame.style.transform = `scale(${scale}) translate(${-region.x}px, ${-region.y}px)`;
    } else {
      const scale = choice.zoom === "fit" ? Math.min(1, room.w / w, Math.max(room.h, 200) / h) : Number(choice.zoom) || 1;
      game.style.width = `${Math.round(w * scale)}px`;
      game.style.height = `${Math.round(h * scale)}px`;
      frame.style.transform = `scale(${scale})`;
    }
    const image = choice.bg.startsWith("image:");
    game.dataset.backdrop = image ? "image" : choice.bg;
    game.style.backgroundImage = image ? `url("/backgrounds/${encodeURIComponent(choice.bg.slice(6))}")` : "";
  }

  function syncControls() {
    $("scenario").value = choice.scenario;
    $("lang").value = choice.lang;
    $("theme").value = choice.theme;
    $("scale").value = choice.zoom;
    $("view").value = choice.view;
    $("scale").disabled = choice.view === "close";
    $("backdrop").value = choice.bg;
    const preset = PRESETS.includes(choice.size);
    $("size").value = preset ? choice.size : "custom";
    $("custom-size").hidden = preset;
    const { w, h } = parseSize(choice.size);
    $("size-w").value = w;
    $("size-h").value = h;
  }

  function change(key, value, reload) {
    choice[key] = value;
    writeAddress();
    syncControls();
    if (reload) loadFrame();
    layout();
  }

  function logEvent(text) {
    $("event-log").textContent = `${new Date().toLocaleTimeString()} · ${text}`;
  }

  // The bridge stand-in in the frame is the page's window.__TAURI__.
  function fire(event) {
    const win = frame.contentWindow;
    if (!win || !win.__TAURI__ || typeof win.__TAURI__.emit !== "function") return logEvent("the overlay page is not ready");
    const scenario = scenarios.find((s) => s.id === choice.scenario);
    let payload = null;
    if (event.payload === "hover") payload = scenario && scenario.hover;
    if (event.payload === "language") payload = { language: choice.lang };
    if (event.payload === "hover" && !payload) return logEvent("this scenario has no hover to show; pick hover-solo or hover-duos");
    win.__TAURI__.emit(event.name, payload);
    logEvent(`sent ${event.name}`);
  }

  // Every scenario at every size: nothing the overlay shows may cover anything
  // else (the rendered rectangles, overlaps.js). Resolves to the failures.
  async function checkOverlaps() {
    const report = $("overlap-report");
    const button = $("check-overlaps");
    const before = { size: choice.size, scenario: choice.scenario };
    const failures = [];
    button.disabled = true;
    checking = true;
    report.hidden = false;
    try {
      for (const size of CHECK_SIZES) {
        const { w, h } = parseSize(size);
        frame.style.width = `${w}px`;
        frame.style.height = `${h}px`;
        for (const s of scenarios) {
          report.textContent = `Checking ${s.id} at ${size}…`;
          const q = overlayQuery();
          q.set("scenario", s.id);
          const loaded = await new Promise((resolve) => {
            const timer = setTimeout(() => resolve(false), LOAD_TIMEOUT_MS);
            frame.onload = () => {
              clearTimeout(timer);
              resolve(true);
            };
            frame.src = `/ui/overlay.html?${q}`;
          });
          if (!loaded) {
            failures.push(`${s.id} at ${size}: the page did not load`);
            continue;
          }
          await new Promise((resolve) => setTimeout(resolve, LOAD_WAIT_MS));
          const shown = window.TLOverlaps.collect(frame.contentDocument);
          // A page that drew nothing (an error) must not pass for "no overlaps".
          if (shown.length === 0) failures.push(`${s.id} at ${size}: nothing is shown`);
          for (const f of window.TLOverlaps.findOverlaps(shown)) failures.push(`${s.id} at ${size}: ${f.a} and ${f.b} share ${f.w}×${f.h}`);
        }
      }
    } finally {
      frame.onload = null;
      checking = false;
      button.disabled = false;
      choice.size = before.size;
      choice.scenario = before.scenario;
      loadFrame();
      layout();
    }
    report.textContent = failures.length
      ? `${failures.length} overlap(s): ${failures.join("; ")}`
      : `No overlaps in ${scenarios.length} scenarios at ${CHECK_SIZES.length} sizes.`;
    return failures;
  }
  window.checkOverlaps = checkOverlaps;

  function option(value, text) {
    const o = document.createElement("option");
    o.value = value;
    o.textContent = text;
    return o;
  }

  async function fetchJson(url) {
    const res = await fetch(url);
    if (!res.ok) throw new Error(`${url}: ${res.status}`);
    return res.json();
  }

  async function start() {
    const [list, backgrounds] = await Promise.all([fetchJson("/api/scenarios"), fetchJson("/api/backgrounds")]);
    scenarios = list.scenarios;
    $("scenario").replaceChildren(...scenarios.map((s) => option(s.id, `${s.id} · ${s.caption.en.title}`)));
    if (!scenarios.some((s) => s.id === choice.scenario)) choice.scenario = scenarios[0].id;
    $("backdrop").replaceChildren(
      option("dark", "Plain dark"),
      option("checker", "Checkerboard"),
      ...backgrounds.files.map((f) => option(`image:${f}`, `Image · ${f}`)),
    );
    if (!Array.from($("backdrop").options).some((o) => o.value === choice.bg)) choice.bg = DEFAULTS.bg;

    $("scenario").addEventListener("change", (e) => change("scenario", e.target.value, true));
    $("lang").addEventListener("change", (e) => change("lang", e.target.value, true));
    $("theme").addEventListener("change", (e) => change("theme", e.target.value, true));
    $("backdrop").addEventListener("change", (e) => change("bg", e.target.value, false));
    $("view").addEventListener("change", (e) => change("view", e.target.value, false));
    $("scale").addEventListener("change", (e) => change("zoom", e.target.value, false));
    $("size").addEventListener("change", (e) => {
      if (e.target.value !== "custom") return change("size", e.target.value, true);
      $("custom-size").hidden = false;
      change("size", `${$("size-w").value}x${$("size-h").value}`, true);
    });
    const custom = () => change("size", `${$("size-w").value}x${$("size-h").value}`, true);
    $("size-w").addEventListener("change", custom);
    $("size-h").addEventListener("change", custom);
    window.addEventListener("resize", layout);
    $("check-overlaps").addEventListener("click", () => checkOverlaps().catch((error) => { $("overlap-report").textContent = `The check failed: ${error.message}`; }));

    const buttons = EVENTS.map((event) => {
      const b = document.createElement("button");
      b.type = "button";
      b.textContent = event.label;
      b.addEventListener("click", () => fire(event));
      return b;
    });
    $("event-buttons").replaceChildren(...buttons);

    writeAddress();
    syncControls();
    loadFrame();
    layout();
  }

  start().catch((error) => {
    $("caption").textContent = `The workbench could not start: ${error.message}`;
  });
})();

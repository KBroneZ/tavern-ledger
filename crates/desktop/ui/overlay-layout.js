// Layout editing for the overlay (T-302). The app owns the settings and the
// rules (overlay_layout.rs: what fits the screen, which themes exist); this
// file draws them and turns mouse moves into a rectangle to save. Locked, the
// window is click-through and none of this can be reached.
"use strict";

// In its own scope: overlay.js has functions with the same names (refresh).
(() => {
  const { invoke: call } = window.__TAURI__.core;
  const { listen: onEvent } = window.__TAURI__.event;

  const PANEL_IDS = ["status", "tribes", "opponents", "legend"];
  const PANEL_NAMES = { status: "Status", tribes: "Tribes", opponents: "Opponents" };
  // Rough limits while dragging; the app enforces the real ones when it saves.
  const MIN_W = 160;
  const MAX_W = 800;
  const MIN_H = 40;
  const GRAB_H = 32;
  const OPACITY_STEP = 0.05;

  const stack = document.getElementById("stack");
  const slots = new Map(
    PANEL_IDS.map((id) => [id, document.querySelector(`.slot[data-panel="${id}"]`)]),
  );
  let view = null;
  // The rectangles of the panels the user has placed.
  let rects = {};
  let dragging = false;
  let pending = null;

  function px(n) {
    return `${Math.round(n)}px`;
  }

  function place(id, slot) {
    const r = rects[id];
    if (!r) {
      slot.classList.remove("placed");
      for (const prop of ["left", "top", "width", "minHeight"]) slot.style[prop] = "";
      return false;
    }
    slot.classList.add("placed");
    slot.style.left = px(r.x);
    slot.style.top = px(r.y);
    slot.style.width = px(r.w);
    slot.style.minHeight = r.h ? px(r.h) : "";
    return true;
  }

  function button(text, onClick, pressed) {
    const b = document.createElement("button");
    b.type = "button";
    b.textContent = text;
    if (pressed !== undefined) b.setAttribute("aria-pressed", String(pressed));
    b.addEventListener("click", onClick);
    return b;
  }

  function drawToolbar(v) {
    const themes = document.getElementById("theme-buttons");
    themes.replaceChildren(
      ...v.themes.map((t) =>
        button(t.label, () => send("overlay_set_theme", { theme: t.id }).catch(fail), t.id === v.theme),
      ),
    );
    document.getElementById("opacity-value").textContent = `${Math.round(v.opacity * 100)}%`;
    const toggles = document.getElementById("panel-toggles");
    toggles.replaceChildren(
      ...Object.entries(PANEL_NAMES).map(([id, name]) => {
        const shown = !v.hidden.includes(id);
        return button(
          name,
          () => send("overlay_set_shown", { panel: id, shown: !shown }).catch(fail),
          shown,
        );
      }),
    );
    const warning = document.getElementById("toolbar-warning");
    warning.hidden = !v.warning;
    warning.textContent = v.warning || "";
  }

  function draw(v) {
    view = v;
    const root = document.documentElement;
    for (const [name, value] of Object.entries(v.vars)) root.style.setProperty(name, value);
    document.body.classList.toggle("unlocked", v.unlocked);
    document.getElementById("scrim").hidden = !v.unlocked;
    document.getElementById("toolbar").hidden = !v.unlocked;
    setEditing(v.unlocked);
    rects = { ...v.layout.panels };
    const inStack = [];
    for (const id of PANEL_IDS) {
      const slot = slots.get(id);
      slot.hidden = v.hidden.includes(id);
      slot.querySelector(".grip").hidden = !v.unlocked;
      if (place(id, slot)) document.body.append(slot);
      else inStack.push(slot);
    }
    stack.append(...inStack);
    if (v.unlocked) drawToolbar(v);
  }

  // A change that arrives while a panel is being dragged waits for the drop.
  function show(v) {
    if (dragging) pending = v;
    else draw(v);
  }

  async function send(command, args) {
    show(await call(command, args));
  }

  function clamp(n, lo, hi) {
    return Math.min(Math.max(n, lo), Math.max(lo, hi));
  }

  function startDrag(event) {
    if (!view || !view.unlocked || event.button !== 0) return;
    const slot = event.target.closest(".slot");
    if (!slot) return;
    const id = slot.dataset.panel;
    const box = slot.getBoundingClientRect();
    if (!rects[id]) rects[id] = { x: box.left, y: box.top, w: box.width, h: null };
    const start = { ...rects[id], px: event.clientX, py: event.clientY, height: box.height };
    const resizing = event.target.classList.contains("grip");
    dragging = true;
    slot.classList.add("dragging");
    event.preventDefault();

    const move = (e) => {
      const sw = window.innerWidth;
      const sh = window.innerHeight;
      const dx = e.clientX - start.px;
      const dy = e.clientY - start.py;
      const r = rects[id];
      if (resizing) {
        r.w = clamp(start.w + dx, MIN_W, Math.min(MAX_W, sw - r.x));
        r.h = clamp(start.height + dy, MIN_H, sh - r.y);
      } else {
        r.x = clamp(start.x + dx, 0, sw - r.w);
        r.y = clamp(start.y + dy, 0, sh - GRAB_H);
      }
      place(id, slot);
      if (slot.parentElement !== document.body) document.body.append(slot);
    };
    const end = () => {
      window.removeEventListener("pointermove", move);
      window.removeEventListener("pointerup", end);
      window.removeEventListener("pointercancel", end);
      slot.classList.remove("dragging");
      dragging = false;
      pending = null;
      send("overlay_save_layout", { panels: rects }).catch(fail);
    };
    // On the window, not the panel: the pointer can leave the panel faster than it follows.
    window.addEventListener("pointermove", move);
    window.addEventListener("pointerup", end);
    window.addEventListener("pointercancel", end);
  }

  function fail(error) {
    const warning = document.getElementById("toolbar-warning");
    warning.hidden = false;
    warning.textContent = `The overlay settings could not be updated (${error}).`;
  }

  async function refresh() {
    show(await call("overlay_settings"));
  }

  async function init() {
    document
      .getElementById("lock")
      .addEventListener("click", () => send("overlay_set_unlocked", { unlocked: false }).catch(fail));
    document
      .getElementById("reset-layout")
      .addEventListener("click", () => send("overlay_reset_layout").catch(fail));
    const nudge = (delta) => () => {
      if (view) send("overlay_set_opacity", { value: view.opacity + delta }).catch(fail);
    };
    document.getElementById("opacity-down").addEventListener("click", nudge(-OPACITY_STEP));
    document.getElementById("opacity-up").addEventListener("click", nudge(OPACITY_STEP));
    document.addEventListener("pointerdown", (event) => {
      try {
        startDrag(event);
      } catch (error) {
        dragging = false;
        fail(error);
      }
    });
    await onEvent("overlay-settings-changed", () => refresh().catch(fail));
    await refresh();
  }

  init().catch(fail);
})();

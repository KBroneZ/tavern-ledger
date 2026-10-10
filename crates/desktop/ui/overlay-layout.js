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
  const SLOT_NAMES = { ...PANEL_NAMES, legend: "Legend" };
  // Rough limits while dragging; the app enforces the real ones when it saves.
  const MIN_W = 160;
  const MAX_W = 800;
  const MIN_H = 40;
  const GRAB_H = 32;
  const OPACITY_STEP = 0.05;
  // Keys are saved once they stop, not once per key press (the app writes a file each time).
  const KEY_SAVE_DELAY_MS = 400;
  const KEYSHORTCUTS = "ArrowLeft ArrowRight ArrowUp ArrowDown Shift+ArrowLeft Shift+ArrowRight Shift+ArrowUp Shift+ArrowDown Alt+ArrowLeft Alt+ArrowRight Alt+ArrowUp Alt+ArrowDown";
  // Space kept from the bottom edge of the screen.
  const EDGE = 8;
  // The smallest leaderboard box, in the window's own pixels.
  const BOX_MIN_W = 12;
  const BOX_MIN_H = 40;

  const stack = document.getElementById("stack");
  const slots = new Map(
    PANEL_IDS.map((id) => [id, document.querySelector(`.slot[data-panel="${id}"]`)]),
  );
  let view = null;
  // The rectangles of the panels the user has placed.
  let rects = {};
  let dragging = false;
  let pending = null;
  // Stops the drag or key session in progress without saving it (the overlay got locked under it).
  let abortDrag = null;
  // The key presses not saved yet: { timer, save }.
  let keySession = null;

  function px(n) {
    return `${Math.round(n)}px`;
  }

  function place(id, slot) {
    const r = rects[id];
    if (!r) {
      slot.classList.remove("placed");
      for (const prop of ["left", "top", "width", "minHeight", "maxHeight"]) slot.style[prop] = "";
      slot.querySelector(".panel").style.maxHeight = "";
      return false;
    }
    slot.classList.add("placed");
    slot.style.left = px(r.x);
    slot.style.top = px(r.y);
    slot.style.width = px(r.w);
    slot.style.minHeight = r.h ? px(r.h) : "";
    // Never past the bottom of the screen (T-308): what does not fit is cut.
    const limit = px(Math.max(GRAB_H, window.innerHeight - r.y - EDGE));
    slot.style.maxHeight = limit;
    slot.querySelector(".panel").style.maxHeight = limit;
    return true;
  }

  // The leaderboard box (T-306), in the window's own pixels; shown only
  // while unlocked and once the app knows where the game window is.
  const box = document.getElementById("board-box");
  let boxRect = null;

  function placeBox(r) {
    box.style.left = px(r.x);
    box.style.top = px(r.y);
    box.style.width = px(r.w);
    box.style.height = px(r.h);
  }

  function drawBox(v) {
    const known = v.unlocked && v.leaderboard;
    box.hidden = !known;
    boxRect = known ? { ...v.leaderboard.area } : null;
    if (!known) return;
    box.tabIndex = 0;
    box.setAttribute("role", "group");
    box.setAttribute("aria-label", "Leaderboard box");
    box.setAttribute("aria-describedby", "key-hint");
    box.setAttribute("aria-roledescription", "movable box");
    box.setAttribute("aria-keyshortcuts", KEYSHORTCUTS);
    box.classList.toggle("custom", v.leaderboard.custom);
    box.querySelector(".board-label").textContent = v.leaderboard.custom
      ? "Leaderboard (your box)"
      : "Leaderboard (measured default)";
    placeBox(boxRect);
  }

  function button(text, onClick, pressed, focusKey) {
    const b = document.createElement("button");
    b.type = "button";
    b.textContent = text;
    b.setAttribute("data-focus", focusKey);
    if (pressed !== undefined) b.setAttribute("aria-pressed", String(pressed));
    b.addEventListener("click", onClick);
    return b;
  }

  function drawToolbar(v) {
    const themes = document.getElementById("theme-buttons");
    themes.replaceChildren(
      ...v.themes.map((t) =>
        button(t.label, () => send("overlay_set_theme", { theme: t.id }).catch(fail), t.id === v.theme, `theme:${t.id}`),
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
          `panel:${id}`,
        );
      }),
    );
    const warning = document.getElementById("toolbar-warning");
    const noBoard = v.leaderboard ? null : "Bring Hearthstone to the front once to adjust the leaderboard box.";
    const text = [v.warning, noBoard].filter(Boolean).join(" ");
    warning.hidden = !text;
    warning.textContent = text;
    document.getElementById("reset-board").disabled = !v.leaderboard || !v.leaderboard.custom;
  }

  // Whether the user can reach a panel by keyboard: only while arranging.
  function makeReachable(id, slot, unlocked) {
    if (!unlocked) {
      for (const name of ["tabindex", "role", "aria-label", "aria-describedby", "aria-roledescription", "aria-keyshortcuts"]) slot.removeAttribute(name);
      return;
    }
    slot.tabIndex = 0;
    slot.setAttribute("role", "group");
    slot.setAttribute("aria-label", `${SLOT_NAMES[id]} panel position`);
    slot.setAttribute("aria-describedby", "key-hint");
    slot.setAttribute("aria-roledescription", "movable panel");
    slot.setAttribute("aria-keyshortcuts", KEYSHORTCUTS);
  }

  // Redrawing re-parents panels and rebuilds the toolbar: focus is kept through it.
  function draw(v) {
    TLFocus.keep(document, () => drawNow(v));
  }

  function drawNow(v) {
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
      makeReachable(id, slot, v.unlocked);
      if (place(id, slot)) document.body.append(slot);
      else inStack.push(slot);
    }
    stack.append(...inStack);
    drawBox(v);
    if (v.unlocked) drawToolbar(v);
  }

  // A change that arrives while a panel is being dragged waits for the drop.
  function show(v) {
    // Locked from elsewhere: keys already pressed are kept, a drag in progress is dropped.
    if (keySession && !v.unlocked) saveKeys().catch(failAndRefresh);
    if (dragging && !v.unlocked && abortDrag) abortDrag();
    if (dragging || keySession) pending = v;
    else draw(v);
  }

  async function send(command, args) {
    show(await call(command, args));
  }

  function clamp(n, lo, hi) {
    return Math.min(Math.max(n, lo), Math.max(lo, hi));
  }

  // Moves or resizes the leaderboard box; saved on release.
  function startBoxDrag(event) {
    saveKeys().catch(failAndRefresh);
    const start = { ...boxRect, px: event.clientX, py: event.clientY };
    const resizing = event.target.classList.contains("grip");
    let moved = false;
    dragging = true;
    event.preventDefault();
    const move = (e) => {
      const sw = window.innerWidth;
      const sh = window.innerHeight;
      const dx = e.clientX - start.px;
      const dy = e.clientY - start.py;
      moved = moved || dx !== 0 || dy !== 0;
      if (resizing) {
        boxRect.w = clamp(start.w + dx, BOX_MIN_W, sw - boxRect.x);
        boxRect.h = clamp(start.h + dy, BOX_MIN_H, sh - boxRect.y);
      } else {
        boxRect.x = clamp(start.x + dx, 0, sw - boxRect.w);
        boxRect.y = clamp(start.y + dy, 0, sh - boxRect.h);
      }
      placeBox(boxRect);
    };
    const stop = () => {
      window.removeEventListener("pointermove", move);
      window.removeEventListener("pointerup", end);
      window.removeEventListener("pointercancel", end);
      dragging = false;
      pending = null;
      abortDrag = null;
    };
    const end = () => {
      stop();
      if (moved) send("overlay_save_leaderboard", { area: { x: boxRect.x, y: boxRect.y, w: boxRect.w, h: boxRect.h } }).catch(fail);
      else if (view) draw(view);
    };
    abortDrag = stop;
    window.addEventListener("pointermove", move);
    window.addEventListener("pointerup", end);
    window.addEventListener("pointercancel", end);
  }

  function startDrag(event) {
    if (!view || !view.unlocked || event.button !== 0) return;
    if (boxRect && event.target.closest("#board-box")) {
      startBoxDrag(event);
      return;
    }
    const slot = event.target.closest(".slot");
    if (!slot) return;
    const id = slot.dataset.panel;
    saveKeys().catch(failAndRefresh);
    const box = slot.getBoundingClientRect();
    if (!rects[id]) rects[id] = { x: box.left, y: box.top, w: box.width, h: null };
    const start = { ...rects[id], px: event.clientX, py: event.clientY, height: box.height };
    const resizing = event.target.classList.contains("grip");
    let moved = false;
    dragging = true;
    slot.classList.add("dragging");
    event.preventDefault();

    const move = (e) => {
      const sw = window.innerWidth;
      const sh = window.innerHeight;
      const dx = e.clientX - start.px;
      const dy = e.clientY - start.py;
      moved = moved || dx !== 0 || dy !== 0;
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
    const stop = () => {
      window.removeEventListener("pointermove", move);
      window.removeEventListener("pointerup", end);
      window.removeEventListener("pointercancel", end);
      slot.classList.remove("dragging");
      dragging = false;
      pending = null;
      abortDrag = null;
    };
    const end = () => {
      stop();
      // A press with no movement changes nothing: it must not pin a stacked panel.
      if (moved) send("overlay_save_layout", { panels: rects }).catch(fail);
      else if (view) draw(view);
    };
    abortDrag = stop;
    // On the window, not the panel: the pointer can leave the panel faster than it follows.
    window.addEventListener("pointermove", move);
    window.addEventListener("pointerup", end);
    window.addEventListener("pointercancel", end);
  }

  // Saves the key presses made so far, now. Resolves once the app has answered.
  async function saveKeys() {
    if (!keySession) return;
    const { timer, save } = keySession;
    clearTimeout(timer);
    keySession = null;
    pending = null;
    abortDrag = null;
    await save();
  }

  // Applies a key press now and saves it once the keys pause, like the end of a drag.
  function queueKeySave(owner, save, spoken) {
    // Another panel (or the box) than the one waiting to be saved: save that one first.
    if (keySession && keySession.owner !== owner) saveKeys().catch(failAndRefresh);
    if (keySession) clearTimeout(keySession.timer);
    announce(spoken);
    const timer = setTimeout(() => saveKeys().catch(failAndRefresh), KEY_SAVE_DELAY_MS);
    keySession = { timer, save, owner };
    abortDrag = () => {
      clearTimeout(timer);
      keySession = null;
      pending = null;
      abortDrag = null;
    };
  }

  // The panel or box that holds keyboard focus itself (not a button inside it).
  function focusedTarget() {
    const node = document.activeElement;
    if (!node || !view || !view.unlocked) return null;
    if (node === box && boxRect) return { kind: "box" };
    if (node.classList.contains("slot") && node.dataset.panel) return { kind: "panel", id: node.dataset.panel, slot: node };
    return null;
  }

  function keyArrange(event) {
    if (event.ctrlKey || event.metaKey || dragging) return;
    const target = focusedTarget();
    if (!target) return;
    const screen = { w: window.innerWidth, h: window.innerHeight };
    if (target.kind === "box") {
      const next = TLKeys.nudge(boxRect, event, screen, { minW: BOX_MIN_W, maxW: screen.w, minH: BOX_MIN_H, keepInside: true });
      if (!next) return;
      event.preventDefault();
      if (next === boxRect) return announce("Leaderboard box: edge of the screen");
      boxRect = next;
      placeBox(boxRect);
      queueKeySave("box", () => send("overlay_save_leaderboard", { area: { x: next.x, y: next.y, w: next.w, h: next.h } }), `Leaderboard box: x ${Math.round(next.x)}, y ${Math.round(next.y)}, width ${Math.round(next.w)}, height ${Math.round(next.h)}`);
      return;
    }
    const { id, slot } = target;
    const shown = slot.getBoundingClientRect();
    const current = rects[id] || { x: shown.left, y: shown.top, w: shown.width, h: null };
    const from = { ...current, h: current.h ?? shown.height };
    const next = TLKeys.nudge(from, event, screen, { minW: MIN_W, maxW: MAX_W, minH: MIN_H, grabH: GRAB_H });
    if (!next) return;
    event.preventDefault();
    if (next === from) return announce(`${SLOT_NAMES[id]} panel: edge or size limit reached`);
    // Height is only pinned when the user resized; a move keeps the panel's own height.
    rects[id] = { x: next.x, y: next.y, w: next.w, h: event.altKey ? next.h : current.h };
    place(id, slot);
    // Putting a stacked panel on the page moves it in the document: focus is kept through that.
    TLFocus.keep(document, () => {
      if (slot.parentElement !== document.body) document.body.append(slot);
    });
    queueKeySave(id, () => send("overlay_save_layout", { panels: rects }), `${SLOT_NAMES[id]} panel: x ${Math.round(next.x)}, y ${Math.round(next.y)}, width ${Math.round(next.w)}`);
  }

  // Said by screen readers; nothing on screen shows it.
  function announce(text) {
    document.getElementById("key-status").textContent = text;
  }

  // A short line on screen, only while a panel or the box has the keyboard.
  function showKeyHint() {
    document.getElementById("key-hint").hidden = !focusedTarget();
  }

  // A save that failed: show what the app really has, then say why.
  function failAndRefresh(error) {
    refresh().then(() => fail(error), fail);
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
    document.getElementById("lock").addEventListener("click", async () => {
      try {
        await saveKeys();
        await send("overlay_set_unlocked", { unlocked: false });
      } catch (error) {
        fail(error);
      }
    });
    document
      .getElementById("reset-layout")
      .addEventListener("click", () => send("overlay_reset_layout").catch(fail));
    document
      .getElementById("reset-board")
      .addEventListener("click", () => send("overlay_reset_leaderboard").catch(fail));
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
    document.addEventListener("keydown", keyArrange);
    document.addEventListener("focusin", showKeyHint);
    document.addEventListener("focusout", () => setTimeout(showKeyHint, 0));
    await onEvent("overlay-settings-changed", () => refresh().catch(fail));
    await refresh();
  }

  init().catch(fail);
})();

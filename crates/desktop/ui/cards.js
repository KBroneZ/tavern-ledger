// Card names and art (T-304, D-038), shared by the window and the overlay.
// Names come from the app (HearthstoneJSON's card data, cached on this PC);
// art comes from the app's cardart protocol. A card the data does not know
// shows its id; an image that does not load is removed, so no wrong or
// broken picture is ever shown. Own scope: it must not clash with the
// page's own names (see overlay-layout.js).
"use strict";

window.TLCards = (() => {
  const { invoke } = window.__TAURI__.core;
  const { listen } = window.__TAURI__.event;
  const ID = /^[A-Za-z0-9_]{1,64}$/;
  const ART = `${location.protocol === "https:" ? "https" : "http"}://cardart.localhost/`;
  const BATCH = 200; // the app answers at most this many ids at once

  const names = new Map(); // id -> name, or null when the card data does not know it
  const waiting = new Set(); // asked while the app had no card data yet
  const asking = new Set();
  const watchers = [];
  let queued = new Set();
  let timer = null;
  let status = null;

  const changed = () => watchers.forEach((f) => f());

  async function flush() {
    timer = null;
    const ids = [...queued];
    queued = new Set();
    ids.forEach((id) => asking.add(id));
    for (let i = 0; i < ids.length; i += BATCH) {
      const part = ids.slice(i, i + BATCH);
      let got = {};
      try {
        got = await invoke("card_names", { ids: part });
      } catch (_) {
        got = {}; // the names stay unknown; the ids show
      }
      for (const id of part) {
        asking.delete(id);
        // Left out of the answer: no card data yet. Asked again on the next change.
        if (!got || !Object.prototype.hasOwnProperty.call(got, id)) waiting.add(id);
        else names.set(id, typeof got[id] === "string" ? got[id] : null);
      }
    }
    changed();
  }

  function want(id) {
    if (typeof id !== "string" || !ID.test(id) || asking.has(id)) return;
    queued.add(id);
    if (!timer) timer = setTimeout(flush, 0);
  }

  // A string: the name. null: the card data does not know this id (show it,
  // marked unknown). undefined: not answered yet (show the id, no mark).
  function name(id) {
    if (typeof id !== "string" || !ID.test(id)) return null;
    if (names.has(id)) return names.get(id);
    if (!waiting.has(id)) want(id);
    return undefined;
  }

  // An <img> for the card's art, or null for an id that is not a card id.
  function art(id, className) {
    if (typeof id !== "string" || !ID.test(id)) return null;
    const img = document.createElement("img");
    img.className = className;
    img.alt = "";
    img.decoding = "async";
    img.draggable = false;
    img.addEventListener("error", () => img.remove());
    img.src = ART + id;
    return img;
  }

  async function refreshStatus() {
    try {
      status = await invoke("card_data_status");
    } catch (_) {
      status = null;
    }
    changed();
  }

  // The card data changed: ask again for every id seen, keeping the old
  // answers on screen until the new ones arrive (no flicker).
  listen("cards-changed", () => {
    for (const id of [...names.keys(), ...waiting]) want(id);
    waiting.clear();
    refreshStatus();
  });
  refreshStatus();

  return {
    name,
    art,
    status: () => status,
    onChange: (f) => watchers.push(f),
  };
})();

// Card names and art (T-304, D-038), shared by the window and the overlay.
// Names come from the app (HearthstoneJSON's card data, cached on this PC)
// in the app's language (Spanish names with the app in
// Spanish, English ones in English), whatever language the game's log is in;
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
    // Names in the page's language: asked once it is known.
    const I18n = window.TLI18n;
    if (I18n) await I18n.ready();
    const lang = I18n ? I18n.lang() : "en";
    const ids = [...queued];
    queued = new Set();
    ids.forEach((id) => asking.add(id));
    let news = false;
    for (let i = 0; i < ids.length; i += BATCH) {
      const part = ids.slice(i, i + BATCH);
      let got = {};
      try {
        got = await invoke("card_names", { ids: part, lang });
      } catch (_) {
        got = {}; // the names stay unknown; the ids show
      }
      for (const id of part) {
        asking.delete(id);
        // Left out of the answer: no card data yet. Asked again on the next change.
        if (!got || !Object.prototype.hasOwnProperty.call(got, id)) {
          waiting.add(id);
          continue;
        }
        const answer = typeof got[id] === "string" ? got[id] : null;
        news = news || !names.has(id) || names.get(id) !== answer;
        names.set(id, answer);
      }
    }
    // The pages redraw only when a name is new or changed.
    if (news) changed();
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

  // The name to show: the card data's in the app's language, else the name
  // the game's log printed (`logName`, in the game client's language), else
  // the card id. Asks the app for it when it is not known yet.
  function display(id, logName) {
    const known = name(id);
    if (typeof known === "string" && known) return known;
    return logName || id || null;
  }

  // An <img> for the card's art, or null for an id that is not a card id.
  // Options (the main window, T-320): `lazy` fetches it only when it comes
  // into view (long lists and closed folds then cost no picture), and
  // `placeholder` puts a plain shape of the same size in place of a picture
  // that does not load, instead of removing it.
  function art(id, className, options = {}) {
    if (typeof id !== "string" || !ID.test(id)) return null;
    const img = document.createElement("img");
    img.className = className;
    img.alt = "";
    img.decoding = "async";
    img.draggable = false;
    if (options.lazy) img.loading = "lazy";
    img.addEventListener("error", () => {
      if (!options.placeholder) return img.remove();
      const missing = document.createElement("span");
      missing.className = `${className} art-missing`;
      missing.setAttribute("aria-hidden", "true");
      missing.title = window.TLI18n.t("cards.no_picture", { source: window.TLI18n.t("source.card_data.label") });
      img.replaceWith(missing);
    });
    img.src = ART + id;
    return img;
  }

  // Dark Gifts on a minion (T-528, D-107): `gifts` holds gift card ids,
  // `gift_unknown` marks a minion the game says is gifted with no gift
  // named. One mark per gift, in the minion's order: { id, name, title }.
  // Names and summaries come from strings/gifts.js; a gift it does not know
  // shows its card data name, else its id, with no summary.
  const GIFT_PREFIX = "BG36_MidGameEffect_";
  function gifts(m) {
    const I18n = window.TLI18n;
    const words = (key) => {
      const text = I18n.t(key);
      return text === key ? null : text;
    };
    const ids = m && Array.isArray(m.gifts) ? m.gifts.filter((id) => typeof id === "string" && ID.test(id)) : [];
    const marks = ids.map((id) => {
      const short = id.startsWith(GIFT_PREFIX) ? id.slice(GIFT_PREFIX.length) : null;
      const name = (short && words(`gift.name.${short}`)) || display(id) || id;
      const text = short && words(`gift.text.${short}`);
      const tip = text ? I18n.t("gift.tip", { name, text }) : name;
      return { id, name, title: `${I18n.t("gift.mark")}: ${tip}` };
    });
    if (!marks.length && m && m.gift_unknown === true) {
      marks.push({ id: null, name: I18n.t("gift.unknown"), title: `${I18n.t("gift.unknown")}: ${I18n.t("gift.unknown_text")}` });
    }
    return marks;
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
    display,
    art,
    gifts,
    status: () => status,
    onChange: (f) => watchers.push(f),
  };
})();

// The app's side of the minion drawing (T-372): asks the app for the game's
// texture set, builds the shared drawing (minion-art/minion-art.js, which has
// no app API inside) from it, and tells the page when it is ready. Without a
// usable set (not downloaded yet, offline, refused) there is no drawing:
// `window.TLMinions` stays null and the page draws plain card art. The set
// changes when the app finishes downloading it (`textures-changed`).
import { createMinionArt } from "./minion-art/minion-art.js";

const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;
const ART = `${location.protocol === "https:" ? "https" : "http"}://cardart.localhost/`;

let current = null;
// Each load takes a number; only the newest may publish (a slow older one is dropped).
let generation = 0;

function publish(art) {
  if (current) current.destroy();
  current = art;
  window.TLMinions = art;
  window.dispatchEvent(new Event("tl-minions"));
}

// One line in the app's log for each kind of trouble (the app rate-limits).
function log(message) {
  invoke("log_ui_error", { kind: "textures", message, place: null }).catch(() => {});
}

async function load() {
  const mine = ++generation;
  let view = null;
  try {
    view = await invoke("texture_set");
  } catch (_) {
    view = null;
  }
  if (mine !== generation) return;
  if (!view || typeof view !== "object" || !view.files || typeof view.files !== "object") return publish(null);
  const base = typeof view.base === "string" ? view.base : `${ART}tex/${view.version}/`;
  const art = createMinionArt({
    layout: view.layout,
    imageUrl: (name) => (Object.prototype.hasOwnProperty.call(view.files, name) ? base + view.files[name] : null),
    artImage: (id) => window.TLCards.art(id, "art"),
    formatStat: (value) => window.TLI18n.stat(value),
    markText: (keyword) => window.TLI18n.t(`keyword.${keyword}`),
    log,
  });
  if (!art) {
    log("the texture layout could not be read: minions are drawn without it");
    return publish(null);
  }
  await art.ready;
  if (mine !== generation) return art.destroy();
  publish(art);
}

load();
listen("textures-changed", () => load()).catch(() => {});

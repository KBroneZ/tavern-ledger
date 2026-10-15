// A minion drawn like the game (T-372, D-150, D-153): the game's frame
// (normal or golden), the card art under its oval mask, attack and health
// gems with their numbers, and a layer for each keyword, stacked as the game
// stacks them. One plain ES module with no page or app API inside, so the
// app's overlay and views and the website use the same source: the host
// passes the texture set's layout, where its images are, how to get a card's
// picture, how to write a number and a word, and where to log.
//
//   import { createMinionArt } from "./minion-art/minion-art.js";
//   const art = createMinionArt({ layout, imageUrl, artImage, formatStat, markText, log });
//   if (art) { await art.ready; list.append(art.render(minion)); }
//
// Link `minion-art.css` as well. The element it returns has no size of its
// own: it fills the width its parent gives it (aspect ratio of the frame).
//
// Fallbacks (empty is not unknown): a missing or broken texture leaves only
// that layer out; with no usable layout `createMinionArt` returns null and
// the host draws plain card art; Windfury and Stealth become a word when
// WebGL cannot run them. Every such case is told once through `log`.

import { createFx, FX_TEXTURES } from "./fx.js";
import { percent, planLayers, readLayout } from "./layout.js";

// The opaque part of the oval mask and the gem numbers, in canvas pixels of
// the 400 x 524 frame canvas (texture set v1); css/minion-art.css has the
// same art box as shares of the canvas.
const NUMBER_AT = { attack: [84, 419], health: [322, 430] };
const PRELOAD = ["oval_mask", "frame_normal", "frame_golden", "gem_attack", "gem_health"];
const WORDED_FX = ["stealth", "windfury"];

/**
 * options:
 *   layout     the texture set's layout.json, parsed
 *   imageUrl   (name) => url of that texture, or null when the set lacks it
 *   artImage   (cardId) => an <img> with the card's picture, or null
 *   formatStat (value) => the text of an attack or health value
 *   markText   (keyword) => the word shown when an effect cannot run
 *   log        (message) => void; optional
 *   document   the page's document (default: the global one)
 *   fx         an effects runtime, null for none; default: WebGL effects
 *   fps        effect frame rate (default 30)
 *   preload    false to skip loading the main images first (tests)
 * Returns { render(minion), ready, setActive(on), destroy() }, or null when
 * the layout cannot be used.
 */
export function createMinionArt(options) {
  const layout = readLayout(options.layout);
  if (!layout) return null;
  const doc = options.document || globalThis.document;
  const { imageUrl, artImage, formatStat, markText } = options;
  const log = typeof options.log === "function" ? options.log : () => {};
  const bad = new Set();

  const noteBad = (name) => {
    if (bad.has(name)) return;
    bad.add(name);
    log(`texture ${name} could not be loaded: that layer is left out`);
  };
  const has = (name) => !bad.has(name) && imageUrl(name) != null;

  const images = new Map();
  /** One CORS-clean image per texture, shared by every effect. */
  function loadImage(name) {
    if (!images.has(name)) {
      images.set(
        name,
        new Promise((resolve, reject) => {
          const url = imageUrl(name);
          if (url == null) return reject(new Error(`${name} is not in the set`));
          const img = doc.createElement("img");
          img.crossOrigin = "anonymous";
          img.addEventListener("load", () => resolve(img));
          img.addEventListener("error", () => {
            noteBad(name);
            reject(new Error(`${name} did not load`));
          });
          img.src = url;
        }),
      );
    }
    return images.get(name);
  }

  const wantsFx = layout.windfury || layout.stealth;
  const fx =
    options.fx === undefined
      ? wantsFx
        ? createFx({ document: doc, layout, loadImage, log, fps: options.fps })
        : null
      : options.fx;

  const ready =
    options.preload === false
      ? Promise.resolve()
      : Promise.allSettled(PRELOAD.filter(has).map(loadImage)).then(() => undefined);

  const place = (el, box) => {
    el.style.left = percent(box[0], layout.width);
    el.style.top = percent(box[1], layout.height);
    el.style.width = percent(box[2] - box[0], layout.width);
    el.style.height = percent(box[3] - box[1], layout.height);
  };

  const cssUrl = (url) => `url("${String(url).replace(/["\\\n]/g, "")}")`;

  function imageLayer(op) {
    const img = doc.createElement("img");
    img.className = `tlm-layer tlm-${op.name}`;
    img.alt = "";
    img.crossOrigin = "anonymous";
    img.decoding = "async";
    img.draggable = false;
    place(img, op.box);
    img.addEventListener("error", () => {
      noteBad(op.name);
      img.remove();
    });
    img.src = imageUrl(op.name);
    return img;
  }

  function artLayer(minion) {
    const box = doc.createElement("div");
    box.className = "tlm-art";
    if (has("oval_mask")) {
      const mask = cssUrl(imageUrl("oval_mask"));
      box.style.maskImage = mask;
      box.style.webkitMaskImage = mask;
    } else {
      box.className += " tlm-art-plain";
    }
    const picture = artImage ? artImage(minion.card_id) : null;
    if (picture) {
      if (!/tlm-art-img/.test(picture.className)) picture.className = `${picture.className} tlm-art-img`.trim();
      box.append(picture);
    }
    return box;
  }

  function numberLayer(op) {
    const text = String(formatStat(op.value));
    const el = doc.createElement("div");
    const size = text.length > 5 ? " tlm-num-xlong" : text.length > 3 ? " tlm-num-long" : "";
    el.className = `tlm-num tlm-num-${op.which}${size}`;
    el.textContent = text;
    const [x, y] = NUMBER_AT[op.which];
    el.style.left = percent(x, layout.width);
    el.style.top = percent(y, layout.height);
    return el;
  }

  const fxReady = (kind) => FX_TEXTURES[kind].every(has);

  function render(minion) {
    const root = doc.createElement("div");
    root.className = minion.golden === true ? "tlm tlm-golden" : "tlm";
    root.setAttribute("aria-hidden", "true");
    let marks = null;
    const addMark = (kind) => {
      if (!marks) {
        marks = doc.createElement("div");
        marks.className = "tlm-marks";
        root.append(marks);
      }
      const word = doc.createElement("span");
      word.className = "tlm-mark";
      word.textContent = markText(kind);
      marks.append(word);
    };
    for (const op of planLayers(minion, layout, has)) {
      if (op.kind === "img") root.append(imageLayer(op));
      else if (op.kind === "art") root.append(artLayer(minion));
      else if (op.kind === "number") root.append(numberLayer(op));
      else if (op.kind === "fx" && WORDED_FX.includes(op.fx)) {
        const spec = layout[op.fx];
        const canvas = doc.createElement("canvas");
        canvas.width = spec.size[0];
        canvas.height = spec.size[1];
        canvas.className = `tlm-fx tlm-fx-${op.fx}`;
        place(canvas, spec.box);
        const swap = () => {
          canvas.remove();
          addMark(op.fx);
        };
        if (fx && fxReady(op.fx) && fx.attach(op.fx, canvas, swap)) root.append(canvas);
        else addMark(op.fx);
      }
    }
    return root;
  }

  return {
    render,
    ready,
    setActive: (on) => fx && fx.setActive(on),
    destroy: () => fx && fx.destroy(),
  };
}

// Minions drawn like the game, in the browser (T-372, D-150, D-153). Client
// only: pages call paintMinions() after they draw minions, never while the
// site is built. The rules (when it loads, the fallbacks, the one warning) are
// in core.ts; this file wires them to the page: the shared drawing
// (crates/desktop/ui/minion-art/, one source with the app), the card pictures,
// the console and the page's own events. The drawing runs Windfury and
// Stealth in one WebGL canvas for the whole page, paused while off screen or
// while the tab is hidden, and everything is let go when the page goes away.
import { FALLBACK_ATTR, pictureFor } from "../data/pictures.ts";
import { g } from "../game/strings.ts";
import { shortStat } from "../stat.ts";
import { type ArtHandle, createMinionCore } from "./core.ts";
import { TLM_CARD } from "./drawn.ts";
import { TEXTURE_LAYOUT, textureUrl } from "./textureSet.ts";

const HOST = `[${TLM_CARD}]`;

/** The card's picture for the drawing's oval, with the placeholder to fall back on. */
function artImage(cardId: string): HTMLImageElement | null {
  let picture;
  try {
    picture = pictureFor("card", cardId);
  } catch {
    return null;
  }
  const img = document.createElement("img");
  img.src = picture.src;
  img.setAttribute(FALLBACK_ATTR, picture.fallback);
  img.alt = "";
  img.loading = "lazy";
  img.decoding = "async";
  return img;
}

/** Whether the set can be reached at all: its frame loads as a CORS-clean image. */
function probe(): Promise<void> {
  return new Promise((resolve, reject) => {
    const url = textureUrl("frame_normal");
    if (!url) return reject(new Error("no frame"));
    const img = document.createElement("img");
    img.crossOrigin = "anonymous";
    img.addEventListener("load", () => resolve());
    img.addEventListener("error", () => reject(new Error("the frame did not load")));
    img.src = url;
  });
}

async function loadDrawing(log: (message: string) => void): Promise<ArtHandle | null> {
  // The drawing and its stylesheet are the app's own module; a page with no minion never loads them.
  const [shared] = await Promise.all([
    import("../../../../crates/desktop/ui/minion-art/minion-art.js"),
    import("../../../../crates/desktop/ui/minion-art/minion-art.css"),
    probe(),
  ]);
  return shared.createMinionArt({
    layout: TEXTURE_LAYOUT,
    imageUrl: textureUrl,
    artImage,
    formatStat: shortStat,
    markText: (keyword: string) => g(keyword === "stealth" ? "keyword.stealth" : "keyword.windfury"),
    log,
  }) as ArtHandle | null;
}

const core = createMinionCore({
  loadDrawing,
  warn: (message) => console.warn(`Tavern Ledger: ${message}`),
  hosts: (root) => [...root.querySelectorAll<HTMLElement>(HOST)],
  allHosts: () => [...document.querySelectorAll<HTMLElement>(HOST)],
  onLeave: (release) => {
    window.addEventListener("pagehide", release);
    // Shown again from the browser's back-forward cache: the drawing was let go, draw it anew.
    window.addEventListener("pageshow", (event) => {
      if (!event.persisted) return;
      for (const slot of document.querySelectorAll(".tlm-slot")) slot.replaceChildren();
      core.paint(document);
    });
  },
});

/**
 * Draws the minions under `root` that carry the attributes of drawn.ts. With
 * the drawing not ready yet it starts loading it (once) and returns; the
 * minions are drawn when it is. Safe to call as often as a page redraws.
 */
export function paintMinions(root: ParentNode = document): void {
  core.paint(root);
}

/** Lets go of the drawing (its canvas, loop, observers and WebGL context). */
export function releaseMinionArt(): void {
  core.release();
}

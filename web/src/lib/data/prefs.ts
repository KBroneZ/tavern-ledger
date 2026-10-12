// Viewer choices on the data pages (T-501). Simple / Advanced is a
// per-viewer convenience kept in this browser's storage; a blocked or
// broken store just means Simple. Solo / Duos lives in the address, so a
// shared link opens the same view; without one, a page with the player's
// games opens on the mode they played most (T-559).

export type Detail = "simple" | "advanced";
export type ModeChoice = "solo" | "duos";

export const DETAIL_KEY = "tl.detail";

export function readDetail(store: Pick<Storage, "getItem"> | null): Detail {
  try {
    return store?.getItem(DETAIL_KEY) === "advanced" ? "advanced" : "simple";
  } catch {
    return "simple";
  }
}

/** True when the choice was kept; false when the browser refused it. */
export function saveDetail(store: Pick<Storage, "setItem"> | null, detail: Detail): boolean {
  try {
    if (!store) return false;
    store.setItem(DETAIL_KEY, detail);
    return true;
  } catch {
    return false;
  }
}

export function parseMode(search: string): ModeChoice {
  return modeInAddress(search) ?? "solo";
}

/** The mode the address names, or null when it names none (or not one of ours). */
export function modeInAddress(search: string): ModeChoice | null {
  const mode = new URLSearchParams(search).get("mode");
  return mode === "solo" || mode === "duos" ? mode : null;
}

/** The mode with more games; Solo on a tie. */
export function busierMode(games: Readonly<Record<ModeChoice, number>>): ModeChoice {
  return games.duos > games.solo ? "duos" : "solo";
}

/** The event a page sends its mode switch once it knows which mode has more games. */
export const MODE_DEFAULT_EVENT = "tl-mode-default";

/** The same address (path, other parameters, hash) with the mode set, so the link opens that mode. */
export function withMode(href: string, mode: ModeChoice): string {
  const url = new URL(href);
  url.searchParams.set("mode", mode);
  return `${url.pathname}${url.search}${url.hash}`;
}

// Viewer choices on the data pages (T-501). Simple / Advanced is a
// per-viewer convenience kept in this browser's storage; a blocked or
// broken store just means Simple. Solo / Duos lives in the address, so a
// shared link opens the same view.

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
  return new URLSearchParams(search).get("mode") === "duos" ? "duos" : "solo";
}

/** The same address (path, other parameters, hash) with the mode set; Solo is the default and leaves none. */
export function withMode(href: string, mode: ModeChoice): string {
  const url = new URL(href);
  if (mode === "duos") url.searchParams.set("mode", "duos");
  else url.searchParams.delete("mode");
  return `${url.pathname}${url.search}${url.hash}`;
}

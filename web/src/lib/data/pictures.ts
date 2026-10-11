// Where hero and card pictures come from: one setting. Since T-504 (D-074)
// they load at view time from HearthstoneJSON's image host, the same 256x
// art the desktop app uses (D-038); the Content-Security-Policy allows that
// one host (img-src). Blizzard art is never bundled in the repo (absolute
// rule 5). A picture that fails to load, or a card the host does not know,
// falls back to a local, obviously synthetic placeholder.

export type PictureKind = "hero" | "card";

export type PictureSource =
  | { kind: "placeholder" }
  /** base ends with "/"; the picture is base + card id + ext. */
  | { kind: "remote"; base: string; ext: string };

/** The image host the CSP allows (D-074); nothing else is loaded from outside. */
export const ART_ORIGIN = "https://art.hearthstonejson.com";

/** The one setting: the app's own art (crates/cards ART_BASE, .jpg). */
export const PICTURE_SOURCE: PictureSource = { kind: "remote", base: `${ART_ORIGIN}/v1/256x/`, ext: ".jpg" };

/**
 * Pages built with made-up data (the style page) use the placeholders, so
 * no built page loads anything from another origin; real games are drawn
 * in the browser with PICTURE_SOURCE.
 */
export const PLACEHOLDER_SOURCE: PictureSource = { kind: "placeholder" };

/** How many placeholder files exist per kind (public/placeholders/). */
const PLACEHOLDERS: Record<PictureKind, number> = { hero: 4, card: 4 };

const CARD_ID = /^[A-Za-z0-9_]{1,64}$/;

/** A small stable hash, so one id always gets the same placeholder. */
function bucket(id: string, size: number): number {
  let h = 0;
  for (const ch of id) h = (h * 31 + ch.charCodeAt(0)) >>> 0;
  return h % size;
}

export interface Picture {
  src: string;
  synthetic: boolean;
  /** The local placeholder to show if src fails to load. */
  fallback: string;
}

export function pictureFor(kind: PictureKind, cardId: string, source: PictureSource = PICTURE_SOURCE): Picture {
  if (!CARD_ID.test(cardId)) throw new Error(`not a card id: ${JSON.stringify(cardId)}`);
  const fallback = `/placeholders/${kind}-${bucket(cardId, PLACEHOLDERS[kind])}.svg`;
  if (source.kind === "remote") return { src: `${source.base}${cardId}${source.ext}`, synthetic: false, fallback };
  return { src: fallback, synthetic: true, fallback };
}

/** The attribute that carries an <img>'s fallback address. */
export const FALLBACK_ATTR = "data-fallback";

/**
 * Swaps a failed picture for its placeholder, once. Only a local
 * /placeholders/ address is accepted, so the attribute cannot point the
 * page anywhere else.
 */
export function useFallback(img: HTMLImageElement): boolean {
  const fallback = img.getAttribute(FALLBACK_ATTR);
  img.removeAttribute(FALLBACK_ATTR);
  if (!fallback || !/^\/placeholders\/(?:hero|card)-\d\.svg$/.test(fallback)) return false;
  img.src = fallback;
  return true;
}

/**
 * One listener for every picture on the page: error events do not bubble,
 * so it listens in the capture phase. No inline handler (the CSP has none).
 */
export function watchPictureFallback(root: Pick<Document, "addEventListener">): void {
  root.addEventListener(
    "error",
    (event) => {
      const target = event.target;
      if (target && (target as Element).tagName === "IMG") useFallback(target as HTMLImageElement);
    },
    true,
  );
}

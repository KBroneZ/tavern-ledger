// Where hero and card pictures come from (T-501): one setting. Card art is
// not allowed by the Content-Security-Policy yet (img-src 'self'; the
// decision waits for T-504), so the site uses local, obviously synthetic
// placeholders. Blizzard art is never bundled in the repo (absolute rule 5).

export type PictureKind = "hero" | "card";

export type PictureSource =
  | { kind: "placeholder" }
  /** base ends with "/"; the picture is base + card id + ext. */
  | { kind: "remote"; base: string; ext: string };

/** The one setting. Changing it to "remote" needs the CSP decision first. */
export const PICTURE_SOURCE: PictureSource = { kind: "placeholder" };

/** How many placeholder files exist per kind (public/placeholders/). */
const PLACEHOLDERS: Record<PictureKind, number> = { hero: 4, card: 4 };

const CARD_ID = /^[A-Za-z0-9_]{1,64}$/;

/** A small stable hash, so one id always gets the same placeholder. */
function bucket(id: string, size: number): number {
  let h = 0;
  for (const ch of id) h = (h * 31 + ch.charCodeAt(0)) >>> 0;
  return h % size;
}

export function pictureFor(
  kind: PictureKind,
  cardId: string,
  source: PictureSource = PICTURE_SOURCE,
): { src: string; synthetic: boolean } {
  if (!CARD_ID.test(cardId)) throw new Error(`not a card id: ${JSON.stringify(cardId)}`);
  if (source.kind === "remote") return { src: `${source.base}${cardId}${source.ext}`, synthetic: false };
  return { src: `/placeholders/${kind}-${bucket(cardId, PLACEHOLDERS[kind])}.svg`, synthetic: true };
}

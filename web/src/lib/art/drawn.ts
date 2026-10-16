// A minion on its way to the shared drawing (crates/desktop/ui/minion-art/).
// The pages build their views as plain element trees (lib/view/node.ts), so a
// minion rides in the tree as a few data attributes on the element that will
// hold its drawing; once the textures are loaded the painter (paint.ts) reads
// them back and draws. Whatever the attributes say is untrusted input: a card
// id that is not shaped like one, a number that is not a number or a keyword
// that is not a word is dropped, never drawn.
import type { Attrs } from "../view/node.ts";

export interface DrawnMinion {
  cardId: string;
  /** null when the record lacks it (the drawing writes "?"). */
  atk: number | null;
  health: number | null;
  golden: boolean;
  keywords: readonly string[];
}

const CARD_ID = /^[A-Za-z0-9_]{1,64}$/;
const KEYWORD = /^[a-z_]{1,32}$/;
const MAX_KEYWORDS = 16;

export const TLM_CARD = "data-tlm-card";
export const TLM_ATK = "data-tlm-atk";
export const TLM_HP = "data-tlm-hp";
export const TLM_GOLD = "data-tlm-gold";
export const TLM_KW = "data-tlm-kw";
export const TLM_ATTRS = [TLM_CARD, TLM_ATK, TLM_HP, TLM_GOLD, TLM_KW] as const;

/**
 * The attributes that carry a minion (an empty value is an unknown number);
 * none for a minion with no card id: it has nothing to draw.
 */
export function drawnAttrs(m: DrawnMinion | null): Attrs {
  if (!m) return {};
  return {
    [TLM_CARD]: m.cardId,
    [TLM_ATK]: m.atk === null ? "" : String(m.atk),
    [TLM_HP]: m.health === null ? "" : String(m.health),
    [TLM_GOLD]: m.golden ? "1" : undefined,
    [TLM_KW]: m.keywords.join(" "),
  };
}

function numberOf(raw: string | undefined | null): number | null {
  if (raw === undefined || raw === null || raw.trim() === "") return null;
  const n = Number(raw);
  return Number.isFinite(n) ? n : null;
}

/** The minion the attributes carry, or null when there is no usable card id. */
export function readDrawn(attrs: Readonly<Record<string, string | undefined | null>>): DrawnMinion | null {
  const cardId = attrs[TLM_CARD];
  if (typeof cardId !== "string" || !CARD_ID.test(cardId)) return null;
  const words = (attrs[TLM_KW] ?? "").split(/\s+/).filter((w) => KEYWORD.test(w));
  return {
    cardId,
    atk: numberOf(attrs[TLM_ATK]),
    health: numberOf(attrs[TLM_HP]),
    golden: attrs[TLM_GOLD] === "1",
    keywords: words.slice(0, MAX_KEYWORDS),
  };
}

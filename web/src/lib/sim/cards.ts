// The simulator's card data in the browser (T-606): lookup by id, the
// golden twin of a card, and the name search of "Add a minion". The file is
// written at build time by tools/prepare-sim.ts from HearthstoneJSON and
// served from the site's own origin (a script chunk, so the CSP needs no
// new connection).
import type { SimCard, SimCardFile } from "./cardFile.ts";

export type { SimCard, SimCardFile };

/** Results shown for one search. */
export const SEARCH_LIMIT = 12;
const MAX_QUERY = 40;

export interface CardFacts {
  tier: number | null;
  tribes: string[];
  attack: number;
  health: number;
  keywords: string[];
}

export class CardIndex {
  readonly source: string;
  readonly size: number;
  private readonly byId: Map<string, SimCard>;
  private readonly normalOf: Map<string, string>;
  private readonly folded: { card: SimCard; name: string }[];

  constructor(file: SimCardFile) {
    this.source = file.source;
    this.byId = new Map(file.cards.map((c) => [c.i, c]));
    this.size = this.byId.size;
    this.normalOf = new Map(file.cards.filter((c) => c.g).map((c) => [c.g as string, c.i]));
    // Golden cards are reached with the golden switch, not by search.
    this.folded = file.cards.filter((c) => !this.normalOf.has(c.i)).map((card) => ({ card, name: fold(card.n) }));
  }

  get(id: string): SimCard | undefined {
    return this.byId.get(id);
  }

  name(id: string): string {
    return this.byId.get(id)?.n ?? id;
  }

  /** The facts the engine needs, as the request carries them; null when the data lacks the card. */
  facts(id: string): CardFacts | null {
    const c = this.byId.get(id);
    return c ? { tier: c.t, tribes: c.r, attack: c.a, health: c.h, keywords: c.k } : null;
  }

  isGolden(id: string): boolean {
    return this.normalOf.has(id);
  }

  /** The card's golden or normal twin, or null when the data has none. */
  twin(id: string): string | null {
    return this.byId.get(id)?.g ?? this.normalOf.get(id) ?? null;
  }

  /** Cards whose name contains every word of the query; tavern cards first, then by tier and name. */
  search(query: string): SimCard[] {
    const words = fold(query.slice(0, MAX_QUERY)).split(/\s+/).filter(Boolean);
    if (words.length === 0) return [];
    return this.folded
      .filter((x) => words.every((w) => x.name.includes(w)))
      .map((x) => x.card)
      .toSorted((a, b) => (b.p ?? 0) - (a.p ?? 0) || (a.t ?? 9) - (b.t ?? 9) || a.n.localeCompare(b.n))
      .slice(0, SEARCH_LIMIT);
  }
}

/** Lower case without accents, so "cabeza" finds "Cabeza" and "esbirro" finds "Esbírro". */
export function fold(text: string): string {
  return text.normalize("NFD").replace(/\p{M}/gu, "").toLowerCase();
}

/** A file that does not look like ours is treated as empty, never half-read. */
export function readCardFile(value: unknown): SimCardFile {
  const v = value as Partial<SimCardFile> | null;
  if (!v || v.version !== 1 || !Array.isArray(v.cards) || typeof v.source !== "string") return { version: 1, source: "none", cards: [] };
  return v as SimCardFile;
}

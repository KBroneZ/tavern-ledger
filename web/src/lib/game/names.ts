// Card names on the game page (T-559): the game's record keeps card ids
// only, so the page names cards from the card data the build writes (the
// simulator's minions, and names.<lang>.json for spells, trinkets and the
// rest), in the page's language, plus the Dark Gift table. The page loads
// the table once before it draws the game (loadCardNames); a card the data
// does not know, or a page drawn before the table arrived, shows its id.
import { lang } from "../i18n.ts";
import type { CardNameFile } from "../sim/cardFile.ts";
import { CardIndex, readCardFile } from "../sim/cards.ts";
import { giftName } from "./gifts.ts";

const CARD_ID = /^[A-Za-z0-9_]{1,64}$/;
const MAX_NAME = 100;

export interface CardNames {
  /** The minions, for the engine's facts as well as their names. */
  cards: CardIndex | null;
  names: ReadonlyMap<string, string>;
  /** A trinket's card database id to its card id. */
  trinkets: ReadonlyMap<number, string>;
}

const NONE: CardNames = { cards: null, names: new Map(), trinkets: new Map() };
let table: CardNames = NONE;

/** The names the page uses from now on (the loaded table, or one a test builds). */
export function useCardNames(names: CardNames | null): void {
  table = names ?? NONE;
}

/** The table in use (the "why" request reads the minions' facts from it). */
export function cardNames(): CardNames {
  return table;
}

/** A card's name in the page's language; null when the data does not know it. */
export function cardName(id: string | null): string | null {
  if (id === null) return null;
  const minion = table.cards?.get(id)?.n;
  if (minion) return minion;
  const other = table.names.get(id);
  if (other) return other;
  const gift = giftName(id);
  return gift === id ? null : gift;
}

/** The name, else the id: what a picture's text and a step's words say. */
export function cardLabel(id: string | null, unknown: string): string {
  return id === null ? unknown : (cardName(id) ?? id);
}

/** A trinket named by its card database id, as the lobby keeps it; null when unknown. */
export function trinketName(dbfId: number): string | null {
  const id = table.trinkets.get(dbfId);
  return id ? cardName(id) : null;
}

/**
 * The name file as read: an entry that is not an id and a short name is
 * dropped (the file is the site's own, but it is read like any data).
 */
export function readNameFile(value: unknown): Pick<CardNames, "names" | "trinkets"> {
  const file = (typeof value === "object" && value !== null ? value : {}) as Partial<CardNameFile>;
  const names = new Map<string, string>();
  for (const [id, name] of Object.entries(file.names ?? {})) {
    if (CARD_ID.test(id) && typeof name === "string" && name.length > 0 && name.length <= MAX_NAME) names.set(id, name);
  }
  const trinkets = new Map<number, string>();
  for (const [dbf, id] of Object.entries(file.trinkets ?? {})) {
    const n = Number(dbf);
    if (Number.isInteger(n) && n > 0 && typeof id === "string" && names.has(id)) trinkets.set(n, id);
  }
  return { names, trinkets };
}

/** Loads both files for the page's language; null when either cannot be loaded (ids show). */
export async function loadCardNames(): Promise<CardNames | null> {
  try {
    const es = lang() === "es";
    const [cards, others] = await Promise.all([
      es ? import("../sim/generated/cards.es.json") : import("../sim/generated/cards.en.json"),
      es ? import("../sim/generated/names.es.json") : import("../sim/generated/names.en.json"),
    ]);
    return { cards: new CardIndex(readCardFile(cards.default)), ...readNameFile(others.default) };
  } catch {
    return null;
  }
}

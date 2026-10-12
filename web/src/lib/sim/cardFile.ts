// The simulator's card data file (T-606): written at build time by
// tools/prepare-sim.ts, read by the page (cards.ts).

/** One minion as the page reads it (short keys: the file ships to every visitor). */
export interface SimCard {
  /** Card id. */
  i: string;
  /** Name, in the file's language. */
  n: string;
  /** Tavern tier; null when the data has none (the damage is then unknown). */
  t: number | null;
  a: number;
  h: number;
  /** Types as the log names them (BEAST, MECHANICAL, ALL). */
  r: string[];
  /** Printed keywords of COMBAT_KEYWORDS. */
  k: string[];
  /** 1 when the tavern offers it (search lists these first). */
  p?: 1;
  /** The golden card's id. */
  g?: string;
}

export interface SimCardFile {
  version: 1;
  /** Where it came from: HearthstoneJSON's build number, or "fixture". */
  source: string;
  cards: SimCard[];
}

/**
 * The names of the Battlegrounds cards that are not minions (tavern spells,
 * trinkets, quest rewards, anomalies, hero powers): written at build time
 * next to the minions, read by the game page only (T-559).
 */
export interface CardNameFile {
  version: 1;
  source: string;
  /** Card id to name, in the file's language. */
  names: Record<string, string>;
  /** A trinket's card database id to its card id (the lobby names trinkets by database id). */
  trinkets: Record<string, string>;
}

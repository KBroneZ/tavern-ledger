// The minion pages' data file (T-578, D-134): written at build time by
// tools/prepare-sim.ts (tools/minionPool.ts), read when the pages are built.
// It never ships to the browser: the pages are plain HTML.

/** A run that is a line break (a keyword alone on its line). */
export const NEW_LINE = "\n";

/** A piece of card text: bold and italic as the card prints them, or NEW_LINE. */
export interface TextRun {
  s: string;
  b?: 1;
  i?: 1;
}

export interface PoolMinion {
  id: string;
  /** Name, in the file's language. */
  name: string;
  /** Tavern tier, 1 to 7. */
  tier: number;
  attack: number;
  health: number;
  /** Types as the card data names them (BEAST, MECHANICAL, ALL); empty for none. */
  tribes: string[];
  /** Card text in the file's language; null when the data has none we can show. */
  text: TextRun[] | null;
  /** The golden card's stats, when the data names a golden card. */
  golden?: { attack: number; health: number };
  /** Offered in Duos only. */
  duosOnly?: true;
}

export interface PoolFile {
  version: 1;
  /** Where it came from: HearthstoneJSON's build number, "fixture", or none (accounts closed). */
  source: string;
  minions: PoolMinion[];
}

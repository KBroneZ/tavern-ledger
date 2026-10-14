// Buffs on a minion (parser revision 14, T-619, D-142): the card that gave
// them and how many of its enchantments the minion has, as the game's log
// shows them (crates/bg-parser/src/buffs.rs). The log does not say how much
// attack and health each one gave, so only the count is known.

export interface Buff {
  cardId: string;
  count: number;
}

/** The first parser revision that reads buffs; a record of an older one does not know them. */
export const BUFFS_REVISION = 14;
/** The parser's bounds (crates/bg-parser/src/buffs.rs), as the server checks them. */
export const MAX_BUFF_KINDS = 12;
export const MAX_BUFF_COUNT = 999;

/** Buffs as a record of `revision` holds them: unknown (null) before revision 14. */
export const buffsRecorded = (revision: number | null): boolean => revision !== null && revision >= BUFFS_REVISION;

/** The buffs with each card once (the first count stays), in order. */
export function oneEachCard(buffs: readonly Buff[]): Buff[] {
  const seen = new Set<string>();
  return buffs.filter((b) => !seen.has(b.cardId) && seen.add(b.cardId));
}

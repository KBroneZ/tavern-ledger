// The simulator's hero powers and trinkets (T-628, D-157): what each board
// brings beside its minions, the edits on them, their part of the engine's
// request and how a saved round gives them (parser revision 16). Plain data
// in, new data out: nothing here touches the page.
import needsTable from "./kitNeeds.json" with { type: "json" };
import type { KitCard as ReportKitCard, Kit as ReportKit, Minion as ReportMinion } from "../game/report.ts";
import type { SimMinion } from "./board.ts";

/**
 * What a hero power the engine plays needs beside the board
 * (crates/combat-sim kit_effects.rs; a Rust test keeps this table in step):
 * the minion it names, a minion it holds, the turn, the teammate's board,
 * and "first" when it acts only while its first number is not 0.
 */
export type KitNeed = "target" | "held" | "round" | "teammate" | "first";
const NEEDS = needsTable as Readonly<Record<string, readonly KitNeed[]>>;

export const needs = (cardId: string): readonly KitNeed[] => NEEDS[cardId] ?? [];

export interface SimKitCard {
  cardId: string;
  /** Hero powers: 0 the hero's own, 1 a second one. Trinkets: 1 lesser, 2 greater, 3 a hero power become a trinket. */
  slot: number;
  /** The five numbers its text shows (TAG_SCRIPT_DATA_NUM_1..5); 0 where none is given. */
  counters: number[];
  /** Hero powers: the minion it names, by its key on the same board; null when none. */
  target: number | null;
  /** Hero powers: the minion it holds off the board (Lock and Load); null when none. */
  held: SimMinion | null;
}

export interface SimKit {
  powers: SimKitCard[];
  trinkets: SimKitCard[];
}

export type KitKind = "power" | "trinket";

/** The slots the editor offers: the hero's own power and a second one; lesser, greater and a third trinket. */
export const POWER_SLOTS = [0, 1] as const;
export const TRINKET_SLOTS = [1, 2, 3] as const;
/** The turn some hero powers unlock at (Frostwolf Fervor, Stormpike Strength). */
export const UNLOCK_TURN = 7;
const KIT_NUMBERS = 5;

export const emptyKit = (): SimKit => ({ powers: [], trinkets: [] });

const listOf = (kind: KitKind): keyof SimKit => (kind === "power" ? "powers" : "trinkets");

/** The card in a slot, if any. */
export function inSlot(kit: SimKit | null, kind: KitKind, slot: number): SimKitCard | undefined {
  return kit?.[listOf(kind)].find((c) => c.slot === slot);
}

/**
 * A new card in a slot. A power that acts only while its first number is not
 * 0 gets 1 there (it is on until the page says otherwise), except one that
 * holds a minion: its number follows the minion (none held, it does not act).
 */
function newCard(cardId: string, slot: number): SimKitCard {
  const n = needs(cardId);
  const first = n.includes("first") && !n.includes("held") ? 1 : 0;
  return { cardId, slot, counters: [first, 0, 0, 0, 0], target: null, held: null };
}

/** The kit with `cardId` in the slot (or the slot emptied); a kit not recorded becomes one. */
export function withCard(kit: SimKit | null, kind: KitKind, slot: number, cardId: string | null): SimKit {
  const base = kit ?? emptyKit();
  const key = listOf(kind);
  const now = inSlot(base, kind, slot);
  if ((now?.cardId ?? null) === cardId) return base;
  const rest = base[key].filter((c) => c.slot !== slot);
  const list = cardId === null ? rest : [...rest, newCard(cardId, slot)].sort((a, b) => a.slot - b.slot);
  return { ...base, [key]: list };
}

function withPower(kit: SimKit, slot: number, change: (c: SimKitCard) => SimKitCard): SimKit {
  return { ...kit, powers: kit.powers.map((c) => (c.slot === slot ? change(c) : c)) };
}

/** The minion a hero power names, by its key (null: none). */
export function withTarget(kit: SimKit, slot: number, key: number | null): SimKit {
  return withPower(kit, slot, (c) => ({ ...c, target: key }));
}

/** The minion a hero power holds; its first number follows (1 while it holds one not fired yet). */
export function withHeld(kit: SimKit, slot: number, held: SimMinion | null): SimKit {
  return withPower(kit, slot, (c) => ({
    ...c,
    held,
    counters: needs(c.cardId).includes("first") ? [held ? 1 : 0, ...c.counters.slice(1)] : c.counters,
  }));
}

/** A minion left the board: a hero power that named it names none. */
export function withoutTarget(kit: SimKit | null, key: number): SimKit | null {
  if (!kit?.powers.some((c) => c.target === key)) return kit;
  return { ...kit, powers: kit.powers.map((c) => (c.target === key ? { ...c, target: null } : c)) };
}

/** Every card of the kit, powers first. */
export const kitCards = (kit: SimKit | null): SimKitCard[] => (kit ? [...kit.powers, ...kit.trinkets] : []);

// ---------- The engine's request (crates/sim-wasm input.rs) ----------

/**
 * The kit as the request carries it: a named minion becomes its board
 * position (1 is the left-most), numbers only when one is not 0.
 */
export function kitRequest(kit: SimKit, minions: readonly SimMinion[], minion: (m: SimMinion) => unknown): unknown {
  const card = (c: SimKitCard, power: boolean) => {
    const at = c.target === null ? -1 : minions.findIndex((m) => m.key === c.target);
    return {
      card_id: c.cardId,
      slot: c.slot,
      ...(c.counters.some((n) => n !== 0) ? { counters: c.counters } : {}),
      ...(power && at >= 0 ? { target: at + 1 } : {}),
      ...(power && c.held ? { held: minion(c.held) } : {}),
    };
  };
  return { powers: kit.powers.map((c) => card(c, true)), trinkets: kit.trinkets.map((c) => card(c, false)) };
}

// ---------- A saved round (parser revision 16) ----------

/**
 * A board's kit from the record; `held` makes a held minion the editor's.
 * A named minion is not kept: its position is on the board before start of
 * combat, and the board loaded is the one at the first attack (it may be
 * gone); start of combat is off for it anyway.
 */
export function kitFromRecord(kit: ReportKit, held: (m: ReportMinion) => SimMinion | null): SimKit {
  const card = (c: ReportKitCard): SimKitCard => ({
    cardId: c.cardId,
    slot: c.slot,
    counters: c.counters.length === KIT_NUMBERS ? [...c.counters] : new Array<number>(KIT_NUMBERS).fill(0),
    target: null,
    held: c.held ? held(c.held) : null,
  });
  return { powers: kit.powers.map(card), trinkets: kit.trinkets.map(card) };
}

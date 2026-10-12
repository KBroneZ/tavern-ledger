// Reads one round's combat replay (T-536, D-099; parser revision 8) as
// untrusted input, like the rest of the record (D-079). Unlike the record's
// other values, a replay is all or nothing: a minion or a step outside the
// server's own check (supabase/functions/upload-game/replay.ts) makes the
// whole replay "not available", because a step read wrong would play a
// combat that did not happen. Two things a newer app may write are not
// malformed: keys and keywords this site does not know are ignored, and a
// kind of step (or trigger) it does not know is kept as "unknown", which the
// player leaves out and counts, as the app does.
//
// Lists are refused on their length before any item is read, so an
// oversized file costs nothing to turn down.
import { KEYWORDS, type Keyword } from "./keywords.ts";

/** The parser's bounds (crates/bg-parser/src/replay.rs), as the server checks them. */
export const MAX_REPLAY_MINIONS = 200;
export const MAX_REPLAY_EVENTS = 2000;
const MAX_INT = 2 ** 31 - 1;
const MAX_HEALTH = 100_000;
const MAX_SLOT = 64;
// The parser keeps at most 4 Dark Gifts on a minion (crates/bg-parser/src/gifts.rs).
const MAX_GIFTS = 4;
const CARD_ID = /^[A-Za-z0-9_]{1,64}$/;

export type Side = "own" | "opponent";
export const SIDES: readonly Side[] = ["own", "opponent"];
export const TRIGGERS = ["start_of_combat", "deathrattle", "avenge", "rally"] as const;
export type Trigger = (typeof TRIGGERS)[number];

/** A minion as it enters the replay; the events name it by its index. */
export interface ReplayMinion {
  side: Side;
  cardId: string | null;
  atk: number;
  health: number;
  /** Board slot when it entered (1 is the left-most; 0 when the log has none). */
  position: number;
  golden: boolean;
  keywords: Keyword[];
  gifts: string[];
  giftUnknown: boolean;
}

export type ReplayEvent =
  | { kind: "attack"; attacker: number; defender: number | null }
  | { kind: "damage"; minion: number; amount: number; health: number }
  | { kind: "shield"; minion: number; on: boolean }
  | { kind: "stats"; minion: number; atk: number; health: number }
  | { kind: "death"; minion: number }
  | { kind: "summon"; minion: number }
  | { kind: "trigger"; trigger: Trigger; side: Side; cardId: string; source: number | null }
  | { kind: "hero_damage"; side: Side; amount: number }
  | { kind: "board_in"; side: Side; hero: string | null }
  /** A step written by a newer app than this site knows. */
  | { kind: "unknown" };

/** One combat: the first `start` minions are both boards as it starts. */
export interface Replay {
  minions: ReplayMinion[];
  start: number;
  events: ReplayEvent[];
  /** False when the log stops before the combat ends, or it had more steps than are kept. */
  complete: boolean;
}

export type ReplayRead = { ok: true; replay: Replay } | { ok: false };

type Obj = Record<string, unknown>;

/** Thrown inside the reader only: the replay is not available. */
class Malformed extends Error {}

const isObject = (v: unknown): v is Obj => typeof v === "object" && v !== null && !Array.isArray(v);

function obj(v: unknown): Obj {
  if (!isObject(v)) throw new Malformed();
  return v;
}

function int(v: unknown, min: number, max: number): number {
  if (typeof v !== "number" || !Number.isInteger(v) || v < min || v > max) throw new Malformed();
  return v;
}

function bool(v: unknown): boolean {
  if (typeof v !== "boolean") throw new Malformed();
  return v;
}

function side(v: unknown): Side {
  if (v !== "own" && v !== "opponent") throw new Malformed();
  return v;
}

/** A card id, or null when the value is null (or missing, when `optional`). */
function card(v: unknown, optional = true): string | null {
  if (v === null || (optional && v === undefined)) return null;
  if (typeof v !== "string" || !CARD_ID.test(v)) throw new Malformed();
  return v;
}

function list(v: unknown, max: number): unknown[] {
  if (!Array.isArray(v) || v.length > max) throw new Malformed();
  return v;
}

function keywords(v: unknown): Keyword[] {
  const words = list(v, 2 * KEYWORDS.length);
  if (words.some((w) => typeof w !== "string")) throw new Malformed();
  return KEYWORDS.filter((k) => words.includes(k));
}

function gifts(m: Obj): string[] {
  if (m.gifts === undefined || m.gifts === null) return [];
  const ids = list(m.gifts, MAX_GIFTS).map((g) => card(g, false) as string);
  return [...new Set(ids)];
}

function minion(v: unknown): ReplayMinion {
  const m = obj(v);
  return {
    side: side(m.side),
    cardId: card(m.card_id),
    atk: int(m.atk, -MAX_INT, MAX_INT),
    health: int(m.health, -MAX_INT, MAX_INT),
    position: int(m.position, 0, MAX_SLOT),
    golden: bool(m.golden),
    keywords: keywords(m.keywords),
    gifts: gifts(m),
    giftUnknown: m.gift_unknown === undefined || m.gift_unknown === null ? false : bool(m.gift_unknown),
  };
}

/**
 * The events, checked the server's way: a minion past the starting boards
 * may be named only after its summon, and is summoned once.
 */
function events(raw: unknown[], count: number, start: number): ReplayEvent[] {
  const arrived = new Set<number>();
  const known = (n: unknown) => {
    const i = int(n, 0, count - 1);
    if (i >= start && !arrived.has(i)) throw new Malformed();
    return i;
  };
  return raw.map((x): ReplayEvent => {
    const e = obj(x);
    if (typeof e.kind !== "string") throw new Malformed();
    switch (e.kind) {
      case "attack":
        if (e.defender === undefined) throw new Malformed();
        return { kind: "attack", attacker: known(e.attacker), defender: e.defender === null ? null : known(e.defender) };
      case "damage":
        return { kind: "damage", amount: int(e.amount, 1, MAX_INT), health: int(e.health, -MAX_INT, MAX_INT), minion: known(e.minion) };
      case "shield":
        return { kind: "shield", on: bool(e.on), minion: known(e.minion) };
      case "stats":
        return { kind: "stats", atk: int(e.atk, -MAX_INT, MAX_INT), health: int(e.health, -MAX_INT, MAX_INT), minion: known(e.minion) };
      case "death":
        return { kind: "death", minion: known(e.minion) };
      case "summon": {
        const m = int(e.minion, start, count - 1);
        if (arrived.has(m)) throw new Malformed();
        arrived.add(m);
        return { kind: "summon", minion: m };
      }
      case "trigger": {
        if (typeof e.trigger !== "string") throw new Malformed();
        const s = side(e.side);
        const cardId = card(e.card_id, false);
        if (cardId === null) throw new Malformed();
        const source = e.source === null ? null : known(e.source);
        const trigger = TRIGGERS.find((t) => t === e.trigger);
        return trigger ? { kind: "trigger", trigger, side: s, cardId, source } : { kind: "unknown" };
      }
      case "hero_damage":
        return { kind: "hero_damage", side: side(e.side), amount: int(e.amount, 1, MAX_HEALTH) };
      case "board_in":
        return { kind: "board_in", side: side(e.side), hero: card(e.hero) };
      default:
        return { kind: "unknown" };
    }
  });
}

/** One round's replay: null when it has none, not ok when it cannot be read. */
export function readReplay(v: unknown): ReplayRead | null {
  if (v === undefined || v === null) return null;
  try {
    const r = obj(v);
    const minions = list(r.minions, MAX_REPLAY_MINIONS).map(minion);
    const raw = list(r.events, MAX_REPLAY_EVENTS);
    const start = int(r.start, 0, minions.length);
    const complete = bool(r.complete);
    return { ok: true, replay: { minions, start, events: events(raw, minions.length, start), complete } };
  } catch (error) {
    if (error instanceof Malformed) return { ok: false };
    throw error;
  }
}

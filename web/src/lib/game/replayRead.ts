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
//
// Parser revision 13 (T-575) adds three things, each optional: the beats
// (how many steps each log block gave, so the page plays them at once), each
// damage's source, and keyword steps (a keyword given or used mid-combat).
import { type Buff, MAX_BUFF_COUNT, MAX_BUFF_KINDS, oneEachCard } from "./buffs.ts";
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
/** How a damage came about (revision 13). */
export const CAUSES = ["attack", "start_of_combat", "deathrattle", "avenge", "rally", "hero_power", "trinket", "effect"] as const;
export type Cause = (typeof CAUSES)[number];

/** Where a damage came from: a minion of the replay, or a card (a hero power, a trinket) on a side. */
export type DamageSource =
  | { how: Cause; minion: number }
  | { how: Cause; cardId: string; side: Side };

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
  /** Buffs by card (revision 14); null when the record is older and did not read them. */
  buffs: Buff[] | null;
}

export type ReplayEvent =
  | { kind: "attack"; attacker: number; defender: number | null }
  /** `source`: undefined in a record older than revision 13, null when the log does not say. */
  | { kind: "damage"; minion: number; amount: number; health: number; source?: DamageSource | null }
  | { kind: "shield"; minion: number; on: boolean }
  | { kind: "stats"; minion: number; atk: number; health: number }
  | { kind: "death"; minion: number }
  | { kind: "summon"; minion: number }
  | { kind: "trigger"; trigger: Trigger; side: Side; cardId: string; source: number | null }
  | { kind: "hero_damage"; side: Side; amount: number }
  | { kind: "board_in"; side: Side; hero: string | null }
  /** A keyword other than Divine Shield given (`on`) or lost mid-combat (revision 13). */
  | { kind: "keyword"; minion: number; keyword: Keyword; on: boolean }
  /** A step written by a newer app than this site knows. */
  | { kind: "unknown" };

/** One combat: the first `start` minions are both boards as it starts. */
export interface Replay {
  minions: ReplayMinion[];
  start: number;
  events: ReplayEvent[];
  /**
   * How many events each beat holds, in order (revision 13): the steps the
   * game plays at once. null in an older record: each event is a beat.
   */
  beats: number[] | null;
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

function buffs(m: Obj, recorded: boolean): Buff[] | null {
  if (!recorded) return null;
  if (m.buffs === undefined || m.buffs === null) return [];
  const items = list(m.buffs, MAX_BUFF_KINDS).map((item) => {
    const o = obj(item);
    return { cardId: card(o.card_id, false) as string, count: int(o.count, 1, MAX_BUFF_COUNT) };
  });
  return oneEachCard(items);
}

function minion(v: unknown, recorded: boolean): ReplayMinion {
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
    buffs: buffs(m, recorded),
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
      case "damage": {
        const damage: Extract<ReplayEvent, { kind: "damage" }> = { kind: "damage", amount: int(e.amount, 1, MAX_INT), health: int(e.health, -MAX_INT, MAX_INT), minion: known(e.minion) };
        if (e.source !== undefined) damage.source = e.source === null ? null : source(e.source, known);
        return damage;
      }
      case "keyword": {
        const minion = known(e.minion);
        const on = bool(e.on);
        if (typeof e.keyword !== "string") throw new Malformed();
        // A keyword this site does not know (a newer app's) is a step it leaves out.
        const keyword = KEYWORDS.find((k) => k === e.keyword && k !== "divine_shield");
        return keyword ? { kind: "keyword", minion, keyword, on } : { kind: "unknown" };
      }
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

/**
 * A damage's source: a minion named by its index, or a card on a side; null
 * for a cause a newer app names that this site does not know (an unknown
 * source, as a step of an unknown kind is left out rather than breaking the
 * replay).
 */
function source(v: unknown, known: (n: unknown) => number): DamageSource | null {
  const s = obj(v);
  if (typeof s.how !== "string") throw new Malformed();
  const how = CAUSES.find((c) => c === s.how);
  let named: DamageSource | null;
  if (s.minion !== undefined) {
    named = how ? { how, minion: known(s.minion) } : (known(s.minion), null);
  } else {
    const cardId = card(s.card_id, false);
    if (cardId === null) throw new Malformed();
    const at = side(s.side);
    named = how ? { how, cardId, side: at } : null;
  }
  return named;
}

/** The beats: each at least one event, adding up to the events; null when the record has none. */
function beats(v: unknown, events: number): number[] | null {
  if (v === undefined || v === null) return null;
  const runs = list(v, MAX_REPLAY_EVENTS).map((n) => int(n, 1, MAX_REPLAY_EVENTS));
  if (runs.reduce((a, b) => a + b, 0) !== events) throw new Malformed();
  return runs;
}

/** One round's replay: null when it has none, not ok when it cannot be read. */
/** `buffsRecorded`: the record's parser read buffs (revision 14), so a minion without any has none. */
export function readReplay(v: unknown, buffsRecorded = false): ReplayRead | null {
  if (v === undefined || v === null) return null;
  try {
    const r = obj(v);
    const minions = list(r.minions, MAX_REPLAY_MINIONS).map((m) => minion(m, buffsRecorded));
    const raw = list(r.events, MAX_REPLAY_EVENTS);
    const start = int(r.start, 0, minions.length);
    const complete = bool(r.complete);
    return { ok: true, replay: { minions, start, events: events(raw, minions.length, start), beats: beats(r.beats, raw.length), complete } };
  } catch (error) {
    if (error instanceof Malformed) return { ok: false };
    throw error;
  }
}

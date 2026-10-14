// Why each combat went as it did (T-549, D-127), worked out in the browser
// by the site's own engine (the simulator's Web Worker) from the game's
// stored report: the odds on the boards as they fought and, when the most
// likely result did not happen, the cause, with the same ids as the accuracy
// page's cause list and the same rules as the app's recap. This file builds
// the request and reads the answer as untrusted input: a field out of shape
// makes the whole answer unreadable, never a half-shown line.
import { CAUSE_IDS, type CauseId } from "../accuracy/data.ts";
import type { CardFacts } from "../sim/cards.ts";

export type WhyResult = "won" | "lost" | "tie";
export type NoOdds = "no_boards" | "no_card_data" | "not_modelled" | "bad_board";
export type Unseen = "blood_gems" | "spells" | "knights" | "beetles" | "hand" | "earlier" | "gift";
export type ReplayGap = "not_saved" | "unreadable" | "stops";

export interface WhyCause {
  /** A miss's id from the shared list, or "matched". */
  id: CauseId | "matched";
  /** Chance: the attacks the replay followed. */
  attacks: number | null;
  /** Where the replay parts: the attack (1-based), the rule's id and the card. */
  attack: number | null;
  rule: CauseId | null;
  card: string | null;
  inputs: { what: Unseen; card: string }[];
  cards: string[];
  replay: ReplayGap | null;
}

export interface WhyCombat {
  round: number;
  result: WhyResult;
  odds: { won: number; tied: number; lost: number } | null;
  noOdds: NoOdds | null;
  cards: string[];
  /** Why the simulator refused each card of `cards` (only when it did). */
  reasons: { cardId: string; reason: string }[];
  cause: WhyCause | null;
}

const RESULTS: readonly string[] = ["won", "lost", "tie"];
const NO_ODDS: readonly string[] = ["no_boards", "no_card_data", "not_modelled", "bad_board"];
const UNSEEN: readonly string[] = ["blood_gems", "spells", "knights", "beetles", "hand", "earlier", "gift"];
const REPLAY: readonly string[] = ["not_saved", "unreadable", "stops"];
const IDS: readonly string[] = [...CAUSE_IDS, "matched"];
const CARD_ID = /^[A-Za-z0-9_]{1,64}$/;
/** Far above a game (about 25 rounds) or a board's cards. */
const MAX_COMBATS = 200;
const MAX_LIST = 64;

class Bad extends Error {}
type Obj = Record<string, unknown>;

function obj(v: unknown): Obj {
  if (typeof v !== "object" || v === null || Array.isArray(v)) throw new Bad();
  return v as Obj;
}

function oneOf<T extends string>(v: unknown, set: readonly string[]): T {
  if (typeof v !== "string" || !set.includes(v)) throw new Bad();
  return v as T;
}

function maybe<T>(v: unknown, read: (v: unknown) => T): T | null {
  return v === undefined || v === null ? null : read(v);
}

/** Turns and attacks: far above the engine's own limit on attacks (1 000) and the server's on replay events (2 000). */
function int(v: unknown, max = 9_999): number {
  if (typeof v !== "number" || !Number.isInteger(v) || v < 0 || v > max) throw new Bad();
  return v;
}

function share(v: unknown): number {
  if (typeof v !== "number" || !Number.isFinite(v) || v < 0 || v > 1) throw new Bad();
  return v;
}

function cardId(v: unknown): string {
  if (typeof v !== "string" || !CARD_ID.test(v)) throw new Bad();
  return v;
}

function reasonOf(v: unknown): { cardId: string; reason: string } {
  const r = obj(v);
  if (typeof r.reason !== "string" || r.reason.length > 300) throw new Bad();
  return { cardId: cardId(r.card_id), reason: r.reason };
}

function list<T>(v: unknown, read: (v: unknown) => T): T[] {
  if (v === undefined) return [];
  if (!Array.isArray(v) || v.length > MAX_LIST) throw new Bad();
  return v.map(read);
}

function cause(v: unknown): WhyCause {
  const c = obj(v);
  const rule = maybe(c.rule, (r) => oneOf<CauseId>(r, CAUSE_IDS));
  if (rule !== null && !rule.startsWith("replay_")) throw new Bad();
  return {
    id: oneOf<CauseId | "matched">(c.id, IDS),
    attacks: maybe(c.attacks, (n) => int(n)),
    attack: maybe(c.attack, (n) => int(n)),
    rule,
    card: maybe(c.card, cardId),
    inputs: list(c.inputs, (i) => {
      const o = obj(i);
      return { what: oneOf<Unseen>(o.what, UNSEEN), card: cardId(o.card) };
    }),
    cards: list(c.cards, cardId),
    replay: maybe(c.replay, (r) => oneOf<ReplayGap>(r, REPLAY)),
  };
}

function combat(v: unknown): WhyCombat {
  const c = obj(v);
  const odds = maybe(c.odds, (o) => {
    const x = obj(o);
    return { won: share(x.won), tied: share(x.tied), lost: share(x.lost) };
  });
  const noOdds = maybe(c.no_odds, (n) => oneOf<NoOdds>(n, NO_ODDS));
  // Odds or a reason for none, never both or neither.
  if ((odds === null) === (noOdds === null)) throw new Bad();
  return {
    round: int(c.round),
    result: oneOf<WhyResult>(c.result, RESULTS),
    odds,
    noOdds,
    cards: list(c.cards, cardId),
    reasons: list(c.reasons, reasonOf),
    cause: maybe(c.cause, cause),
  };
}

/** The engine's answer (JSON text): every combat in round order, or null when it is not one. */
export function readWhy(text: string): WhyCombat[] | null {
  try {
    const a = obj(JSON.parse(text));
    if (a.status !== "explained" || !Array.isArray(a.combats) || a.combats.length > MAX_COMBATS) return null;
    return a.combats.map(combat);
  } catch {
    return null;
  }
}

/** Every card id on the report's boards and in its replays (what the engine needs facts for). */
export function cardIdsOf(report: unknown): string[] {
  const ids = new Set<string>();
  const add = (m: unknown) => {
    const id = (m as { card_id?: unknown } | null)?.card_id;
    if (typeof id === "string" && CARD_ID.test(id)) ids.add(id);
  };
  const rounds = (report as { rounds?: unknown } | null)?.rounds;
  for (const round of Array.isArray(rounds) ? rounds : []) {
    const r = round as { entries?: unknown; replay?: { minions?: unknown } | null };
    for (const e of Array.isArray(r.entries) ? r.entries : []) {
      const board = (e as { board?: unknown } | null)?.board;
      if (Array.isArray(board)) board.forEach(add);
    }
    const minions = r.replay?.minions;
    if (Array.isArray(minions)) minions.forEach(add);
  }
  return [...ids].sort();
}

/** The request for the engine: the stored report as it is, and the facts of its cards (null when the data lacks one). */
export function explainRequest(report: unknown, facts: (id: string) => CardFacts | null): string {
  const known: Record<string, CardFacts | null> = {};
  for (const id of cardIdsOf(report)) known[id] = facts(id);
  return JSON.stringify({ explain: { report, facts: known } });
}

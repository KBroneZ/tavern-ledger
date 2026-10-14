// The simulator's boards (T-606): what the editor holds, how a round of one
// of the player's games becomes boards, the edits, and the engine's request.
// Plain data in, new data out: nothing here touches the page.
//
// Red line (D-004, D-054): a round gives only the boards that fought in it,
// the ones the player saw on screen. The saved report never holds the
// combats the game does not play back (T-213 left them out), so no
// SETASIDE board can come in this way.
import type { Buff } from "../game/buffs.ts";
import type { GameRecord, GameValues, Minion as ReportMinion, Round } from "../game/report.ts";
import type { CardIndex } from "./cards.ts";

/** The keywords the engine tracks, by the names the request uses. */
export const SIM_KEYWORDS = ["taunt", "divine_shield", "reborn", "venomous", "poisonous", "windfury", "mega_windfury", "stealth"] as const;
export type SimKeyword = (typeof SIM_KEYWORDS)[number];
/** The card data's spelling of each (HearthstoneJSON mechanics). */
const TAG: Record<SimKeyword, string> = {
  taunt: "TAUNT",
  divine_shield: "DIVINE_SHIELD",
  reborn: "REBORN",
  venomous: "VENOMOUS",
  poisonous: "POISONOUS",
  windfury: "WINDFURY",
  mega_windfury: "MEGA_WINDFURY",
  stealth: "STEALTH",
};
/** The parser does not save these two; both are lasting, so the card's own count (as the app's harness). */
const UNSAVED: SimKeyword[] = ["poisonous", "mega_windfury"];

export const MAX_MINIONS = 7;
/** The most Attack or Health a minion takes: the 32-bit limit the engine saturates at (T-621). */
export const MAX_STAT = 2 ** 31 - 1;
export const MAX_TIER = 7;
/** Dark Gifts on one minion (crates/sim-wasm takes at most 4). */
export const MAX_GIFTS = 4;
const GIFT_ID = /^[A-Za-z0-9_]{1,64}$/;

export interface SimMinion {
  /** Stable within the page, for focus and lists. */
  key: number;
  cardId: string;
  golden: boolean;
  attack: number;
  health: number;
  keywords: SimKeyword[];
  /**
   * Dark Gifts by the gift's card id (`BG36_MidGameEffect_000t16`), as the
   * engine keys them (crates/combat-sim/src/gifts.rs). The saved report does
   * not hold them yet (T-528); the editor's controls come with T-526.
   */
  gifts: string[];
  /**
   * The buffs the game's log showed on the minion, for its tooltip only (T-619):
   * the engine never reads them. [] for a minion added by hand; null when the
   * game was saved before they were read.
   */
  buffs: Buff[] | null;
}

export interface SimBoard {
  /** The hero's tavern tier; null is unknown (the damage is then unknown). */
  tier: number | null;
  minions: SimMinion[];
  /**
   * The hero's game-long values from the saved round (parser revision 10,
   * T-610): Blood Gem and spell upgrades, Eternal Knights, Undead Attack,
   * Beetles. Absent when the record holds none: the printed numbers.
   */
  values?: GameValues;
}

export type Mode = "solo" | "duos";

export interface SimState {
  mode: Mode;
  /** One board in Solo; in Duos one or two, in the order they fight. */
  player: SimBoard[];
  opponent: SimBoard[];
  /** The game's cap on damage to a hero; null is none. */
  cap: number | null;
  /** The Hearthstone build of the loaded game; null for boards built by hand or a file that does not say. */
  build: number | null;
}

export type Side = "player" | "opponent";

let nextKey = 1;
export const newKey = (): number => nextKey++;

export const emptyBoard = (): SimBoard => ({ tier: null, minions: [] });

export function emptyState(mode: Mode = "solo"): SimState {
  return { mode, player: [emptyBoard()], opponent: [emptyBoard()], cap: null, build: null };
}

/** A round that can be loaded: it has a board on each side. */
export interface RoundChoice {
  number: number;
  result: Round["result"];
}

interface Fought {
  playerId: number | null;
  board: ReportMinion[];
  values: GameValues | null;
}

/** The boards that fought on one side, in the order they came in, one per player. */
function sideBoards(round: Round, side: "own" | "opponent"): Fought[] {
  const seen = new Set<number>();
  const out: Fought[] = [];
  for (const e of round.entries) {
    if (e.side !== side || e.board === null) continue;
    if (e.playerId !== null && seen.has(e.playerId)) continue;
    if (e.playerId !== null) seen.add(e.playerId);
    out.push({ playerId: e.playerId, board: e.board, values: e.values });
  }
  return out;
}

export function loadableRounds(record: GameRecord): RoundChoice[] {
  return record.report.rounds
    .filter((r) => sideBoards(r, "own").length > 0 && sideBoards(r, "opponent").length > 0)
    .map((r) => ({ number: r.number, result: r.result }));
}

export interface Loaded {
  state: SimState;
  /** Minions left out: no card id, or no Attack or Health in the record. */
  skipped: number;
  /**
   * Minions loaded with a Dark Gift the log does not name (T-528): they play
   * as the plain card, so the odds are approximate.
   */
  unknownGifts: number;
  /** Saved before the parser recorded Dark Gifts (revision 9): any gift is missing. */
  giftsNotRecorded: boolean;
}

/** The parser revision that records Dark Gifts on board minions (D-107). */
export const GIFTS_REVISION = 9;

function minionFrom(m: ReportMinion, cards: CardIndex): SimMinion | null {
  if (m.cardId === null || m.atk === null || m.health === null || m.atk < 0 || m.health < 1) return null;
  const printed = cards.get(m.cardId)?.k ?? [];
  const keywords = SIM_KEYWORDS.filter(
    (k) => (m.keywords as string[]).includes(k) || (UNSAVED.includes(k) && printed.includes(TAG[k])),
  );
  return {
    key: newKey(),
    cardId: m.cardId,
    golden: m.golden,
    attack: Math.min(m.atk, MAX_STAT),
    health: Math.min(m.health, MAX_STAT),
    keywords,
    gifts: [...new Set(m.gifts.filter((g) => GIFT_ID.test(g)))].slice(0, MAX_GIFTS),
    buffs: m.buffs,
  };
}

/**
 * The boards of one round of a game: the player's side and the opponent's,
 * each in the order they fought (Duos, D-087). The player's own tier is the
 * shop's for that turn; other heroes' tiers are not in the record (unknown).
 */
export function fromRound(record: GameRecord, roundNumber: number, cards: CardIndex): Loaded | null {
  const report = record.report;
  const round = report.rounds.find((r) => r.number === roundNumber);
  if (!round) return null;
  const own = sideBoards(round, "own");
  const opp = sideBoards(round, "opponent");
  if (own.length === 0 || opp.length === 0) return null;
  const mode: Mode = report.mode === "duos" ? "duos" : "solo";
  const keep = mode === "duos" ? 2 : 1;
  let skipped = 0;
  let unknownGifts = 0;
  const board = (f: Fought, tier: number | null): SimBoard => {
    const kept = f.board.map((m) => [m, minionFrom(m, cards)] as const).filter((p): p is readonly [ReportMinion, SimMinion] => p[1] !== null).slice(0, MAX_MINIONS);
    skipped += f.board.length - kept.length;
    unknownGifts += kept.filter(([m]) => m.giftUnknown).length;
    const out: SimBoard = { tier, minions: kept.map(([, s]) => s) };
    return f.values ? { ...out, values: f.values } : out;
  };
  const ownTier = report.shop?.turns.find((t) => t.turn === roundNumber)?.tier ?? null;
  const isLocal = (f: Fought) => f.playerId !== null && f.playerId === report.localPlayerId;
  return {
    state: {
      mode,
      player: own.slice(0, keep).map((f) => board(f, isLocal(f) ? ownTier : null)),
      opponent: opp.slice(0, keep).map((f) => board(f, null)),
      cap: null,
      build: report.build,
    },
    skipped,
    unknownGifts,
    giftsNotRecorded: record.revision === null || record.revision < GIFTS_REVISION,
  };
}

// ---------- Edits: each returns a new state ----------

function mapBoard(state: SimState, side: Side, index: number, change: (b: SimBoard) => SimBoard): SimState {
  return { ...state, [side]: state[side].map((b, i) => (i === index ? change(b) : b)) };
}

function mapMinion(state: SimState, side: Side, index: number, key: number, change: (m: SimMinion) => SimMinion): SimState {
  return mapBoard(state, side, index, (b) => ({ ...b, minions: b.minions.map((m) => (m.key === key ? change(m) : m)) }));
}

const clampStat = (n: number, min: number) => (Number.isFinite(n) ? Math.min(MAX_STAT, Math.max(min, Math.trunc(n))) : min);

export function setMode(state: SimState, mode: Mode): SimState {
  if (state.mode === mode) return state;
  if (mode === "solo") return { ...state, mode, player: state.player.slice(0, 1), opponent: state.opponent.slice(0, 1) };
  return { ...state, mode };
}

export function addBoard(state: SimState, side: Side): SimState {
  if (state.mode !== "duos" || state[side].length >= 2) return state;
  return { ...state, [side]: [...state[side], emptyBoard()] };
}

export function removeBoard(state: SimState, side: Side, index: number): SimState {
  if (state[side].length <= 1) return state;
  return { ...state, [side]: state[side].filter((_, i) => i !== index) };
}

export function setTier(state: SimState, side: Side, index: number, tier: number | null): SimState {
  const value = tier === null || !Number.isInteger(tier) || tier < 1 || tier > MAX_TIER ? null : tier;
  if (state[side][index]?.tier === value) return state;
  return mapBoard(state, side, index, (b) => ({ ...b, tier: value }));
}

export function setCap(state: SimState, cap: number | null): SimState {
  const value = cap === null || !Number.isInteger(cap) || cap < 1 ? null : Math.min(cap, 1000);
  return value === state.cap ? state : { ...state, cap: value };
}

/** A new minion with the card's own stats and keywords, at the right end; none past seven. */
export function addMinion(state: SimState, side: Side, index: number, cardId: string, cards: CardIndex): SimState {
  const card = cards.get(cardId);
  if (!card || state[side][index]?.minions.length >= MAX_MINIONS) return state;
  const minion: SimMinion = {
    key: newKey(),
    cardId,
    golden: cards.isGolden(cardId),
    attack: card.a,
    health: card.h,
    keywords: SIM_KEYWORDS.filter((k) => card.k.includes(TAG[k])),
    gifts: [],
    buffs: [],
  };
  return mapBoard(state, side, index, (b) => ({ ...b, minions: [...b.minions, minion] }));
}

export function removeMinion(state: SimState, side: Side, index: number, key: number): SimState {
  return mapBoard(state, side, index, (b) => ({ ...b, minions: b.minions.filter((m) => m.key !== key) }));
}

/** Moves a minion one place left (-1) or right (+1). */
export function moveMinion(state: SimState, side: Side, index: number, key: number, step: -1 | 1): SimState {
  return mapBoard(state, side, index, (b) => {
    const from = b.minions.findIndex((m) => m.key === key);
    const to = from + step;
    if (from < 0 || to < 0 || to >= b.minions.length) return b;
    const minions = [...b.minions];
    [minions[from], minions[to]] = [minions[to], minions[from]];
    return { ...b, minions };
  });
}

export function setStats(state: SimState, side: Side, index: number, key: number, attack: number, health: number): SimState {
  const m = state[side][index]?.minions.find((x) => x.key === key);
  if (m && m.attack === clampStat(attack, 0) && m.health === clampStat(health, 1)) return state;
  return mapMinion(state, side, index, key, (m) => ({ ...m, attack: clampStat(attack, 0), health: clampStat(health, 1) }));
}

export function toggleKeyword(state: SimState, side: Side, index: number, key: number, keyword: SimKeyword): SimState {
  return mapMinion(state, side, index, key, (m) => ({
    ...m,
    keywords: m.keywords.includes(keyword) ? m.keywords.filter((k) => k !== keyword) : SIM_KEYWORDS.filter((k) => k === keyword || m.keywords.includes(k)),
  }));
}

/** Adds a Dark Gift to a minion; none past four, and the same gift once. */
export function addGift(state: SimState, side: Side, index: number, key: number, giftId: string): SimState {
  const m = state[side][index]?.minions.find((x) => x.key === key);
  if (!m || !GIFT_ID.test(giftId) || m.gifts.includes(giftId) || m.gifts.length >= MAX_GIFTS) return state;
  return mapMinion(state, side, index, key, (x) => ({ ...x, gifts: [...x.gifts, giftId] }));
}

export function removeGift(state: SimState, side: Side, index: number, key: number, giftId: string): SimState {
  const m = state[side][index]?.minions.find((x) => x.key === key);
  if (!m?.gifts.includes(giftId)) return state;
  return mapMinion(state, side, index, key, (x) => ({ ...x, gifts: x.gifts.filter((g) => g !== giftId) }));
}

/**
 * Golden on or off: the card becomes its golden or normal twin (the engine
 * keys golden effects on the golden card's id). Stats are the player's to
 * set; a minion still at its card's printed stats takes the twin's.
 */
export function toggleGolden(state: SimState, side: Side, index: number, key: number, cards: CardIndex): SimState {
  return mapMinion(state, side, index, key, (m) => {
    const twin = cards.twin(m.cardId);
    const now = cards.get(m.cardId);
    const next = twin ? cards.get(twin) : undefined;
    if (!twin || !next) return { ...m, golden: !m.golden };
    const printed = now !== undefined && now.a === m.attack && now.h === m.health;
    return { ...m, cardId: twin, golden: cards.isGolden(twin), attack: printed ? next.a : m.attack, health: printed ? next.h : m.health };
  });
}

// ---------- The engine's request ----------

/** Every minion in both teams, for coverage checks and names. */
export function allMinions(state: SimState): SimMinion[] {
  return [...state.player, ...state.opponent].flatMap((b) => b.minions);
}

/** Why the boards cannot be played yet, or null. */
export function notReady(state: SimState): "empty" | null {
  const empty = (boards: SimBoard[]) => boards.every((b) => b.minions.length === 0);
  return empty(state.player) || empty(state.opponent) ? "empty" : null;
}

/** The request for crates/sim-wasm (its input.rs): both teams with the card facts. */
export function toRequest(state: SimState, cards: CardIndex): string {
  const board = (b: SimBoard) => ({
    tier: b.tier,
    ...(b.values ? { values: b.values } : {}),
    minions: b.minions.map((m) => ({
      card_id: m.cardId,
      golden: m.golden,
      attack: m.attack,
      health: m.health,
      keywords: m.keywords,
      gifts: m.gifts,
      facts: cards.facts(m.cardId),
    })),
  });
  // An empty board in Duos stands for "no teammate": it is left out.
  const team = (boards: SimBoard[]) => {
    const kept = boards.filter((b, i) => i === 0 || b.minions.length > 0);
    return kept.map(board);
  };
  return JSON.stringify({ player: team(state.player), opponent: team(state.opponent), damage_cap: state.cap, build: state.build });
}

// The simulator's boards (T-606): what the editor holds, how a round of one
// of the player's games becomes boards, the edits, and the engine's request.
// Plain data in, new data out: nothing here touches the page.
//
// Red line (D-004, D-054): a round gives only the boards that fought in it,
// the ones the player saw on screen. The saved report never holds the
// combats the game does not play back (T-213 left them out), so no
// SETASIDE board can come in this way.
import type { GameRecord, Minion as ReportMinion, Round } from "../game/report.ts";
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
export const MAX_STAT = 99_999;
export const MAX_TIER = 7;

export interface SimMinion {
  /** Stable within the page, for focus and lists. */
  key: number;
  cardId: string;
  golden: boolean;
  attack: number;
  health: number;
  keywords: SimKeyword[];
}

export interface SimBoard {
  /** The hero's tavern tier; null is unknown (the damage is then unknown). */
  tier: number | null;
  minions: SimMinion[];
}

export type Mode = "solo" | "duos";

export interface SimState {
  mode: Mode;
  /** One board in Solo; in Duos one or two, in the order they fight. */
  player: SimBoard[];
  opponent: SimBoard[];
  /** The game's cap on damage to a hero; null is none. */
  cap: number | null;
}

export type Side = "player" | "opponent";

let nextKey = 1;
export const newKey = (): number => nextKey++;

export const emptyBoard = (): SimBoard => ({ tier: null, minions: [] });

export function emptyState(mode: Mode = "solo"): SimState {
  return { mode, player: [emptyBoard()], opponent: [emptyBoard()], cap: null };
}

/** A round that can be loaded: it has a board on each side. */
export interface RoundChoice {
  number: number;
  result: Round["result"];
}

/** The boards that fought on one side, in the order they came in, one per player. */
function sideBoards(round: Round, side: "own" | "opponent"): ReportMinion[][] {
  const seen = new Set<number | null>();
  const out: ReportMinion[][] = [];
  for (const e of round.entries) {
    if (e.side !== side || e.board === null) continue;
    const who = e.playerId;
    if (who !== null && seen.has(who)) continue;
    seen.add(who);
    out.push(e.board);
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
}

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
  const board = (list: ReportMinion[], tier: number | null): SimBoard => {
    const minions = list.map((m) => minionFrom(m, cards)).filter((m): m is SimMinion => m !== null);
    skipped += list.length - minions.length + Math.max(0, minions.length - MAX_MINIONS);
    return { tier, minions: minions.slice(0, MAX_MINIONS) };
  };
  const ownTier = report.shop?.turns.find((t) => t.turn === roundNumber)?.tier ?? null;
  const ownEntries = round.entries.filter((e) => e.side === "own" && e.board !== null);
  const isLocal = (i: number) => ownEntries[i]?.playerId === report.localPlayerId;
  return {
    state: {
      mode,
      player: own.slice(0, keep).map((b, i) => board(b, isLocal(i) ? ownTier : null)),
      opponent: opp.slice(0, keep).map((b) => board(b, null)),
      cap: null,
    },
    skipped,
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
  return mapBoard(state, side, index, (b) => ({ ...b, tier: value }));
}

export function setCap(state: SimState, cap: number | null): SimState {
  return { ...state, cap: cap === null || !Number.isInteger(cap) || cap < 1 ? null : Math.min(cap, 1000) };
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
  return mapMinion(state, side, index, key, (m) => ({ ...m, attack: clampStat(attack, 0), health: clampStat(health, 1) }));
}

export function toggleKeyword(state: SimState, side: Side, index: number, key: number, keyword: SimKeyword): SimState {
  return mapMinion(state, side, index, key, (m) => ({
    ...m,
    keywords: m.keywords.includes(keyword) ? m.keywords.filter((k) => k !== keyword) : SIM_KEYWORDS.filter((k) => k === keyword || m.keywords.includes(k)),
  }));
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
    minions: b.minions.map((m) => ({
      card_id: m.cardId,
      golden: m.golden,
      attack: m.attack,
      health: m.health,
      keywords: m.keywords,
      facts: cards.facts(m.cardId),
    })),
  });
  // An empty board in Duos stands for "no teammate": it is left out.
  const team = (boards: SimBoard[]) => {
    const kept = boards.filter((b, i) => i === 0 || b.minions.length > 0);
    return kept.map(board);
  };
  return JSON.stringify({ player: team(state.player), opponent: team(state.opponent), damage_cap: state.cap });
}

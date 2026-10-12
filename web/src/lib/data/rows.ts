// Rows from the data API's views (T-502, design notes),
// read as untrusted input (T-503). A count that is not a count, or a mode
// that is not Solo or Duos, refuses the whole answer: the page says it could
// not read the data rather than show a wrong number. A value out of its range
// is unknown (null), never 0.
import { PLACES, type Mode } from "./format.ts";

export class DataError extends Error {}

type Row = Record<string, unknown>;

const CARD_ID = /^[A-Za-z0-9_]{1,64}$/;
const DAY = /^(\d{4})-(\d{2})-(\d{2})$/;
// The app's session name (Hearthstone_2026_10_09_18_30_00); checked before it goes in a link.
const SESSION = /^[A-Za-z0-9_.-]{1,128}$/;

function asRow(value: unknown): Row {
  if (typeof value !== "object" || value === null || Array.isArray(value)) throw new DataError("not a row");
  return value as Row;
}

export function asRows(value: unknown): Row[] {
  if (!Array.isArray(value)) throw new DataError("not a list of rows");
  return value.map(asRow);
}

/** A number, or null. PostgREST sends numeric as a number; a numeric string is accepted too. */
export function num(value: unknown): number | null {
  if (typeof value === "number") return Number.isFinite(value) ? value : null;
  if (typeof value === "string" && /^-?\d+(?:\.\d+)?$/.test(value)) return Number(value);
  return null;
}

/** A count the view always returns (count(*)): anything else refuses the answer. */
export function count(value: unknown, name: string): number {
  const n = num(value);
  if (n === null || !Number.isInteger(n) || n < 0) throw new DataError(`${name} is not a count`);
  return n;
}

/** A sum or a value that may be unknown (null); a negative one is unknown too. */
export function amount(value: unknown): number | null {
  const n = num(value);
  return n !== null && n >= 0 ? n : null;
}

export function mode(value: unknown): Mode {
  if (value === "solo" || value === "duos") return value;
  throw new DataError("not a game mode");
}

export function place(value: unknown, m: Mode): number | null {
  const n = num(value);
  return n !== null && Number.isInteger(n) && n >= 1 && n <= PLACES[m] ? n : null;
}

export function cardId(value: unknown): string | null {
  return typeof value === "string" && CARD_ID.test(value) ? value : null;
}

/** A real calendar day as YYYY-MM-DD, or null. */
export function day(value: unknown): string | null {
  if (typeof value !== "string") return null;
  const m = DAY.exec(value);
  if (!m) return null;
  const date = new Date(Date.UTC(Number(m[1]), Number(m[2]) - 1, Number(m[3])));
  return date.toISOString().slice(0, 10) === value ? value : null;
}

/** The columns overview() reads, for the select. */
export const OVERVIEW_COLUMNS =
  "mode,games,incomplete_games,unsupported_games,placed_games,avg_place,top_half,wins,first_played_on,last_played_on,summarized_games," +
  "combats_won,combats_lost,combats_tied,combats_unknown,combats_inferred,combat_games,damage_dealt,damage_dealt_games,damage_taken,damage_taken_games," +
  "gold_spent,gold_spent_games,gold_beyond_income,gold_beyond_games,sell_gold,sell_gold_games,rolls,free_rolls,buys,sells,shop_games," +
  "avg_apm,apm_games,triples,triples_games";

export interface Overview {
  mode: Mode;
  games: number;
  incompleteGames: number;
  unsupportedGames: number;
  placedGames: number;
  avgPlace: number | null;
  topHalf: number;
  wins: number;
  firstPlayedOn: string | null;
  lastPlayedOn: string | null;
  summarizedGames: number;
  combatsWon: number | null;
  combatsLost: number | null;
  combatsTied: number | null;
  combatsUnknown: number | null;
  combatsInferred: number | null;
  combatGames: number;
  damageDealt: number | null;
  damageDealtGames: number;
  damageTaken: number | null;
  damageTakenGames: number;
  goldSpent: number | null;
  goldSpentGames: number;
  goldBeyond: number | null;
  goldBeyondGames: number;
  sellGold: number | null;
  sellGoldGames: number;
  rolls: number | null;
  freeRolls: number | null;
  buys: number | null;
  sells: number | null;
  shopGames: number;
  avgApm: number | null;
  apmGames: number;
  triples: number | null;
  triplesGames: number;
}

export function overview(value: unknown): Overview {
  const r = asRow(value);
  const m = mode(r.mode);
  const games = count(r.games, "games");
  // Every "*_games" count is part of games; more would be a broken answer.
  const part = (key: string) => {
    const n = count(r[key], key);
    if (n > games) throw new DataError(`${key} is more than games`);
    return n;
  };
  const avg = num(r.avg_place);
  return {
    mode: m,
    games,
    incompleteGames: part("incomplete_games"),
    unsupportedGames: part("unsupported_games"),
    placedGames: part("placed_games"),
    avgPlace: avg !== null && avg >= 1 && avg <= PLACES[m] ? avg : null,
    topHalf: part("top_half"),
    wins: part("wins"),
    firstPlayedOn: day(r.first_played_on),
    lastPlayedOn: day(r.last_played_on),
    summarizedGames: part("summarized_games"),
    combatsWon: amount(r.combats_won),
    combatsLost: amount(r.combats_lost),
    combatsTied: amount(r.combats_tied),
    combatsUnknown: amount(r.combats_unknown),
    combatsInferred: amount(r.combats_inferred),
    combatGames: part("combat_games"),
    damageDealt: amount(r.damage_dealt),
    damageDealtGames: part("damage_dealt_games"),
    damageTaken: amount(r.damage_taken),
    damageTakenGames: part("damage_taken_games"),
    goldSpent: amount(r.gold_spent),
    goldSpentGames: part("gold_spent_games"),
    goldBeyond: amount(r.gold_beyond_income),
    goldBeyondGames: part("gold_beyond_games"),
    sellGold: amount(r.sell_gold),
    sellGoldGames: part("sell_gold_games"),
    rolls: amount(r.rolls),
    freeRolls: amount(r.free_rolls),
    buys: amount(r.buys),
    sells: amount(r.sells),
    shopGames: part("shop_games"),
    avgApm: amount(r.avg_apm),
    apmGames: part("apm_games"),
    triples: amount(r.triples),
    triplesGames: part("triples_games"),
  };
}

export interface HeroRow {
  mode: Mode;
  hero: string | null;
  games: number;
  placedGames: number;
  avgPlace: number | null;
  topHalf: number;
  wins: number;
  bestPlace: number | null;
}

export function heroRow(value: unknown): HeroRow {
  const r = asRow(value);
  const m = mode(r.mode);
  const avg = num(r.avg_place);
  return {
    mode: m,
    hero: cardId(r.hero),
    games: count(r.games, "games"),
    placedGames: count(r.placed_games, "placed_games"),
    avgPlace: avg !== null && avg >= 1 && avg <= PLACES[m] ? avg : null,
    topHalf: count(r.top_half, "top_half"),
    wins: count(r.wins, "wins"),
    bestPlace: place(r.best_place, m),
  };
}

export const RECORDS = [
  "win_streak",
  "top_half_streak",
  "damage_dealt_max",
  "damage_taken_max",
  "board_stats_max",
  "golden_max",
  "triples",
  "gold_spent",
  "gold_beyond_income",
  "rolls",
  "apm",
  "rounds",
] as const;

export type RecordName = (typeof RECORDS)[number];

export interface RecordRow {
  mode: Mode;
  record: RecordName;
  value: number;
  playedOn: string | null;
  games: number;
  inferred: boolean;
}

/** A record row, or null for a record this page does not know (a newer view). */
export function recordRow(value: unknown): RecordRow | null {
  const r = asRow(value);
  const m = mode(r.mode);
  const name = RECORDS.find((x) => x === r.record);
  const v = amount(r.value);
  if (!name || v === null) return null;
  return { mode: m, record: name, value: v, playedOn: day(r.played_on), games: count(r.games, "games"), inferred: r.inferred === true };
}

/** A game's key (session and index) for its link; each part null when not shaped as expected. */
export function gameKey(value: unknown): { session: string | null; gameIndex: number | null } {
  const r = asRow(value);
  const index = num(r.game_index);
  return {
    session: typeof r.session === "string" && SESSION.test(r.session) ? r.session : null,
    gameIndex: index !== null && Number.isInteger(index) && index >= 0 ? index : null,
  };
}

export type GameStatus = "ok" | "incomplete" | "unsupported" | "unknown";

export interface GameRow {
  /** The game's key, for its link; null when not shaped as expected. */
  session: string | null;
  gameIndex: number | null;
  mode: Mode;
  status: GameStatus;
  playedOn: string | null;
  hero: string | null;
  place: number | null;
  rounds: number | null;
  combatsWon: number | null;
  combatsLost: number | null;
  combatsInferred: number | null;
  triples: number | null;
  apm: number | null;
  goldSpent: number | null;
}

export function gameRow(value: unknown): GameRow {
  const r = asRow(value);
  const m = mode(r.mode);
  const status: GameStatus = r.status === "ok" || r.status === "incomplete" || r.status === "unsupported" ? r.status : "unknown";
  return {
    ...gameKey(r),
    mode: m,
    status,
    playedOn: day(r.played_on),
    hero: cardId(r.hero),
    // counted_place: the place of a finished game only.
    place: status === "ok" ? place(r.counted_place, m) : null,
    rounds: amount(r.rounds),
    combatsWon: amount(r.combats_won),
    combatsLost: amount(r.combats_lost),
    combatsInferred: amount(r.combats_inferred),
    triples: amount(r.triples),
    apm: amount(r.apm),
    goldSpent: amount(r.gold_spent),
  };
}

/** The columns gameRow() reads, for the select. */
export const GAME_COLUMNS =
  "session,game_index,mode,status,played_on,hero,counted_place,rounds,combats_won,combats_lost,combats_inferred,triples,apm,gold_spent";

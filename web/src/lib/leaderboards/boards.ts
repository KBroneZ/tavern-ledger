// The public leaderboards' boards (T-506, D-075): which there are, how each
// value reads, and the rows from public.leaderboard() checked before use.
// Pure, so the page script and the tests share it.
import { average, compact, PLACES, type Mode } from "../data/format.ts";
import { asRows, DataError } from "../data/rows.ts";
import { lt, type LbKey } from "./strings.ts";
import { dateFormat, numberFormat } from "../i18n.ts";

export type Period = "7d" | "30d" | "all";
export const PERIODS: readonly Period[] = ["7d", "30d", "all"];
export const DEFAULT_PERIOD: Period = "30d";

/** The database's minimums (private.leaderboard_min_games and _min_hero_games). */
export const MIN_GAMES = 20;
export const MIN_HERO_GAMES = 10;
/** Rows a board shows before "Show all" (the server sends at most 50). */
export const TOP_ROWS = 10;
export const MAX_ROWS = 50;

export const BOARD_KEYS = [
  "avg_place",
  "top_half_rate",
  "win_rate",
  "games",
  "top_half_streak",
  "hero_avg_place",
  "apm",
  "gold_beyond_income",
  "rolls",
  "triples",
  "biggest_board",
  "most_golden",
] as const;
export type BoardKey = (typeof BOARD_KEYS)[number];

type Format = "average" | "percent" | "count";

export interface BoardDef {
  key: BoardKey;
  /** Simple shows the main boards; Advanced adds the rest. */
  simple: boolean;
  format: Format;
  /** The board's name and value column change with the mode (top 4 / top 2). */
  perMode?: boolean;
}

export const BOARDS: readonly BoardDef[] = [
  { key: "avg_place", simple: true, format: "average" },
  { key: "top_half_rate", simple: true, format: "percent", perMode: true },
  { key: "win_rate", simple: true, format: "percent" },
  { key: "games", simple: true, format: "count" },
  { key: "top_half_streak", simple: true, format: "count", perMode: true },
  { key: "hero_avg_place", simple: true, format: "average" },
  { key: "apm", simple: false, format: "average" },
  { key: "gold_beyond_income", simple: false, format: "average" },
  { key: "rolls", simple: false, format: "average" },
  { key: "triples", simple: false, format: "average" },
  { key: "biggest_board", simple: false, format: "count" },
  { key: "most_golden", simple: false, format: "count" },
];

export interface BoardRow {
  board: BoardKey;
  standing: number;
  userId: string;
  name: string;
  hero: string | null;
  value: number;
  games: number;
}

const UUID = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/;
const HERO = /^[A-Za-z0-9_]{1,64}$/;
// The database's display name rule (profiles.display_name).
const NAME = /^[\p{L}\p{N} _.-]{1,32}$/u;

function positive(value: unknown, what: string): number {
  if (typeof value !== "number" || !Number.isInteger(value) || value < 1) throw new DataError(`bad ${what}`);
  return value;
}

/** The range a board's value can take; anything outside is a fault on the server. */
function inRange(board: BoardKey, value: number, m: Mode): boolean {
  if (board === "avg_place" || board === "hero_avg_place") return value >= 1 && value <= PLACES[m];
  if (board === "top_half_rate" || board === "win_rate") return value <= 100;
  if (board === "most_golden") return value <= 7;
  return true;
}

/** One row of public.leaderboard() for mode m, checked; anything unexpected throws. */
export function boardRow(value: unknown, m: Mode): BoardRow {
  const row = asRows([value])[0];
  if (!(BOARD_KEYS as readonly unknown[]).includes(row.board)) throw new DataError("bad board");
  const board = row.board as BoardKey;
  if (typeof row.user_id !== "string" || !UUID.test(row.user_id)) throw new DataError("bad user id");
  if (typeof row.display_name !== "string" || !NAME.test(row.display_name)) throw new DataError("bad name");
  const hero = row.hero === null || row.hero === undefined ? null : row.hero;
  if (hero !== null && (typeof hero !== "string" || !HERO.test(hero))) throw new DataError("bad hero");
  if ((board === "hero_avg_place") !== (hero !== null)) throw new DataError("hero on the wrong board");
  // PostgREST sends numeric as a JSON number; a string would be a change on the server.
  if (typeof row.value !== "number" || !Number.isFinite(row.value) || row.value < 0) throw new DataError("bad value");
  if (!inRange(board, row.value, m)) throw new DataError("value out of range");
  return {
    board,
    standing: positive(row.standing, "standing"),
    userId: row.user_id,
    name: row.display_name,
    hero,
    value: row.value,
    games: positive(row.games, "games"),
  };
}

/** Rows grouped per board, in the server's order (standing, then more games). */
export function groupBoards(rows: readonly BoardRow[]): Map<BoardKey, BoardRow[]> {
  const boards = new Map<BoardKey, BoardRow[]>(BOARD_KEYS.map((k) => [k, []]));
  for (const row of rows) boards.get(row.board)?.push(row);
  return boards;
}


/** A board's value as the page shows it. */
export function formatValue(def: BoardDef, value: number): string {
  if (def.format === "average") return average(value);
  if (def.format === "percent") return numberFormat({ style: "percent", minimumFractionDigits: 1, maximumFractionDigits: 1 }).format(value / 100);
  return compact(value);
}

function modeKey(def: BoardDef, base: string, m: Mode): LbKey {
  return (def.perMode && m === "duos" ? `${base}.duos` : base) as LbKey;
}

export function boardTitle(def: BoardDef, m: Mode): string {
  return lt(modeKey(def, `board.${def.key}`, m));
}

export function valueLabel(def: BoardDef, m: Mode): string {
  return lt(modeKey(def, `board.${def.key}.col`, m));
}

export function boardHow(def: BoardDef): string {
  return lt(`board.${def.key}.how` as LbKey, { hero: MIN_HERO_GAMES });
}

export function periodName(p: Period, long = false): string {
  return lt(`period.${p}${long ? ".long" : ""}` as LbKey);
}

/** The period from the address (?period=7d|30d|all); the default otherwise. */
export function parsePeriod(search: string): Period {
  const value = new URLSearchParams(search).get("period");
  return (PERIODS as readonly (string | null)[]).includes(value) ? (value as Period) : DEFAULT_PERIOD;
}

/** The same address with the period set (the default drops it). */
export function withPeriod(href: string, period: Period): string {
  const url = new URL(href);
  if (period === DEFAULT_PERIOD) url.searchParams.delete("period");
  else url.searchParams.set("period", period);
  return url.href;
}

/** "11 Oct 2026, 14:20 UTC" from the server's timestamp; null when there is none. */
export function refreshedText(value: unknown): string | null {
  if (typeof value !== "string") return null;
  const at = new Date(value);
  if (Number.isNaN(at.getTime())) return null;
  const text = dateFormat(
    { day: "numeric", month: "short", year: "numeric", hour: "2-digit", minute: "2-digit", timeZone: "UTC" },
    "en-GB",
  ).format(at);
  return `${text} UTC`;
}

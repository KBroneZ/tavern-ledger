// "Today and this week" on the dashboard (T-361, D-172): per mode, the games
// of the viewer's day and week with their average place and record, and up to
// three games worth revisiting. Pure functions over the player's own synced
// games; the page draws them (lib/view/periodView.ts).
//
// A game counts on the day its session started, as the player's PC dated it
// (`played_on`, a calendar day: the server keeps no clock time). The day is the
// viewer's local calendar day and the week runs Monday to Sunday, so all of it
// is plain calendar arithmetic on "YYYY-MM-DD" strings, no time zone to get
// wrong. A game with no day is in neither.
//
// The rating is never uploaded (it stays on the PC), so there is no rating
// change here at all; the page says so instead of showing a made-up number.
//
// Same rules as the app's summary (tracker::period_summary): the best place,
// a close loss (just outside the top half) and a big swing (two or more places
// from the average), each a different game, newest first on a tie.
import { PLACES, type Mode } from "./format.ts";
import type { GameRow } from "./rows.ts";

/** How many games the summary suggests revisiting. */
export const REVISIT = 3;
/** A big swing is a place at least this many places from the average. */
const SWING_PLACES = 2;

const DAY = /^(\d{4})-(\d{2})-(\d{2})$/;

export interface PeriodBounds {
  /** The viewer's local day. */
  today: string;
  /** The Monday of this week. */
  weekStart: string;
  /** The Sunday that ends it. */
  weekEnd: string;
}

/** A local calendar day, "YYYY-MM-DD", from a moment on the viewer's own clock. */
export function localDay(now: Date): string {
  const p = (n: number, width = 2) => String(n).padStart(width, "0");
  return `${p(now.getFullYear(), 4)}-${p(now.getMonth() + 1)}-${p(now.getDate())}`;
}

function utcMs(day: string): number {
  const m = DAY.exec(day);
  if (!m) throw new RangeError(`not a day: ${day}`);
  return Date.UTC(Number(m[1]), Number(m[2]) - 1, Number(m[3]));
}

/** `day` moved by `days` calendar days. */
export function addDays(day: string, days: number): string {
  const d = new Date(utcMs(day) + days * 86_400_000);
  const p = (n: number, width = 2) => String(n).padStart(width, "0");
  return `${p(d.getUTCFullYear(), 4)}-${p(d.getUTCMonth() + 1)}-${p(d.getUTCDate())}`;
}

/** The week of `today`: Monday to Sunday. */
export function periodBounds(today: string): PeriodBounds {
  const sinceMonday = (new Date(utcMs(today)).getUTCDay() + 6) % 7;
  const weekStart = addDays(today, -sinceMonday);
  return { today, weekStart, weekEnd: addDays(weekStart, 6) };
}

export type RevisitReason = "best" | "close" | "swing";

export interface Revisit {
  reason: RevisitReason;
  game: GameRow;
  place: number;
}

export interface ModePeriod {
  mode: Mode;
  games: number;
  /** Games with a counted place: the average and the record rest on these. */
  placed: number;
  avgPlace: number | null;
  /** Places that count as top half (4 in Solo, 2 in Duos). */
  topHalf: number;
  topHalfGames: number;
  revisit: Revisit[];
}

interface Placed {
  game: GameRow;
  place: number;
  /** Position among the period's games of the mode, oldest first. */
  at: number;
}

function revisits(placed: readonly Placed[], topHalf: number): Revisit[] {
  const chosen: { reason: RevisitReason; p: Placed }[] = [];
  const free = (p: Placed) => chosen.every((c) => c.p.at !== p.at);

  // Best place: the lowest number, the newest game on a tie.
  const best = placed.reduce<Placed | null>((b, p) => (b === null || p.place < b.place || (p.place === b.place && p.at > b.at) ? p : b), null);
  if (best) chosen.push({ reason: "best", p: best });

  const close = placed.toReversed().find((p) => p.place === topHalf + 1 && free(p));
  if (close) chosen.push({ reason: "close", p: close });

  // Distance from the average in units of 1/n place, to stay in integers.
  const n = placed.length;
  const sum = placed.reduce((s, p) => s + p.place, 0);
  const distance = (p: Placed) => Math.abs(p.place * n - sum);
  const swing = placed
    .filter((p) => free(p) && distance(p) >= SWING_PLACES * n)
    .reduce<Placed | null>((b, p) => (b === null || distance(p) > distance(b) || (distance(p) === distance(b) && p.at > b.at) ? p : b), null);
  if (swing) chosen.push({ reason: "swing", p: swing });

  return chosen.slice(0, REVISIT).map(({ reason, p }) => ({ reason, game: p.game, place: p.place }));
}

/**
 * One mode's numbers over the games played from `first` to `last` (both
 * days, inclusive). `rows` are the player's games newest first, as the data
 * API returns them; null when the mode has no game in the stretch.
 */
export function summarize(rows: readonly GameRow[], mode: Mode, first: string, last: string): ModePeriod | null {
  const inStretch = rows
    .filter((g) => g.mode === mode && g.playedOn !== null && g.playedOn >= first && g.playedOn <= last)
    // The API gives games newest first (its play order); oldest first here.
    .toReversed();
  if (inStretch.length === 0) return null;
  const placed: Placed[] = [];
  inStretch.forEach((game, at) => {
    if (game.place !== null) placed.push({ game, place: game.place, at });
  });
  const topHalf = PLACES[mode] / 2;
  return {
    mode,
    games: inStretch.length,
    placed: placed.length,
    avgPlace: placed.length === 0 ? null : placed.reduce((s, p) => s + p.place, 0) / placed.length,
    topHalf,
    topHalfGames: placed.filter((p) => p.place <= topHalf).length,
    revisit: revisits(placed, topHalf),
  };
}

export interface PeriodSummary {
  bounds: PeriodBounds;
  today: ModePeriod[];
  week: ModePeriod[];
}

/** Today and this week, Solo then Duos, only the modes with a game. */
export function summarizePeriod(rows: readonly GameRow[], bounds: PeriodBounds): PeriodSummary {
  const of = (first: string, last: string) =>
    (["solo", "duos"] as const).map((m) => summarize(rows, m, first, last)).filter((p): p is ModePeriod => p !== null);
  return { bounds, today: of(bounds.today, bounds.today), week: of(bounds.weekStart, bounds.weekEnd) };
}

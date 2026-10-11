// The community hero stats (T-507, D-085): the rows of
// public.community_heroes() and its state, checked as untrusted input,
// sorted, and shaped into table cells. Pure, so the page script and the tests
// share it. The server only sends heroes above the thresholds, with counts
// floored (games to tens, players to fives) and values rounded; a row that
// breaks that, or whose numbers cannot hold together, refuses the answer.
import { compact, PLACES, type Mode } from "../data/format.ts";
import { asRows, count, DataError } from "../data/rows.ts";
import { t } from "../data/strings.ts";
import { h, type VNode } from "../view/node.ts";
import { heroCell, type Cell, type Column } from "../view/pieces.ts";
import { byHeroName } from "../heroes/names.ts";
import { ct, type ChKey } from "./strings.ts";
import { numberFormat } from "../i18n.ts";

export type Period = "30d" | "all";
export const MODES: readonly Mode[] = ["solo", "duos"];
export const PERIODS: readonly Period[] = ["30d", "all"];
export const DEFAULT_PERIOD: Period = "30d";

/** The database's thresholds (private.community_min_games, _min_players). */
export const MIN_GAMES = 50;
export const MIN_PLAYERS = 5;
/** How the server floors the counts it sends (games to tens, players to fives). */
export const GAMES_STEP = 10;
export const PLAYERS_STEP = 5;
/** Heroes a table shows before "Show all". */
export const TOP_HEROES = 20;
/** More rows than any hero list could have: a sign of a fault on the server. */
export const MAX_ROWS = 1000;

export interface CommunityHero {
  hero: string;
  /** Floored to tens: 50 means 50 to 59. */
  games: number;
  /** Floored to fives: 5 means 5 to 9. */
  players: number;
  /** One decimal. */
  avgPlace: number;
  /** Whole percent of the games in the top 4 (Solo) or top 2 (Duos). */
  topHalfRate: number;
  winRate: number;
  /** Null when the games that offered the hero are below the thresholds. */
  pick: { rate: number; offeredGames: number; offeredPlayers: number } | null;
}

export type CommunityHeroes = Record<Mode, Record<Period, CommunityHero[]>>;

export interface CommunityState {
  /** Null before the first rebuild. */
  refreshedAt: string | null;
  /** The last day the snapshot counts (YYYY-MM-DD); null before the first rebuild. */
  countedUntil: string | null;
  /** False when the snapshot is too old to show (a rebuild failed): the server sends no rows then. */
  available: boolean;
}

const HERO = /^[A-Za-z0-9_]{1,64}$/;
const DAY = /^\d{4}-\d{2}-\d{2}$/;

function percent(value: unknown, what: string): number {
  if (typeof value !== "number" || !Number.isInteger(value) || value < 0 || value > 100) throw new DataError(`bad ${what}`);
  return value;
}

/** Floored counts above the thresholds, players never more than games. */
function counts(games: unknown, players: unknown, what: string): [number, number] {
  const g = count(games, `${what} games`);
  const p = count(players, `${what} players`);
  if (g < MIN_GAMES || p < MIN_PLAYERS) throw new DataError(`${what} below the thresholds`);
  if (g % GAMES_STEP !== 0 || p % PLAYERS_STEP !== 0) throw new DataError(`${what} not floored`);
  if (p > g) throw new DataError(`${what}: more players than games`);
  return [g, p];
}

function oneOf<T extends string>(value: unknown, values: readonly T[], what: string): T {
  if (!(values as readonly unknown[]).includes(value)) throw new DataError(`bad ${what}`);
  return value as T;
}

/** One row of public.community_heroes(), checked; anything unexpected throws. */
export function communityHero(value: unknown): { mode: Mode; period: Period; hero: CommunityHero } {
  const row = asRows([value])[0];
  const mode = oneOf(row.mode, MODES, "mode");
  const period = oneOf(row.period, PERIODS, "period");
  if (typeof row.hero !== "string" || !HERO.test(row.hero)) throw new DataError("bad hero");
  const [games, players] = counts(row.games, row.players, "hero");
  // PostgREST sends numeric as a JSON number; a string would be a change on the server.
  const avg = row.avg_place;
  if (typeof avg !== "number" || !(avg >= 1 && avg <= PLACES[mode]) || Math.abs(avg * 10 - Math.round(avg * 10)) > 1e-9) {
    throw new DataError("bad avg_place");
  }
  const topHalfRate = percent(row.top_half_rate, "top_half_rate");
  const winRate = percent(row.win_rate, "win_rate");
  if (winRate > topHalfRate) throw new DataError("more wins than top finishes");
  const pickParts = [row.offered_games, row.offered_players, row.pick_rate];
  let pick: CommunityHero["pick"] = null;
  if (pickParts.some((p) => p !== null && p !== undefined)) {
    if (pickParts.some((p) => p === null || p === undefined)) throw new DataError("half a pick rate");
    const [offeredGames, offeredPlayers] = counts(row.offered_games, row.offered_players, "offers");
    pick = { rate: percent(row.pick_rate, "pick_rate"), offeredGames, offeredPlayers };
  }
  return { mode, period, hero: { hero: row.hero, games, players, avgPlace: avg, topHalfRate, winRate, pick } };
}

/** Every row, checked and grouped per mode and period: each hero once in each. */
export function communityHeroes(value: unknown): CommunityHeroes {
  const list = asRows(value);
  if (list.length > MAX_ROWS) throw new DataError("too many rows");
  const out: CommunityHeroes = { solo: { "30d": [], all: [] }, duos: { "30d": [], all: [] } };
  const seen = new Set<string>();
  for (const raw of list) {
    const { mode, period, hero } = communityHero(raw);
    const key = `${mode} ${period} ${hero.hero}`;
    if (seen.has(key)) throw new DataError("a hero twice");
    seen.add(key);
    out[mode][period].push(hero);
  }
  return out;
}

/** The one row of public.community_heroes_state(), checked. */
export function communityState(value: unknown): CommunityState {
  const list = asRows(value);
  if (list.length !== 1) throw new DataError("not one state row");
  const row = list[0];
  const at = row.refreshed_at;
  if (at !== null && (typeof at !== "string" || Number.isNaN(Date.parse(at)))) throw new DataError("bad time");
  const until = row.counted_until;
  if (until !== null && (typeof until !== "string" || !DAY.test(until))) throw new DataError("bad day");
  if ((at === null) !== (until === null)) throw new DataError("half a state");
  if (typeof row.available !== "boolean") throw new DataError("bad available");
  if (at === null && row.available) throw new DataError("available before any rebuild");
  return { refreshedAt: at, countedUntil: until, available: row.available };
}

/** True when rows came although the state says the snapshot is too old to show: a fault on the server. */
export function rowsWhileUnavailable(state: CommunityState, heroes: CommunityHeroes): boolean {
  return !state.available && MODES.some((m) => PERIODS.some((p) => heroes[m][p].length > 0));
}

// ------------------------------------------------------------------ sorting

export type HeroSort = "games" | "avg" | "top" | "pick";
export const SORTS: readonly HeroSort[] = ["games", "avg", "top", "pick"];

const byHero = (a: CommunityHero, b: CommunityHero) => byHeroName(a.hero, b.hero);
// Heroes with no pick rate go last; two of them tie (0), then the next rule decides.
const pickOrder = (a: CommunityHero, b: CommunityHero) => {
  if (a.pick && b.pick) return b.pick.rate - a.pick.rate;
  return (a.pick ? 0 : 1) - (b.pick ? 0 : 1);
};

/**
 * "games": most games first. "avg": the better (lower) average place first.
 * "top": the higher top-half rate first. "pick": the higher pick rate first,
 * heroes without one last. Then more games, then the name, so the order
 * is the same on every read.
 */
export function sortHeroes(rows: readonly CommunityHero[], sort: HeroSort): CommunityHero[] {
  const key: Record<HeroSort, (a: CommunityHero, b: CommunityHero) => number> = {
    games: () => 0,
    avg: (a, b) => a.avgPlace - b.avgPlace,
    top: (a, b) => b.topHalfRate - a.topHalfRate,
    pick: pickOrder,
  };
  return rows.toSorted((a, b) => key[sort](a, b) || b.games - a.games || byHero(a, b));
}

// ------------------------------------------------------------------ the address

function param<T extends string>(search: string, name: string, values: readonly T[], fallback: T): T {
  const value = new URLSearchParams(search).get(name);
  return (values as readonly (string | null)[]).includes(value) ? (value as T) : fallback;
}

/** The period from the address (?period=all); 30 days otherwise. */
export function parsePeriod(search: string): Period {
  return param(search, "period", PERIODS, DEFAULT_PERIOD);
}

/** The sort from the address (?sort=avg|top|pick); games otherwise. */
export function parseSort(search: string): HeroSort {
  return param(search, "sort", SORTS, "games");
}

/** The same address with one setting changed; a default leaves none. */
export function withParam(href: string, name: "period" | "sort", value: string): string {
  const url = new URL(href);
  const fallback = name === "period" ? DEFAULT_PERIOD : "games";
  if (value === fallback) url.searchParams.delete(name);
  else url.searchParams.set(name, value);
  return `${url.pathname}${url.search}${url.hash}`;
}

// ------------------------------------------------------------------ cells

export function periodName(p: Period, form: "short" | "in" = "short"): string {
  return ct(`period.${p}${form === "in" ? ".in" : ""}` as ChKey);
}

export function heroColumns(m: Mode): Column[] {
  return [
    { label: ct("col.hero") },
    { label: ct("col.games"), numeric: true },
    { label: ct("col.players"), numeric: true },
    { label: ct("col.pick"), numeric: true },
    { label: ct("col.avg"), numeric: true },
    { label: ct(m === "solo" ? "col.top.solo" : "col.top.duos"), numeric: true },
    { label: ct("col.win"), numeric: true, advanced: true },
    { label: ct("col.offered"), numeric: true, advanced: true },
    { label: ct("col.offeredPlayers"), numeric: true, advanced: true },
  ];
}


/** A floored count as the page shows it: "50+" is 50 to 59 games. */
export function atLeast(value: number): string {
  return ct("count.atLeast", { n: compact(value) });
}

/** A whole percent from the server (57 → "57%" / "57 %"), in the page's language. */
export function rate(value: number): string {
  return numberFormat({ style: "percent", maximumFractionDigits: 1 }).format(value / 100);
}

export function place(value: number): string {
  return numberFormat({ minimumFractionDigits: 1, maximumFractionDigits: 1 }).format(value);
}

/** A dash that screen readers read as "Not enough hero choices yet". */
export function fewPicks(): VNode[] {
  return [h("span", { "aria-hidden": "true", title: ct("pick.few") }, t("value.none")), h("span", { class: "visually-hidden" }, ct("pick.few"))];
}

export function heroCells(r: CommunityHero): Cell[] {
  return [
    heroCell(r.hero),
    atLeast(r.games),
    atLeast(r.players),
    r.pick ? rate(r.pick.rate) : { node: fewPicks() },
    place(r.avgPlace),
    rate(r.topHalfRate),
    rate(r.winRate),
    r.pick ? atLeast(r.pick.offeredGames) : { node: fewPicks() },
    r.pick ? atLeast(r.pick.offeredPlayers) : { node: fewPicks() },
  ];
}

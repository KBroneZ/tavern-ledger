// The heroes page (T-505): one row per hero and mode from user_heroes
// (T-502), read as untrusted input, sorted, and shaped into table cells.
// Pure functions; lib/heroes/views.ts draws them. A rate that rests on few
// games says so; a hero only ever offered, never played, is listed apart.
import { compact, placeAverage, share, type Mode } from "../data/format.ts";
import { DEFAULT_FILTERS, filtersQuery } from "../data/filters.ts";
import { count, DataError, heroRow, place, type HeroRow } from "../data/rows.ts";
import { t } from "../data/strings.ts";
import { h, type VNode } from "../view/node.ts";
import { heroCell, unknownValue, type Cell, type Column } from "../view/pieces.ts";
import { byHeroName } from "./names.ts";
import { href } from "../i18n.ts";

/** Below this many games, an average or a rate is marked "few games". */
export const MIN_GAMES = 5;

/** The columns heroStat() reads, for the select. */
export const HERO_STAT_COLUMNS = "mode,hero,games,placed_games,avg_place,top_half,wins,best_place,worst_place,offered_games,picked_games";

export interface HeroStat extends HeroRow {
  worstPlace: number | null;
  /** Games whose known hero choice offered this hero. */
  offeredGames: number;
  /** Games played with it from those choices (never more than offeredGames). */
  pickedGames: number;
}

export function heroStat(value: unknown): HeroStat {
  const base = heroRow(value);
  const r = value as Record<string, unknown>;
  const offeredGames = count(r.offered_games, "offered_games");
  const pickedGames = count(r.picked_games, "picked_games");
  // Counts that cannot hold together refuse the answer rather than show a wrong rate.
  if (pickedGames > offeredGames) throw new DataError("picked_games is more than offered_games");
  if (pickedGames > base.games) throw new DataError("picked_games is more than games");
  if (base.placedGames > base.games) throw new DataError("placed_games is more than games");
  if (base.topHalf > base.placedGames || base.wins > base.topHalf) throw new DataError("top_half or wins do not add up");
  return { ...base, worstPlace: place(r.worst_place, base.mode), offeredGames, pickedGames };
}

export type HeroSort = "games" | "avg";

/** The sort in the address (?sort=avg); most games first by default. */
export function parseSort(search: string): HeroSort {
  return new URLSearchParams(search).get("sort") === "avg" ? "avg" : "games";
}

/** The same address with the sort set; the default leaves none. */
export function withSort(href: string, sort: HeroSort): string {
  const url = new URL(href);
  if (sort === "avg") url.searchParams.set("sort", "avg");
  else url.searchParams.delete("sort");
  return `${url.pathname}${url.search}${url.hash}`;
}

const byName = (a: HeroStat, b: HeroStat) => byHeroName(a.hero, b.hero);
const avgOf = (r: HeroStat) => r.avgPlace ?? Number.POSITIVE_INFINITY;
const enough = (r: HeroStat) => r.placedGames >= MIN_GAMES;

/**
 * Heroes played in one mode. "games": most games first, then the better
 * average place. "avg": the better average place first among heroes with
 * enough finished games, then those with few, then those with none; ties by
 * more games. The name (then the card id) breaks any tie left, so the order is stable.
 */
export function sortHeroes(rows: readonly HeroStat[], m: Mode, sort: HeroSort): HeroStat[] {
  const played = rows.filter((r) => r.mode === m && r.games > 0);
  if (sort === "games") return played.toSorted((a, b) => b.games - a.games || avgOf(a) - avgOf(b) || byName(a, b));
  const tier = (r: HeroStat) => (r.avgPlace === null ? 2 : enough(r) ? 0 : 1);
  return played.toSorted((a, b) => tier(a) - tier(b) || avgOf(a) - avgOf(b) || b.games - a.games || byName(a, b));
}

/** Heroes offered in a known hero choice but never played in this mode, most offered first. */
export function offeredOnly(rows: readonly HeroStat[], m: Mode): HeroStat[] {
  return rows.filter((r) => r.mode === m && r.games === 0 && r.offeredGames > 0).toSorted((a, b) => b.offeredGames - a.offeredGames || byName(a, b));
}

/** True when a mode has at least one hero played. */
export function hasHeroes(rows: readonly HeroStat[], m: Mode): boolean {
  return rows.some((r) => r.mode === m && r.games > 0);
}

export function heroColumns(m: Mode): Column[] {
  return [
    { label: t("heroes.col.hero") },
    { label: t("heroes.col.games"), numeric: true },
    { label: t("heroes.col.avg"), numeric: true },
    { label: t(m === "solo" ? "tile.topHalf.solo" : "tile.topHalf.duos"), numeric: true },
    { label: t("hstats.col.wins"), numeric: true, advanced: true },
    { label: t("heroes.col.best"), numeric: true, advanced: true },
    { label: t("hstats.col.worst"), numeric: true, advanced: true },
    { label: t("hstats.col.picked"), numeric: true, advanced: true },
  ];
}

/** The "few games" mark next to a value that rests on fewer than MIN_GAMES games. */
export function fewMark(): VNode {
  return h("span", { class: "mark-src mark-few", title: t("hstats.few.tip", { min: MIN_GAMES }) }, t("hstats.few"));
}

/** A value (null: unknown, a dash read as "Unknown"), with the mark when it rests on few games. */
function withFew(value: number | null, text: string, few: boolean): Cell {
  if (value === null) return { node: unknownValue() };
  return few ? { node: [h("span", {}, text), fewMark()] } : text;
}

/** True when a hero's pick rate rests on fewer than MIN_GAMES hero choices. */
export const fewPicks = (r: HeroStat): boolean => r.offeredGames > 0 && r.offeredGames < MIN_GAMES;

/** "40% · 2 of 5", marked when offered in few games; unknown when never offered in a known choice. */
export function pickCell(r: HeroStat): Cell {
  if (r.offeredGames === 0) return { node: unknownValue() };
  const text = t("hstats.picked.value", { rate: share(r.pickedGames, r.offeredGames), picked: r.pickedGames, offered: r.offeredGames });
  return withFew(r.pickedGames, text, fewPicks(r));
}

/** The games list filtered by this hero and mode (the owner's, or the public profile's). */
export function heroGamesHref(hero: string | null, m: Mode, profileId: string | null): string | undefined {
  return hero === null ? undefined : href(`/games/${filtersQuery({ ...DEFAULT_FILTERS, mode: m, hero }, profileId)}`);
}

/** True when a hero's average and rate rest on fewer than MIN_GAMES finished games. */
export const fewGames = (r: HeroStat): boolean => r.placedGames > 0 && r.placedGames < MIN_GAMES;

/** A hero's row; markFew is false when every row would carry the mark (the note then says it once). */
export function heroStatCells(r: HeroStat, profileId: string | null, markFew = true): Cell[] {
  const few = markFew && fewGames(r);
  // Places count finished games only: say how many when some are not.
  const games = r.placedGames < r.games ? t("heroes.games.placed", { games: r.games, placed: r.placedGames }) : compact(r.games);
  return [
    heroCell(r.hero, heroGamesHref(r.hero, r.mode, profileId)),
    // "Few games" once per row, on the games it rests on, not on every value (T-559).
    few ? { node: [h("span", {}, games), fewMark()] } : games,
    r.avgPlace === null ? { node: unknownValue() } : placeAverage(r.avgPlace),
    r.placedGames === 0 ? { node: unknownValue() } : share(r.topHalf, r.placedGames),
    r.placedGames === 0 ? { node: unknownValue() } : compact(r.wins),
    { place: r.bestPlace, mode: r.mode },
    { place: r.worstPlace, mode: r.mode },
    pickCell(r),
  ];
}

/** The offered-heroes table's columns, in the page's language. */
export const offeredColumns = (): Column[] => [{ label: t("heroes.col.hero") }, { label: t("hstats.col.offered"), numeric: true }];

export function offeredCells(r: HeroStat): Cell[] {
  return [heroCell(r.hero), compact(r.offeredGames)];
}

export type StatsPage = "heroes" | "curious";

const STATS_PATHS: Record<StatsPage, string> = { heroes: "/player-heroes/", curious: "/curious/" };

/** The heroes or curious stats page for a mode: the owner's, or a public profile's (?id=). */
export function statsPagePath(page: StatsPage, m: Mode, profileId: string | null): string {
  const q = new URLSearchParams();
  if (profileId) q.set("id", profileId);
  // Always named: without a mode the page opens on the one with more games (T-559).
  q.set("mode", m);
  return href(`${STATS_PATHS[page]}?${q.toString()}`);
}

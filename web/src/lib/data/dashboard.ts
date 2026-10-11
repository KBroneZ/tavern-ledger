// What the dashboard, the public profile and the games list show (T-503),
// worked out from the parsed view rows (rows.ts). Pure functions: the
// pages draw these with lib/view/pieces.ts. Every value says how many games
// back it; a value no game backs is a dash, never 0.
import { average, backing, compact, PLACES, share, type Mode } from "./format.ts";
import { RECORDS, type GameRow, type HeroRow, type Overview, type RecordRow } from "./rows.ts";
import { t } from "./strings.ts";
import { heroCell, unknownValue, sourceMark, type Cell, type Column, type TileProps } from "../view/pieces.ts";
import { byHeroName, heroLabel } from "../heroes/names.ts";
import { h, type VNode } from "../view/node.ts";
import { gamePath } from "../game/url.ts";
import { dateFormat } from "../i18n.ts";

export const modeName = (m: Mode): string => (m === "solo" ? t("mode.solo") : t("mode.duos"));

/** A sum over the games that have it, per game; null when no game has it. */
export function perGame(sum: number | null, games: number): number | null {
  return sum === null || games === 0 ? null : sum / games;
}

// "1 Oct 2026" / "1 oct 2026", in the page's language (T-508).
const DATE = { format: (d: Date) => dateFormat({ day: "numeric", month: "short", year: "numeric", timeZone: "UTC" }).format(d) };

/** "9 Oct 2026" from "2026-10-09"; a dash when unknown. */
export function dateText(isoDay: string | null): string {
  return isoDay === null ? t("value.none") : DATE.format(new Date(`${isoDay}T00:00:00Z`));
}

/** The backing line, kept honest when a count is larger than its total. */
function backed(known: number, total: number): string {
  return backing(Math.min(known, total), total);
}

export interface Tiles {
  simple: TileProps[];
  advanced: TileProps[];
}

/** The key numbers (Simple) and the rest (Advanced) for one mode. */
export function overviewTiles(ov: Overview, recentPlaces: (number | null)[]): Tiles {
  const unplaced = ov.games - ov.placedGames;
  const simple: TileProps[] = [
    {
      hero: true,
      label: t("tile.avgPlace"),
      value: average(ov.avgPlace),
      backing: backed(ov.placedGames, ov.games),
      spark: recentPlaces.length > 1 ? { values: recentPlaces, min: 1, max: PLACES[ov.mode], invert: true } : undefined,
    },
    { label: t("tile.games"), value: compact(ov.games), backing: unplaced > 0 ? t("tile.games.back", { count: unplaced }) : undefined },
    { label: t(ov.mode === "solo" ? "tile.topHalf.solo" : "tile.topHalf.duos"), value: share(ov.topHalf, ov.placedGames), backing: backed(ov.placedGames, ov.games) },
    { label: t("tile.wins"), value: ov.placedGames === 0 ? t("value.none") : compact(ov.wins), backing: backed(ov.placedGames, ov.games) },
  ];
  const known = [ov.combatsWon, ov.combatsLost, ov.combatsTied];
  const fought = known.every((x) => x !== null) ? (known as number[]).reduce((a, b) => a + b, 0) : null;
  const per = (label: Parameters<typeof t>[0], sum: number | null, games: number, inferred = false): TileProps => ({
    label: t(label),
    value: average(perGame(sum, games)),
    backing: backed(games, ov.games),
    source: games === 0 ? "unknown" : inferred ? "inferred" : undefined,
  });
  const advanced: TileProps[] = [
    {
      label: t("tile.combats"),
      value: share(ov.combatsWon, fought),
      // The share is of combats with a result; unknown ones are said, never hidden.
      backing:
        fought !== null && (ov.combatsUnknown ?? 0) > 0
          ? t("tile.combats.back", { known: fought, unknown: ov.combatsUnknown as number })
          : backed(ov.combatGames, ov.games),
      source: ov.combatGames === 0 ? "unknown" : (ov.combatsInferred ?? 0) > 0 ? "inferred" : undefined,
    },
    per("tile.damageDealt", ov.damageDealt, ov.damageDealtGames, true),
    per("tile.damageTaken", ov.damageTaken, ov.damageTakenGames, true),
    per("tile.goldSpent", ov.goldSpent, ov.goldSpentGames),
    per("tile.goldBeyond", ov.goldBeyond, ov.goldBeyondGames),
    per("tile.sellGold", ov.sellGold, ov.sellGoldGames),
    per("tile.rolls", ov.rolls, ov.shopGames),
    per("tile.freeRolls", ov.freeRolls, ov.shopGames),
    per("tile.buys", ov.buys, ov.shopGames),
    per("tile.sells", ov.sells, ov.shopGames),
    { label: t("tile.apm"), value: average(ov.avgApm), backing: backed(ov.apmGames, ov.games), source: ov.apmGames === 0 ? "unknown" : undefined },
    per("tile.triples", ov.triples, ov.triplesGames),
  ];
  return { simple, advanced };
}

/** The hero's name in the page's language (T-510), never the bare card id. */
const heroName = (hero: string | null) => heroLabel(hero, t("hero.unknown"), t("hero.unknownId")).name;

export const HERO_COLUMNS = (m: Mode): Column[] => [
  { label: t("heroes.col.hero") },
  { label: t("heroes.col.games"), numeric: true },
  { label: t("heroes.col.avg"), numeric: true },
  { label: t(m === "solo" ? "tile.topHalf.solo" : "tile.topHalf.duos"), numeric: true },
  { label: t("heroes.col.best"), numeric: true },
];

/** Heroes played in one mode: most games first, then the better average place, then the name. */
export function heroCells(rows: readonly HeroRow[], m: Mode): Cell[][] {
  return rows
    .filter((r) => r.mode === m && r.games > 0)
    .toSorted((a, b) => b.games - a.games || (a.avgPlace ?? 99) - (b.avgPlace ?? 99) || byHeroName(a.hero, b.hero))
    .map((r) => [
      heroCell(r.hero),
      // Places count finished games only: say how many when some are not.
      r.placedGames < r.games ? t("heroes.games.placed", { games: r.games, placed: r.placedGames }) : r.games,
      average(r.avgPlace),
      share(r.topHalf, r.placedGames),
      { place: r.bestPlace, mode: m },
    ]);
}

export const RECORD_COLUMNS: Column[] = [
  { label: t("records.col.record") },
  { label: t("records.col.value"), numeric: true },
  { label: t("records.col.date") },
  { label: t("records.col.games") },
];

function recordLabel(r: RecordRow): string {
  if (r.record === "top_half_streak") return t(r.mode === "solo" ? "record.top_half_streak.solo" : "record.top_half_streak.duos");
  return t(`record.${r.record}`);
}

/** Records in one mode, in a fixed order; each with the games that back it. */
export function recordCells(rows: readonly RecordRow[], m: Mode, totalGames: number): Cell[][] {
  return rows
    .filter((r) => r.mode === m)
    .toSorted((a, b) => RECORDS.indexOf(a.record) - RECORDS.indexOf(b.record))
    .map((r) => [
      recordLabel(r),
      { node: [h("span", {}, r.record === "apm" ? average(r.value) : compact(r.value)), r.inferred ? sourceMark("inferred") : null].filter((n): n is VNode => n !== null) },
      dateText(r.playedOn),
      backed(r.games, Math.max(totalGames, r.games)),
    ]);
}

function maybe(value: number | null, format: (n: number) => string = (n) => compact(n)): Cell {
  return value === null ? { node: unknownValue() } : format(value);
}

function placeCell(g: GameRow): Cell {
  if (g.status === "ok") return { place: g.place, mode: g.mode };
  const note = g.status === "incomplete" ? t("games.status.incomplete") : g.status === "unsupported" ? t("games.status.unsupported") : t("source.unknown");
  return { node: [h("span", { class: "place place-unknown" }, h("span", { "aria-hidden": "true" }, "?")), h("span", { class: "cell-note" }, note)] };
}

function combatsCell(g: GameRow): Cell {
  if (g.combatsWon === null || g.combatsLost === null) return { node: unknownValue() };
  const text = h("span", {}, `${g.combatsWon}–${g.combatsLost}`);
  return { node: (g.combatsInferred ?? 0) > 0 ? [text, sourceMark("inferred")] : [text] };
}

export function gameColumns(withMode: boolean): Column[] {
  return [
    { label: t("games.col.hero") },
    { label: t("games.col.place"), numeric: true },
    { label: t("games.col.date") },
    ...(withMode ? [{ label: t("games.col.mode") }] : []),
    { label: t("games.col.rounds"), numeric: true },
    { label: t("games.col.combats"), numeric: true },
    { label: t("games.col.triples"), numeric: true },
    { label: t("games.col.apm"), numeric: true },
  ];
}

/** The game viewer's address (T-504): lib/game/url.ts owns it. */
export const gameHref = gamePath;

/** The date, as a link to the game when its key is known. */
function dateCell(g: GameRow, userId: string | null): Cell {
  const text = dateText(g.playedOn);
  if (!userId || g.session === null || g.gameIndex === null) return text;
  // Same-day games would share the link text: the hero tells them apart for screen readers.
  const which = h("span", { class: "visually-hidden" }, t("games.link", { hero: heroName(g.hero) }));
  return { node: h("a", { href: gameHref(userId, g.session, g.gameIndex) }, text, which) };
}

/** One game as a row: hero, place badge (or why it has none), date (linking to the game) and the key numbers. */
export function gameCells(g: GameRow, withMode: boolean, userId: string | null = null): Cell[] {
  return [
    heroCell(g.hero),
    placeCell(g),
    dateCell(g, userId),
    ...(withMode ? [modeName(g.mode)] : []),
    maybe(g.rounds),
    combatsCell(g),
    maybe(g.triples),
    maybe(g.apm, average),
  ];
}

/** The last n places, oldest first (the trend chart and the tile's line). */
export function recentPlaces(newestFirst: readonly (number | null)[], n: number): (number | null)[] {
  return newestFirst.slice(0, n).toReversed();
}

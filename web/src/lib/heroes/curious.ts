// The curious stats page (T-505): the records the data API backs
// (user_records, each with the game it came from) and fun totals over all
// games (user_overview), per mode. Pure functions; lib/heroes/views.ts draws
// them. Every value says how many games back it; inferred ones are marked.
import { average, backing, compact, type Mode } from "../data/format.ts";
import { gameKey, RECORDS, recordRow, type Overview, type RecordRow } from "../data/rows.ts";
import { t, type StringKey } from "../data/strings.ts";
import { dateText } from "../data/dashboard.ts";
import { gamePath } from "../game/url.ts";
import { h, type VNode } from "../view/node.ts";
import { sourceMark, type Cell, type Column, type TileProps } from "../view/pieces.ts";

/** The columns curiousRecord() reads, for the select. */
export const CURIOUS_RECORD_COLUMNS = "mode,record,value,session,game_index,played_on,games,inferred";

export interface CuriousRecord extends RecordRow {
  /** The game it came from (the first that reached the value), for its link. */
  session: string | null;
  gameIndex: number | null;
}

/** A record with its game, or null for a record this page does not know (a newer view). */
export function curiousRecord(value: unknown): CuriousRecord | null {
  const base = recordRow(value);
  return base === null ? null : { ...base, ...gameKey(value) };
}

export function recordLabel(r: Pick<RecordRow, "record" | "mode">): string {
  if (r.record === "top_half_streak") return t(r.mode === "solo" ? "record.top_half_streak.solo" : "record.top_half_streak.duos");
  return t(`record.${r.record}`);
}

/** Records in one mode, in the fixed order of RECORDS. */
export function modeRecords(rows: readonly CuriousRecord[], m: Mode): CuriousRecord[] {
  return rows.filter((r) => r.mode === m).toSorted((a, b) => RECORDS.indexOf(a.record) - RECORDS.indexOf(b.record));
}

export const CURIOUS_RECORD_COLUMNS_VIEW: Column[] = [
  { label: t("records.col.record") },
  { label: t("records.col.value"), numeric: true },
  { label: t("curious.col.game") },
  { label: t("records.col.games") },
];

/** The game's date, linked to the game viewer when its key is known. */
function gameCell(r: CuriousRecord, userId: string): Cell {
  const text = dateText(r.playedOn);
  if (r.session === null || r.gameIndex === null) return text;
  // Every row links a date: the record names the link for screen readers.
  const which = h("span", { class: "visually-hidden" }, t("curious.link", { record: recordLabel(r) }));
  return { node: h("a", { href: gamePath(userId, r.session, r.gameIndex) }, text, which) };
}

export function recordCellsOf(r: CuriousRecord, userId: string, totalGames: number): Cell[] {
  const value = h("span", {}, r.record === "apm" ? average(r.value) : compact(r.value));
  return [
    recordLabel(r),
    { node: r.inferred ? [value, sourceMark("inferred")] : [value] },
    gameCell(r, userId),
    // Kept honest if the record's count is ever larger than the mode's games.
    backing(r.games, Math.max(totalGames, r.games)),
  ];
}

interface TotalSpec {
  label: StringKey;
  sum: (o: Overview) => number | null;
  games: (o: Overview) => number;
  inferred?: boolean;
  advanced?: boolean;
}

const combats = (o: Overview) => o.combatGames;
const shop = (o: Overview) => o.shopGames;

/** The fun totals, over every game that has the value. Simple shows the first four. */
export const TOTALS: readonly TotalSpec[] = [
  { label: "curious.total.goldSpent", sum: (o) => o.goldSpent, games: (o) => o.goldSpentGames },
  { label: "curious.total.rolls", sum: (o) => o.rolls, games: shop },
  { label: "curious.total.triples", sum: (o) => o.triples, games: (o) => o.triplesGames },
  { label: "curious.total.combatsWon", sum: (o) => o.combatsWon, games: combats },
  { label: "curious.total.goldBeyond", sum: (o) => o.goldBeyond, games: (o) => o.goldBeyondGames, advanced: true },
  { label: "curious.total.sellGold", sum: (o) => o.sellGold, games: (o) => o.sellGoldGames, advanced: true },
  { label: "curious.total.freeRolls", sum: (o) => o.freeRolls, games: shop, advanced: true },
  { label: "curious.total.buys", sum: (o) => o.buys, games: shop, advanced: true },
  { label: "curious.total.sells", sum: (o) => o.sells, games: shop, advanced: true },
  { label: "curious.total.damageDealt", sum: (o) => o.damageDealt, games: (o) => o.damageDealtGames, inferred: true, advanced: true },
  { label: "curious.total.damageTaken", sum: (o) => o.damageTaken, games: (o) => o.damageTakenGames, inferred: true, advanced: true },
];

/** One total as a tile: the sum, the games behind it, and its source mark. */
export function totalTile(spec: TotalSpec, o: Overview): TileProps {
  const games = Math.min(spec.games(o), o.games);
  const sum = games === 0 ? null : spec.sum(o);
  return {
    label: t(spec.label),
    value: compact(sum),
    backing: backing(games, o.games),
    source: sum === null ? "unknown" : spec.inferred ? "inferred" : undefined,
  };
}

export function totalTiles(o: Overview): { simple: TileProps[]; advanced: TileProps[] } {
  return {
    simple: TOTALS.filter((s) => !s.advanced).map((s) => totalTile(s, o)),
    advanced: TOTALS.filter((s) => s.advanced).map((s) => totalTile(s, o)),
  };
}

/**
 * The app's curious stats (T-216, D-057) the website cannot show yet, and
 * why: each needs data the data API does not summarize (T-502). Listed on
 * the page so nothing is silently missing.
 */
export const NOT_YET: readonly StringKey[] = ["curious.gap.goldenSold", "curious.gap.mostBought", "curious.gap.extraGold", "curious.gap.bothModes"];

export function gapItems(): VNode[] {
  return NOT_YET.map((key) => h("li", {}, t(key)));
}

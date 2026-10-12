// Formatting and small calculations for the data pages (T-501). Pure
// functions, built at build time or in the browser alike. An unknown value
// is null and prints as a dash: unknown is never shown as 0.
import { numberFormat, pluralForm } from "../i18n.ts";
import { t } from "./strings.ts";

export type Mode = "solo" | "duos";
export type PlaceGroup = "first" | "top" | "bottom" | "unknown";
export type Result = "win" | "loss" | "tie" | "unknown";
export type Unit = "games" | "turns";

// The same in every language (test/unit/i18n.test.ts), so it can be a constant.
const DASH = "—";
// In the page's language (T-508): 2.50 / 2,50, 12.9K / 12,9 mil, 50% / 50 %.
const PLAIN = { format: (v: number) => numberFormat({ maximumFractionDigits: 0 }).format(v) };
const SHORT = { format: (v: number) => numberFormat({ notation: "compact", maximumFractionDigits: 1 }).format(v) };
const TWO = { format: (v: number) => numberFormat({ minimumFractionDigits: 2, maximumFractionDigits: 2 }).format(v) };
const ONE = { format: (v: number) => numberFormat({ minimumFractionDigits: 1, maximumFractionDigits: 1 }).format(v) };
const PERCENT = { format: (v: number) => numberFormat({ style: "percent", maximumFractionDigits: 0 }).format(v) };

/** Plain up to 9,999, compact from 10,000 (12.9K, 4.2M). */
const COMPACT_FROM = 10_000;

const known = (value: number | null | undefined): value is number =>
  typeof value === "number" && Number.isFinite(value);

export function compact(value: number | null | undefined): string {
  if (!known(value)) return DASH;
  return Math.abs(value) < COMPACT_FROM ? PLAIN.format(value) : SHORT.format(value);
}

/** An average (place, gold per game) with two decimals. */
export function average(value: number | null | undefined): string {
  return known(value) ? TWO.format(value) : DASH;
}

/** An average place on a player's pages, with one decimal (4.3): T-559, D-131. */
export function placeAverage(value: number | null | undefined): string {
  return known(value) ? ONE.format(value) : DASH;
}

/** Where a figure stops being a whole number: from here up the decimal adds nothing. */
const WHOLE_FROM = 10;

/** A per-game figure (gold, damage, rolls, actions per minute): whole from 10 up (63), one decimal below (2.1). */
export function perGameText(value: number | null | undefined): string {
  if (!known(value)) return DASH;
  return Math.abs(value) >= WHOLE_FROM ? compact(Math.round(value)) : ONE.format(value);
}

/** part / whole as a whole percent; a dash when the whole is 0 or unknown. */
export function share(part: number | null | undefined, whole: number | null | undefined): string {
  if (!known(part) || !known(whole) || whole === 0) return DASH;
  return PERCENT.format(part / whole);
}

function unitWord(unit: Unit, count: number): string {
  return t(`unit.${unit}.${pluralForm(count)}`);
}

function checkCount(value: number): void {
  if (!Number.isInteger(value) || value < 0) throw new Error(`not a count: ${value}`);
}

/** The backing line under a value: how many games (or turns) it comes from. */
export function backing(knownCount: number, total: number, unit: Unit = "games"): string {
  checkCount(knownCount);
  checkCount(total);
  if (knownCount > total) throw new Error(`${knownCount} known is more than ${total} in total`);
  if (total === 0) return t("backing.empty");
  if (knownCount === 0) return t("backing.none", { total, unit: unitWord(unit, total) });
  if (total === 1) return t("backing.one", { unit: unitWord(unit, 1) });
  if (knownCount === total) return t("backing.all", { total, unit: unitWord(unit, total) });
  return t("backing.some", { known: knownCount, total, unit: unitWord(unit, total) });
}

/** The first n rows and the rest; one hidden row is shown rather than folded. */
export function topN<T>(rows: readonly T[], n: number): { shown: T[]; rest: T[] } {
  if (!Number.isInteger(n) || n < 1) throw new Error("topN needs at least 1 row");
  if (rows.length <= n + 1) return { shown: [...rows], rest: [] };
  return { shown: rows.slice(0, n), rest: rows.slice(n) };
}

export const PLACES: Record<Mode, number> = { solo: 8, duos: 4 };

/** 1st; top 4 (Solo) or top 2 (Duos); the bottom half; anything else unknown. */
export function placeGroup(place: number | null | undefined, mode: Mode): PlaceGroup {
  if (!known(place) || !Number.isInteger(place) || place < 1 || place > PLACES[mode]) return "unknown";
  if (place === 1) return "first";
  return place <= PLACES[mode] / 2 ? "top" : "bottom";
}

/** "1st", "2nd", "3rd", "4th"… (Spanish "1.º"); places only go to 8. */
export function ordinal(place: number): string {
  if (place === 1) return t("place.1");
  if (place === 2) return t("place.2");
  if (place === 3) return t("place.3");
  return t("place.n", { n: place });
}

export interface PlaceBar {
  place: number;
  count: number;
  share: number;
  group: PlaceGroup;
}

/** One bar per place with its share of the games that have a place. */
export function placeDistribution(
  places: readonly (number | null)[],
  mode: Mode,
): { bars: PlaceBar[]; unknown: number; total: number } {
  const counts = new Array<number>(PLACES[mode]).fill(0);
  let unknown = 0;
  for (const place of places) {
    if (placeGroup(place, mode) === "unknown") unknown += 1;
    else counts[(place as number) - 1] += 1;
  }
  const total = places.length - unknown;
  const bars = counts.map((count, i) => ({
    place: i + 1,
    count,
    share: total === 0 ? 0 : count / total,
    group: placeGroup(i + 1, mode),
  }));
  return { bars, unknown, total };
}

export type Direction = "better" | "worse" | "same";

export interface Trend {
  text: string;
  direction: Direction;
  /** Which way the number moved, for the arrow (up is not always better). */
  sign: -1 | 0 | 1;
}

/** A signed change ("−0.30", "+5%") and whether it is an improvement. */
export function trend(
  delta: number | null | undefined,
  betterWhen: "lower" | "higher",
  format: "number" | "percent" = "number",
): Trend | null {
  if (!known(delta)) return null;
  const size = format === "percent" ? PERCENT.format(Math.abs(delta)) : TWO.format(Math.abs(delta));
  // Judged on the shown value: a change that rounds to zero is no change.
  if (!/[1-9]/.test(size)) return { text: `±${size}`, direction: "same", sign: 0 };
  const improved = betterWhen === "lower" ? delta < 0 : delta > 0;
  return { text: `${delta < 0 ? "−" : "+"}${size}`, direction: improved ? "better" : "worse", sign: delta < 0 ? -1 : 1 };
}

/** What a player still needs to rank on a board, or null when they rank. */
export function threshold(games: number, min: number): string | null {
  return games < min ? t("threshold.needs", { min, games }) : null;
}

const round2 = (value: number) => Math.round(value * 100) / 100;

export interface SparkPoint {
  x: number;
  y: number;
  value: number;
  index: number;
}

/**
 * A line through the values with a gap at each null. The y range is fixed
 * (min..max) so two sparklines of the same stat compare; a value outside it
 * is left out like an unknown one; invert puts min on
 * top (place: 1st is best). A 1px inset keeps the 2px line inside the box.
 */
export function sparkline(
  values: readonly (number | null)[],
  box: { width: number; height: number; min: number; max: number; invert?: boolean },
): { path: string; points: SparkPoint[] } {
  const { width, height, min, max, invert = false } = box;
  const step = values.length > 1 ? width / (values.length - 1) : 0;
  const inner = height - 2;
  const span = max - min || 1;
  const points: SparkPoint[] = [];
  let path = "";
  let pen = false;
  values.forEach((value, index) => {
    // Unknown or out of range: a gap, never drawn at the edge as if it were min or max.
    if (!known(value) || value < min || value > max) {
      pen = false;
      return;
    }
    const ratio = (value - min) / span;
    const x = round2(values.length > 1 ? index * step : width);
    const y = round2(1 + inner * (invert ? ratio : 1 - ratio));
    path += `${pen ? "L" : "M"}${x} ${y}`;
    pen = true;
    points.push({ x, y, value, index });
  });
  return { path, points };
}

/** Bar length on one shared scale; a value that is not 0 is at least 2px. */
export function barScale(values: readonly number[], maxLength: number): (value: number) => number {
  const top = Math.max(0, ...values.filter(known).map((v) => Math.abs(v)));
  return (value) => {
    if (top === 0 || value === 0 || !known(value)) return 0;
    return round2(Math.max(2, (Math.abs(value) / top) * maxLength));
  };
}

/**
 * SVG path of a column growing from a baseline: square at the base, the data
 * end rounded (4px, less on a short bar). "down" grows below the baseline.
 */
export function columnPath(x: number, baseline: number, width: number, length: number, dir: "up" | "down"): string {
  if (length <= 0) return "";
  const r = round2(Math.min(4, length, width / 2));
  const s = dir === "up" ? -1 : 1;
  const end = round2(baseline + s * length);
  const bend = round2(end - s * r);
  const x2 = round2(x + width);
  return `M${x} ${baseline}V${bend}Q${x} ${end} ${round2(x + r)} ${end}H${round2(x2 - r)}Q${x2} ${end} ${x2} ${bend}V${baseline}Z`;
}

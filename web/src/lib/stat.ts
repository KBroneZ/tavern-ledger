// A minion's Attack or Health the way the game writes a big one (T-616,
// D-141): from 1,000 up it is 1K, 1M, 1B with one decimal that is cut, not
// rounded (1,999 is 1.9K, 999,999 is 999.9K, so a value never jumps a
// unit), and no ".0". The exact value is for the tooltip: `exactStat`.
//
// The desktop app has the same rule in crates/desktop/ui/stat-format.js; both
// run the same table of cases in their tests. An unknown value is "?".
import { lang, numberFormat } from "./i18n.ts";

const UNKNOWN = "?";

/** Biggest first: from this value up the number is written in this unit. */
const UNITS: readonly (readonly [number, string])[] = [
  [1_000_000_000, "B"],
  [1_000_000, "M"],
  [1_000, "K"],
];

const known = (value: number | null | undefined): value is number =>
  typeof value === "number" && Number.isFinite(value);

export function shortStat(value: number | null | undefined): string {
  if (!known(value)) return UNKNOWN;
  const whole = Math.trunc(value);
  const sign = whole < 0 ? "-" : "";
  const size = Math.abs(whole);
  const unit = UNITS.find(([from]) => size >= from);
  if (!unit) return `${sign}${size}`;
  const [from, letter] = unit;
  const tenths = Math.floor(size / (from / 10));
  const decimal = tenths % 10;
  const point = lang() === "es" ? "," : ".";
  return `${sign}${Math.floor(tenths / 10)}${decimal === 0 ? "" : point + decimal}${letter}`;
}

/** Every digit, grouped the page's way: 2,147,483,647 / 2.147.483.647. */
export function exactStat(value: number | null | undefined): string {
  if (!known(value)) return UNKNOWN;
  return numberFormat({ maximumFractionDigits: 0 }).format(Math.trunc(value));
}

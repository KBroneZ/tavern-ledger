// The accuracy page's figures (T-607): pure functions over the checked
// export, drawn at build time in the page's language. Every accuracy figure
// comes with the blocked share beside it (D-089): modeFigures returns both,
// and the page draws them in one row. Unknown is a dash, never 0.
import { dateFormat, lang, numberFormat, pluralForm } from "../i18n.ts";
import type { AccuracyMode, BlockingCard, Bucket, Builds, CardNames, Kits, MissCause, ModeAccuracy } from "./data.ts";
import { at } from "./strings.ts";

const DASH = "—";

export interface Figure {
  value: string;
  /** The line under the value. */
  backing: string;
}

export interface ModeFigures {
  modeName: string;
  combats: string;
  right: Figure;
  meanP: Figure;
  blocked: { value: string; note: string };
  brier: Figure;
  refused: string;
}

const percent = (value: number | null): string =>
  value === null ? DASH : numberFormat({ style: "percent", maximumFractionDigits: 0 }).format(value);

/**
 * part / whole as a whole percent that never rounds to a claim the counts
 * do not make: above 0 is at least "<1%", below all is at most 99%.
 */
function sharePercent(part: number, whole: number): string {
  if (whole === 0) return DASH;
  const value = part / whole;
  if (part > 0 && value < 0.005) return `<${percent(0.01)}`;
  if (part < whole && value >= 0.995) return percent(0.99);
  return percent(value);
}
const plain = (value: number): string => numberFormat({ maximumFractionDigits: 0 }).format(value);

export const modeName = (mode: AccuracyMode): string => at(mode === "solo" ? "mode.solo" : "mode.duos");

function blockedNote(m: ModeAccuracy, name: string): string {
  if (m.combats_with_result === 0) return at("blocked.empty", { mode: name });
  if (m.blocked === 0) return at("blocked.none", { mode: name, total: plain(m.combats_with_result) });
  return at("blocked.some", {
    share: sharePercent(m.blocked, m.combats_with_result),
    mode: name,
    blocked: plain(m.blocked),
    total: plain(m.combats_with_result),
  });
}

/** One mode's numbers, the blocked share always among them. */
export function modeFigures(m: ModeAccuracy, mode: AccuracyMode): ModeFigures {
  const name = modeName(mode);
  const scored = m.scored > 0;
  return {
    modeName: name,
    combats: at("mode.combats", { count: plain(m.combats_with_result) }),
    right: {
      value: scored ? sharePercent(m.most_likely_right, m.scored) : DASH,
      backing: scored ? at("right.backing", { right: plain(m.most_likely_right), scored: plain(m.scored) }) : "",
    },
    meanP: {
      value: percent(m.mean_p_real),
      backing: scored ? at("tile.meanP.back", { count: plain(m.scored) }) : "",
    },
    blocked: { value: sharePercent(m.blocked, m.combats_with_result), note: blockedNote(m, name) },
    brier: {
      value: m.brier === null ? DASH : numberFormat({ minimumFractionDigits: 3, maximumFractionDigits: 3 }).format(m.brier),
      backing: at("tile.brier.back"),
    },
    refused: plain(m.refused),
  };
}

export interface CalibrationBar {
  range: string;
  predictions: number;
  said: number | null;
  happened: number | null;
  saidText: string;
  happenedText: string;
}

/** "0–20%": the bucket's bounds in the page's language. */
function rangeText(b: Bucket): string {
  const whole = (v: number) => Math.round(v * 100);
  const end = percent(whole(b.to) / 100);
  return `${plain(whole(b.from))}–${end}`;
}

/** The calibration buckets as bars: an empty bucket has no bar, never a bar of 0. */
export function calibrationBars(buckets: readonly Bucket[]): CalibrationBar[] {
  return buckets.map((b) => ({
    range: rangeText(b),
    predictions: b.predictions,
    said: b.mean_said,
    happened: b.happened,
    saidText: percent(b.mean_said),
    happenedText: percent(b.happened),
  }));
}

/** A bar's label: its group, how many predictions (one or more) and the two values. */
export function calibrationMark(b: CalibrationBar): string {
  if (b.predictions === 0) return `${b.range}: ${at("cal.none")}`;
  return at(`cal.mark.${pluralForm(b.predictions)}`, {
    range: b.range,
    predictions: plain(b.predictions),
    said: b.saidText,
    happened: b.happenedText,
  });
}

export interface BlockingRow {
  name: string;
  id: string;
  combats: string;
}

/** The blocking cards named in the page's language; a card with no name keeps its id. */
export function blockingRows(cards: readonly BlockingCard[], names: CardNames): BlockingRow[] {
  return cards.map((c) => {
    const entry = names[c.card_id];
    const base = entry ? entry[lang()] : c.card_id;
    return {
      name: entry?.golden ? at("card.golden", { name: base }) : base,
      id: c.card_id,
      combats: plain(c.combats),
    };
  });
}

/** The measuring day in the page's language ("11 October 2026" / "11 de octubre de 2026"). */
export function dayText(date: string): string {
  const [y, m, d] = date.split("-").map(Number);
  const day = new Date(Date.UTC(y, m - 1, d));
  return dateFormat({ day: "numeric", month: "long", year: "numeric", timeZone: "UTC" }, "en-GB").format(day);
}

export interface CauseRow {
  id: string;
  name: string;
  /** One plain line. */
  why: string;
  solo: string;
  duos: string;
  combats: string;
  share: string;
}

/** A share as a whole percent that never rounds to a claim the counts do not make. */
function shareText(share: number): string {
  if (share > 0 && share < 0.005) return `<${percent(0.01)}`;
  if (share < 1 && share >= 0.995) return percent(0.99);
  return percent(share);
}

/** The causes of the misses in the page's language, in the export's order (largest first). */
export function causeRows(causes: readonly MissCause[]): CauseRow[] {
  return causes.map((c) => ({
    id: c.cause,
    name: at(`cause.${c.cause}`),
    why: at(`cause.${c.cause}.why`),
    solo: plain(c.solo),
    duos: plain(c.duos),
    combats: plain(c.solo + c.duos),
    share: shareText(c.share),
  }));
}

/**
 * The lines on the combats played with the heroes' powers and trinkets: how
 * many, and (Advanced only, like every Brier score) their scores with and
 * without them; `scores` is null when there is none to give.
 */
export function kitsNote(k: Kits): { text: string; scores: string | null } {
  if (k.combats === 0) return { text: at("about.kits.none"), scores: null };
  const brier = (v: number | null) => (v === null ? DASH : numberFormat({ minimumFractionDigits: 3, maximumFractionDigits: 3 }).format(v));
  const text = at("about.kits", { combats: plain(k.combats) });
  if (k.scored === 0) return { text, scores: null };
  const scores = at("about.kits.scores", { scored: plain(k.scored), with: brier(k.with.brier), without: brier(k.without.brier) });
  return { text, scores };
}

export interface BuildsView {
  /** The newest build and the day, or that none was read. */
  summary: string;
  rows: { build: string; games: string }[];
}

/** The game builds checked, newest first; a build number is not a count, so it is never grouped. */
export function buildsView(builds: Builds, date: string): BuildsView {
  if (builds.newest === null) return { summary: at("builds.none"), rows: [] };
  return {
    summary: at("builds.newest", { build: String(builds.newest), date: dayText(date) }),
    rows: builds.checked.map((b) => ({
      build: String(b.build),
      games: at(`builds.row.${pluralForm(b.count)}`, { count: plain(b.count) }),
    })),
  };
}

// The accuracy page's figures (T-607): pure functions over the checked
// export, drawn at build time in the page's language. Every accuracy figure
// comes with the blocked share beside it (D-089): modeFigures returns both,
// and the page draws them in one row. Unknown is a dash, never 0.
import { dateFormat, lang, numberFormat } from "../i18n.ts";
import type { AccuracyMode, BlockingCard, Bucket, CardNames, ModeAccuracy } from "./data.ts";
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
const plain = (value: number): string => numberFormat({ maximumFractionDigits: 0 }).format(value);

export const modeName = (mode: AccuracyMode): string => at(mode === "solo" ? "mode.solo" : "mode.duos");

function blockedNote(m: ModeAccuracy, name: string): string {
  if (m.combats_with_result === 0) return at("blocked.empty", { mode: name });
  if (m.blocked === 0) return at("blocked.none", { mode: name, total: plain(m.combats_with_result) });
  return at("blocked.some", {
    share: percent(m.blocked_share),
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
      value: scored ? percent(m.most_likely_right / m.scored) : DASH,
      backing: scored ? at("right.backing", { right: plain(m.most_likely_right), scored: plain(m.scored) }) : "",
    },
    meanP: {
      value: percent(m.mean_p_real),
      backing: scored ? at("tile.meanP.back", { count: plain(m.scored) }) : "",
    },
    blocked: { value: percent(m.blocked_share), note: blockedNote(m, name) },
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

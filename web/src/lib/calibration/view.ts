// The calibration page's figures (T-655): pure functions over the checked
// file, drawn at build time in the page's language. Empty (no model), thin
// (too few games) and none (no combat in a group) each read differently;
// a withheld mean is a dash with its reason in words, never a 0.
import { dateFormat, numberFormat, pluralForm } from "../i18n.ts";
import { MODES, MIN_GAMES, THIN_GAMES, type CalibratedMode, type Calibration, type CalibrationBucket, type CalibrationBuild, type CalibrationMode } from "./data.ts";
import { ct } from "./strings.ts";

const DASH = "—";
const PLACES: Record<CalibrationMode, number> = { solo: 8, duos: 4 };

const plain = (value: number): string => numberFormat({ maximumFractionDigits: 0 }).format(value);
const twoDecimals = (value: number): string => numberFormat({ minimumFractionDigits: 2, maximumFractionDigits: 2 }).format(value);

export interface RowView {
  range: string;
  said: string;
  real: string;
  games: string;
  rows: string;
  /** For said, real, games and rows in that order: whether the value is withheld (drawn with `why` for screen readers). */
  withheld: [boolean, boolean, boolean, boolean];
  /** Why the averages are a dash, in words; null when they are shown. */
  why: string | null;
}

export interface BuildView {
  caption: string;
  summary: string;
  rows: RowView[];
}

interface ModeBase {
  mode: CalibrationMode;
  name: string;
}

export type ModeView =
  | (ModeBase & { published: false; none: string })
  | (ModeBase & {
      published: true;
      summary: string;
      readiness: string;
      distance: { value: string; backing: string };
      note: string;
      builds: BuildView[];
      /** The games counted but not shown by build; null when there are none. */
      other: string | null;
      /** Why no build is listed; null when one is. */
      buildsNone: string | null;
    });

export interface CalibrationView {
  /** No model published for either mode. */
  empty: boolean;
  /** The run's day in the page's language; null when empty. */
  date: string | null;
  /** The floors the empty state explains. */
  minGames: string;
  thinGames: string;
  modes: ModeView[];
}

export const modeName = (mode: CalibrationMode): string => ct(mode === "solo" ? "mode.solo" : "mode.duos");

/** The run's day ("2 November 2026" / "2 de noviembre de 2026"). */
function dayText(date: string): string {
  const [y, m, d] = date.split("-").map(Number);
  return dateFormat({ day: "numeric", month: "long", year: "numeric", timeZone: "UTC" }, "en-GB").format(new Date(Date.UTC(y, m - 1, d)));
}

function rowView(b: CalibrationBucket, thin: number): RowView {
  const why = b.said !== null ? null : b.games === 0 ? ct("build.empty") : ct("build.thin", { thin: plain(thin) });
  return {
    range: ct("build.range", { from: plain(b.from), to: plain(b.to) }),
    said: b.said === null ? DASH : twoDecimals(b.said),
    real: b.real === null ? DASH : twoDecimals(b.real),
    // A thin bucket's counts are not published: only that there are fewer than thin.
    games: b.games === null ? `<${plain(thin)}` : plain(b.games),
    rows: b.rows === null ? DASH : plain(b.rows),
    withheld: [b.said === null, b.real === null, b.games === null, b.rows === null],
    why,
  };
}

function buildView(b: CalibrationBuild, thin: number): BuildView {
  return {
    caption: ct("build.caption", { build: String(b.build) }),
    summary: ct("build.summary", { games: plain(b.games), rows: plain(b.rows) }),
    rows: b.buckets.map((bucket) => rowView(bucket, thin)),
  };
}

function publishedView(m: CalibratedMode, mode: CalibrationMode): ModeView {
  const thin = plain(m.thin_games);
  return {
    mode,
    name: modeName(mode),
    published: true,
    summary: ct("mode.summary", { games: plain(m.games), rows: plain(m.rows), folds: plain(m.folds) }),
    readiness: ct(m.ready ? "mode.ready" : "mode.notReady", { min: plain(m.min_games) }),
    distance: {
      value: twoDecimals(m.mean_distance.model),
      backing: ct("distance.back", { perTurn: twoDecimals(m.mean_distance.per_turn), overall: twoDecimals(m.mean_distance.overall) }),
    },
    note: ct("build.note", { last: plain(PLACES[mode]), thin }),
    builds: m.builds.map((b) => buildView(b, m.thin_games)),
    other: m.other_games === 0 ? null : ct(`builds.other.${pluralForm(m.other_games)}`, { count: plain(m.other_games), thin }),
    buildsNone: m.builds.length === 0 ? ct("builds.none", { thin }) : null,
  };
}

/** The whole page: the empty state when no mode has a model, else each mode's numbers or its own empty line. */
export function calibrationView(data: Calibration): CalibrationView {
  return {
    empty: data.modes.solo === null && data.modes.duos === null,
    date: data.date === null ? null : dayText(data.date),
    minGames: plain(MIN_GAMES),
    thinGames: plain(THIN_GAMES),
    modes: MODES.map((mode) => {
      const m = data.modes[mode];
      return m === null ? { mode, name: modeName(mode), published: false, none: ct("mode.none", { mode: modeName(mode) }) } : publishedView(m, mode);
    }),
  };
}

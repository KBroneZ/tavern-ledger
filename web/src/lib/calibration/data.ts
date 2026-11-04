// The expected-place calibration file (T-655): what `place-train` writes
// as calibration.json (crates/expected-place/src/calibration.rs), committed
// as src/data/calibration.json and read at build time. Until a model is
// published the committed file is the empty state: both modes null and no
// date. It is read strictly: the format first, then exactly the file's
// keys, its totals checked against each other, and no mean with fewer than
// THIN_GAMES games behind it, so a bad or too-detailed file fails the build
// instead of reaching the page.

export const CALIBRATION_FORMAT = 1;
export const MODES = ["solo", "duos"] as const;
export type CalibrationMode = (typeof MODES)[number];

/** The fewest games of a mode before players see the model's numbers (expected_place::model::MIN_GAMES). */
export const MIN_GAMES = 150;
/** The fewest games behind a published mean (expected_place::calibration::THIN_GAMES); a file may raise it, never lower it. */
export const THIN_GAMES = 20;

const MAX_BUILDS = 20;
const MAX_BUILD = 1_000_000_000;
const PLACES: Record<CalibrationMode, number> = { solo: 8, duos: 4 };

export interface CalibrationBucket {
  /** Expected places from `from` up to `to` (the last bucket includes `to`). */
  from: number;
  to: number;
  /** Combats whose expected place fell here; null for a thin bucket (1 to thin_games - 1 games), whose counts are not published. */
  rows: number | null;
  /** The games they come from; null for a thin bucket, 0 for an empty one. */
  games: number | null;
  /** null exactly when fewer than thin_games games are behind the bucket (thin or empty). */
  said: number | null;
  real: number | null;
}

export interface CalibrationBuild {
  build: number;
  games: number;
  rows: number;
  buckets: CalibrationBucket[];
}

export interface MeanDistance {
  model: number;
  per_turn: number;
  overall: number;
}

export interface CalibratedMode {
  /** The rows' format the model was trained on. */
  encoder: number;
  games: number;
  rows: number;
  folds: number;
  min_games: number;
  /** games >= min_games: players may see the model's numbers. */
  ready: boolean;
  thin_games: number;
  /** Mean distance between the expected place and the real one, in places. */
  mean_distance: MeanDistance;
  /** Newest first; only builds with at least thin_games games. */
  builds: CalibrationBuild[];
  /** Games on the other builds, or with no build: counted, not shown. */
  other_games: number;
}

export interface Calibration {
  format: number;
  /** The training run's day; null exactly when no model is published. */
  date: string | null;
  /** null: no model published for that mode. */
  modes: Record<CalibrationMode, CalibratedMode | null>;
}

const DATE = /^\d{4}-\d{2}-\d{2}$/;

/** A real calendar day, not just the shape (2026-13-45 fails). */
function isDay(value: string): boolean {
  if (!DATE.test(value)) return false;
  const [y, m, d] = value.split("-").map(Number);
  const day = new Date(Date.UTC(y, m - 1, d));
  return day.getUTCFullYear() === y && day.getUTCMonth() === m - 1 && day.getUTCDate() === d;
}

type Raw = Record<string, unknown>;

function object(value: unknown, where: string): Raw {
  if (typeof value !== "object" || value === null || Array.isArray(value)) throw new Error(`${where}: not an object`);
  return value as Raw;
}

/** Exactly these keys: a missing one and an extra one both fail, by name. */
function keys(raw: Raw, expected: readonly string[], where: string): void {
  for (const key of expected) if (!(key in raw)) throw new Error(`${where}: no ${key}`);
  for (const key of Object.keys(raw)) if (!expected.includes(key)) throw new Error(`${where}: unexpected key ${key}`);
}

function count(value: unknown, where: string): number {
  if (typeof value !== "number" || !Number.isInteger(value) || value < 0) throw new Error(`${where}: not a count`);
  return value;
}

function between(value: unknown, min: number, max: number, where: string): number {
  if (typeof value !== "number" || !Number.isFinite(value) || value < min || value > max) throw new Error(`${where}: out of range`);
  return value;
}

function bucket(value: unknown, index: number, places: number, thin: number, where: string): CalibrationBucket {
  const raw = object(value, where);
  keys(raw, ["from", "to", "rows", "games", "said", "real"], where);
  const from = count(raw.from, `${where}.from`);
  const to = count(raw.to, `${where}.to`);
  if (from !== index + 1 || to !== from + 1) throw new Error(`${where}.from: not the bucket ${index + 1} to ${index + 2}`);
  // A thin bucket publishes no count at all: rows and games are both null.
  if ((raw.rows === null) !== (raw.games === null)) throw new Error(`${where}: rows and games are null together`);
  const rows = raw.rows === null ? null : count(raw.rows, `${where}.rows`);
  const games = raw.games === null ? null : count(raw.games, `${where}.games`);
  if (rows !== null && games !== null) {
    if ((rows === 0) !== (games === 0) || games > rows) throw new Error(`${where}: games and rows are 0 together, and games are at most rows`);
    if (games > 0 && games < thin) throw new Error(`${where}.games: a count under ${thin} games is published`);
  }
  const shown = games !== null && games >= thin;
  if ((raw.said === null) === shown || (raw.real === null) === shown) {
    throw new Error(`${where}: said and real are null exactly with fewer than ${thin} games`);
  }
  return {
    from,
    to,
    rows,
    games,
    // A mean is rounded to 2 decimals, so it may sit on the bucket's upper bound.
    said: raw.said === null ? null : between(raw.said, from, to, `${where}.said`),
    real: raw.real === null ? null : between(raw.real, 1, places, `${where}.real`),
  };
}

function build(value: unknown, places: number, thin: number, where: string): CalibrationBuild {
  const raw = object(value, where);
  keys(raw, ["build", "games", "rows", "buckets"], where);
  const number = count(raw.build, `${where}.build`);
  if (number === 0 || number > MAX_BUILD) throw new Error(`${where}.build: not a build number`);
  const games = count(raw.games, `${where}.games`);
  if (games < thin) throw new Error(`${where}.games: fewer than ${thin} games`);
  const rows = count(raw.rows, `${where}.rows`);
  if (!Array.isArray(raw.buckets) || raw.buckets.length !== places - 1) throw new Error(`${where}.buckets: not ${places - 1} buckets`);
  const buckets = raw.buckets.map((b, i) => bucket(b, i, places, thin, `${where}.buckets[${i}]`));
  for (const b of buckets) if ((b.games ?? 0) > games) throw new Error(`${where}: a bucket has more games than the build`);
  // A thin bucket's rows are not published but are at least 1 each.
  const known = buckets.reduce((sum, b) => sum + (b.rows ?? 0), 0);
  const thinBuckets = buckets.filter((b) => b.rows === null).length;
  if (thinBuckets === 0 ? known !== rows : known + thinBuckets > rows) throw new Error(`${where}.rows: not the sum of the buckets' rows`);
  return { build: number, games, rows, buckets };
}

const MODE_KEYS = ["encoder", "games", "rows", "folds", "min_games", "ready", "thin_games", "mean_distance", "builds", "other_games"] as const;

function meanDistance(value: unknown, places: number, where: string): MeanDistance {
  const raw = object(value, where);
  keys(raw, ["model", "per_turn", "overall"], where);
  return {
    model: between(raw.model, 0, places - 1, `${where}.model`),
    per_turn: between(raw.per_turn, 0, places - 1, `${where}.per_turn`),
    overall: between(raw.overall, 0, places - 1, `${where}.overall`),
  };
}

function calibratedMode(value: unknown, mode: CalibrationMode): CalibratedMode {
  const where = `calibration.modes.${mode}`;
  const raw = object(value, where);
  keys(raw, MODE_KEYS, where);
  const places = PLACES[mode];
  const encoder = count(raw.encoder, `${where}.encoder`);
  if (encoder === 0) throw new Error(`${where}.encoder: not a format`);
  const thin = count(raw.thin_games, `${where}.thin_games`);
  if (thin < THIN_GAMES) throw new Error(`${where}.thin_games: below ${THIN_GAMES}`);
  const games = count(raw.games, `${where}.games`);
  if (games < thin) throw new Error(`${where}.games: fewer than ${thin} games`);
  const rows = count(raw.rows, `${where}.rows`);
  if (rows < games) throw new Error(`${where}.rows: fewer than the games`);
  const folds = count(raw.folds, `${where}.folds`);
  if (folds < 2 || folds > games) throw new Error(`${where}.folds: not between 2 and the games`);
  const minGames = count(raw.min_games, `${where}.min_games`);
  if (minGames < MIN_GAMES) throw new Error(`${where}.min_games: below ${MIN_GAMES}`);
  if (raw.ready !== (games >= minGames)) throw new Error(`${where}.ready: not games >= min_games`);
  if (!Array.isArray(raw.builds) || raw.builds.length > MAX_BUILDS) throw new Error(`${where}.builds: not a list of up to ${MAX_BUILDS}`);
  const builds = raw.builds.map((b, i) => build(b, places, thin, `${where}.builds[${i}]`));
  builds.forEach((b, i) => {
    if (i > 0 && b.build >= builds[i - 1].build) throw new Error(`${where}.builds: builds twice or not newest first`);
  });
  const other = count(raw.other_games, `${where}.other_games`);
  if (builds.reduce((sum, b) => sum + b.games, 0) + other !== games) throw new Error(`${where}.other_games: the builds' games and other_games are not the games`);
  if (builds.reduce((sum, b) => sum + b.rows, 0) > rows) throw new Error(`${where}.rows: fewer than the builds' rows`);
  return {
    encoder,
    games,
    rows,
    folds,
    min_games: minGames,
    ready: games >= minGames,
    thin_games: thin,
    mean_distance: meanDistance(raw.mean_distance, places, `${where}.mean_distance`),
    builds,
    other_games: other,
  };
}

/** Reads the calibration file (untrusted until checked); throws, naming the key, on anything else. */
export function parseCalibration(value: unknown): Calibration {
  const raw = object(value, "calibration");
  // The format first: another shape is not read at all.
  if (raw.format !== CALIBRATION_FORMAT) throw new Error(`calibration: format is not ${CALIBRATION_FORMAT}`);
  keys(raw, ["format", "date", "modes"], "calibration");
  const rawModes = object(raw.modes, "calibration.modes");
  keys(rawModes, MODES, "calibration.modes");
  const modes = {
    solo: rawModes.solo === null ? null : calibratedMode(rawModes.solo, "solo"),
    duos: rawModes.duos === null ? null : calibratedMode(rawModes.duos, "duos"),
  };
  const published = modes.solo !== null || modes.duos !== null;
  if (raw.date === null) {
    if (published) throw new Error("calibration.date: null with a model published");
  } else if (typeof raw.date !== "string" || !isDay(raw.date)) {
    throw new Error("calibration.date: not a YYYY-MM-DD day");
  } else if (!published) {
    throw new Error("calibration.date: a date with no model published");
  }
  return { format: CALIBRATION_FORMAT, date: raw.date as string | null, modes };
}

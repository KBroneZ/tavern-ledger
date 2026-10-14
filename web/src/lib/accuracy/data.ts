// The simulator accuracy export (T-607, D-088, D-089): the numbers
// `combat-validate --export` writes, committed as src/data/accuracy.json and
// read at build time. The shape is the design doc's (combat-simulator.md
// section 7.6). It is read strictly: the format first, then exactly the
// export's keys with their types, so a key the export does not have (a name,
// a seat, a game) or one it lost fails the build instead of reaching the page.

export const ACCURACY_FORMAT = 3;
export const MODES = ["solo", "duos"] as const;
export type AccuracyMode = (typeof MODES)[number];

export interface Bucket {
  from: number;
  to: number;
  predictions: number;
  /** null for an empty bucket. */
  mean_said: number | null;
  happened: number | null;
}

export interface ModeAccuracy {
  combats_with_result: number;
  blocked: number;
  /** null with no combat. */
  blocked_share: number | null;
  refused: number;
  scored: number;
  /** null when nothing was scored. */
  mean_p_real: number | null;
  brier: number | null;
  most_likely_right: number;
  calibration: Bucket[];
}

export interface BlockingCard {
  card_id: string;
  combats: number;
}

/**
 * Why a combat's most likely result was not the real one (D-115, D-127), as
 * the export names them: chance (the replay follows the game to the end);
 * where the replay parts from the game, something the log does not show
 * (missing inputs), else a card or Dark Gift not modelled or approximate,
 * else the first rule that differs; or, with no replay saved, what the cards
 * say. The app's recap and the game page use the same ids.
 */
export const CAUSE_IDS = [
  "chance",
  "missing_inputs",
  "approximate_card",
  "replay_first_attacker",
  "replay_after_teammate",
  "replay_attack_order",
  "replay_target",
  "replay_minions",
  "replay_stats",
  "replay_end",
  "replay_no_defender",
  "no_replay_not_modelled",
  "no_replay_approximate",
  "no_replay_exact",
] as const;
export type CauseId = (typeof CAUSE_IDS)[number];

export interface MissCause {
  cause: CauseId;
  solo: number;
  duos: number;
  /** Of all the misses. */
  share: number;
}

export interface Misses {
  total: number;
  /** Largest first. */
  causes: MissCause[];
}

export interface CheckedBuild {
  build: number;
  /** Games read on this build. */
  count: number;
}

export interface Builds {
  /** null when no game gave a build. */
  newest: number | null;
  /** Newest first. */
  checked: CheckedBuild[];
}

export interface Accuracy {
  format: number;
  date: string;
  engine_version: string;
  trials: number;
  modes: Record<AccuracyMode, ModeAccuracy>;
  blocking_cards: BlockingCard[];
  misses: Misses;
  builds: Builds;
}

/** Card names for the blocking cards, from the card data (D-091). */
export interface CardName {
  en: string;
  es: string;
  golden: boolean;
}
export type CardNames = Readonly<Record<string, CardName>>;

const CARD_ID = /^[A-Za-z0-9_]{1,64}$/;
const DATE = /^\d{4}-\d{2}-\d{2}$/;
const VERSION = /^\d+\.\d+\.\d+$/;
const BUCKETS = 5;
const MAX_BLOCKING = 10;
const MAX_BUILDS = 20;
const MAX_BUILD = 1_000_000_000;
const BRIER_MAX = 2;
/** A share written as a float may differ from part / whole in its last digits. */
const SHARE_TOLERANCE = 1e-9;

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

function between(value: unknown, max: number, where: string): number {
  if (typeof value !== "number" || !Number.isFinite(value) || value < 0 || value > max) throw new Error(`${where}: out of range`);
  return value;
}

const maybe = (value: unknown, max: number, where: string): number | null => (value === null ? null : between(value, max, where));

function bucket(value: unknown, where: string): Bucket {
  const raw = object(value, where);
  keys(raw, ["from", "to", "predictions", "mean_said", "happened"], where);
  const b: Bucket = {
    from: between(raw.from, 1, `${where}.from`),
    to: between(raw.to, 1, `${where}.to`),
    predictions: count(raw.predictions, `${where}.predictions`),
    mean_said: maybe(raw.mean_said, 1, `${where}.mean_said`),
    happened: maybe(raw.happened, 1, `${where}.happened`),
  };
  if (b.to <= b.from) throw new Error(`${where}: to is not above from`);
  if ((b.predictions === 0) !== (b.mean_said === null) || (b.predictions === 0) !== (b.happened === null)) {
    throw new Error(`${where}: mean_said and happened are null exactly when there is no prediction`);
  }
  return b;
}

const MODE_KEYS = [
  "combats_with_result",
  "blocked",
  "blocked_share",
  "refused",
  "scored",
  "mean_p_real",
  "brier",
  "most_likely_right",
  "calibration",
] as const;

function modeAccuracy(value: unknown, where: string): ModeAccuracy {
  const raw = object(value, where);
  keys(raw, MODE_KEYS, where);
  if (!Array.isArray(raw.calibration) || raw.calibration.length !== BUCKETS) throw new Error(`${where}.calibration: not ${BUCKETS} buckets`);
  const m: ModeAccuracy = {
    combats_with_result: count(raw.combats_with_result, `${where}.combats_with_result`),
    blocked: count(raw.blocked, `${where}.blocked`),
    blocked_share: maybe(raw.blocked_share, 1, `${where}.blocked_share`),
    refused: count(raw.refused, `${where}.refused`),
    scored: count(raw.scored, `${where}.scored`),
    mean_p_real: maybe(raw.mean_p_real, 1, `${where}.mean_p_real`),
    brier: maybe(raw.brier, BRIER_MAX, `${where}.brier`),
    most_likely_right: count(raw.most_likely_right, `${where}.most_likely_right`),
    calibration: raw.calibration.map((b, i) => bucket(b, `${where}.calibration[${i}]`)),
  };
  if (m.blocked + m.refused + m.scored > m.combats_with_result) throw new Error(`${where}: blocked, refused and scored exceed combats_with_result`);
  if (m.most_likely_right > m.scored) throw new Error(`${where}: most_likely_right exceeds scored`);
  if ((m.combats_with_result === 0) !== (m.blocked_share === null)) throw new Error(`${where}: blocked_share is null exactly with no combat`);
  if (m.blocked_share !== null && Math.abs(m.blocked_share - m.blocked / m.combats_with_result) > SHARE_TOLERANCE) {
    throw new Error(`${where}: blocked_share is not blocked / combats_with_result`);
  }
  if ((m.scored === 0) !== (m.mean_p_real === null) || (m.scored === 0) !== (m.brier === null)) {
    throw new Error(`${where}: mean_p_real and brier are null exactly when nothing was scored`);
  }
  return m;
}

function blockingCard(value: unknown, where: string): BlockingCard {
  const raw = object(value, where);
  keys(raw, ["card_id", "combats"], where);
  if (typeof raw.card_id !== "string" || !CARD_ID.test(raw.card_id)) throw new Error(`${where}.card_id: not a card id`);
  return { card_id: raw.card_id, combats: count(raw.combats, `${where}.combats`) };
}

function missCause(value: unknown, where: string): MissCause {
  const raw = object(value, where);
  keys(raw, ["cause", "solo", "duos", "share"], where);
  if (typeof raw.cause !== "string" || !(CAUSE_IDS as readonly string[]).includes(raw.cause)) throw new Error(`${where}.cause: not a known cause`);
  return {
    cause: raw.cause as CauseId,
    solo: count(raw.solo, `${where}.solo`),
    duos: count(raw.duos, `${where}.duos`),
    share: between(raw.share, 1, `${where}.share`),
  };
}

function misses(value: unknown): Misses {
  const raw = object(value, "accuracy.misses");
  keys(raw, ["total", "causes"], "accuracy.misses");
  const total = count(raw.total, "accuracy.misses.total");
  if (!Array.isArray(raw.causes) || raw.causes.length > CAUSE_IDS.length) throw new Error("accuracy.misses.causes: not a list of causes");
  const causes = raw.causes.map((c, i) => missCause(c, `accuracy.misses.causes[${i}]`));
  const seen = new Set<string>();
  let sum = 0;
  let previous = Infinity;
  for (const c of causes) {
    const n = c.solo + c.duos;
    if (seen.has(c.cause)) throw new Error(`accuracy.misses.causes: ${c.cause} twice`);
    seen.add(c.cause);
    if (n === 0) throw new Error(`accuracy.misses.causes: ${c.cause} has no combat`);
    if (n > previous) throw new Error("accuracy.misses.causes: not the largest first");
    if (total === 0 || Math.abs(c.share - n / total) > SHARE_TOLERANCE) throw new Error(`accuracy.misses.causes: ${c.cause} share is not its combats / total`);
    previous = n;
    sum += n;
  }
  if (sum !== total) throw new Error("accuracy.misses.total: not the sum of the causes");
  return { total, causes };
}

function builds(value: unknown): Builds {
  const raw = object(value, "accuracy.builds");
  keys(raw, ["newest", "checked"], "accuracy.builds");
  if (!Array.isArray(raw.checked) || raw.checked.length > MAX_BUILDS) throw new Error(`accuracy.builds.checked: not a list of up to ${MAX_BUILDS}`);
  const checked = raw.checked.map((b, i): CheckedBuild => {
    const where = `accuracy.builds.checked[${i}]`;
    const row = object(b, where);
    keys(row, ["build", "count"], where);
    const build = count(row.build, `${where}.build`);
    if (build === 0 || build > MAX_BUILD) throw new Error(`${where}.build: not a build number`);
    const games = count(row.count, `${where}.count`);
    if (games === 0) throw new Error(`${where}.count: not a count above 0`);
    return { build, count: games };
  });
  checked.forEach((b, i) => {
    if (i > 0 && b.build >= checked[i - 1].build) throw new Error("accuracy.builds.checked: builds twice or not newest first");
  });
  const newest = raw.newest === null ? null : count(raw.newest, "accuracy.builds.newest");
  if (newest !== (checked[0]?.build ?? null)) throw new Error("accuracy.builds.newest: not the first build of the list");
  return { newest, checked };
}

/** Reads the export (untrusted until checked); throws, naming the key, on anything else. */
export function parseAccuracy(value: unknown): Accuracy {
  const raw = object(value, "accuracy");
  // The format first: another shape is not read at all.
  if (raw.format !== ACCURACY_FORMAT) throw new Error(`accuracy: format is not ${ACCURACY_FORMAT}`);
  keys(raw, ["format", "date", "engine_version", "trials", "modes", "blocking_cards", "misses", "builds"], "accuracy");
  if (typeof raw.date !== "string" || !isDay(raw.date)) throw new Error("accuracy.date: not a YYYY-MM-DD day");
  if (typeof raw.engine_version !== "string" || !VERSION.test(raw.engine_version)) throw new Error("accuracy.engine_version: not a version");
  const modes = object(raw.modes, "accuracy.modes");
  keys(modes, MODES, "accuracy.modes");
  if (!Array.isArray(raw.blocking_cards) || raw.blocking_cards.length > MAX_BLOCKING) throw new Error("accuracy.blocking_cards: not a list of up to 10");
  const trials = count(raw.trials, "accuracy.trials");
  if (trials === 0) throw new Error("accuracy.trials: not a count above 0");
  return {
    format: ACCURACY_FORMAT,
    date: raw.date,
    engine_version: raw.engine_version,
    trials,
    modes: { solo: modeAccuracy(modes.solo, "accuracy.modes.solo"), duos: modeAccuracy(modes.duos, "accuracy.modes.duos") },
    blocking_cards: raw.blocking_cards.map((c, i) => blockingCard(c, `accuracy.blocking_cards[${i}]`)),
    misses: misses(raw.misses),
    builds: builds(raw.builds),
  };
}

const NAMES_FORMAT = 1;
const NAME_MAX = 80;

function cardName(value: unknown, where: string): CardName {
  const raw = object(value, where);
  keys(raw, ["en", "es", "golden"], where);
  for (const lang of ["en", "es"] as const) {
    const name = raw[lang];
    if (typeof name !== "string" || name.trim() === "" || name.length > NAME_MAX) throw new Error(`${where}.${lang}: not a name`);
  }
  if (typeof raw.golden !== "boolean") throw new Error(`${where}.golden: not true or false`);
  return { en: raw.en as string, es: raw.es as string, golden: raw.golden };
}

/** Reads src/data/accuracy-cards.json (tools/accuracy_card_names.py). */
export function parseCardNames(value: unknown): CardNames {
  const raw = object(value, "card names");
  if (raw.format !== NAMES_FORMAT) throw new Error(`card names: format is not ${NAMES_FORMAT}`);
  keys(raw, ["format", "cards"], "card names");
  const cards = object(raw.cards, "card names.cards");
  const out: Record<string, CardName> = {};
  for (const [id, entry] of Object.entries(cards)) {
    if (!CARD_ID.test(id)) throw new Error(`card names: not a card id: ${JSON.stringify(id)}`);
    out[id] = cardName(entry, `card names.${id}`);
  }
  return out;
}

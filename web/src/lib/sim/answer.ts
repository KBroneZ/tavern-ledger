// The engine's answer (crates/sim-wasm odds.rs), read as untrusted input
// like any other data the page did not make itself: a field out of shape
// makes the whole answer unreadable, never a half-shown number.

export interface Spread {
  low: number;
  likely: number;
  high: number;
}

export interface Approximate {
  cardId: string;
  reason: string;
}

export type Answer =
  | {
      kind: "odds";
      engine: string;
      trials: number;
      won: number;
      tied: number;
      lost: number;
      stalled: number;
      damageKnown: boolean;
      dealt: Spread | null;
      taken: Spread | null;
      approximate: Approximate[];
      startOfCombat: boolean;
      /** The game build when the rules were not checked against it: the odds still come, with a warning. */
      newBuild: number | null;
    }
  | { kind: "not_modelled"; engine: string; notModelled: string[]; reasons: Approximate[]; approximate: Approximate[] }
  | { kind: "no_card_data"; engine: string; missing: string[] }
  | { kind: "bad_board" | "bad_input"; engine: string; error: string }
  | { kind: "unreadable" };

type Obj = Record<string, unknown>;

const CARD_ID = /^[A-Za-z0-9_]{1,64}$/;
const VERSION = /^\d{1,4}\.\d{1,4}\.\d{1,4}$/;
const MAX_LIST = 64;
const MAX_BUILD = 99_999_999;

class Bad extends Error {}

const obj = (v: unknown): Obj => {
  if (typeof v !== "object" || v === null || Array.isArray(v)) throw new Bad();
  return v as Obj;
};
const count = (v: unknown, max: number): number => {
  if (typeof v !== "number" || !Number.isInteger(v) || v < 0 || v > max) throw new Bad();
  return v;
};
const bool = (v: unknown): boolean => {
  if (typeof v !== "boolean") throw new Bad();
  return v;
};
const text = (v: unknown, max = 200): string => {
  if (typeof v !== "string" || v.length > max) throw new Bad();
  return v;
};
const ids = (v: unknown): string[] => {
  if (!Array.isArray(v) || v.length > MAX_LIST) throw new Bad();
  return v.map((x) => {
    if (typeof x !== "string" || !CARD_ID.test(x)) throw new Bad();
    return x;
  });
};
const approximate = (v: unknown): Approximate[] => {
  if (!Array.isArray(v) || v.length > MAX_LIST) throw new Bad();
  return v.map((x) => {
    const o = obj(x);
    const cardId = text(o.card_id, 64);
    if (!CARD_ID.test(cardId)) throw new Bad();
    return { cardId, reason: text(o.reason) };
  });
};
/** The build the engine warns about; an engine older than the warning sends none. */
export const newBuild = (v: unknown): number | null => {
  if (v === undefined || v === null) return null;
  if (typeof v !== "number" || !Number.isInteger(v) || v < 1 || v > MAX_BUILD) throw new Bad();
  return v;
};
const spread = (v: unknown): Spread | null => {
  if (v === null) return null;
  const o = obj(v);
  const s = { low: count(o.low, 1_000_000), likely: count(o.likely, 1_000_000), high: count(o.high, 1_000_000) };
  if (!(s.low <= s.likely && s.likely <= s.high)) throw new Bad();
  return s;
};

function read(o: Obj): Answer {
  const engine = text(o.engine, 20);
  if (!VERSION.test(engine)) throw new Bad();
  switch (o.status) {
    case "odds": {
      const trials = count(o.trials, 1_000_000);
      const won = count(o.won, trials);
      const tied = count(o.tied, trials);
      const lost = count(o.lost, trials);
      if (trials === 0 || won + tied + lost !== trials) throw new Bad();
      return {
        kind: "odds",
        engine,
        trials,
        won,
        tied,
        lost,
        stalled: count(o.stalled, trials),
        damageKnown: bool(o.damage_known),
        dealt: spread(o.dealt),
        taken: spread(o.taken),
        approximate: approximate(o.approximate),
        startOfCombat: bool(o.start_of_combat),
        newBuild: newBuild(o.new_build),
      };
    }
    case "not_modelled":
      return {
        kind: "not_modelled",
        engine,
        notModelled: ids(o.not_modelled),
        // Why each card blocks the odds; an engine older than this has none.
        reasons: o.reasons === undefined ? [] : approximate(o.reasons),
        approximate: approximate(o.approximate),
      };
    case "no_card_data":
      return { kind: "no_card_data", engine, missing: ids(o.missing) };
    case "bad_board":
    case "bad_input":
      return { kind: o.status, engine, error: text(o.error) };
    default:
      throw new Bad();
  }
}

export function readAnswer(json: string): Answer {
  try {
    return read(obj(JSON.parse(json)));
  } catch {
    return { kind: "unreadable" };
  }
}

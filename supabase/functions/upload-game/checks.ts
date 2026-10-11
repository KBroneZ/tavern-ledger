// Small checks shared by upload-game's validator (validate.ts) and its
// replay checks (replay.ts). Every failure throws `Invalid` with the field.

export const CARD_ID = /^[A-Za-z0-9_]{1,64}$/;
export const MAX_INT = 2 ** 31 - 1;
export const KEYWORDS = new Set([
  "taunt",
  "divine_shield",
  "reborn",
  "venomous",
  "windfury",
  "stealth",
  "deathrattle",
  "rally",
]);

export class Invalid extends Error {
  constructor(readonly field: string) {
    super(field);
  }
}

export type Obj = Record<string, unknown>;

export function isObject(v: unknown): v is Obj {
  return typeof v === "object" && v !== null && !Array.isArray(v);
}

/** The object's keys must all be in `allowed`; those in `required` must exist. */
export function shape(v: unknown, field: string, allowed: string[], required: string[] = []): Obj {
  if (!isObject(v)) throw new Invalid(field);
  for (const key of Object.keys(v)) {
    if (!allowed.includes(key)) throw new Invalid(field ? `${field}.${key}` : key);
  }
  for (const key of required) {
    if (!(key in v) || v[key] === undefined) throw new Invalid(field ? `${field}.${key}` : key);
  }
  return v;
}

export function int(v: unknown, field: string, min: number, max: number): number {
  if (typeof v !== "number" || !Number.isInteger(v) || v < min || v > max) {
    throw new Invalid(field);
  }
  return v;
}

export function optInt(v: unknown, field: string, min: number, max: number): number | null {
  return v === null || v === undefined ? null : int(v, field, min, max);
}

export function cardId(v: unknown, field: string): string | null {
  if (v === null || v === undefined) return null;
  if (typeof v !== "string" || !CARD_ID.test(v)) throw new Invalid(field);
  return v;
}

export function list(v: unknown, field: string, max: number): unknown[] {
  if (!Array.isArray(v) || v.length > max) throw new Invalid(field);
  return v;
}

export function bool(v: unknown, field: string): void {
  if (typeof v !== "boolean") throw new Invalid(field);
}

// Dark Gifts on a minion (parser revision 9, T-528, D-107): gift spell
// card ids, each once; the parser keeps at most 4.
export const MAX_GIFTS = 4;

/** Optional `gifts` and `gift_unknown` of a board or replay minion. */
export function gifts(m: Obj, field: string): void {
  if (m.gifts !== undefined) {
    const ids = list(m.gifts, `${field}.gifts`, MAX_GIFTS);
    if (
      ids.length === 0 ||
      ids.some((g) => typeof g !== "string" || !CARD_ID.test(g)) ||
      new Set(ids).size !== ids.length
    ) {
      throw new Invalid(`${field}.gifts`);
    }
  }
  if (m.gift_unknown !== undefined && m.gift_unknown !== true) {
    throw new Invalid(`${field}.gift_unknown`);
  }
}

export function keywords(v: unknown, field: string): void {
  const words = list(v, field, KEYWORDS.size);
  if (
    words.some((w) => typeof w !== "string" || !KEYWORDS.has(w)) ||
    new Set(words).size !== words.length
  ) {
    throw new Invalid(field);
  }
}

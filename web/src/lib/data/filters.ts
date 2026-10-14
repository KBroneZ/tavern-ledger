// The games list's filters (T-503): read from the address (URL as state, so a
// link opens the same list), checked, and turned into the view's conditions.
// A value that is not valid is left out and named, never guessed.
import { cardId, day } from "./rows.ts";

export type ModeFilter = "all" | "solo" | "duos";
export type PlaceFilter = "any" | "first" | "top" | "bottom" | "none";

export interface GameFilters {
  mode: ModeFilter;
  hero: string | null;
  from: string | null;
  to: string | null;
  place: PlaceFilter;
  page: number;
}

export const PAGE_SIZE = 25;
/** Far beyond any real list; keeps the offset small. */
export const MAX_PAGE = 400;

export const DEFAULT_FILTERS: GameFilters = { mode: "all", hero: null, from: null, to: null, place: "any", page: 1 };

const MODES: readonly ModeFilter[] = ["all", "solo", "duos"];
export const PLACE_FILTERS: readonly PlaceFilter[] = ["any", "first", "top", "bottom", "none"];

/** The filters in the address, and the names of the ones left out as not valid. */
export function parseFilters(search: string): { filters: GameFilters; invalid: string[] } {
  const q = new URLSearchParams(search);
  const invalid: string[] = [];
  const pick = <T>(name: string, read: (raw: string) => T | null, fallback: T): T => {
    const raw = q.get(name);
    if (raw === null || raw === "") return fallback;
    const value = read(raw);
    if (value === null) invalid.push(name);
    return value ?? fallback;
  };
  const filters: GameFilters = {
    mode: pick("mode", (r) => (MODES.includes(r as ModeFilter) ? (r as ModeFilter) : null), DEFAULT_FILTERS.mode),
    hero: pick("hero", cardId, null),
    from: pick("from", day, null),
    to: pick("to", day, null),
    place: pick("place", (r) => (PLACE_FILTERS.includes(r as PlaceFilter) ? (r as PlaceFilter) : null), DEFAULT_FILTERS.place),
    page: pick("page", (r) => (/^\d{1,3}$/.test(r) && Number(r) >= 1 && Number(r) <= MAX_PAGE ? Number(r) : null), 1),
  };
  if (filters.from && filters.to && filters.from > filters.to) {
    invalid.push("from", "to");
    filters.from = null;
    filters.to = null;
  }
  return { filters, invalid };
}

/** The address's query for these filters (and the profile id on a public list); defaults are left out. */
export function filtersQuery(f: GameFilters, profileId: string | null = null): string {
  const q = new URLSearchParams();
  if (profileId) q.set("id", profileId);
  if (f.mode !== "all") q.set("mode", f.mode);
  if (f.hero) q.set("hero", f.hero);
  if (f.from) q.set("from", f.from);
  if (f.to) q.set("to", f.to);
  if (f.place !== "any") q.set("place", f.place);
  if (f.page !== 1) q.set("page", String(f.page));
  const text = q.toString();
  return text ? `?${text}` : "";
}

export type Condition =
  | { column: string; op: "eq"; value: string | number | boolean }
  | { column: string; op: "gte" | "lte"; value: string }
  | { column: string; op: "is"; value: null };

/** The game_summaries conditions besides the user. */
export function conditions(f: GameFilters): Condition[] {
  const list: Condition[] = [];
  if (f.mode !== "all") list.push({ column: "mode", op: "eq", value: f.mode });
  if (f.hero) list.push({ column: "hero", op: "eq", value: f.hero });
  if (f.from) list.push({ column: "played_on", op: "gte", value: f.from });
  if (f.to) list.push({ column: "played_on", op: "lte", value: f.to });
  if (f.place === "first") list.push({ column: "counted_place", op: "eq", value: 1 });
  if (f.place === "top") list.push({ column: "top_half", op: "eq", value: true });
  if (f.place === "bottom") list.push({ column: "top_half", op: "eq", value: false });
  if (f.place === "none") list.push({ column: "counted_place", op: "is", value: null });
  return list;
}

/** The rows of one page, as the API's inclusive range. */
export function pageRange(page: number): { from: number; to: number } {
  return { from: (page - 1) * PAGE_SIZE, to: page * PAGE_SIZE - 1 };
}

export function pageCount(total: number): number {
  return Math.max(1, Math.ceil(total / PAGE_SIZE));
}

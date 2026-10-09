// Display name rules, the public profile address and how a game row is shown.
// Rows come from the server and are treated as untrusted: anything outside
// the schema's ranges is shown as "unknown", never as it is.

const UUID = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/;
// Same rule as the profiles.display_name check (letters, digits, space, _ . -),
// so a BattleTag (Name#1234) cannot be entered. The database has the last word.
const NAME = /^[\p{L}\p{N} _.-]+$/u;
const MAX_NAME = 32;

export type NameCheck = { ok: true; value: string | null } | { ok: false; message: string };

export function checkDisplayName(raw: string): NameCheck {
  const value = raw.trim();
  if (value === "") return { ok: true, value: null };
  if ([...value].length > MAX_NAME) {
    return { ok: false, message: `Up to ${MAX_NAME} characters.` };
  }
  if (!NAME.test(value)) {
    return { ok: false, message: "Use letters, digits, spaces, _ . or - only." };
  }
  return { ok: true, value };
}

/** The profile id from a page's query string (?id=<uuid>), or null. */
export function parseProfileId(search: string): string | null {
  const id = new URLSearchParams(search).get("id")?.toLowerCase() ?? "";
  return UUID.test(id) ? id : null;
}

export function publicProfilePath(userId: string): string {
  return `/profile/?id=${encodeURIComponent(userId)}`;
}

export interface GameRow {
  game_type?: unknown;
  status?: unknown;
  hero_card_id?: unknown;
  final_place?: unknown;
  played_on?: unknown;
}

export interface GameView {
  date: string;
  mode: string;
  hero: string;
  place: string;
}

const MODES: Record<string, { name: string; places: number }> = {
  GT_BATTLEGROUNDS: { name: "Solo", places: 8 },
  GT_BATTLEGROUNDS_DUO: { name: "Duos", places: 4 },
};

function ordinal(n: number): string {
  const suffix = n === 1 ? "st" : n === 2 ? "nd" : n === 3 ? "rd" : "th";
  return `${n}${suffix}`;
}

function placeText(row: GameRow, places: number | undefined): string {
  if (row.status === "incomplete") return "unfinished";
  if (row.status === "unsupported") return "unreadable";
  const p = row.final_place;
  if (row.status !== "ok" || places === undefined) return "unknown";
  if (typeof p !== "number" || !Number.isInteger(p) || p < 1 || p > places) return "unknown";
  return ordinal(p);
}

export function gameView(row: GameRow): GameView {
  const mode = typeof row.game_type === "string" ? MODES[row.game_type] : undefined;
  const hero = typeof row.hero_card_id === "string" && /^[A-Za-z0-9_]{1,64}$/.test(row.hero_card_id)
    ? row.hero_card_id
    : "unknown";
  const date = typeof row.played_on === "string" && /^\d{4}-\d{2}-\d{2}$/.test(row.played_on)
    ? row.played_on
    : "unknown";
  return { date, mode: mode?.name ?? "unknown", hero, place: placeText(row, mode?.places) };
}

/** A PostgREST error as the profile form shows it. */
export function profileErrorMessage(e: { code?: string; message?: string }): string {
  if (e.code === "23514") return "Use 1-32 letters, digits, spaces, _ . or - only.";
  if (e.code === "PGRST301" || e.code === "42501") return "Your session ended. Sign in again.";
  return "Could not save. Try again in a moment.";
}

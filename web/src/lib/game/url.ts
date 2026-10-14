import { href } from "../i18n.ts";
// The game viewer's address (T-504, T-586): /game/?u=<user id>&id=<public id>.
// The public id is a random uuid (games.public_id) that says nothing about
// when the game was played; the games list (T-503) links each row here with
// gamePath(). The earlier address (?u&s=<session>&g=<index>) carried the
// game's start time to the second and is no longer read: it is "not found".

const UUID = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/;

export interface GameRef {
  userId: string;
  publicId: string;
}

/** The game named by a page's query string, or null when any part is not a valid id. */
export function parseGameRef(search: string): GameRef | null {
  const query = new URLSearchParams(search);
  const userId = query.get("u")?.toLowerCase() ?? "";
  const publicId = query.get("id")?.toLowerCase() ?? "";
  return UUID.test(userId) && UUID.test(publicId) ? { userId, publicId } : null;
}

/** Whether a value is a game's public id. */
export const isPublicId = (value: unknown): value is string => typeof value === "string" && UUID.test(value);

/** The viewer's address for one game. */
export function gamePath(userId: string, publicId: string): string {
  const query = new URLSearchParams({ u: userId, id: publicId });
  return href(`/game/?${query.toString()}`);
}

/**
 * Where the game's report is in the `games` bucket: games.storage_path of a
 * game whose file has its opaque name (the daily sweep renames the older files).
 */
export function storagePath(ref: GameRef): string {
  return `${ref.userId}/${ref.publicId}.json.gz`;
}

/** The game's date from its session name (YYYY-MM-DD, the client's local start). */
export function playedOn(session: string): string | null {
  const m = /^Hearthstone_(\d{4})_(\d{2})_(\d{2})_/.exec(session);
  if (!m) return null;
  const iso = `${m[1]}-${m[2]}-${m[3]}`;
  const d = new Date(`${iso}T00:00:00Z`);
  return Number.isNaN(d.getTime()) || d.toISOString().slice(0, 10) !== iso ? null : iso;
}

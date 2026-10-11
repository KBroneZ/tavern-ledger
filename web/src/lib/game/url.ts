import { href } from "../i18n.ts";
// The game viewer's address (T-504): /game/?u=<user id>&s=<session>&g=<index>.
// The three values are the game's key in public.games, so the page needs no
// other id; the games list (T-503) links each row here with gamePath().

const UUID = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/;
// The same rule as the games.session check in the database.
const SESSION = /^Hearthstone_\d{4}(?:_\d{2}){5}$/;
const INDEX = /^[1-9]\d{0,3}$/;
const MAX_INDEX = 1000;

export interface GameRef {
  userId: string;
  session: string;
  index: number;
}

/** The game named by a page's query string, or null when any part is not a valid key. */
export function parseGameRef(search: string): GameRef | null {
  const query = new URLSearchParams(search);
  const userId = query.get("u")?.toLowerCase() ?? "";
  const session = query.get("s") ?? "";
  const index = query.get("g") ?? "";
  if (!UUID.test(userId) || !SESSION.test(session) || !INDEX.test(index)) return null;
  const n = Number(index);
  return n <= MAX_INDEX ? { userId, session, index: n } : null;
}

/** The viewer's address for one game. */
export function gamePath(userId: string, session: string, index: number): string {
  const query = new URLSearchParams({ u: userId, s: session, g: String(index) });
  return href(`/game/?${query.toString()}`);
}

/** Where the game's report is in the `games` bucket (games.storage_path). */
export function storagePath(ref: GameRef): string {
  return `${ref.userId}/${ref.session}-${ref.index}.json.gz`;
}

/** The game's date from its session name (YYYY-MM-DD, the client's local start). */
export function playedOn(session: string): string | null {
  const m = /^Hearthstone_(\d{4})_(\d{2})_(\d{2})_/.exec(session);
  if (!m) return null;
  const iso = `${m[1]}-${m[2]}-${m[3]}`;
  const d = new Date(`${iso}T00:00:00Z`);
  return Number.isNaN(d.getTime()) || d.toISOString().slice(0, 10) !== iso ? null : iso;
}

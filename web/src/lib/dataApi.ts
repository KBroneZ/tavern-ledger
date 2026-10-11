// The data pages' reads (T-503) from the data API's views (T-502). Row-level
// security decides what comes back: the owner's own games, or a public
// profile's. Any failure throws, so the page shows its error state with a
// retry; nothing is shown from a half-read answer.
import type { SupabaseClient } from "@supabase/supabase-js";
import type { Mode } from "./data/format.ts";
import { conditions, pageRange, type GameFilters } from "./data/filters.ts";
import {
  asRows,
  count,
  DataError,
  GAME_COLUMNS,
  gameRow,
  heroRow,
  overview,
  place,
  recordRow,
  type GameRow,
  type HeroRow,
  type Overview,
  type RecordRow,
} from "./data/rows.ts";

/** The charts count at most this many of the latest games per mode. */
export const CHART_GAMES = 1000;
/** Last games on the dashboard, per mode. */
export const LAST_GAMES = 10;

const MODES: Mode[] = ["solo", "duos"];

interface Answer {
  data: unknown;
  error: unknown;
  count?: number | null;
}

function rowsOf(answer: Answer): Record<string, unknown>[] {
  if (answer.error) throw new DataError("the request failed");
  return asRows(answer.data);
}

export type ProfileAccess =
  | { kind: "public"; name: string | null; own: boolean }
  | { kind: "own-private"; name: string | null }
  | { kind: "private" };

/**
 * Whether this viewer may see the profile. A private profile and a missing
 * one give the same answer to anyone but the owner, so the page cannot be
 * used to find out which accounts exist.
 */
export async function profileAccess(client: SupabaseClient, userId: string, viewerId: string | null): Promise<ProfileAccess> {
  const answer = await client.from("profiles").select("display_name,is_public").eq("user_id", userId).limit(1);
  const rows = rowsOf(answer);
  if (rows.length === 0) return { kind: "private" };
  const name = typeof rows[0].display_name === "string" ? rows[0].display_name : null;
  const own = viewerId === userId;
  if (rows[0].is_public === true) return { kind: "public", name, own };
  // Row-level security only returns a private row to its owner.
  return own ? { kind: "own-private", name } : { kind: "private" };
}

export interface ModeData {
  overview: Overview | null;
  /** Places of the latest games (up to CHART_GAMES), newest first; null when the game has none. */
  places: (number | null)[];
  /** Games in this mode, for "the charts count the last N". */
  total: number;
  lastGames: GameRow[];
}

export interface Dashboard {
  modes: Record<Mode, ModeData>;
  heroes: HeroRow[];
  records: RecordRow[];
}

async function modeData(client: SupabaseClient, userId: string, m: Mode): Promise<Omit<ModeData, "overview">> {
  const [places, last] = await Promise.all([
    client
      .from("game_summaries")
      .select("counted_place", { count: "exact" })
      .eq("user_id", userId)
      .eq("mode", m)
      .order("played_on", { ascending: false })
      .order("session", { ascending: false })
      .order("game_index", { ascending: false })
      .limit(CHART_GAMES),
    client
      .from("game_summaries")
      .select(GAME_COLUMNS)
      .eq("user_id", userId)
      .eq("mode", m)
      .order("played_on", { ascending: false })
      .order("session", { ascending: false })
      .order("game_index", { ascending: false })
      .limit(LAST_GAMES),
  ]);
  const placeRows = rowsOf(places);
  return {
    places: placeRows.map((r) => place(r.counted_place, m)),
    total: totalOf(places.count, placeRows.length),
    lastGames: rowsOf(last).map(gameRow),
  };
}

/** The exact count PostgREST sent, checked; without one, the rows in hand. */
function totalOf(raw: number | null | undefined, fallback: number): number {
  return raw === null || raw === undefined ? fallback : count(raw, "count");
}

/** Everything the dashboard shows for one player, both modes. */
export async function loadDashboard(client: SupabaseClient, userId: string): Promise<Dashboard> {
  const [ov, heroes, records, solo, duos] = await Promise.all([
    client.from("user_overview").select("*").eq("user_id", userId),
    client.from("user_heroes").select("mode,hero,games,placed_games,avg_place,top_half,wins,best_place").eq("user_id", userId),
    client.from("user_records").select("mode,record,value,played_on,games,inferred").eq("user_id", userId),
    modeData(client, userId, "solo"),
    modeData(client, userId, "duos"),
  ]);
  const overviews = rowsOf(ov).map(overview);
  const of = (m: Mode) => overviews.find((o) => o.mode === m) ?? null;
  return {
    modes: { solo: { ...solo, overview: of("solo") }, duos: { ...duos, overview: of("duos") } },
    heroes: rowsOf(heroes).map(heroRow),
    records: rowsOf(records)
      .map(recordRow)
      .filter((r): r is RecordRow => r !== null),
  };
}

/** The heroes a player has games with (for the hero filter), most games first. */
export async function loadHeroes(client: SupabaseClient, userId: string): Promise<string[]> {
  const rows = rowsOf(await client.from("user_heroes").select("mode,hero,games,placed_games,avg_place,top_half,wins,best_place").eq("user_id", userId)).map(heroRow);
  const games = new Map<string, number>();
  for (const r of rows) if (r.hero && r.games > 0) games.set(r.hero, (games.get(r.hero) ?? 0) + r.games);
  return [...games.entries()].toSorted((a, b) => b[1] - a[1] || a[0].localeCompare(b[0])).map(([hero]) => hero);
}

interface Filterable<Q> {
  filter(column: string, operator: string, value: unknown): Q;
}

/** The filters' conditions on a games query, as PostgREST operators. */
function filtered<Q extends Filterable<Q>>(query: Q, filters: GameFilters): Q {
  return conditions(filters).reduce((q, c) => q.filter(c.column, c.op, c.value === null ? "null" : String(c.value)), query);
}

/** One page of the games list with the number of games that match. */
export async function loadGames(client: SupabaseClient, userId: string, filters: GameFilters): Promise<{ rows: GameRow[]; total: number }> {
  const { from, to } = pageRange(filters.page);
  const answer = await filtered(client.from("game_summaries").select(GAME_COLUMNS, { count: "exact" }).eq("user_id", userId), filters)
    .order("played_on", { ascending: false })
    .order("session", { ascending: false })
    .order("game_index", { ascending: false })
    .range(from, to);
  if (answer.error && answer.status === 416) {
    // Past the last page: no rows, but the page still says how many games match.
    const head = await filtered(client.from("game_summaries").select("session", { count: "exact", head: true }).eq("user_id", userId), filters);
    if (head.error) throw new DataError("the request failed");
    return { rows: [], total: totalOf(head.count, 0) };
  }
  const rows = rowsOf(answer).map(gameRow);
  return { rows, total: totalOf(answer.count, rows.length) };
}

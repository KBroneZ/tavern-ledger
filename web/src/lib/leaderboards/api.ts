// The leaderboards' reads (T-506): public.leaderboard() for each mode and
// period, and when the boards were last rebuilt. The server picks who is on
// a board (public profiles with an allowed name and enough games); this only
// checks the rows. Any failure throws, so the page shows its error state.
import type { SupabaseClient } from "@supabase/supabase-js";
import type { Mode } from "../data/format.ts";
import { asRows, DataError } from "../data/rows.ts";
import { boardRow, groupBoards, MAX_ROWS, PERIODS, type BoardKey, type BoardRow, type Period } from "./boards.ts";

const MODES: readonly Mode[] = ["solo", "duos"];

export type Boards = Map<BoardKey, BoardRow[]>;

export interface Leaderboards {
  /** The server's timestamp of the last rebuild; null before the first. */
  refreshedAt: string | null;
  boards: Record<Mode, Record<Period, Boards>>;
}

async function one(client: SupabaseClient, m: Mode, p: Period): Promise<Boards> {
  const answer = await client.rpc("leaderboard", { p_mode: m, p_period: p });
  if (answer.error) throw new DataError("the request failed");
  const rows = asRows(answer.data).map((r) => boardRow(r, m));
  const boards = groupBoards(rows);
  for (const list of boards.values()) if (list.length > MAX_ROWS) throw new DataError("too many rows");
  return boards;
}

/** Every board of both modes and the three periods, read at once. */
export async function loadLeaderboards(client: SupabaseClient): Promise<Leaderboards> {
  const pairs = MODES.flatMap((m) => PERIODS.map((p) => [m, p] as const));
  const [refreshed, ...lists] = await Promise.all([
    client.rpc("leaderboard_refreshed_at"),
    ...pairs.map(([m, p]) => one(client, m, p)),
  ]);
  if (refreshed.error) throw new DataError("the request failed");
  // Null only before the first rebuild; anything else must be a time.
  if (refreshed.data !== null && (typeof refreshed.data !== "string" || Number.isNaN(Date.parse(refreshed.data)))) {
    throw new DataError("bad time");
  }
  const boards = { solo: {}, duos: {} } as Record<Mode, Record<Period, Boards>>;
  pairs.forEach(([m, p], i) => {
    boards[m][p] = lists[i];
  });
  return { refreshedAt: refreshed.data, boards };
}

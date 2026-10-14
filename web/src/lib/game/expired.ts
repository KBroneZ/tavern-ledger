// A game whose full file has expired (T-544, D-109, D-113): the daily sweep
// deletes a free account's game file 28 days after the game was played and
// marks the game row (games.file_expired_at); the summary row stays. When
// Storage refuses the file, the viewer asks for that row: an expired game
// shows its summary and a plain line instead of "not available". Row-level
// security answers as for the file, so a private game still gets no row.
import { gameCells, gameColumns } from "../data/dashboard.ts";
import { ordinal } from "../data/format.ts";
import { GAME_FILE_COLUMNS, gameRow, type GameRow } from "../data/rows.ts";
import { heroLabel } from "../heroes/names.ts";
import { h, type VNode } from "../view/node.ts";
import { topTable } from "../view/pieces.ts";
import { g } from "./strings.ts";
import { type GameRef, storagePath } from "./url.ts";

/** The columns read for an expired game: the games list's, plus the file's name and the mark. */
export const EXPIRED_COLUMNS = `${GAME_FILE_COLUMNS},file_expired_at`;

/** The game's row from game_summaries (EXPIRED_COLUMNS), or a stand-in in tests. */
export type ReadSummary = (ref: GameRef) => Promise<{ data: unknown; error: unknown }>;

export type ExpiredResult =
  | { kind: "expired"; row: GameRow }
  /**
   * No row, or a row whose file has not expired: the page says "not available".
   * `path`: where the row says the file is, when that is not the opaque name (the
   * owner's own game the sweep has not renamed yet), so the viewer can try it.
   */
  | { kind: "none"; path: string | null }
  /** The request failed: worth a retry. */
  | { kind: "error" };

/** Whether the game named by ref has a summary row whose file has expired. */
export async function loadExpired(read: ReadSummary, ref: GameRef): Promise<ExpiredResult> {
  let answer: Awaited<ReturnType<ReadSummary>>;
  try {
    answer = await read(ref);
  } catch {
    return { kind: "error" };
  }
  if (answer.error || !Array.isArray(answer.data)) return { kind: "error" };
  const value: unknown = answer.data[0];
  if (typeof value !== "object" || value === null) return { kind: "none", path: null };
  const row = gameRow(value);
  // Only the game the address names.
  if (row.publicId !== ref.publicId) return { kind: "none", path: null };
  const expired = (value as Record<string, unknown>).file_expired_at;
  if (typeof expired !== "string" || expired === "") {
    return { kind: "none", path: row.storagePath !== storagePath(ref) ? row.storagePath : null };
  }
  return { kind: "expired", row };
}

/** The page's heading for an expired game: the hero, and the place when known. */
export function expiredHeading(row: GameRow): string {
  const hero = heroLabel(row.hero, g("hero.unknown"), g("hero.unknown")).name;
  return row.place === null ? g("page.heading.noPlace", { hero }) : g("page.heading", { hero, place: ordinal(row.place) });
}

/** The line saying why, then the summary as one row of the games list's table. */
export function renderExpired(row: GameRow): VNode[] {
  return [
    h("p", { class: "state state-unknown gv-banner" }, g("state.expired")),
    topTable({ caption: g("state.expired.caption"), columns: gameColumns(true), rows: [gameCells(row, true)], top: 1 }),
  ];
}

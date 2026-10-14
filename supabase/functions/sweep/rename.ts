// Sweep step: renames the game files that still have the old name (T-586).
//
// A file used to be stored as <user>/<session>-<index>.json.gz, a name that
// carries the game's start time to the second. New uploads get
// <user>/<public_id>.json.gz. This step moves the old ones, a few hundred per
// run, until none is left (a run that finds none costs one database call).
//
// Safe to repeat and to stop anywhere:
//   1. game_files_to_move() names the files whose row still has the old name;
//   2. the Storage API renames the object (one atomic call);
//   3. game_file_moved() writes the new name in the row.
// Between 2 and 3 the file is under the new name while the row has the old
// one: sweep_orphan_files() knows a file under its game's new name is not an
// orphan, and the next run finds the move done (the source is gone, the
// target is there) and only writes the row. A file that cannot be moved or
// found is counted and tried again next run; nothing is ever deleted here.
//
// A database from before game_files_to_move() answers 404: nothing to do yet
// (functions and migrations deploy in no fixed order).

import type { Env, Fetch } from "./handler.ts";

/** Files listed per call, calls per run, and the time after which no rename is started. */
const RENAME_LIMIT = 200;
const MAX_ROUNDS = 25;
const RENAME_BUDGET_MS = 60_000;
const TIMEOUT_MS = 10_000;

const UUID = "[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}";
const OLD_NAME = new RegExp(
  `^(${UUID})/Hearthstone_\\d{4}(?:_\\d{2}){5}-[1-9][0-9]{0,3}[.]json[.]gz$`,
);
const NEW_NAME = new RegExp(`^(${UUID})/${UUID}[.]json[.]gz$`);

export interface Renames {
  listed: number;
  /** The file is under the new name and the row says so. */
  renamed: number;
  /** The file could not be moved or found, or the row could not be written. */
  failed: number;
  /** Not started: out of time. */
  left: number;
}

interface Move {
  old_path: string;
  new_path: string;
}

/** A row of game_files_to_move(): an old name and a new one in the same user's folder. */
function isMove(v: unknown): v is Move {
  if (typeof v !== "object" || v === null) return false;
  const m = v as Record<string, unknown>;
  if (typeof m.old_path !== "string" || typeof m.new_path !== "string") return false;
  const before = OLD_NAME.exec(m.old_path);
  const after = NEW_NAME.exec(m.new_path);
  return before !== null && after !== null && before[1] === after[1];
}

async function send(env: Env, fetchFn: Fetch, path: string, init: RequestInit): Promise<Response | null> {
  try {
    const res = await fetchFn(`${env.url}${path}`, { ...init, signal: AbortSignal.timeout(TIMEOUT_MS) });
    if (!res.ok) await res.body?.cancel();
    return res;
  } catch (_e) {
    return null;
  }
}

function headers(env: Env): Record<string, string> {
  return { apikey: env.serviceKey, "Content-Type": "application/json" };
}

/** True once the file is under its new name: moved now, or by an earlier run. */
async function moved(env: Env, fetchFn: Fetch, move: Move): Promise<boolean> {
  const res = await send(env, fetchFn, "/storage/v1/object/move", {
    method: "POST",
    headers: headers(env),
    body: JSON.stringify({ bucketId: "games", sourceKey: move.old_path, destinationKey: move.new_path }),
  });
  if (res?.ok) return true;
  // Not moved now: perhaps moved before (a run that stopped before the row).
  const there = await send(env, fetchFn, `/storage/v1/object/authenticated/games/${move.new_path}`, {
    method: "HEAD",
    headers: { apikey: env.serviceKey },
  });
  return there?.ok === true;
}

async function rowUpdated(env: Env, fetchFn: Fetch, move: Move): Promise<boolean> {
  const res = await send(env, fetchFn, "/rest/v1/rpc/game_file_moved", {
    method: "POST",
    headers: headers(env),
    body: JSON.stringify({ p_old: move.old_path, p_new: move.new_path }),
  });
  if (!res?.ok) return false;
  // false: the row is gone or already has the new name; either way nothing is left to do.
  return typeof (await res.json()) === "boolean";
}

/** One call of game_files_to_move(): the moves, null when the database has none to give (404 or a failure, logged), or "shape" when the list is not as asked. */
async function listMoves(env: Env, fetchFn: Fetch): Promise<Move[] | null | "shape"> {
  const list = await send(env, fetchFn, "/rest/v1/rpc/game_files_to_move", {
    method: "POST",
    headers: headers(env),
    body: JSON.stringify({ p_limit: RENAME_LIMIT }),
  });
  if (list === null || (!list.ok && list.status !== 404)) {
    console.error("sweep: file renames not run (list failed)");
    return null;
  }
  if (!list.ok) return null;
  let moves: unknown;
  try {
    moves = await list.json();
  } catch (_e) {
    moves = null;
  }
  if (!Array.isArray(moves) || !moves.every(isMove)) {
    console.error("sweep: file renames not run (list out of shape)");
    return "shape";
  }
  return moves;
}

/**
 * Renames old-named files, RENAME_LIMIT at a time, until none is left, a round
 * renames nothing (the same failures would come back), or time is up. Null when
 * there is nothing to report: the database does not have the list yet, or the
 * first list failed (logged). Never throws.
 */
export async function renameFiles(env: Env, fetchFn: Fetch, runDeadline: number): Promise<Renames | null> {
  const now = env.now ?? Date.now;
  const deadline = Math.min(now() + RENAME_BUDGET_MS, runDeadline);
  const counts: Renames = { listed: 0, renamed: 0, failed: 0, left: 0 };
  for (let round = 0; round < MAX_ROUNDS; round++) {
    const moves = await listMoves(env, fetchFn);
    if (moves === null || moves === "shape") {
      if (round === 0) return null;
      break;
    }
    if (moves.length === 0) break;
    counts.listed += moves.length;
    let renamed = 0;
    for (const move of moves) {
      if (now() >= deadline) {
        counts.left += 1;
      } else if (await moved(env, fetchFn, move) && await rowUpdated(env, fetchFn, move)) {
        renamed += 1;
      } else {
        counts.failed += 1;
      }
    }
    counts.renamed += renamed;
    // A short list was the last; no progress means the same failures would be listed again.
    if (moves.length < RENAME_LIMIT || renamed === 0 || now() >= deadline) break;
  }
  // Counts only: no user id, session or file name in the logs.
  if (counts.failed + counts.left > 0) {
    console.error(
      `sweep: file renames ${counts.renamed} of ${counts.listed} (${counts.failed} failed, ${counts.left} left)`,
    );
  }
  return counts;
}

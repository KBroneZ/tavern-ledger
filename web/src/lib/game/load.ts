// Fetches one game's stored report (T-504) from the `games` bucket and reads
// it. Storage answers the owner, or everyone when the profile is public
// (policy games_bucket_read_own_or_public); a private game and a missing one
// get the same answer, and the page says the same for both. The file is
// gzipped JSON, capped at the server's own limits before and after inflating,
// and inflated in the browser (DecompressionStream, no dependency).
import { type GameRecord, readRecord } from "./report.ts";
import { type GameRef, storagePath } from "./url.ts";

/**
 * The bucket's file size limit (64 KB; the largest real game is about 3.3 KB
 * gzipped). Storage already enforces it, and the download is in memory before
 * this check, so here it only refuses a file that should not exist.
 */
export const MAX_GZIPPED = 64 * 1024;
/**
 * upload-game's own cap on an inflated record (the largest real game is about
 * 200 KB with its combat replays, D-099).
 */
export const MAX_INFLATED = 1024 * 1024;

export type LoadResult =
  /** `report`: the stored report as read, for the engine (T-549). */
  | { kind: "ok"; record: GameRecord; report: unknown }
  /** Private or missing: the same answer for both. */
  | { kind: "unavailable" }
  /** The file is there but is not a game record this site can read. */
  | { kind: "unreadable" }
  /** Network or server trouble: worth a retry. */
  | { kind: "error" };

/** client.storage.from("games").download, or a stand-in in tests. */
export type Download = (path: string) => Promise<{ data: Blob | null; error: unknown }>;

/** 4xx from Storage: no such object for this caller (or a token it refused). */
function refused(error: unknown): boolean {
  const status = (error as { status?: unknown } | null)?.status;
  return typeof status === "number" && status >= 400 && status < 500 && status !== 408 && status !== 429;
}

class TooLarge extends Error {}

async function readCapped(stream: ReadableStream<Uint8Array>, max: number): Promise<Uint8Array> {
  const reader = stream.getReader();
  const chunks: Uint8Array[] = [];
  let size = 0;
  try {
    for (;;) {
      const { done, value } = await reader.read();
      if (done) break;
      size += value.length;
      if (size > max) throw new TooLarge();
      chunks.push(value);
    }
  } finally {
    await reader.cancel().catch(() => {});
  }
  const out = new Uint8Array(size);
  let at = 0;
  for (const chunk of chunks) {
    out.set(chunk, at);
    at += chunk.length;
  }
  return out;
}

const GZIP_MAGIC = [0x1f, 0x8b];

/**
 * The JSON text of a stored file. Gzip as stored; plain JSON is accepted too,
 * in case a proxy inflated it on the way (Content-Encoding).
 */
export async function inflate(blob: Blob): Promise<string> {
  if (blob.size > MAX_GZIPPED) throw new TooLarge();
  const head = new Uint8Array(await blob.slice(0, 2).arrayBuffer());
  const gzipped = head[0] === GZIP_MAGIC[0] && head[1] === GZIP_MAGIC[1];
  const stream = gzipped ? blob.stream().pipeThrough(new DecompressionStream("gzip")) : blob.stream();
  const bytes = await readCapped(stream, MAX_INFLATED);
  return new TextDecoder("utf-8", { fatal: true }).decode(bytes);
}

export async function loadGame(download: Download, ref: GameRef): Promise<LoadResult> {
  let answer: Awaited<ReturnType<Download>>;
  try {
    answer = await download(storagePath(ref));
  } catch {
    return { kind: "error" };
  }
  if (answer.error || !answer.data) return refused(answer.error) ? { kind: "unavailable" } : { kind: "error" };
  let value: unknown;
  try {
    value = JSON.parse(await inflate(answer.data));
  } catch {
    // Too large, not gzip, not UTF-8 or not JSON: the file is not one we can read.
    return { kind: "unreadable" };
  }
  const result = readRecord(value, ref.session, ref.index);
  return result.ok ? { kind: "ok", record: result.record, report: (value as { report: unknown }).report } : { kind: "unreadable" };
}

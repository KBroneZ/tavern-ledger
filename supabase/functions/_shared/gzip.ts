// Capped reads and gzip inflation for upload-game and sweep. DecompressionStream
// hands back all that one input chunk inflates to at once, so a small body
// that inflates a thousand times over would be held whole before any cap
// could see it. inflateCapped() feeds it the gzip in small slices instead:
// each inflated chunk stays near a megabyte at most, and the cap stops the
// inflation as soon as it is passed.

type Bytes = Uint8Array<ArrayBuffer>;

export class TooLarge extends Error {
  constructor() {
    super("too large");
  }
}

/** Bytes of gzip fed to the inflater at a time (deflate inflates ~1032:1 at most). */
export const SLICE = 1024;

/** Reads a stream into memory, or throws TooLarge past `max` bytes. */
export async function readCapped(stream: ReadableStream<Uint8Array>, max: number): Promise<Bytes> {
  const reader = stream.getReader();
  const chunks: Uint8Array[] = [];
  let size = 0;
  try {
    while (true) {
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
  for (const c of chunks) {
    out.set(c, at);
    at += c.length;
  }
  return out;
}

/** `bytes` as a stream of `size`-byte slices, made only as they are read. */
export function slices(bytes: Uint8Array, size = SLICE): ReadableStream<BufferSource> {
  let at = 0;
  return new ReadableStream<BufferSource>({
    pull(controller) {
      if (at >= bytes.length) {
        controller.close();
        return;
      }
      controller.enqueue(bytes.slice(at, at + size));
      at += size;
    },
  });
}

/**
 * The bytes a gzip body inflates to, or TooLarge as soon as they pass `max`.
 * Anything that is not complete, valid gzip rejects with another error.
 */
export async function inflateCapped(gz: Uint8Array, max: number): Promise<Bytes> {
  return await readCapped(slices(gz).pipeThrough(new DecompressionStream("gzip")), max);
}

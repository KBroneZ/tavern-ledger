// Run: deno test supabase/functions/_shared/
import { inflateCapped, readCapped, SLICE, slices, TooLarge } from "./gzip.ts";

type Bytes = Uint8Array<ArrayBuffer>;

const MIB = 1024 * 1024;

function assertEquals(actual: unknown, expected: unknown, msg = ""): void {
  const a = JSON.stringify(actual);
  const e = JSON.stringify(expected);
  if (a !== e) throw new Error(`${msg}\n  actual:   ${a}\n  expected: ${e}`);
}

async function rejects(p: Promise<unknown>): Promise<unknown> {
  try {
    await p;
  } catch (e) {
    return e;
  }
  throw new Error("expected a rejection");
}

const gzip = async (bytes: Bytes): Promise<Bytes> =>
  new Uint8Array(
    await new Response(new Blob([bytes]).stream().pipeThrough(new CompressionStream("gzip")))
      .arrayBuffer(),
  );

/** `size` zero bytes, gzipped without ever holding them all in memory. */
async function bomb(size: number): Promise<Bytes> {
  const zeros = new Uint8Array(MIB);
  let left = size;
  const source = new ReadableStream<BufferSource>({
    pull(c) {
      if (left <= 0) return c.close();
      c.enqueue(zeros.slice(0, Math.min(left, MIB)));
      left -= MIB;
    },
  });
  return new Uint8Array(
    await new Response(source.pipeThrough(new CompressionStream("gzip"))).arrayBuffer(),
  );
}

Deno.test("valid gzip inflates to the same bytes", async () => {
  const text = new TextEncoder().encode(JSON.stringify({ a: "x".repeat(5000), b: [1, 2, 3] }));
  const random = crypto.getRandomValues(new Uint8Array(40_000));
  for (const bytes of [text, random, new Uint8Array(0)]) {
    const out = await inflateCapped(await gzip(bytes as Bytes), MIB);
    assertEquals(Array.from(out), Array.from(bytes));
  }
});

Deno.test("the cap is exact: max bytes pass, one more is too large", async () => {
  const exact = await gzip(new Uint8Array(4096));
  assertEquals((await inflateCapped(exact, 4096)).length, 4096);
  assertEquals((await rejects(inflateCapped(exact, 4095))) instanceof TooLarge, true);
});

Deno.test("corrupt or truncated gzip is refused, and not as too large", async () => {
  const good = await gzip(new TextEncoder().encode("hello ".repeat(1000)) as Bytes);
  const flipped = good.slice();
  flipped[20] ^= 0xff;
  for (const bad of [new TextEncoder().encode("not gzip") as Bytes, flipped, good.slice(0, 30)]) {
    const e = await rejects(inflateCapped(bad, MIB));
    assertEquals(e instanceof TooLarge, false);
  }
});

Deno.test("a high-ratio gzip trips the cap while it inflates", async () => {
  const gz = await bomb(64 * MIB);
  assertEquals(gz.length < 70 * 1024, true, "setup: about 1000 to 1");
  assertEquals((await rejects(inflateCapped(gz, MIB))) instanceof TooLarge, true);
  // Fed in slices, no inflated chunk is larger than one slice can make
  // (deflate's ratio is at most about 1032 to 1), so the cap is passed by at
  // most that much: never the whole 64 MiB at once.
  const reader = slices(gz).pipeThrough(new DecompressionStream("gzip")).getReader();
  let largest = 0;
  let total = 0;
  while (true) {
    const { done, value } = await reader.read();
    if (done) break;
    largest = Math.max(largest, value.length);
    total += value.length;
  }
  assertEquals(total, 64 * MIB, "setup: the whole bomb was read");
  assertEquals(largest <= 1032 * SLICE, true, `largest chunk ${largest}`);
});

Deno.test("readCapped stops past the cap and joins the chunks", async () => {
  const parts = [new Uint8Array([1, 2]), new Uint8Array([3]), new Uint8Array([4, 5, 6])];
  const stream = () => new Blob(parts).stream();
  assertEquals(Array.from(await readCapped(stream(), 6)), [1, 2, 3, 4, 5, 6]);
  assertEquals((await rejects(readCapped(stream(), 5))) instanceof TooLarge, true);
});

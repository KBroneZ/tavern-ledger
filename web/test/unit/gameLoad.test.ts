import { test } from "node:test";
import assert from "node:assert/strict";
import { gzipSync } from "node:zlib";
import { type Download, loadGame, MAX_GZIPPED, MAX_INFLATED } from "../../src/lib/game/load.ts";
import { PUBLIC_ID, record, USER } from "./gameFixtures.ts";

const REF = { userId: USER, publicId: PUBLIC_ID };
const blob = (bytes: Uint8Array | string) => new Blob([bytes]);
const gz = (value: unknown) => blob(gzipSync(JSON.stringify(value)));

function serve(data: Blob | null, error: unknown = null): { download: Download; paths: string[] } {
  const paths: string[] = [];
  return {
    paths,
    download: async (path) => {
      paths.push(path);
      return { data, error };
    },
  };
}

test("load: reads the game's own file once", async () => {
  const { download, paths } = serve(gz(record()));
  const result = await loadGame(download, REF);
  assert.equal(result.kind, "ok");
  assert.deepEqual(paths, [`${USER}/${PUBLIC_ID}.json.gz`]);
  if (result.kind === "ok") assert.equal(result.record.report.hero, "BG22_HERO_002");
});

test("load: a file a proxy already inflated still reads", async () => {
  const result = await loadGame(serve(blob(JSON.stringify(record()))).download, REF);
  assert.equal(result.kind, "ok");
});

test("load: private and missing look the same (any 4xx from Storage)", async () => {
  for (const status of [400, 401, 403, 404]) {
    const result = await loadGame(serve(null, { status, message: "Object not found" }).download, REF);
    assert.deepEqual(result, { kind: "unavailable" }, String(status));
  }
});

test("load: network trouble and server errors can be retried", async () => {
  for (const error of [{ status: 500 }, { status: 503 }, { status: 429 }, { status: 408 }, new TypeError("fetch failed"), { name: "AbortError" }]) {
    assert.deepEqual(await loadGame(serve(null, error).download, REF), { kind: "error" });
  }
  const throwing: Download = async () => {
    throw new Error("offline");
  };
  assert.deepEqual(await loadGame(throwing, REF), { kind: "error" });
  // No data and no error is not a game either.
  assert.deepEqual(await loadGame(serve(null).download, REF), { kind: "error" });
});

test("load: broken files are unreadable, never a crash", async () => {
  const cases: [string, Blob][] = [
    ["not gzip, not JSON", blob("hello")],
    ["truncated gzip", blob(gzipSync(JSON.stringify(record())).subarray(0, 20))],
    ["gzip of text", blob(gzipSync("not json"))],
    ["bad UTF-8", blob(gzipSync(new Uint8Array([0x7b, 0xff, 0xfe, 0x7d])))],
    ["JSON whose index is not its report's", gz(record(undefined, { index: 99 }))],
    ["JSON array", gz([1, 2, 3])],
    ["empty", blob("")],
  ];
  for (const [name, data] of cases) {
    assert.deepEqual(await loadGame(serve(data).download, REF), { kind: "unreadable" }, name);
  }
});

test("load: oversized files are refused before and after inflating", async () => {
  const big = blob(new Uint8Array(MAX_GZIPPED + 1));
  assert.deepEqual(await loadGame(serve(big).download, REF), { kind: "unreadable" });
  // A small gzip that inflates past the cap (a "zip bomb").
  const bomb = { ...record(), padding: " ".repeat(MAX_INFLATED) };
  const bytes = gzipSync(JSON.stringify(bomb));
  assert.ok(bytes.length < MAX_GZIPPED, "the bomb itself is small");
  assert.deepEqual(await loadGame(serve(blob(bytes)).download, REF), { kind: "unreadable" });
});

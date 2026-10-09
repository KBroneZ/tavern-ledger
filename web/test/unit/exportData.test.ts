import { test } from "node:test";
import assert from "node:assert/strict";
import { buildExport, bytesToBase64, exportFileName, ownFileNames } from "../../src/lib/exportData.ts";

const USER = "0f8fad5b-d9cb-469f-a165-70867728950e";

function sample(files: unknown[]) {
  return {
    format: "tavern-ledger-export",
    format_version: 1,
    account: { id: USER, email: "a@example.test" },
    profile: { user_id: USER },
    games: [],
    files,
  };
}

test("file name has the date", () => {
  assert.equal(exportFileName(new Date("2026-10-09T23:30:00Z")), "tavern-ledger-export-2026-10-09.json");
});

test("own files: only the games bucket and the user's folder", () => {
  const data = sample([
    { bucket: "games", name: `${USER}/Hearthstone_2026_10_09_18_30_00-1.json.gz` },
    { bucket: "games", name: "someone-else/x.json.gz" },
    { bucket: "other", name: `${USER}/y.json.gz` },
    { bucket: "games", name: `${USER}/../z.json.gz` },
    "garbage",
  ]);
  assert.deepEqual(ownFileNames(data), [`${USER}/Hearthstone_2026_10_09_18_30_00-1.json.gz`]);
});

test("an answer that is not an export is refused", () => {
  assert.throws(() => ownFileNames({ format: "other" }), /export/);
  assert.throws(() => ownFileNames(null), /export/);
  assert.throws(() => ownFileNames({ ...sample([]), account: null }), /export/);
});

test("base64 of bytes", () => {
  assert.equal(bytesToBase64(new Uint8Array([])), "");
  assert.equal(bytesToBase64(new Uint8Array([0x1f, 0x8b, 0x08, 0x00, 0xff])), "H4sIAP8=");
  const big = new Uint8Array(100_000).map((_, i) => i % 256);
  assert.equal(bytesToBase64(big), Buffer.from(big).toString("base64"));
});

test("the export adds every file's bytes next to the rows", () => {
  const name = `${USER}/Hearthstone_2026_10_09_18_30_00-1.json.gz`;
  const data = sample([{ bucket: "games", name }]);
  const out = buildExport(data, [{ name, bytes: new Uint8Array([1, 2, 3]) }]);
  assert.equal(out.format, "tavern-ledger-export");
  assert.deepEqual(out.games, []);
  assert.deepEqual(out.file_contents, [{ name, encoding: "gzip+base64", data: "AQID" }]);
  // The input is not changed.
  assert.equal("file_contents" in data, false);
});

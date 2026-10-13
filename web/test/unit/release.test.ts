// The release facts the download page and the home page's button share
// (T-558): one source, and the addresses must agree with the version.
import { test } from "node:test";
import assert from "node:assert/strict";
import { RELEASE, installerUrl, releasePageUrl, releasesUrl, sizeMb, sumsUrls } from "../../src/lib/site/release.ts";
import releases from "../../src/lib/site/releases.json" with { type: "json" };

test("the file name, tag and addresses all follow from the version", () => {
  assert.match(RELEASE.version, /^\d+\.\d+\.\d+$/);
  assert.equal(RELEASE.tag, `v${RELEASE.version}`);
  assert.equal(RELEASE.file, `TavernLedger_${RELEASE.version}_x64-setup.exe`);
  assert.equal(installerUrl(), `https://github.com/KBroneZ/tavern-ledger/releases/download/${RELEASE.tag}/${RELEASE.file}`);
  assert.equal(releasePageUrl(), `https://github.com/KBroneZ/tavern-ledger/releases/tag/${RELEASE.tag}`);
  assert.equal(releasesUrl(), "https://github.com/KBroneZ/tavern-ledger/releases");
});

test("the hash is 64 lower-case hex digits and the date is a day", () => {
  assert.match(RELEASE.sha256, /^[0-9a-f]{64}$/);
  assert.match(RELEASE.date, /^\d{4}-\d{2}-\d{2}$/);
  assert.ok(!Number.isNaN(Date.parse(`${RELEASE.date}T00:00:00Z`)));
});

test("the size is shown in megabytes with one decimal, the way Windows shows it", () => {
  assert.equal(sizeMb(6_847_192), 6.5);
  assert.equal(sizeMb(RELEASE.bytes), 7.6);
  assert.ok(RELEASE.bytes > 1_000_000 && RELEASE.bytes < 50_000_000);
});

test("the facts are those of the published v0.2.1 release, with v0.1.0 kept below it", () => {
  assert.equal(RELEASE.version, "0.2.1");
  assert.equal(RELEASE.sha256, "1e69f959b7b593ec4703e7cedf050647df7794b208a9e48ff23aeaac53120b9c");
  assert.deepEqual(releases.map((r) => r.version), ["0.2.1", "0.1.0"]);
});

test("the facts are the newest release recorded from GitHub, not typed by hand", () => {
  const newest = releases[0];
  assert.equal(RELEASE.version, newest.version);
  assert.equal(RELEASE.sha256, newest.sha256);
  assert.equal(RELEASE.bytes, newest.bytes);
  // The installer's own record among the release's files says the same.
  assert.deepEqual(newest.assets[RELEASE.file as keyof typeof newest.assets], { bytes: RELEASE.bytes, sha256: RELEASE.sha256 });
});

test("the signed list of hashes is linked only for a release that has one", () => {
  assert.equal(RELEASE.sums, true, "v0.2.1 has a signed SHA256SUMS");
  assert.deepEqual(sumsUrls(), {
    list: "https://github.com/KBroneZ/tavern-ledger/releases/download/v0.2.1/SHA256SUMS",
    signature: "https://github.com/KBroneZ/tavern-ledger/releases/download/v0.2.1/SHA256SUMS.minisig",
  });
});

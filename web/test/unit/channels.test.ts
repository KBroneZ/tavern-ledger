// Other ways to install (T-707): nothing is announced until a listing is live,
// and a malformed value fails the build instead of showing a broken link.
import { test } from "node:test";
import assert from "node:assert/strict";
import { liveChannels, STORE_URL, WINGET_ID } from "../../src/lib/site/channels.ts";

test("no channel is announced yet: the Store and winget listings are not live", () => {
  assert.equal(STORE_URL, null);
  assert.equal(WINGET_ID, null);
  assert.deepEqual(liveChannels(), []);
});

test("a live Store listing and winget package come out Store first, with the install command", () => {
  assert.deepEqual(liveChannels("https://apps.microsoft.com/detail/9NBLGGH4NNS1", "KBroneZ.TavernLedger"), [
    { kind: "store", url: "https://apps.microsoft.com/detail/9NBLGGH4NNS1" },
    { kind: "winget", command: "winget install KBroneZ.TavernLedger" },
  ]);
  assert.deepEqual(liveChannels(null, "KBroneZ.TavernLedger"), [{ kind: "winget", command: "winget install KBroneZ.TavernLedger" }]);
});

test("a malformed value stops the build", () => {
  for (const store of ["http://apps.microsoft.com/detail/9NBLGGH4NNS1", "https://www.microsoft.com/store/apps/9NBLGGH4NNS1", "https://apps.microsoft.com/detail/9nblggh4nns1", "https://apps.microsoft.com/detail/9NBLGGH4NNS1?hl=en"]) {
    assert.throws(() => liveChannels(store, null), /not a Store listing address/, store);
  }
  for (const id of ["TavernLedger", "KBroneZ.TavernLedger; rm", "KBroneZ TavernLedger", ""]) {
    assert.throws(() => liveChannels(null, id), /not a winget package identifier/, id);
  }
});

// The Simple / Advanced choice (kept per viewer) and the Solo / Duos mode
// (kept in the address) for the data pages (T-501).
import { test } from "node:test";
import assert from "node:assert/strict";
import { busierMode, DETAIL_KEY, modeInAddress, parseMode, readDetail, saveDetail, withMode } from "../../src/lib/data/prefs.ts";

const store = (value: string | null) => ({ getItem: (k: string) => (k === DETAIL_KEY ? value : null) });

test("readDetail: Simple unless the viewer chose Advanced; a broken or blocked store means Simple", () => {
  assert.equal(readDetail(store("advanced")), "advanced");
  assert.equal(readDetail(store("simple")), "simple");
  assert.equal(readDetail(store("ADVANCED")), "simple");
  assert.equal(readDetail(store(null)), "simple");
  assert.equal(readDetail(null), "simple");
  assert.equal(readDetail({ getItem: () => { throw new Error("blocked"); } }), "simple");
});

test("saveDetail: writes the choice and never throws when storage is blocked", () => {
  const saved = new Map<string, string>();
  assert.equal(saveDetail({ setItem: (k, v) => void saved.set(k, v) }, "advanced"), true);
  assert.equal(saved.get(DETAIL_KEY), "advanced");
  assert.equal(saveDetail({ setItem: () => { throw new Error("quota"); } }, "simple"), false);
  assert.equal(saveDetail(null, "simple"), false);
});

test("parseMode: duos only when the address says so, Solo otherwise", () => {
  assert.equal(parseMode("?mode=duos"), "duos");
  assert.equal(parseMode("?x=1&mode=solo"), "solo");
  assert.equal(parseMode("?mode=DUOS"), "solo");
  assert.equal(parseMode(""), "solo");
});

test("withMode: the address names the mode either way (the default follows the games), the rest stays", () => {
  assert.equal(withMode("https://tavernledger.net/style/?a=1#tables", "duos"), "/style/?a=1&mode=duos#tables");
  assert.equal(withMode("https://tavernledger.net/style/?mode=duos&a=1", "solo"), "/style/?mode=solo&a=1");
  assert.equal(withMode("https://tavernledger.net/style/", "solo"), "/style/?mode=solo");
});

test("with no mode in the address, a page opens on the mode with more games (T-559)", () => {
  assert.equal(modeInAddress("?mode=duos"), "duos");
  assert.equal(modeInAddress("?mode=solo"), "solo");
  assert.equal(modeInAddress(""), null);
  assert.equal(modeInAddress("?mode=DUOS"), null);
  assert.equal(busierMode({ solo: 2, duos: 39 }), "duos");
  assert.equal(busierMode({ solo: 5, duos: 5 }), "solo", "a tie keeps Solo");
  assert.equal(busierMode({ solo: 0, duos: 0 }), "solo");
});

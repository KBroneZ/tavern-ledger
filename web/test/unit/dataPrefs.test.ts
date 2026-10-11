// The Simple / Advanced choice (kept per viewer) and the Solo / Duos mode
// (kept in the address) for the data pages (T-501).
import { test } from "node:test";
import assert from "node:assert/strict";
import { DETAIL_KEY, parseMode, readDetail, saveDetail, withMode } from "../../src/lib/data/prefs.ts";

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

test("withMode: Solo drops the parameter, Duos sets it, the rest of the address stays", () => {
  assert.equal(withMode("https://tavernledger.net/style/?a=1#tables", "duos"), "/style/?a=1&mode=duos#tables");
  assert.equal(withMode("https://tavernledger.net/style/?mode=duos&a=1", "solo"), "/style/?a=1");
  assert.equal(withMode("https://tavernledger.net/style/?mode=duos", "solo"), "/style/");
});

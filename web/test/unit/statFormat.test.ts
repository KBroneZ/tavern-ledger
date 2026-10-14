// The short form of a minion's Attack or Health (T-616): 1,000 and up is
// 1K, 1M, 1B like the game, one decimal cut (not rounded), no ".0". The
// exact value goes in the tooltip.
import { test } from "node:test";
import assert from "node:assert/strict";
import { setLang } from "../../src/lib/i18n.ts";
import { exactStat, shortStat } from "../../src/lib/stat.ts";

// The desktop app has the same table (crates/desktop/ui-tests/stat-format.test.js).
const SHORT: [number, string][] = [
  [0, "0"],
  [7, "7"],
  [999, "999"],
  [1000, "1K"],
  [1049, "1K"],
  [1050, "1K"],
  [1099, "1K"],
  [1100, "1.1K"],
  [1999, "1.9K"],
  [9999, "9.9K"],
  [10_000, "10K"],
  [12_345, "12.3K"],
  [99_999, "99.9K"],
  [100_000, "100K"],
  [999_999, "999.9K"],
  [1_000_000, "1M"],
  [1_500_000, "1.5M"],
  [999_999_999, "999.9M"],
  [1_000_000_000, "1B"],
  [2_147_483_647, "2.1B"],
  [1_230_000_000_000, "1230B"],
  [-5, "-5"],
  [-1500, "-1.5K"],
  [-1_000_000, "-1M"],
];

test("shortStat: thresholds and the cut decimal", () => {
  setLang("en");
  for (const [value, text] of SHORT) assert.equal(shortStat(value), text, String(value));
});

test("shortStat: a decimal comma in Spanish, the same letters", () => {
  setLang("es");
  assert.equal(shortStat(1500), "1,5K");
  assert.equal(shortStat(2_147_483_647), "2,1B");
  assert.equal(shortStat(1000), "1K");
  assert.equal(shortStat(999), "999");
  setLang("en");
});

test("shortStat: a fraction is cut to a whole number first, an unknown value is a question mark", () => {
  setLang("en");
  assert.equal(shortStat(999.9), "999");
  assert.equal(shortStat(1999.9), "1.9K");
  assert.equal(shortStat(Number.NaN), "?");
  assert.equal(shortStat(Number.POSITIVE_INFINITY), "?");
  assert.equal(shortStat(null), "?");
  assert.equal(shortStat(undefined), "?");
});

test("exactStat: every digit, grouped in the page's language", () => {
  setLang("en");
  assert.equal(exactStat(999), "999");
  assert.equal(exactStat(1234), "1,234");
  assert.equal(exactStat(2_147_483_647), "2,147,483,647");
  assert.equal(exactStat(-1500), "-1,500");
  assert.equal(exactStat(null), "?");
  setLang("es");
  assert.equal(exactStat(1_234_567), "1.234.567");
  assert.equal(exactStat(1234), "1234");
  setLang("en");
});

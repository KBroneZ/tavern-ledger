"use strict";
// A minion's Attack or Health the way the game writes a big one (T-611):
// 1,000 and up is 1K, 1M, 1B with one decimal that is cut, not rounded, and
// no ".0". The exact value is for the tooltip. The website runs the same
// table (web/test/unit/statFormat.test.ts).
const test = require("node:test");
const assert = require("node:assert/strict");
const I18n = require("../ui/i18n.js");

const SHORT = [
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
  [10000, "10K"],
  [12345, "12.3K"],
  [99999, "99.9K"],
  [100000, "100K"],
  [999999, "999.9K"],
  [1000000, "1M"],
  [1500000, "1.5M"],
  [999999999, "999.9M"],
  [1000000000, "1B"],
  [2147483647, "2.1B"],
  [1230000000000, "1230B"],
  [-5, "-5"],
  [-1500, "-1.5K"],
  [-1000000, "-1M"],
];

test("stat: thresholds and the cut decimal", () => {
  I18n.setLang("en");
  for (const [value, text] of SHORT) assert.equal(I18n.stat(value), text, String(value));
});

test("stat: a decimal comma in Spanish, the same letters", () => {
  I18n.setLang("es");
  assert.equal(I18n.stat(1500), "1,5K");
  assert.equal(I18n.stat(2147483647), "2,1B");
  assert.equal(I18n.stat(1000), "1K");
  assert.equal(I18n.stat(999), "999");
  I18n.setLang("en");
});

test("stat: a fraction is cut to a whole number first, an unknown value is a question mark", () => {
  I18n.setLang("en");
  assert.equal(I18n.stat(999.9), "999");
  assert.equal(I18n.stat(1999.9), "1.9K");
  assert.equal(I18n.stat(NaN), "?");
  assert.equal(I18n.stat(Infinity), "?");
  assert.equal(I18n.stat(null), "?");
  assert.equal(I18n.stat(undefined), "?");
});

test("exact: every digit, grouped the language's way", () => {
  I18n.setLang("en");
  assert.equal(I18n.exact(999), "999");
  assert.equal(I18n.exact(1234), "1,234");
  assert.equal(I18n.exact(2147483647), "2,147,483,647");
  assert.equal(I18n.exact(-1500), "-1,500");
  assert.equal(I18n.exact(null), "?");
  I18n.setLang("es");
  assert.equal(I18n.exact(1234567), "1.234.567");
  assert.equal(I18n.exact(1234), "1234");
  I18n.setLang("en");
});

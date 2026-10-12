// The blurred hero tokens in the home page's scene (T-530, D-105): which
// heroes, picked from the known base heroes, the same all day.
import { test } from "node:test";
import assert from "node:assert/strict";
import { pickHeroes, tokenDay } from "../../src/lib/home/tokens.ts";

const IDS = ["BG20_HERO_100", "BG20_HERO_101", "BG22_HERO_007t", "TB_BaconShop_HERO_01", "BG21_HERO_000_SKIN_A", "BG25_HERO_105", "TB_BaconShop_HERO_16"];

test("only base heroes are picked: no tokens, no skins", () => {
  const picked = pickHeroes(IDS, 10, 3);
  assert.deepEqual([...picked].sort(), ["BG20_HERO_100", "BG20_HERO_101", "BG25_HERO_105", "TB_BaconShop_HERO_01", "TB_BaconShop_HERO_16"]);
});

test("picks the asked number, all different", () => {
  const picked = pickHeroes(IDS, 3, 11);
  assert.equal(picked.length, 3);
  assert.equal(new Set(picked).size, 3);
});

test("the same day gives the same heroes; another day may differ", () => {
  assert.deepEqual(pickHeroes(IDS, 3, 20000), pickHeroes(IDS, 3, 20000));
  const days = new Set(Array.from({ length: 12 }, (_, d) => pickHeroes(IDS, 3, d).join()));
  assert.ok(days.size > 1);
});

test("asking for none, or from no heroes, gives none", () => {
  assert.deepEqual(pickHeroes(IDS, 0, 1), []);
  assert.deepEqual(pickHeroes([], 4, 1), []);
  assert.deepEqual(pickHeroes(["not a hero"], 4, 1), []);
});

test("tokenDay counts whole UTC days", () => {
  assert.equal(tokenDay(Date.UTC(1970, 0, 1, 23, 59)), 0);
  assert.equal(tokenDay(Date.UTC(1970, 0, 2)), 1);
});

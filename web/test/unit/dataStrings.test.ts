// The data pages keep every string in one table (T-501, for T-508's Spanish),
// and pictures come from one setting (T-501; card art waits on the CSP decision).
import { test } from "node:test";
import assert from "node:assert/strict";
import { readdirSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { STRINGS, t } from "../../src/lib/data/strings.ts";
import { pictureFor, PICTURE_SOURCE, type PictureSource } from "../../src/lib/data/pictures.ts";

const ROOT = join(import.meta.dirname, "..", "..");

test("t fills each placeholder and refuses a missing one or an unknown key", () => {
  assert.equal(t("backing.some", { known: 28, total: 31, unit: "games" }), "in 28 of 31 games");
  assert.throws(() => t("backing.some", { known: 28 }), /total/);
  assert.throws(() => t("no.such.key" as keyof typeof STRINGS), /no.such.key/);
});

test("every key the data components name exists in the table", () => {
  const dir = join(ROOT, "src", "components", "data");
  const sources = readdirSync(dir).map((f) => readFileSync(join(dir, f), "utf8"));
  sources.push(readFileSync(join(ROOT, "src", "pages", "style.astro"), "utf8"));
  const keys = sources.flatMap((s) => [...s.matchAll(/\bt\("([a-z0-9.]+)"/g)].map((m) => m[1]));
  assert.ok(keys.length > 10, "the components use the table");
  for (const key of keys) assert.ok(key in STRINGS, key);
});

test("the default picture source is the local synthetic placeholders", () => {
  assert.equal(PICTURE_SOURCE.kind, "placeholder");
  const hero = pictureFor("hero", "TB_BaconShop_HERO_01");
  assert.match(hero.src, /^\/placeholders\/hero-[0-9]\.svg$/);
  assert.equal(pictureFor("hero", "TB_BaconShop_HERO_01").src, hero.src, "the same id always gets the same placeholder");
  assert.match(pictureFor("card", "BGS_004").src, /^\/placeholders\/card-[0-9]\.svg$/);
  for (const name of readdirSync(join(ROOT, "public", "placeholders"))) assert.match(name, /^(hero|card)-[0-9]\.svg$/);
});

test("a remote source builds the address from the card id and refuses anything else", () => {
  const remote: PictureSource = { kind: "remote", base: "https://img.example.test/v1/256x/", ext: ".jpg" };
  assert.equal(pictureFor("card", "BGS_004", remote).src, "https://img.example.test/v1/256x/BGS_004.jpg");
  assert.throws(() => pictureFor("card", "../x", remote), /card id/);
  assert.throws(() => pictureFor("card", "a b", remote), /card id/);
  assert.throws(() => pictureFor("card", "", remote), /card id/);
});

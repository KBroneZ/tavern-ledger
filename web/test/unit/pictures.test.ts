// Pictures come from one setting: HearthstoneJSON's art host at view time
// (D-074), with a local placeholder when a picture fails.
import { test } from "node:test";
import assert from "node:assert/strict";
import { readdirSync } from "node:fs";
import { join } from "node:path";
import {
  ART_ORIGIN,
  FALLBACK_ATTR,
  PICTURE_SOURCE,
  PLACEHOLDER_SOURCE,
  pictureFor,
  type PictureSource,
  useFallback,
  watchPictureFallback,
} from "../../src/lib/data/pictures.ts";

const ROOT = join(import.meta.dirname, "..", "..");

test("the picture source is the app's art: HearthstoneJSON 256x JPG (D-038, D-074)", () => {
  assert.equal(ART_ORIGIN, "https://art.hearthstonejson.com");
  const card = pictureFor("card", "BGS_004");
  assert.equal(card.src, "https://art.hearthstonejson.com/v1/256x/BGS_004.jpg");
  assert.equal(card.synthetic, false);
  assert.match(card.fallback, /^\/placeholders\/card-[0-9]\.svg$/);
  assert.equal(new URL(pictureFor("hero", "TB_BaconShop_HERO_01").src).origin, ART_ORIGIN);
  assert.equal(PICTURE_SOURCE.kind, "remote");
});

test("placeholders: local, stable per id, and every file is synthetic", () => {
  const hero = pictureFor("hero", "TB_BaconShop_HERO_01", PLACEHOLDER_SOURCE);
  assert.match(hero.src, /^\/placeholders\/hero-[0-9]\.svg$/);
  assert.equal(hero.synthetic, true);
  assert.equal(pictureFor("hero", "TB_BaconShop_HERO_01", PLACEHOLDER_SOURCE).src, hero.src);
  assert.equal(pictureFor("hero", "TB_BaconShop_HERO_01").fallback, hero.src, "the fallback is the same placeholder");
  for (const name of readdirSync(join(ROOT, "public", "placeholders"))) assert.match(name, /^(hero|card)-[0-9]\.svg$/);
});

test("a remote source builds the address from the card id and refuses anything else", () => {
  const remote: PictureSource = { kind: "remote", base: "https://img.example.test/v1/256x/", ext: ".jpg" };
  assert.equal(pictureFor("card", "BGS_004", remote).src, "https://img.example.test/v1/256x/BGS_004.jpg");
  for (const id of ["../x", "a b", "", "x".repeat(65), "a/b", "a?b", "a#b"]) {
    assert.throws(() => pictureFor("card", id), /card id/, id);
  }
});

class FakeImg {
  tagName = "IMG";
  src = "https://art.hearthstonejson.com/v1/256x/X.jpg";
  attrs = new Map<string, string>();
  getAttribute(name: string) {
    return this.attrs.get(name) ?? null;
  }
  removeAttribute(name: string) {
    this.attrs.delete(name);
  }
}

test("fallback: a failed picture swaps to its local placeholder once", () => {
  const img = new FakeImg();
  img.attrs.set(FALLBACK_ATTR, "/placeholders/card-2.svg");
  assert.equal(useFallback(img as unknown as HTMLImageElement), true);
  assert.equal(img.src, "/placeholders/card-2.svg");
  // The placeholder failing too does not loop.
  assert.equal(useFallback(img as unknown as HTMLImageElement), false);
});

test("fallback: only a local placeholder address is accepted", () => {
  for (const bad of ["https://evil.test/x.svg", "//evil.test/x", "javascript:alert(1)", "/placeholders/../x.svg", ""]) {
    const img = new FakeImg();
    img.attrs.set(FALLBACK_ATTR, bad);
    assert.equal(useFallback(img as unknown as HTMLImageElement), false, bad);
    assert.equal(img.src, "https://art.hearthstonejson.com/v1/256x/X.jpg");
  }
});

test("fallback: one capture-phase listener handles every picture's error", () => {
  const calls: [string, boolean][] = [];
  let handler: ((event: Event) => void) | null = null;
  watchPictureFallback({
    addEventListener: ((type: string, fn: (event: Event) => void, capture: boolean) => {
      calls.push([type, capture]);
      handler = fn;
    }) as Document["addEventListener"],
  });
  assert.deepEqual(calls, [["error", true]]);
  const img = new FakeImg();
  img.attrs.set(FALLBACK_ATTR, "/placeholders/hero-1.svg");
  handler!({ target: img } as unknown as Event);
  assert.equal(img.src, "/placeholders/hero-1.svg");
  // Other elements (a script, a link) are left alone.
  handler!({ target: { tagName: "SCRIPT" } } as unknown as Event);
});

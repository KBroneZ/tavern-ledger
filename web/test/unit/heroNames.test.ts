// Hero names (T-510, D-092): every hero id in the user's own history has a
// name in both languages, skins go by their base hero, an unknown id never
// shows as a name, and the table is shaped as the pages expect.
import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { setLang } from "../../src/lib/i18n.ts";
import { baseHero, byHeroName, heroLabel, heroName, heroOptions } from "../../src/lib/heroes/names.ts";
import { HERO_NAMES, HERO_SKINS } from "../../src/lib/heroes/names.data.ts";

// Every hero card id in the user's games.jsonl (their own heroes, teammates
// and lobbies), card ids only, collected on 2026-10-11.
const HISTORY = readFileSync(new URL("../data/history-hero-ids.txt", import.meta.url), "utf8")
  .split("\n")
  .map((l) => l.trim())
  .filter(Boolean);

test("every hero id in the user's history has a name in English and Spanish", () => {
  assert.ok(HISTORY.length >= 100, "the fixture has the ids");
  const missing = HISTORY.filter((id) => baseHero(id) === null);
  assert.deepEqual(missing, []);
  for (const lang of ["en", "es"]) {
    setLang(lang);
    for (const id of HISTORY) {
      const name = heroName(id);
      assert.ok(name && name.trim() && name !== id, `${lang} ${id}`);
    }
  }
  setLang("en");
});

test("names follow the page's language", () => {
  setLang("en");
  assert.equal(heroName("BG20_HERO_102"), "Overlord Saurfang");
  assert.equal(heroName("TB_BaconShop_HERO_41"), "Reno Jackson");
  setLang("es");
  assert.equal(heroName("BG20_HERO_102"), "Señor supremo Colmillosauro");
  setLang("en");
});

test("skins go by their base hero, by the id rule or the listed exceptions", () => {
  assert.equal(baseHero("TB_BaconShop_HERO_41_SKIN_F"), "TB_BaconShop_HERO_41");
  assert.equal(heroName("TB_BaconShop_HERO_41_SKIN_F"), heroName("TB_BaconShop_HERO_41"));
  assert.equal(baseHero("BG20_HERO_202_SKIN_B4"), "BG20_HERO_202");
  // Listed: the id rule would give a card that is not this skin's hero.
  assert.equal(baseHero("TB_BaconShop_HERO_102_SKIN_F"), "BG20_HERO_102");
  assert.equal(baseHero("BG26_HERO_100_SKIN_A"), "BG25_HERO_105");
});

test("an unknown id, no id and odd input have no name", () => {
  assert.equal(heroName("BG99_HERO_999"), null);
  assert.equal(heroName("BG99_HERO_999_SKIN_A"), null);
  assert.equal(heroName(null), null);
  assert.equal(heroName(""), null);
  assert.equal(heroName("__proto__"), null);
  assert.equal(heroName("constructor"), null);
  assert.equal(baseHero("toString_SKIN_A"), null);
});

test("heroLabel never shows the bare id as a name", () => {
  assert.deepEqual(heroLabel("BG22_HERO_002", "Not recorded", "Unknown hero"), { name: "Drek'Thar", unknownId: null });
  assert.deepEqual(heroLabel("BG99_HERO_999", "Not recorded", "Unknown hero"), { name: "Unknown hero", unknownId: "BG99_HERO_999" });
  assert.deepEqual(heroLabel(null, "Not recorded", "Unknown hero"), { name: "Not recorded", unknownId: null });
});

test("byHeroName sorts by name, unknown ids by id", () => {
  const ids = ["TB_BaconShop_HERO_41", "BG22_HERO_002", "BG99_HERO_999"];
  setLang("en");
  assert.deepEqual(ids.toSorted(byHeroName), ["BG99_HERO_999", "BG22_HERO_002", "TB_BaconShop_HERO_41"]);
});

test("the table is complete and well formed", () => {
  const ids = Object.keys(HERO_NAMES);
  assert.ok(ids.length >= 100);
  for (const [id, [en, es]] of Object.entries(HERO_NAMES)) {
    assert.match(id, /^[A-Za-z0-9_]{1,64}$/);
    assert.ok(en.trim() && es.trim(), id);
  }
  for (const [skin, base] of Object.entries(HERO_SKINS)) {
    assert.ok(Object.hasOwn(HERO_NAMES, base), `${skin} -> ${base}`);
    assert.notEqual(skin.replace(/_SKIN_[A-Za-z0-9]+$/, ""), base, `${skin} needs no listing`);
  }
});

test("the games list's hero filter lists names, sorted, with the address's hero kept", () => {
  setLang("en");
  const options = heroOptions(["TB_BaconShop_HERO_41", "BG22_HERO_002"], "BG99_HERO_999", "Unknown hero");
  assert.deepEqual(options, [
    { value: "BG99_HERO_999", label: "Unknown hero (BG99_HERO_999)" },
    { value: "BG22_HERO_002", label: "Drek'Thar" },
    { value: "TB_BaconShop_HERO_41", label: "Reno Jackson" },
  ]);
  assert.equal(heroOptions(["BG22_HERO_002"], "BG22_HERO_002", "?").length, 1);
});

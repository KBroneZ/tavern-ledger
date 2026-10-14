// Card names on the game page (T-559): the build keeps the names of the
// Battlegrounds cards that are not minions (spells, trinkets, hero powers),
// in English and Spanish, and the page names a card from the minions, then
// that table, then the Dark Gift table, else by its id.
import { afterEach, test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { cardLabel, cardName, readNameFile, trinketName, useCardNames } from "../../src/lib/game/names.ts";
import { setLang } from "../../src/lib/i18n.ts";
import { CardIndex } from "../../src/lib/sim/cards.ts";
import { extractCards, extractNames } from "../../tools/simCards.ts";

const FIXTURES = join(import.meta.dirname, "..", "fixtures", "sim-cards");
const en = JSON.parse(readFileSync(join(FIXTURES, "enUS.json"), "utf8")) as unknown[];
const es = JSON.parse(readFileSync(join(FIXTURES, "esES.json"), "utf8")) as unknown[];

afterEach(() => {
  useCardNames(null);
  setLang("en");
});

test("the build keeps the names of spells, trinkets and hero powers, not minions or enchantments", () => {
  const file = extractNames(en, en, "fixture");
  assert.deepEqual(file.names, { BG_SPELL_SYN: "A spell", BG_TRINKET_SYN: "A trinket", BG_HP_SYN: "A hero power" });
  assert.deepEqual(file.trinkets, { "910004": "BG_TRINKET_SYN" }, "a trinket by its database id, for the lobby");
  const spanish = extractNames(en, es, "fixture");
  assert.equal(spanish.names.BG_SPELL_SYN, "Un hechizo");
  assert.equal(spanish.names.BG_HP_SYN, "A hero power", "a name missing in Spanish falls back to English");
  assert.throws(() => extractNames({}, en, "fixture"), /not a list/);
});

test("the name file is read like any data: bad ids, names and trinkets are dropped", () => {
  const read = readNameFile({ names: { GOOD_1: "Good", "../bad": "Bad", EMPTY: "", LONG: "x".repeat(101), NUM: 3 }, trinkets: { "12": "GOOD_1", "-1": "GOOD_1", "13": "MISSING" } });
  assert.deepEqual([...read.names], [["GOOD_1", "Good"]]);
  assert.deepEqual([...read.trinkets], [[12, "GOOD_1"]]);
  assert.deepEqual([...readNameFile(null).names], []);
});

test("a card is named from the minions, then the other cards, then the Dark Gifts, else by its id", () => {
  assert.equal(cardName("BGS_119"), null, "no table loaded: no name");
  assert.equal(cardLabel("BGS_119", "Unknown card"), "BGS_119");
  assert.equal(cardLabel(null, "Unknown card"), "Unknown card");
  const cards = new CardIndex(extractCards(en, en, "fixture"));
  useCardNames({ cards, ...readNameFile(extractNames(en, en, "fixture")) });
  assert.equal(cardName("BGS_119"), cards.get("BGS_119")?.n);
  assert.equal(cardName("BG_SPELL_SYN"), "A spell");
  assert.equal(cardName("BG36_MidGameEffect_000t9"), "Admiration", "a Dark Gift from its own table");
  assert.equal(cardName("NOT_KNOWN"), null);
  assert.equal(trinketName(910004), "A trinket");
  assert.equal(trinketName(1), null);
  setLang("es");
  assert.equal(cardName("BG36_MidGameEffect_000t9"), "Admiración");
});

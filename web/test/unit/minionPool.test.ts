// The minion pages' data (T-578, D-134): the tavern's current pool from
// HearthstoneJSON's card list, with each minion's tier, stats, types, card
// text and golden stats, in the page's language.
import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { cardText, extractPool } from "../../tools/minionPool.ts";

const FIXTURES = join(import.meta.dirname, "..", "fixtures", "sim-cards");
const en = JSON.parse(readFileSync(join(FIXTURES, "enUS.json"), "utf8")) as unknown[];
const es = JSON.parse(readFileSync(join(FIXTURES, "esES.json"), "utf8")) as unknown[];

const minion = (over: Record<string, unknown> = {}) => ({
  id: "BG_X_1",
  dbfId: 1,
  name: "Test Minion",
  type: "MINION",
  set: "BATTLEGROUNDS",
  techLevel: 2,
  attack: 2,
  health: 3,
  races: ["BEAST"],
  text: "Hello.",
  isBattlegroundsPoolMinion: true,
  ...over,
});

test("only minions in the tavern's pool are kept", () => {
  const pool = extractPool(en, en, "fixture");
  const ids = pool.minions.map((m) => m.id);
  assert.ok(ids.includes("BG_SYN_MURPIR"));
  assert.ok(!ids.includes("BG_SYN_OLDMUR"), "a minion not in the pool");
  assert.ok(!ids.includes("BG_SYN_MURPIR_G"), "a golden card");
  assert.ok(!ids.includes("BG_ICC_026t"), "a token");
  assert.ok(!ids.includes("CORE_SYN"), "not a Battlegrounds card");
  assert.ok(!ids.includes("BG_SPELL_SYN"), "not a minion");
  assert.equal(pool.source, "fixture");
  assert.deepEqual(ids, [...ids].sort(), "sorted by id, so the file is stable");
});

test("a minion keeps its tier, stats, every type and the golden card's stats", () => {
  const pool = extractPool(en, en, "fixture");
  const fin = pool.minions.find((m) => m.id === "BG_SYN_MURPIR");
  assert.deepEqual(fin, {
    id: "BG_SYN_MURPIR",
    name: "Synthetic Deckhand Fin",
    tier: 3,
    attack: 3,
    health: 4,
    tribes: ["MURLOC", "PIRATE"],
    text: [{ s: "Battlecry:", b: 1 }, { s: " Give a friendly Murloc +1/+1. " }, { s: "(Synthetic.)", i: 1 }],
    golden: { attack: 6, health: 8 },
  });
  const amalgam = pool.minions.find((m) => m.id === "BG_SYN_AMALGAM");
  assert.deepEqual(amalgam?.tribes, ["ALL"]);
  assert.equal(amalgam?.duosOnly, true);
  assert.equal(amalgam?.golden, undefined, "no golden card in the data: no golden stats");
  const unknown = pool.minions.find((m) => m.id === "BG_NOT_A_CARD");
  assert.deepEqual(unknown?.tribes, []);
  assert.equal(unknown?.text, null, "no card text: none, not an empty line");
});

test("names and text come in the page's language, else English", () => {
  const pool = extractPool(en, es, "fixture");
  const fin = pool.minions.find((m) => m.id === "BG_SYN_MURPIR");
  assert.equal(fin?.name, "Aleta grumete sintética");
  assert.deepEqual(fin?.text?.[0], { s: "Grito de batalla:", b: 1 });
  const spark = pool.minions.find((m) => m.id === "BGS_119");
  assert.equal(spark?.name, "Chispa sintética");
  const unknown = pool.minions.find((m) => m.id === "BG_NOT_A_CARD");
  assert.equal(unknown?.name, "Desconocido sintético");
  // Missing in Spanish: the English name and text, as the simulator names cards.
  const bare = extractPool([minion()], [{ id: "BG_X_1" }], "t").minions[0];
  assert.equal(bare.name, "Test Minion");
  assert.deepEqual(bare.text, [{ s: "Hello." }]);
});

test("card text: bold and italic kept, layout marks and line breaks dropped", () => {
  assert.deepEqual(cardText("[x]<b>Spellcraft:</b> Give a minion\n<b>Divine Shield</b>\nuntil next turn."), [
    { s: "Spellcraft:", b: 1 },
    { s: " Give a minion " },
    { s: "Divine Shield", b: 1 },
    { s: " until next turn." },
  ]);
  assert.deepEqual(cardText("<b>Battlecry:</b> Give a <b>Taunt</b>\nminion +1/+1."), [
    { s: "Battlecry:", b: 1 },
    { s: " Give a " },
    { s: "Taunt", b: 1 },
    { s: " minion +1/+1." },
  ]);
  assert.deepEqual(cardText("<b><i>Both</i></b>"), [{ s: "Both", b: 1, i: 1 }]);
  // The card data nests bold in places.
  assert.deepEqual(cardText("[x]<b><b>Reborn</b>.</b> <b>Deathrattle:</b>"), [{ s: "Reborn.", b: 1 }, { s: " " }, { s: "Deathrattle:", b: 1 }]);
  assert.deepEqual(cardText("   de los esbirros. "), [{ s: "de los esbirros." }]);
  assert.equal(cardText(""), null);
  assert.equal(cardText(undefined), null);
  assert.equal(cardText(42), null);
});

test("card text: a keyword alone on a first line keeps its own line", () => {
  assert.deepEqual(cardText("[x]<b>Taunt</b>\n<b>Deathrattle:</b> Summon\n  a 1/1."), [
    { s: "Taunt", b: 1 },
    { s: "\n" },
    { s: "Deathrattle:", b: 1 },
    { s: " Summon a 1/1." },
  ]);
  assert.deepEqual(cardText("<b>Divine Shield</b>\n<b><b>Reborn</b>.</b> \nThis minion is <b>Golden</b>."), [
    { s: "Divine Shield", b: 1 },
    { s: "\n" },
    { s: "Reborn.", b: 1 },
    { s: "\n" },
    { s: "This minion is " },
    { s: "Golden", b: 1 },
    { s: "." },
  ]);
  assert.deepEqual(cardText("<b>Taunt</b>"), [{ s: "Taunt", b: 1 }], "no break after the last line");
  assert.deepEqual(cardText("<b>Taunt</b>\n"), [{ s: "Taunt", b: 1 }], "nor before an empty last line");
  assert.deepEqual(cardText("<b>Taunt</b>\n\n<b>Reborn</b>"), [{ s: "Taunt", b: 1 }, { s: "\n" }, { s: "Reborn", b: 1 }], "one break at most");
  // A label ("Battlecry:") reads on with its effect: not a line of its own.
  assert.deepEqual(cardText("[x]<b>Battlecry:</b>\nGive a minion +1/+1."), [{ s: "Battlecry:", b: 1 }, { s: " Give a minion +1/+1." }]);
});

test("card text with other markup or hidden characters is not shown", () => {
  assert.equal(cardText('<a href="x">link</a>'), null);
  assert.equal(cardText("<script>x</script>"), null);
  assert.equal(cardText("bad‮order"), null);
  assert.equal(cardText("bell\u0007"), null);
  assert.equal(cardText("x".repeat(2000)), null);
  assert.equal(cardText("<b>never closed"), null);
  // A value the game fills in (spell damage, a counter) is unknown here: no text rather than a symbol.
  assert.equal(cardText("Deal $3 damage."), null);
  assert.equal(cardText("Gain +@ Attack."), null);
  assert.equal(cardText("(#2 left)"), null);
  assert.equal(cardText("Summon a {0}."), null);
  assert.equal(cardText("closed</b> never opened"), null);
});

test("a pool minion with no tier is left out, with a warning; the build goes on", () => {
  const warnings: string[] = [];
  const pool = extractPool([minion(), minion({ id: "BG_X_2", techLevel: undefined })], [], "t", (w) => warnings.push(w));
  assert.deepEqual(pool.minions.map((m) => m.id), ["BG_X_1"]);
  assert.equal(warnings.length, 1);
  assert.match(warnings[0], /BG_X_2 has no tavern tier: left out/);
});

test("a type the pages do not know keeps its minion, with a warning; the build goes on", () => {
  const warnings: string[] = [];
  const pool = extractPool([minion({ races: ["GNOLL"] }), minion({ id: "BG_X_2", races: ["GNOLL", "BEAST"] })], [], "t", (w) => warnings.push(w));
  assert.deepEqual(pool.minions.map((m) => m.tribes), [["GNOLL"], ["GNOLL", "BEAST"]]);
  assert.equal(warnings.length, 1, "one warning per new type");
  assert.match(warnings[0], /new minion type GNOLL \(2 minions\).*src\/lib\/minions\/tribes\.ts/);
  // No warning callback: the build still goes on.
  assert.equal(extractPool([minion({ techLevel: undefined }), minion({ id: "BG_X_2" })], [], "t").minions.length, 1);
});

test("data with no pool minion is not the card data", () => {
  assert.throws(() => extractPool([minion({ isBattlegroundsPoolMinion: false })], [], "t"), /no minion in the pool/);
  assert.throws(() => extractPool({}, [], "t"), /not a list/);
});

test("a malformed pool minion is left out, like the simulator's data", () => {
  const pool = extractPool([minion(), minion({ id: "BG_X_2", attack: -1 }), minion({ id: "bad id" })], [], "t");
  assert.deepEqual(pool.minions.map((m) => m.id), ["BG_X_1"]);
});

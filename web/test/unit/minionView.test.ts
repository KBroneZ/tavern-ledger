// What the minion pages show (T-578, D-134): one page per type with at least
// one minion of its own, the minions grouped by tier, a minion of two types
// on both pages and one of every type on all of them, and structured data
// that is valid JSON and cannot close its script tag.
import { afterEach, test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { setLang } from "../../src/lib/i18n.ts";
import type { PoolFile } from "../../src/lib/minions/poolFile.ts";
import { TRIBES } from "../../src/lib/minions/tribes.ts";
import {
  breadcrumbLd,
  itemListLd,
  jsonLd,
  marks,
  minionAnchor,
  otherTypes,
  readPool,
  tierGroups,
  tierSpread,
  tribeMinions,
  tribeName,
  tribePages,
  typeName,
} from "../../src/lib/minions/view.ts";
import { extractPool } from "../../tools/minionPool.ts";

const FIXTURES = join(import.meta.dirname, "..", "fixtures", "sim-cards");
const en = JSON.parse(readFileSync(join(FIXTURES, "enUS.json"), "utf8")) as unknown[];
const pool = extractPool(en, en, "fixture");

afterEach(() => setLang("en"));

test("a page for each type with a minion of its own, and one for minions with no type", () => {
  const slugs = tribePages(pool).map((p) => p.slug);
  assert.deepEqual(slugs, ["elemental", "mech", "murloc", "pirate", "quilboar", "undead", "no-tribe"]);
  assert.ok(!slugs.includes("beast"), "only the every-type minion would be there");
});

test("a minion of two types is on both pages; one of every type on every type's page", () => {
  const ids = (slug: string) => tribeMinions(pool, tribePages(pool).find((p) => p.slug === slug)!.key).map((m) => m.id);
  assert.ok(ids("murloc").includes("BG_SYN_MURPIR"));
  assert.ok(ids("pirate").includes("BG_SYN_MURPIR"));
  for (const slug of ["elemental", "mech", "murloc", "pirate", "quilboar", "undead"]) {
    assert.ok(ids(slug).includes("BG_SYN_AMALGAM"), slug);
  }
  assert.deepEqual(ids("no-tribe"), ["BG_NOT_A_CARD"]);
});

test("minions are grouped by tier, lowest first, by name within a tier", () => {
  const murlocs = tribeMinions(pool, "MURLOC");
  const groups = tierGroups(murlocs);
  assert.deepEqual(groups.map((g) => g.tier), [3, 4]);
  const many: PoolFile["minions"] = [
    { id: "B", name: "Zed", tier: 2, attack: 1, health: 1, tribes: [], text: null },
    { id: "A", name: "Ábaco", tier: 2, attack: 1, health: 1, tribes: [], text: null },
    { id: "C", name: "Bee", tier: 1, attack: 1, health: 1, tribes: [], text: null },
  ];
  setLang("es");
  assert.deepEqual(tierGroups(many).map((g) => [g.tier, g.minions.map((m) => m.name)]), [[1, ["Bee"]], [2, ["Ábaco", "Zed"]]]);
  assert.deepEqual(tierSpread(many), [1, 2, 0, 0, 0, 0, 0]);
});

test("a minion's marks: its other types as links, every type, Duos only", () => {
  const fin = pool.minions.find((m) => m.id === "BG_SYN_MURPIR")!;
  assert.deepEqual(marks(fin, "MURLOC"), [{ text: "Pirate", href: "/minions/pirate/" }]);
  setLang("es");
  assert.deepEqual(marks(fin, "PIRATE"), [{ text: "Múrloc", href: "/es/minions/murloc/" }]);
  setLang("en");
  const amalgam = pool.minions.find((m) => m.id === "BG_SYN_AMALGAM")!;
  assert.deepEqual(marks(amalgam, "MURLOC"), [{ text: "Every type" }, { text: "Duos only" }]);
  assert.deepEqual(marks(pool.minions.find((m) => m.id === "BG_NOT_A_CARD")!, null), []);
});

test("a type the pages do not know: no page, listed under other types, named from the card data", () => {
  // The fixture's own new type (the build test's) is set aside here.
  const known = pool.minions.filter((m) => !m.tribes.includes("SYNTHETIC_KIN"));
  const withNew: PoolFile = {
    ...pool,
    minions: [
      ...known,
      { id: "BG_GN_1", name: "Gnoll Pup", tier: 2, attack: 2, health: 2, tribes: ["GNOLL"], text: null },
      { id: "BG_GN_2", name: "Gnoll Fin", tier: 1, attack: 1, health: 1, tribes: ["GNOLL", "MURLOC"], text: null },
      { id: "BG_BB_1", name: "Boss", tier: 3, attack: 1, health: 1, tribes: ["BIG_BOSS"], text: null },
    ],
  };
  assert.ok(!tribePages(withNew).some((p) => p.key === "GNOLL"));
  assert.ok(tribeMinions(withNew, "MURLOC").some((m) => m.id === "BG_GN_2"), "still on its known type's page");
  const others = otherTypes(withNew);
  assert.deepEqual(others.map((o) => [o.key, o.name, o.minions.map((m) => m.id)]), [
    ["BIG_BOSS", "Big boss", ["BG_BB_1"]],
    ["GNOLL", "Gnoll", ["BG_GN_2", "BG_GN_1"]],
  ]);
  assert.deepEqual(otherTypes({ ...pool, minions: known }), [], "none among known types");
  assert.deepEqual(otherTypes(pool).map((o) => o.name), ["Synthetic kin"]);
  const fin = withNew.minions.find((m) => m.id === "BG_GN_2")!;
  assert.deepEqual(marks(fin, "MURLOC"), [{ text: "Gnoll" }], "named, not linked: it has no page");
  assert.equal(typeName(""), "Other");
  setLang("es");
  assert.equal(typeName(""), "Otro");
});

test("every type and the no-type page have names in both languages", () => {
  for (const lang of ["en", "es"]) {
    setLang(lang);
    for (const slug of [...TRIBES.map((t) => t.slug), "no-tribe"]) {
      for (const form of ["title", "one", "plural", "inline"] as const) assert.ok(tribeName(slug, form).trim(), `${lang} ${slug} ${form}`);
    }
  }
  setLang("es");
  assert.equal(tribeName("mech", "title"), "Robots");
  assert.equal(tribeName("murloc", "inline"), "múrloc");
});

test("anchors are stable and safe in an address", () => {
  assert.equal(minionAnchor("BG_SYN_MURPIR"), "m-BG_SYN_MURPIR");
});

test("structured data: a breadcrumb and an item list, valid JSON that cannot end its script", () => {
  const crumbs = breadcrumbLd([
    { name: "Home", url: "https://tavernledger.net/" },
    { name: "Minions", url: "https://tavernledger.net/minions/" },
  ]);
  assert.equal(crumbs["@type"], "BreadcrumbList");
  assert.deepEqual(crumbs.itemListElement[1], { "@type": "ListItem", position: 2, name: "Minions", item: "https://tavernledger.net/minions/" });
  const list = itemListLd("Murloc minions", [{ name: "</script><b>x", url: "https://tavernledger.net/minions/murloc/#m-X" }]);
  assert.equal(list["@type"], "ItemList");
  assert.equal(list.numberOfItems, 1);
  const text = jsonLd([crumbs, list]);
  assert.doesNotMatch(text, /</, "no < at all, so no </script>");
  const parsed = JSON.parse(text);
  assert.equal(parsed["@context"], "https://schema.org");
  assert.equal(parsed["@graph"][1].itemListElement[0].name, "</script><b>x");
});

test("the data file is read strictly", () => {
  assert.deepEqual(readPool({ version: 1, source: "x", minions: [] }).minions, []);
  assert.throws(() => readPool({ version: 2, source: "x", minions: [] }), /pool file/);
  assert.throws(() => readPool({ version: 1, source: "x" }), /pool file/);
  assert.throws(() => readPool(null), /pool file/);
});

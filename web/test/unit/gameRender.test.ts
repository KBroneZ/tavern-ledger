import { test } from "node:test";
import assert from "node:assert/strict";
import { readdirSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { renderGame } from "../../src/lib/game/render.ts";
import { readRecord } from "../../src/lib/game/report.ts";
import { shapeGame } from "../../src/lib/game/shape.ts";
import { GAME_STRINGS } from "../../src/lib/game/strings.ts";
import { combatsBacking, damageMark, damageShort, damageText, heroLabel, minionWords, pageHeading, shopLine } from "../../src/lib/game/words.ts";
import { useCardNames } from "../../src/lib/game/names.ts";
import { CardIndex } from "../../src/lib/sim/cards.ts";
import type { VNode } from "../../src/lib/view/node.ts";
import { duosReport, INDEX, record, SESSION, soloReport } from "./gameFixtures.ts";

const ROOT = join(import.meta.dirname, "..", "..");

function view(report: Record<string, unknown> = soloReport()) {
  const result = readRecord(record(report), SESSION, INDEX);
  assert.ok(result.ok);
  return shapeGame(result.record);
}

function* walk(node: VNode | string): Generator<VNode> {
  if (typeof node === "string") return;
  yield node;
  for (const child of node.children) yield* walk(child);
}

const attr = (n: VNode, name: string) => n.attrs.find(([k]) => k === name)?.[1];
const text = (n: VNode | string): string => (typeof n === "string" ? n : n.children.map(text).join(""));

test("words: hero names, headings, minions and unknown values", () => {
  assert.equal(heroLabel({ id: "SYN_HERO_A", name: "Lantern Keeper" }), "Lantern Keeper");
  assert.equal(heroLabel({ id: "SYN_HERO_D", name: null }), "Unknown hero", "never the bare card id (T-510)");
  assert.equal(heroLabel({ id: null, name: null }), "Unknown hero");
  const v = view();
  assert.equal(pageHeading(v.summary), "Lantern Keeper · 3rd");
  assert.equal(damageText(v.turns[0]), "dealt 3");
  assert.equal(damageText(v.turns[1]), "took 5");
  assert.equal(damageText(v.turns[2]), "took 13");
  assert.equal(damageShort(v.turns[0]), "+3");
  assert.equal(damageShort(v.turns[1]), "−5");
  assert.equal(shopLine(v.turns[0].shop!, v.turns[0]), "Tier 1 · Gold 3 · 3 spent · 1 roll");
  // A lost combat whose own health after it is unknown: the damage that matters is unknown, not 0.
  assert.equal(damageText({ ...v.turns[1], dealt: 0, taken: null }), "—");
  assert.equal(damageText({ ...v.turns[0], dealt: null, taken: 0 }), "—");
  assert.equal(damageText({ ...v.turns[1], result: "tie", dealt: 0, taken: 0 }), "0");
  assert.equal(minionWords(v.turns[1].ownBoard!.minions[1]), "SYN_M_02, 3/3, Golden, Taunt, Divine Shield");
  assert.equal(minionWords({ cardId: null, atk: null, health: 2, golden: false, keywords: [] }), "Unknown card, ?/2");
});

test("render: Simple has the key numbers, the turns and the replays; the rest is Advanced only (T-559)", () => {
  const nodes = renderGame(view(), "2026-10-09");
  const sections = nodes.filter((n) => n.tag === "section");
  const simple = sections.filter((n) => !attr(n, "class")?.split(" ").includes("adv")).map((n) => attr(n, "aria-labelledby"));
  const adv = sections.filter((n) => attr(n, "class")?.split(" ").includes("adv")).map((n) => attr(n, "aria-labelledby"));
  assert.deepEqual(simple, ["gv-turns-title", "gv-replays-title"], "one turn table: no separate boards or odds lists");
  assert.deepEqual(adv, ["gv-opponents-title", "gv-choices-title", "gv-lobby-title", "gv-notes-title"]);
  assert.equal(attr(nodes[0], "class"), "gv-summary");
  const tiles = [...walk(nodes[0])].filter((n) => attr(n, "class")?.split(" ").includes("tile"));
  const label = (n: VNode) => text(n.children[0]);
  assert.deepEqual(tiles.filter((n) => attr(n, "class") === "tile").map(label), ["Combats won–lost", "Damage dealt", "Damage taken", "Final tavern tier"]);
  assert.deepEqual(tiles.filter((n) => attr(n, "class") === "tile adv").map(label), ["Gold spent", "Triples", "Rolls", "Buys", "Sells", "Actions per minute"]);
  // The combats in one short figure; how many back each number only in Advanced.
  assert.match(text(tiles[0]), /^Combats won–lost1–1/);
  const backs = [...walk(nodes[0])].filter((n) => attr(n, "class")?.startsWith("tile-back"));
  assert.ok(backs.length > 0);
  for (const b of backs) assert.equal(attr(b, "class"), "tile-back adv");
});

test("render: each turn is one row whose toggle opens its boards, shop and odds (T-559, D-131)", () => {
  const nodes = renderGame(view(), "2026-10-09");
  const turns = nodes.find((n) => attr(n, "aria-labelledby") === "gv-turns-title")!;
  const all = [...walk(turns)];
  const rows = all.filter((n) => attr(n, "class") === "gv-turn");
  const details = all.filter((n) => attr(n, "class") === "gv-turn-more");
  assert.equal(rows.length, 3);
  assert.equal(details.length, 3);
  for (const [i, row] of rows.entries()) {
    const toggle = [...walk(row)].find((n) => attr(n, "data-toggle") !== undefined)!;
    assert.equal(toggle.tag, "button");
    assert.equal(attr(toggle, "aria-expanded"), "false");
    assert.equal(attr(toggle, "aria-controls"), attr(details[i], "id"));
    assert.equal(attr(details[i], "hidden"), "", "closed until opened");
  }
  // Every combat with a result waits for its odds; the unknown one has none.
  assert.deepEqual(all.filter((n) => attr(n, "data-odds") !== undefined).map((n) => attr(n, "data-odds")), ["1", "2"]);
  assert.deepEqual(all.filter((n) => attr(n, "data-why") !== undefined).map((n) => attr(n, "data-why")), ["1", "2", "3"]);
  // The detail holds the boards and the shop: theirs and yours, then the shop's line.
  const first = text(details[0]);
  assert.match(first, /Brass Captain's board/);
  assert.match(first, /Your board/);
  assert.match(first, /Shop Tier 1 · Gold 3 · 3 spent · 1 roll/);
  assert.match(first, /Watch this combat$/, "a combat with a replay opens it from its row");
});

test("render: every picture has its size, a text alternative and a local fallback", () => {
  const nodes = renderGame(view(), "2026-10-09");
  const imgs = nodes.flatMap((n) => [...walk(n)]).filter((n) => n.tag === "img");
  assert.ok(imgs.length > 20);
  for (const img of imgs) {
    assert.match(attr(img, "src") ?? "", /^https:\/\/art\.hearthstonejson\.com\/v1\/256x\/[A-Za-z0-9_]+\.jpg$/);
    assert.match(attr(img, "data-fallback") ?? "", /^\/placeholders\/(hero|card)-\d\.svg$/);
    assert.match(attr(img, "width") ?? "", /^\d+$/);
    assert.match(attr(img, "height") ?? "", /^\d+$/);
    assert.notEqual(attr(img, "alt"), undefined);
  }
  // A minion whose card id the record lacks is a plain circle with words, not a broken picture.
  const missing = nodes.flatMap((n) => [...walk(n)]).filter((n) => attr(n, "class")?.includes("pic-missing"));
  assert.ok(missing.length >= 1);
});

test("render: a record with no combats shows dashes, not 0 won", () => {
  const rep = soloReport() as Record<string, any>;
  rep.rounds = [];
  const all = renderGame(view(rep), null).map(text).join("\n");
  assert.doesNotMatch(all, /0 won/);
  assert.match(all, /Rounds—/);
});

test("render: inferred results are marked and unknown values are dashes, never 0", () => {
  const nodes = renderGame(view(), "2026-10-09");
  const turns = nodes.find((n) => attr(n, "aria-labelledby") === "gv-turns-title")!;
  const rows = [...walk(turns)].filter((n) => attr(n, "class") === "gv-turn").map(text);
  assert.match(rows[0], /^1Turn 1: boards, shop and odds1WonBrass Captain.*dealt 3\+3/);
  assert.match(rows[1], /^2.*2 ▲Lost inferredMoss Witch.*took 5−5—/);
  assert.match(rows[2], /Unknown/);
});

test("render: Duos shows the teammate and the lobby's teams", () => {
  const nodes = renderGame(view(duosReport()), "2026-10-09");
  const all = nodes.map(text).join("\n");
  assert.match(all, /TeammateBrass Captain/);
  assert.match(all, /Team/);
  assert.match(all, /No shop record for this turn/);
  assert.match(all, /Unknown hero's board 1 minion/);
  assert.match(all, /Unknown hero's board 2 minions/);
  assert.doesNotMatch(all, /SYN_HERO_/, "no card id as a hero name");
  assert.doesNotMatch(all, /Not in the record/);
});

test("render: no player name, BattleTag or markup from the record reaches the page", () => {
  const rep = soloReport() as Record<string, any>;
  rep.lobby[1].name = "SomePlayer";
  rep.card_names.SYN_HERO_C = "Evil#1234";
  rep.warnings = ["<img src=x onerror=alert(1)>", "fine note"];
  const nodes = renderGame(view(rep), "2026-10-09");
  const all = nodes.map(text).join("\n");
  assert.doesNotMatch(all, /SomePlayer|Evil#1234|<img/);
  assert.match(all, /fine note/);
  for (const n of nodes.flatMap((x) => [...walk(x)])) {
    for (const [name] of n.attrs) assert.doesNotMatch(name, /^on/i);
  }
});

test("game strings: every key the viewer names exists", () => {
  const dir = join(ROOT, "src", "lib", "game");
  const sources = readdirSync(dir).map((f) => readFileSync(join(dir, f), "utf8"));
  sources.push(readFileSync(join(ROOT, "src", "components", "game", "GameViewer.astro"), "utf8"));
  const keys = sources.flatMap((s) => [...s.matchAll(/\bg\("([a-zA-Z0-9.]+)"/g)].map((m) => m[1]));
  assert.ok(keys.length > 40);
  for (const key of keys) assert.ok(key in GAME_STRINGS, key);
});

test("render: a gifted minion shows a mark per gift with its name and summary, in English and Spanish (T-528)", async () => {
  const { setLang } = await import("../../src/lib/i18n.ts");
  const report = soloReport() as Record<string, any>;
  const board = report.rounds[1].entries[0].board;
  board[0].gifts = ["BG36_MidGameEffect_000t9", "BG99_Unknown_1"];
  board[1].gift_unknown = true;
  // The turn table's boards (the replay section draws its own: gameReplayView.test.ts).
  const gifts = (nodes: VNode[]) =>
    nodes
      .filter((n) => attr(n, "aria-labelledby") === "gv-turns-title")
      .flatMap((n) => [...walk(n)])
      .filter((n) => attr(n, "class")?.split(" ").includes("gv-gift"));
  const en = gifts(renderGame(view(report), "2026-10-09"));
  assert.ok(en.length >= 3);
  assert.equal(attr(en[0], "title"), "Dark Gift: Admiration: Start of Combat: gain the Attack of the minion to its left.");
  assert.equal(attr(en[1], "title"), "Dark Gift: BG99_Unknown_1", "a gift the table does not know: its id");
  assert.ok(en.some((n) => attr(n, "class")?.includes("is-unknown")));
  setLang("es");
  try {
    const es = gifts(renderGame(view(report), "2026-10-09"));
    assert.equal(attr(es[0], "title"), "Don siniestro: Admiración: Inicio del combate: gana el ataque del esbirro a su izquierda.");
  } finally {
    setLang("en");
  }
});

test("render: only worked-out damage is marked, with one short legend (T-559)", () => {
  const nodes = renderGame(view(), "2026-10-09");
  const all = [...nodes].flatMap((n) => [...walk(n)]);
  const marks = (kind: string) => all.filter((n) => attr(n, "class") === `mark-src mark-${kind}`);
  assert.equal(marks("log").length, 0, "the game's own hit needs no mark");
  const inferred = marks("inferred");
  const marked = all.filter((n) => n.tag === "td" && attr(n, "class") === "num" && n.children.some((c) => typeof c !== "string" && inferred.includes(c)));
  // Turn 3 has only health; turns 1 and 2 have the hit in their replay.
  assert.deepEqual(marked.map(text), ["took 13−13 inferred"]);
  const tiles = all.filter((n) => attr(n, "class") === "tile");
  assert.ok(!inferred.some((m) => [...walk(tiles[1])].includes(m)), "dealt: every combat had the hit");
  assert.ok(inferred.some((m) => [...walk(tiles[2])].includes(m)), "taken: one combat was worked out");
  assert.equal(all.filter((n) => n.tag === "p" && text(n).includes("Worked out from the health")).length, 1);
});

test("render: cards go by their name from the card data, else by their id (T-559)", () => {
  const cards = new CardIndex({ version: 1, source: "test", cards: [{ i: "SYN_M_02", n: "Lantern Moth", t: 1, a: 3, h: 3, r: [], k: [] }] });
  useCardNames({ cards, names: new Map(), trinkets: new Map() });
  try {
    const v = view();
    assert.equal(minionWords(v.turns[1].ownBoard!.minions[1]), "Lantern Moth, 3/3, Golden, Taunt, Divine Shield");
    assert.match(renderGame(v, "2026-10-09").map(text).join("\n"), /Lantern Moth/);
  } finally {
    useCardNames(null);
  }
  assert.equal(minionWords(view().turns[1].ownBoard!.minions[1]), "SYN_M_02, 3/3, Golden, Taunt, Divine Shield");
});

test("words: a damage total is marked inferred when any combat behind it was worked out", () => {
  const known = (value: number, n: number, logged: number) => ({ value, known: n, total: 3, logged });
  assert.equal(damageMark(known(8, 3, 3)), undefined);
  assert.equal(damageMark(known(8, 3, 2)), "inferred");
  assert.equal(damageMark(known(8, 3, 0)), "inferred");
  assert.equal(damageMark({ value: null, known: 0, total: 3, logged: 0 }), undefined, "nothing known: the value is a dash");
  assert.equal(combatsBacking(known(8, 3, 3)), "over 3 of 3 combats");
  assert.equal(combatsBacking(known(8, 3, 2)), "over 3 of 3 combats · 2 from the log");
});

import { test } from "node:test";
import assert from "node:assert/strict";
import { readdirSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { renderGame } from "../../src/lib/game/render.ts";
import { readRecord } from "../../src/lib/game/report.ts";
import { shapeGame } from "../../src/lib/game/shape.ts";
import { GAME_STRINGS } from "../../src/lib/game/strings.ts";
import { damageText, goldText, heroLabel, minionWords, pageHeading } from "../../src/lib/game/words.ts";
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
  assert.equal(goldText(v.turns[0]), "3 of 3 spent");
  assert.equal(goldText(v.turns[1]), "—", "a turn without gold is unknown, never 0");
  // A lost combat whose own health after it is unknown: the damage that matters is unknown, not 0.
  assert.equal(damageText({ ...v.turns[1], dealt: 0, taken: null }), "—");
  assert.equal(damageText({ ...v.turns[0], dealt: null, taken: 0 }), "—");
  assert.equal(damageText({ ...v.turns[1], result: "tie", dealt: 0, taken: 0 }), "0");
  assert.equal(minionWords(v.turns[1].ownBoard!.minions[1]), "SYN_M_02, 3/3, Golden, Taunt, Divine Shield");
  assert.equal(minionWords({ cardId: null, atk: null, health: 2, golden: false, keywords: [] }), "Unknown card, ?/2");
});

test("render: Simple has the summary and the turns; the rest is Advanced only", () => {
  const nodes = renderGame(view(), "2026-10-09");
  const sections = nodes.filter((n) => n.tag === "section");
  const simple = sections.filter((n) => !attr(n, "class")?.split(" ").includes("adv")).map((n) => attr(n, "aria-labelledby"));
  const adv = sections.filter((n) => attr(n, "class")?.split(" ").includes("adv")).map((n) => attr(n, "aria-labelledby"));
  assert.deepEqual(simple, ["gv-turns-title"]);
  assert.deepEqual(adv, ["gv-boards-title", "gv-opponents-title", "gv-choices-title", "gv-lobby-title", "gv-notes-title"]);
  assert.equal(attr(nodes[0], "class"), "gv-summary");
  const advTiles = [...walk(nodes[0])].filter((n) => attr(n, "class") === "tile adv").map((n) => text(n.children[0]));
  assert.deepEqual(advTiles, ["Rolls", "Buys", "Sells", "Actions per minute"]);
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
  const rows = [...walk(turns)].filter((n) => n.tag === "tr").slice(1).map(text);
  assert.match(rows[0], /^1.*Won.*Brass Captain.*dealt 3.*3 of 3 spent$/);
  assert.match(rows[1], /^22 tier up.*Lost inferred.*Moss Witch.*took 5—$/);
  assert.match(rows[2], /Unknown/);
});

test("render: Duos shows the teammate and the lobby's teams", () => {
  const nodes = renderGame(view(duosReport()), "2026-10-09");
  const all = nodes.map(text).join("\n");
  assert.match(all, /TeammateBrass Captain/);
  assert.match(all, /Team/);
  assert.match(all, /This game has no shop record/);
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
  const gifts = (nodes: VNode[]) =>
    nodes.flatMap((n) => [...walk(n)]).filter((n) => attr(n, "class")?.split(" ").includes("gv-gift"));
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

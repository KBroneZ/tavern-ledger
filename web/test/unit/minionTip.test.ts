// The tooltip of a minion (T-619, D-142): its exact Attack and Health and the
// buffs it has, each card with its picture and how many times. Empty is not
// unknown: a game saved before the buffs were read says so.
import { test } from "node:test";
import assert from "node:assert/strict";
import { setLang } from "../../src/lib/i18n.ts";
import { minionTip, tipHost, watchTipEscape } from "../../src/lib/game/minionTip.ts";
import { useCardNames } from "../../src/lib/game/names.ts";
import { CardIndex } from "../../src/lib/sim/cards.ts";
import type { VNode } from "../../src/lib/view/node.ts";

function* walk(node: VNode | string): Generator<VNode> {
  if (typeof node === "string") return;
  yield node;
  for (const child of node.children) yield* walk(child);
}

const attr = (n: VNode, name: string) => n.attrs.find(([k]) => k === name)?.[1];
const text = (n: VNode | string): string => (typeof n === "string" ? n : n.children.map(text).join(""));

function names(): void {
  useCardNames({
    cards: new CardIndex({ version: 1, source: "test", cards: [] }),
    names: new Map([["BG20_GEM", "Blood Gem"], ["BG31_816", "Fire Baller"]]),
    trinkets: new Map(),
  });
}

const BIG = { atk: 2_336, health: 1_263 };

test("the tooltip says the exact numbers and each buff once with its count, picture and name", () => {
  setLang("en");
  names();
  const tip = minionTip("tip-1", { ...BIG, buffs: [{ cardId: "BG20_GEM", count: 3 }, { cardId: "BG31_816", count: 1 }] });
  assert.equal(attr(tip, "id"), "tip-1");
  assert.equal(attr(tip, "role"), "tooltip");
  assert.match(text(tip), /Attack 2,336/);
  assert.match(text(tip), /Health 1,263/);
  const rows = [...walk(tip)].filter((n) => n.tag === "li");
  assert.equal(rows.length, 2);
  assert.match(text(rows[0]), /Blood Gem ×3/);
  assert.match(text(rows[1]), /Fire Baller ×1/);
  const imgs = [...walk(tip)].filter((n) => n.tag === "img");
  assert.equal(imgs.length, 2, "a picture per buff");
  assert.match(attr(imgs[0], "src") ?? "", /BG20_GEM/);
});

test("a buff whose card the data does not know shows its id", () => {
  setLang("en");
  names();
  const tip = minionTip("tip-2", { ...BIG, buffs: [{ cardId: "BG_ShopBuff_Elemental", count: 2 }] });
  assert.match(text(tip), /BG_ShopBuff_Elemental ×2/);
});

test("no buffs is said; a record that did not read them says so; neither shows an empty list", () => {
  setLang("en");
  names();
  const none = minionTip("tip-3", { ...BIG, buffs: [] });
  assert.match(text(none), /No buffs/);
  assert.equal([...walk(none)].some((n) => n.tag === "li"), false);
  const unknown = minionTip("tip-4", { ...BIG, buffs: null });
  assert.match(text(unknown), /not recorded for this game/);
  assert.doesNotMatch(text(unknown), /No buffs/);
});

test("an unknown stat is a question mark, never 0", () => {
  setLang("en");
  names();
  const tip = minionTip("tip-5", { atk: null, health: 4, buffs: [] });
  assert.match(text(tip), /Attack \?/);
  assert.match(text(tip), /Health 4/);
});

test("Spanish: the words, the grouping and the decimal point of the language", () => {
  setLang("es");
  names();
  const tip = minionTip("tip-6", { atk: 1_234_567, health: 8, buffs: [{ cardId: "BG20_GEM", count: 2 }] });
  assert.match(text(tip), /Ataque 1\.234\.567/);
  assert.match(text(tip), /Vida 8/);
  assert.match(text(tip), /Mejoras/);
  assert.match(text(tip), /Blood Gem ×2/);
  const unknown = minionTip("tip-7", { atk: 1, health: 1, buffs: null });
  assert.match(text(unknown), /no se guardaron/);
  setLang("en");
});

test("the host of a tooltip can be focused and points to it", () => {
  assert.deepEqual(tipHost("tip-1"), { tabindex: 0, "aria-describedby": "tip-1" });
});

// ---------- on the pages ----------

import { boardBlock } from "../../src/lib/game/boards.ts";
import { tileView } from "../../src/lib/game/replayView.ts";
import type { Tile } from "../../src/lib/game/replay.ts";
import type { Minion } from "../../src/lib/game/report.ts";

const minion = (over: Partial<Minion> = {}): Minion => ({
  cardId: "BG20_100",
  atk: 2_336,
  health: 1_263,
  golden: false,
  keywords: [],
  gifts: [],
  giftUnknown: false,
  buffs: [{ cardId: "BG20_GEM", count: 3 }],
  ...over,
});

test("a board's minion shows the short numbers, and its tooltip the exact ones and its buffs", () => {
  setLang("en");
  names();
  const board = boardBlock("gv-b1", "Your board", null, { minions: [minion(), minion({ atk: 12, health: 5, buffs: null })], attack: 2_348, health: 1_268 });
  const tiles = [...walk(board)].filter((n) => (attr(n, "class") ?? "").split(" ").includes("gv-minion"));
  assert.equal(tiles.length, 2);
  const first = tiles[0];
  assert.equal(attr(first, "tabindex"), "0");
  assert.equal(attr(first, "title"), undefined, "the tooltip is not a browser title");
  const tip = [...walk(first)].find((n) => attr(n, "role") === "tooltip") as VNode;
  assert.equal(attr(first, "aria-describedby"), attr(tip, "id"));
  const shown = [...walk(first)].filter((n) => ["rp-atk", "rp-hp"].includes(attr(n, "class") ?? "")).map(text);
  assert.deepEqual(shown, ["2.3K", "1.2K"]);
  assert.match(text(tip), /Attack 2,336/);
  assert.match(text(tip), /Blood Gem ×3/);
  const second = [...walk(tiles[1])].find((n) => attr(n, "role") === "tooltip") as VNode;
  assert.match(text(second), /not recorded for this game/);
  assert.notEqual(attr(second, "id"), attr(tip, "id"), "each tooltip has its own id");
});

test("a replay tile shows short numbers and its tooltip the exact ones and the buffs it began with", () => {
  setLang("en");
  names();
  const tile: Tile = {
    ref: 3,
    cardId: "BG20_100",
    atk: 12_345,
    health: 999,
    golden: false,
    shield: false,
    keywords: [],
    gifts: [],
    giftUnknown: false,
    buffs: [{ cardId: "BG31_816", count: 2 }],
    marks: {},
  };
  const li = tileView(tile);
  assert.equal(attr(li, "aria-describedby"), "rp-tip-3");
  const tip = [...walk(li)].find((n) => attr(n, "id") === "rp-tip-3") as VNode;
  assert.match(text(tip), /Attack 12,345/);
  assert.match(text(tip), /Fire Baller ×2/);
  const shown = [...walk(li)].filter((n) => ["rp-atk", "rp-hp"].includes(attr(n, "class") ?? "")).map(text);
  assert.deepEqual(shown, ["12.3K", "999"]);
});

// ---------- what the plaque says it is ----------

test("the plaque names the minion, so no browser title is needed", () => {
  setLang("en");
  names();
  const tip = minionTip("tip-n", { cardId: "BG31_816", ...BIG, buffs: [] });
  const head = [...walk(tip)].find((n) => (attr(n, "class") ?? "") === "tip-name") as VNode;
  assert.equal(text(head), "Fire Baller");
  const unknown = minionTip("tip-m", { cardId: null, ...BIG, buffs: [] });
  assert.equal(text([...walk(unknown)].find((n) => (attr(n, "class") ?? "") === "tip-name") as VNode), "Unknown card");
});

test("the buff list says when it was taken: at the start of a combat, or in the loaded game", () => {
  setLang("en");
  names();
  const buffs = [{ cardId: "BG20_GEM", count: 1 }];
  const head = (tip: VNode) => text([...walk(tip)].find((n) => (attr(n, "class") ?? "") === "tip-head") as VNode);
  assert.equal(head(minionTip("a", { ...BIG, buffs })), "Buffs");
  assert.equal(head(minionTip("b", { ...BIG, buffs }, "start")), "Buffs when the combat began");
  assert.equal(head(minionTip("c", { ...BIG, buffs }, "loaded")), "Buffs in the loaded game");
  setLang("es");
  assert.equal(head(minionTip("d", { ...BIG, buffs }, "start")), "Mejoras al empezar el combate");
  assert.equal(head(minionTip("e", { ...BIG, buffs }, "loaded")), "Mejoras en la partida cargada");
  setLang("en");
});

test("Escape hides the plaques until the pointer or the focus moves again", () => {
  const root = { attrs: new Map<string, string>(), setAttribute(k: string, v: string) { this.attrs.set(k, v); }, removeAttribute(k: string) { this.attrs.delete(k); } };
  const listeners = new Map<string, (e: { key?: string }) => void>();
  const doc = { documentElement: root, addEventListener: (type: string, fn: (e: { key?: string }) => void) => listeners.set(type, fn) };
  watchTipEscape(doc as never);
  listeners.get("keydown")?.({ key: "a" });
  assert.equal(root.attrs.has("data-tips-off"), false, "another key does nothing");
  listeners.get("keydown")?.({ key: "Escape" });
  assert.equal(root.attrs.get("data-tips-off"), "");
  listeners.get("pointermove")?.({});
  assert.equal(root.attrs.has("data-tips-off"), false, "the pointer moving brings them back");
  listeners.get("keydown")?.({ key: "Escape" });
  listeners.get("focusin")?.({});
  assert.equal(root.attrs.has("data-tips-off"), false, "so does the focus moving");
});

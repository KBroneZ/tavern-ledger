// The simulator's combat table (T-526, D-112): minions drawn as cards with
// their Attack, Health and Dark Gifts (D-107), seven card places per board,
// the opponent across the table from the player, Dark Gifts edited on the
// picked minion, and the steps of the main flow numbered.
import { afterEach, test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { setLang } from "../../src/lib/i18n.ts";
import * as B from "../../src/lib/sim/board.ts";
import { CardIndex } from "../../src/lib/sim/cards.ts";
import { loadView } from "../../src/lib/sim/loadView.ts";
import { editorView, NO_MARKS, sideView } from "../../src/lib/sim/view.ts";
import { extractCards } from "../../tools/simCards.ts";
import { elements, textOf, toHtml } from "../helpers/html.ts";
import type { VNode } from "../../src/lib/view/node.ts";

const FIXTURES = join(import.meta.dirname, "..", "fixtures", "sim-cards");
const en = JSON.parse(readFileSync(join(FIXTURES, "enUS.json"), "utf8")) as unknown[];
const es = JSON.parse(readFileSync(join(FIXTURES, "esES.json"), "utf8")) as unknown[];
const cards = new CardIndex(extractCards(en, en, "fixture"));
const GIFTS = JSON.parse(readFileSync(join(import.meta.dirname, "..", "..", "src", "data", "gifts.json"), "utf8")).gifts as Record<string, unknown>;
const SACRIFICE = "BG36_MidGameEffect_000t";

afterEach(() => setLang("en"));

const html = (nodes: VNode[]) => nodes.map(toHtml).join("");
const count = (out: string, re: RegExp) => out.match(re)?.length ?? 0;
const view = (state: B.SimState, extra: Partial<Parameters<typeof editorView>[0]> = {}) =>
  editorView({ state, cards, selected: null, adding: null, marks: NO_MARKS, ...extra });

test("a minion is a card: its art, Attack and Health, and a star per Dark Gift with its words", () => {
  let s = B.addMinion(B.emptyState(), "player", 0, "BGS_119", cards);
  const key = s.player[0].minions[0].key;
  s = B.addGift(s, "player", 0, key, SACRIFICE);
  const out = html(sideView("player", { state: s, cards, selected: null, adding: null, marks: NO_MARKS }));
  assert.match(out, /class="sim-card[ "]/);
  assert.match(out, /class="sim-atk"[^>]*>2</);
  assert.match(out, /class="sim-hp"[^>]*>1</);
  // One star, its name and summary as the tooltip; the card's words name it too.
  assert.equal(count(out, /class="sim-gift[ "]/g), 1);
  assert.match(out, /title="Dark Gift: Offensive Sacrifice: \+10 Attack/);
  assert.match(out, /aria-label="Synthetic Spark, 2\/1, Divine Shield, Windfury, Dark Gift: Offensive Sacrifice"/);
  // The art is decorative: the button carries the words.
  const img = elements(sideView("player", { state: s, cards, selected: null, adding: null, marks: NO_MARKS })[0]).find((n) => n.tag === "img");
  assert.ok(img?.attrs.some(([k, v]) => k === "alt" && v === ""));
});

test("each board shows seven card places: its minions, the one that adds, then empty places", () => {
  const empty = html(sideView("player", { state: B.emptyState(), cards, selected: null, adding: null, marks: NO_MARKS }));
  assert.equal(count(empty, /class="sim-slot[ "]/g), 7);
  assert.equal(count(empty, /data-act="add-open"/g), 1);
  assert.equal(count(empty, /class="sim-slot is-empty" aria-hidden="true"/g), 6);
  let s = B.emptyState();
  for (let i = 0; i < 3; i++) s = B.addMinion(s, "player", 0, "BGS_119", cards);
  const three = html(sideView("player", { state: s, cards, selected: null, adding: null, marks: NO_MARKS }));
  assert.equal(count(three, /class="sim-slot[ "]/g), 7);
  assert.equal(count(three, /data-act="select"/g), 3);
  assert.equal(count(three, /is-empty/g), 3);
  for (let i = 0; i < 4; i++) s = B.addMinion(s, "player", 0, "BGS_119", cards);
  const full = html(sideView("player", { state: s, cards, selected: null, adding: null, marks: NO_MARKS }));
  assert.equal(count(full, /class="sim-slot[ "]/g), 7);
  assert.equal(count(full, /data-act="add-open"/g), 0);
});

test("the table puts the opponent across from you: theirs first, as the game draws it", () => {
  const out = textOf({ tag: "div", attrs: [], children: view(B.emptyState()), svg: false } as VNode);
  assert.ok(out.indexOf("Opponent") >= 0 && out.indexOf("Opponent") < out.indexOf("You"));
});

test("the picked minion's Dark Gifts: each named with a way to remove it, a list to add one, none past four", () => {
  let s = B.addMinion(B.emptyState(), "player", 0, "BGS_119", cards);
  const key = s.player[0].minions[0].key;
  s = B.addGift(s, "player", 0, key, SACRIFICE);
  const selected = { side: "player" as const, board: 0, key };
  const out = html(view(s, { selected }));
  assert.match(out, /data-act="gift-remove"[^>]*data-gift="BG36_MidGameEffect_000t"|data-gift="BG36_MidGameEffect_000t"[^>]*data-act="gift-remove"/);
  assert.match(out, /Remove Offensive Sacrifice/);
  // A list of every gift the table knows, but not the ones it already has.
  assert.match(out, /<select[^>]*data-field="gift"/);
  // Browsing the list adds nothing (a closed list changes on each arrow key): a button adds the one chosen.
  assert.match(out, /data-act="gift-add"/);
  assert.equal(count(out, /<option value="BG36_/g), Object.keys(GIFTS).length - 1);
  assert.doesNotMatch(out, /<option value="BG36_MidGameEffect_000t">/);
  for (const g of Object.keys(GIFTS).filter((id) => id !== SACRIFICE).slice(0, 3)) s = B.addGift(s, "player", 0, key, g);
  const full = html(view(s, { selected }));
  assert.equal(count(full, /data-act="gift-remove"/g), 4);
  assert.doesNotMatch(full, /data-field="gift"/);
  assert.match(full, /A minion holds 4 Dark Gifts/);
});

test("the main flow is numbered: pick a game, then pick a fight", () => {
  const row = { session: "Hearthstone_2026_10_09_20_15_03", gameIndex: 2, mode: "solo" as const, status: "ok" as const, playedOn: "2026-10-09", hero: "SYN_HERO_A", place: 3, rounds: 3, combatsWon: 1, combatsLost: 1, combatsInferred: 0, triples: 1, apm: 3, goldSpent: 9 };
  const out = html(loadView({ kind: "ready", rows: [row], total: 1, more: false }, { key: "Hearthstone_2026_10_09_20_15_03#2", rounds: [{ number: 1, result: "won" }], failed: false, loaded: null }));
  assert.match(out, /<span class="sim-step" aria-hidden="true">2<\/span>/);
  assert.match(out, /Pick a fight/);
});

test("the Spanish table: card places, gifts and steps in Spanish", () => {
  setLang("es");
  const spanish = new CardIndex(extractCards(en, es, "fixture"));
  let s = B.addMinion(B.emptyState(), "player", 0, "BGS_119", spanish);
  const key = s.player[0].minions[0].key;
  s = B.addGift(s, "player", 0, key, SACRIFICE);
  const out = html(editorView({ state: s, cards: spanish, selected: { side: "player", board: 0, key }, adding: null, marks: NO_MARKS }));
  assert.match(out, /Don siniestro: Sacrificio ofensivo/);
  assert.match(out, /Quitar Sacrificio ofensivo/);
  assert.match(out, /Añadir un don siniestro/);
});

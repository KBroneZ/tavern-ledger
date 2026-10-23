// The simulator's hero powers and trinkets (T-628, round 7): the edits, the
// engine's request, a saved round's kits (parser revision 16), the editor's
// lists and controls, and the built engine playing them. Only synthetic
// cards and games (D-038, D-044); the hero powers' ids are real (the engine
// keys its rules on them).
import { afterEach, test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { extractCards, extractNames } from "../../tools/simCards.ts";
import { CardIndex } from "../../src/lib/sim/cards.ts";
import * as B from "../../src/lib/sim/board.ts";
import { needs } from "../../src/lib/sim/kit.ts";
import { resultView } from "../../src/lib/sim/resultView.ts";
import { kitView, socSwitch } from "../../src/lib/sim/kitView.ts";
import { editorView, NO_MARKS } from "../../src/lib/sim/view.ts";
import { readRecord } from "../../src/lib/game/report.ts";
import { readNameFile, useCardNames } from "../../src/lib/game/names.ts";
import { loadEngine } from "../../src/lib/sim/engine.ts";
import { readAnswer } from "../../src/lib/sim/answer.ts";
import { setLang } from "../../src/lib/i18n.ts";
import { WASM_PATH } from "../../tools/simStamp.ts";
import { elements, textOf, toHtml } from "../helpers/html.ts";
import type { VNode } from "../../src/lib/view/node.ts";
import { INDEX, record, SESSION, soloReport } from "./gameFixtures.ts";

const FIXTURES = join(import.meta.dirname, "..", "fixtures", "sim-cards");
const en = JSON.parse(readFileSync(join(FIXTURES, "enUS.json"), "utf8")) as unknown[];
const es = JSON.parse(readFileSync(join(FIXTURES, "esES.json"), "utf8")) as unknown[];
const cards = new CardIndex(extractCards(en, en, "fixture"));
const names = readNameFile(extractNames(en, en, "fixture"));
const lists = names.kit ?? { power: [], lesser: [], greater: [] };

/** Fire Invocation (start of combat: the left-most minion's Attack doubles) and Lock and Load. */
const FIRE = "BG22_HERO_001p_t2";
const LOCK = "BG22_HERO_000p_Alt";
const FROSTWOLF = "BG22_HERO_002p";
const RAPID = "BG25_HERO_103p";

afterEach(() => {
  useCardNames(null);
  setLang("en");
});

const html = (nodes: VNode[]) => nodes.map(toHtml).join("");

/** The player's 2/3 against a 3/3, built by hand. */
function duel(): B.SimState {
  let s = B.addMinion(B.emptyState(), "player", 0, "BGS_119", cards);
  s = B.setStats(s, "player", 0, s.player[0].minions[0].key, 2, 3);
  s = B.addMinion(s, "opponent", 0, "BGS_119", cards);
  return B.setStats(s, "opponent", 0, s.opponent[0].minions[0].key, 3, 3);
}

test("boards built by hand play start of combat and have no hero power or trinket yet", () => {
  const s = B.emptyState();
  assert.equal(s.startOfCombat, true);
  assert.equal(s.round, null);
  assert.deepEqual(s.player[0].kit, { powers: [], trinkets: [] });
  const req = JSON.parse(B.toRequest(duel(), cards));
  assert.equal(req.start_of_combat, true);
  assert.equal("kit" in req.player[0], false, "an empty kit is not sent");
  assert.equal("round" in req, false, "no turn known");
});

test("a hero power or trinket goes in its slot, replaces the one there, and leaves with None", () => {
  let s = B.setKitCard(duel(), "player", 0, "power", 0, FIRE);
  s = B.setKitCard(s, "player", 0, "trinket", 2, "BG_TRINKET2_SYN");
  s = B.setKitCard(s, "player", 0, "trinket", 1, "BG_TRINKET_SYN");
  assert.deepEqual(s.player[0].kit?.trinkets.map((c) => [c.slot, c.cardId]), [[1, "BG_TRINKET_SYN"], [2, "BG_TRINKET2_SYN"]], "in slot order");
  assert.equal(B.setKitCard(s, "player", 0, "power", 0, FIRE), s, "the same card again changes nothing");
  assert.equal(B.setKitCard(s, "player", 0, "power", 4, FIRE), s, "a slot the editor does not offer");
  assert.equal(B.setKitCard(s, "player", 0, "trinket", 0, "BG_TRINKET_SYN"), s, "trinkets start at slot 1");
  s = B.setKitCard(s, "player", 0, "power", 0, RAPID);
  assert.deepEqual(s.player[0].kit?.powers.map((c) => c.cardId), [RAPID]);
  s = B.setKitCard(s, "player", 0, "trinket", 1, null);
  assert.deepEqual(s.player[0].kit?.trinkets.map((c) => c.slot), [2]);
  const req = JSON.parse(B.toRequest(s, cards));
  assert.deepEqual(req.player[0].kit, { powers: [{ card_id: RAPID, slot: 0 }], trinkets: [{ card_id: "BG_TRINKET2_SYN", slot: 2 }] });
});

test("a chosen minion is sent by its board position and forgotten when it leaves", () => {
  let s = B.addMinion(duel(), "player", 0, "BG28_300", cards);
  s = B.setKitCard(s, "player", 0, "power", 0, RAPID);
  assert.deepEqual(needs(RAPID), ["target"]);
  const second = s.player[0].minions[1].key;
  assert.equal(B.setKitTarget(s, "player", 0, 0, 99_999), s, "a minion not on the board is refused");
  s = B.setKitTarget(s, "player", 0, 0, second);
  assert.equal(JSON.parse(B.toRequest(s, cards)).player[0].kit.powers[0].target, 2);
  // Moved left, it is still the one chosen.
  s = B.moveMinion(s, "player", 0, second, -1);
  assert.equal(JSON.parse(B.toRequest(s, cards)).player[0].kit.powers[0].target, 1);
  s = B.removeMinion(s, "player", 0, second);
  assert.equal(s.player[0].kit?.powers[0].target, null);
  assert.equal("target" in JSON.parse(B.toRequest(s, cards)).player[0].kit.powers[0], false);
});

test("a card id that names an object's own property needs nothing and loads", () => {
  for (const id of ["constructor", "toString", "__proto__", "hasOwnProperty"]) assert.deepEqual(needs(id), [], id);
  const s = B.setKitCard(duel(), "player", 0, "power", 0, "constructor");
  assert.ok(html([kitView("player", 0, { state: s, cards, lists, marks: NO_MARKS, holding: null })]).includes("constructor"));
});

test("a held minion sets the power's first number, and its stats are clamped", () => {
  let s = B.setKitCard(duel(), "player", 0, "power", 0, LOCK);
  assert.deepEqual(s.player[0].kit?.powers[0].counters, [0, 0, 0, 0, 0], "holding nothing, it does not act");
  assert.equal(B.setHeld(s, "player", 0, 0, "NOT_A_CARD", cards), s);
  s = B.setHeld(s, "player", 0, 0, "BGS_119", cards);
  assert.deepEqual(s.player[0].kit?.powers[0].counters, [1, 0, 0, 0, 0]);
  s = B.setHeldStats(s, "player", 0, 0, 5, 0);
  assert.deepEqual([s.player[0].kit?.powers[0].held?.attack, s.player[0].kit?.powers[0].held?.health], [5, 1]);
  const held = JSON.parse(B.toRequest(s, cards)).player[0].kit.powers[0].held;
  assert.deepEqual([held.card_id, held.attack, held.health], ["BGS_119", 5, 1]);
  assert.ok(held.facts, "with its card facts");
  s = B.setHeld(s, "player", 0, 0, null, cards);
  assert.deepEqual(s.player[0].kit?.powers[0].counters, [0, 0, 0, 0, 0]);
});

test("a power the turn unlocks starts on, and the turn is chosen for the table", () => {
  let s = B.setKitCard(duel(), "player", 0, "power", 0, FROSTWOLF);
  assert.deepEqual(s.player[0].kit?.powers[0].counters, [1, 0, 0, 0, 0]);
  s = B.setUnlocked(s, true);
  assert.equal(s.round, 7);
  assert.equal(JSON.parse(B.toRequest(s, cards)).round, 7);
  s = B.setUnlocked(s, false);
  assert.equal(s.round, 6);
  assert.equal(B.setUnlocked(s, false), s);
  const loaded = { ...s, round: 11 };
  assert.equal(B.setUnlocked(loaded, true), loaded, "turn 11 is already past 7");
});

test("an edit that changes nothing on a kit gives the same state (no new run)", () => {
  let s = B.addMinion(duel(), "player", 0, "BG28_300", cards);
  s = B.setKitCard(s, "player", 0, "power", 0, LOCK);
  const key = s.player[0].minions[0].key;
  assert.equal(B.setKitTarget(s, "player", 0, 0, null), s, "already none");
  assert.equal(B.setHeld(s, "player", 0, 0, null, cards), s, "holding nothing already");
  assert.equal(B.setKitTarget(s, "player", 0, 1, key), s, "no power in slot 1");
  s = B.setHeld(s, "player", 0, 0, "BGS_119", cards);
  const held = s.player[0].kit?.powers[0].held;
  assert.equal(B.setHeldStats(s, "player", 0, 0, held?.attack ?? 0, held?.health ?? 1), s, "the same stats");
  // A loaded Lock and Load that fired its minion keeps its first number when the minion's stats change.
  const fired = { ...s, player: [{ ...s.player[0], kit: { ...s.player[0].kit!, powers: [{ ...s.player[0].kit!.powers[0], counters: [0, 0, 0, 0, 0] }] } }] };
  assert.deepEqual(B.setHeldStats(fired, "player", 0, 0, 9, 9).player[0].kit?.powers[0].counters, [0, 0, 0, 0, 0]);
});

test("start of combat can be turned off, and is sent only when on", () => {
  const off = B.setStartOfCombat(duel(), false);
  assert.equal("start_of_combat" in JSON.parse(B.toRequest(off, cards)), false);
  assert.equal(B.setStartOfCombat(off, false), off);
});

/** A saved Solo game with each board's kit (parser revision 16). */
function kitGame(revision: number | null) {
  const report = soloReport() as { rounds: { entries: Record<string, unknown>[] }[] };
  const text = JSON.stringify(report).replaceAll("SYN_M_01", "BGS_119").replaceAll("SYN_M_02", "BG28_300").replaceAll("SYN_M_05", "BG29_611").replaceAll("SYN_M_04", "BG_ICC_026t");
  const r = JSON.parse(text) as typeof report;
  if (revision !== null && revision >= 16) {
    r.rounds[1].entries[0].kit = {
      powers: [{ card_id: LOCK, slot: 0, counters: [1, 0, 0, 0, 0], target: 1, held: { card_id: "BGS_119", atk: 4, health: 4, position: 0, golden: false, keywords: [] } }],
      trinkets: [{ card_id: "BG_TRINKET_SYN", slot: 1 }],
    };
    r.rounds[1].entries[1].kit = { powers: [{ card_id: FIRE, slot: 0 }], trinkets: [] };
  }
  const read = readRecord(record(r, revision === null ? {} : { parser: { version: "0.1.0", revision } }), SESSION, INDEX);
  assert.ok(read.ok);
  return read.record;
}

test("a saved round brings each board's hero powers and trinkets, at the first attack", () => {
  const loaded = B.fromRound(kitGame(16), 2, cards);
  assert.ok(loaded);
  const { state } = loaded;
  assert.equal(state.startOfCombat, false, "the boards are the ones at the first attack");
  assert.equal(state.round, 2);
  assert.equal(loaded.kitsNotRecorded, false);
  const mine = state.player[0].kit;
  assert.deepEqual(mine?.powers.map((c) => [c.cardId, c.counters[0], c.target]), [[LOCK, 1, null]], "the chosen minion's place is on the board before start of combat: not kept");
  assert.deepEqual([mine?.powers[0].held?.cardId, mine?.powers[0].held?.attack], ["BGS_119", 4]);
  assert.deepEqual(state.opponent[0].kit?.powers.map((c) => c.cardId), [FIRE]);
  const req = JSON.parse(B.toRequest(state, cards));
  assert.equal(req.round, 2);
  assert.equal("start_of_combat" in req, false);
  assert.deepEqual(req.opponent[0].kit, { powers: [{ card_id: FIRE, slot: 0 }], trinkets: [] });
});

test("a loaded kit fits the slots the editor offers, so nothing is sent unseen", () => {
  const rec = kitGame(16);
  const entry = rec.report.rounds.find((r) => r.number === 2)!.entries[0];
  const card = (cardId: string, slot: number) => ({ cardId, slot, counters: [0, 0, 0, 0, 0], target: null, held: null });
  entry.kit = { powers: [card(FIRE, 0), card(RAPID, 0), card(LOCK, 0)], trinkets: [card("BG_TRINKET_SYN", 0), card("BG_TRINKET2_SYN", 1), card("BG_X1", 2), card("BG_X2", 3)] };
  const loaded = B.fromRound(rec, 2, cards);
  assert.ok(loaded);
  const kit = loaded.state.player[0].kit;
  assert.deepEqual(kit?.powers.map((c) => [c.slot, c.cardId]), [[0, FIRE], [1, RAPID]], "two powers; a third has no slot");
  assert.deepEqual(kit?.trinkets.map((c) => [c.slot, c.cardId]), [[1, "BG_TRINKET2_SYN"], [2, "BG_X1"], [3, "BG_X2"]], "a slot 0 trinket waits for a free slot");
});

test("a game saved before revision 16 says its kits were not recorded", () => {
  const loaded = B.fromRound(kitGame(15), 2, cards);
  assert.ok(loaded);
  assert.equal(loaded.kitsNotRecorded, true);
  assert.equal(loaded.state.player[0].kit, null);
  assert.equal("kit" in JSON.parse(B.toRequest(loaded.state, cards)).player[0], false, "unknown, never empty");
  // Choosing one by hand makes it known.
  const s = B.setKitCard(loaded.state, "player", 0, "power", 0, FIRE);
  assert.deepEqual(s.player[0].kit?.powers.map((c) => c.cardId), [FIRE]);
});

test("the built engine plays a hero power with start of combat, and not without", async () => {
  const engine = await loadEngine(readFileSync(WASM_PATH));
  const fire = B.setKitCard(duel(), "player", 0, "power", 0, FIRE);
  const on = readAnswer(engine.run(B.toRequest(fire, cards)));
  assert.ok(on.kind === "odds");
  assert.equal(on.tied, on.trials, "the doubled 4 Attack trades with the 3/3");
  const off = readAnswer(engine.run(B.toRequest(B.setStartOfCombat(fire, false), cards)));
  assert.ok(off.kind === "odds");
  assert.equal(off.lost, off.trials);
  // The same rule as for a minion (T-632): with start of combat off its effect is
  // already on the board, so the odds are marked; played, nothing is missed.
  assert.equal(off.startOfCombat, true);
  assert.equal(on.startOfCombat, false);
  const plain = readAnswer(engine.run(B.toRequest(B.setStartOfCombat(duel(), false), cards)));
  assert.ok(plain.kind === "odds");
  assert.equal(plain.startOfCombat, false, "no source, no mark");
  // A power the engine has not reviewed is named, never guessed.
  const unknown = readAnswer(engine.run(B.toRequest(B.setKitCard(duel(), "player", 0, "power", 0, "BG_HP_SYN"), cards)));
  assert.ok(unknown.kind === "not_modelled");
  assert.deepEqual(unknown.notModelled, ["BG_HP_SYN"]);
});

test("the editor lists hero powers and trinkets by name, per slot, with what a power needs", () => {
  useCardNames({ cards, ...names });
  let s = B.setKitCard(duel(), "player", 0, "power", 0, LOCK);
  s = B.setKitCard(s, "player", 0, "trinket", 2, "BG_TRINKET2_SYN");
  const view = kitView("player", 0, { state: s, cards, lists, marks: NO_MARKS, holding: null });
  const selects = elements(view).filter((n) => n.tag === "select");
  const fields = selects.map((n) => n.attrs.find(([k]) => k === "data-slot")?.[1]);
  assert.deepEqual(fields, ["0", "1", "2", "3"], "the hero power, lesser, greater and third trinket (no second power)");
  const text = textOf(view);
  assert.ok(text.includes("Lock and Load") && text.includes("Fire Invocation") && text.includes("A greater trinket"));
  assert.ok(text.includes("Find the minion it holds"), "Lock and Load asks for its minion");
  const greater = toHtml(selects[2]);
  assert.match(greater, /<option value="BG_TRINKET2_SYN" selected>/);
  assert.doesNotMatch(greater, /BG_TRINKET_SYN"/, "a lesser trinket is not offered as greater");
  // A searched minion can be held; once held, its stats and a remove button.
  const searching = kitView("player", 0, { state: s, cards, lists, marks: NO_MARKS, holding: { side: "player", board: 0, slot: 0, query: "spark" } });
  assert.match(html([searching]), /data-act="held-pick" data-card="BGS_119"/);
  const holding = B.setHeld(s, "player", 0, 0, "BGS_119", cards);
  const held = html([kitView("player", 0, { state: holding, cards, lists, marks: NO_MARKS, holding: null })]);
  assert.match(held, /data-field="held-attack"/);
  assert.match(held, /data-act="held-remove"[^>]*data-focus="held-x-player-0-0"/);
});

test("a chosen minion and the turn have their own controls; a blocked power says so", () => {
  useCardNames({ cards, ...names });
  let s = B.setKitCard(duel(), "player", 0, "power", 0, RAPID);
  const target = html([kitView("player", 0, { state: s, cards, lists, marks: NO_MARKS, holding: null })]);
  assert.match(target, /data-field="kit-target"/);
  assert.match(target, /1\. Synthetic Spark/);
  s = B.setUnlocked(B.setKitCard(s, "opponent", 0, "power", 0, FROSTWOLF), true);
  const turn = kitView("opponent", 0, { state: s, cards, lists, marks: NO_MARKS, holding: null });
  assert.match(html([turn]), /data-act="kit-turn" data-late="1" data-focus="kit-turn-opponent-0-0-1" aria-pressed="true"/);
  assert.ok(textOf(turn).includes("Turn 7 or later"));
  const marks = { notModelled: new Set([FROSTWOLF]), missing: new Set<string>(), approximate: new Map<string, string>() };
  assert.ok(textOf(kitView("opponent", 0, { state: s, cards, lists, marks, holding: null })).includes("Not modelled"));
});

test("the editor shows the kits on every board, and Spanish names its slots", () => {
  useCardNames({ cards, ...names });
  const board = html(editorView({ state: duel(), cards, selected: null, adding: null, marks: NO_MARKS, lists }));
  assert.equal(board.match(/class="sim-kit"/g)?.length, 2, "one per board");
  setLang("es");
  useCardNames({ cards: new CardIndex(extractCards(en, es, "fixture")), ...readNameFile(extractNames(en, es, "fixture")) });
  const spanish = textOf(kitView("player", 0, { state: B.setKitCard(duel(), "player", 0, "power", 0, FIRE), cards, lists, marks: NO_MARKS, holding: null }));
  for (const words of ["Poder de héroe", "Abalorio menor", "Abalorio mayor", "Tercer abalorio", "Invocación de fuego"]) assert.ok(spanish.includes(words), words);
  assert.ok(textOf(socSwitch(B.emptyState())).includes("Jugar el inicio del combate"));
  // Without the names table there is nothing to choose from, and it says so.
  useCardNames(null);
  const none = textOf(kitView("player", 0, { state: duel(), cards, lists: { power: [], lesser: [], greater: [] }, marks: NO_MARKS, holding: null }));
  assert.ok(none.includes("No se han cargado"));
});

test("a blocked hero power is named in the result, with its reason and a hint that fits", () => {
  useCardNames({ cards, ...names });
  const answer = readAnswer(JSON.stringify({ engine: "0.13.0", status: "not_modelled", not_modelled: [LOCK], reasons: [{ card_id: LOCK, reason: "the minion it holds is not shown" }], approximate: [] }));
  const out = textOf(resultView({ result: { kind: "answer", answer }, cards, cap: null, kitIds: new Set([LOCK]) }).map((n) => n as never)[0]);
  assert.ok(out.includes("Lock and Load"), out);
  assert.ok(out.includes("Choose what a hero power needs"), out);
  // In the slot: the reason, not "Not modelled".
  const s = B.setKitCard(duel(), "player", 0, "power", 0, LOCK);
  const marks = { notModelled: new Set([LOCK]), missing: new Set<string>(), approximate: new Map<string, string>(), reasons: new Map([[LOCK, "the minion it holds is not shown"]]) };
  const slot = textOf(kitView("player", 0, { state: s, cards, lists, marks, holding: null }));
  assert.ok(slot.includes("the minion it holds is not shown") && !slot.includes("Not modelled"), slot);
});

test("a power's needs say they wait for start of combat when it is off", () => {
  useCardNames({ cards, ...names });
  const s = B.setStartOfCombat(B.setKitCard(duel(), "player", 0, "power", 0, RAPID), false);
  assert.ok(textOf(kitView("player", 0, { state: s, cards, lists, marks: NO_MARKS, holding: null })).includes("Used only when start of combat is played"));
  assert.ok(!textOf(kitView("player", 0, { state: B.setStartOfCombat(s, true), cards, lists, marks: NO_MARKS, holding: null })).includes("Used only when"));
});

test("the start-of-combat switch says whether it is on", () => {
  assert.match(toHtml(socSwitch(B.emptyState())), /data-act="soc"[^>]*aria-pressed="true"/);
  assert.match(toHtml(socSwitch(B.setStartOfCombat(B.emptyState(), false))), /aria-pressed="false"/);
});

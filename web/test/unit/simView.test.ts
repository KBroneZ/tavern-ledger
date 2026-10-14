// The simulator's answer reading and views (T-606): an answer out of shape
// is unreadable as a whole; the views say the key numbers first, name the
// cards that block a run, mark approximate ones, and are drawn in Spanish
// on the Spanish page, approximation reasons included.
import { afterEach, test } from "node:test";
import assert from "node:assert/strict";
import { readdirSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { setLang } from "../../src/lib/i18n.ts";
import { readAnswer, type Answer } from "../../src/lib/sim/answer.ts";
import * as B from "../../src/lib/sim/board.ts";
import { CardIndex } from "../../src/lib/sim/cards.ts";
import { reasonText, REASONS_ES } from "../../src/lib/sim/reasons.ts";
import { resultView } from "../../src/lib/sim/resultView.ts";
import { loadView } from "../../src/lib/sim/loadView.ts";
import { SIM_STRINGS } from "../../src/lib/sim/strings.ts";
import { ES } from "../../src/lib/sim/strings.es.ts";
import { editorView, NO_MARKS } from "../../src/lib/sim/view.ts";
import { extractCards } from "../../tools/simCards.ts";
import { elements, textOf, toHtml } from "../helpers/html.ts";
import type { VNode } from "../../src/lib/view/node.ts";

const ROOT = join(import.meta.dirname, "..", "..");
const REPO = join(ROOT, "..");
const FIXTURES = join(ROOT, "test", "fixtures", "sim-cards");
const en = JSON.parse(readFileSync(join(FIXTURES, "enUS.json"), "utf8")) as unknown[];
const es = JSON.parse(readFileSync(join(FIXTURES, "esES.json"), "utf8")) as unknown[];
const cards = new CardIndex(extractCards(en, en, "fixture"));

afterEach(() => setLang("en"));

const ODDS = {
  engine: "0.4.0",
  status: "odds",
  trials: 10000,
  won: 6200,
  tied: 800,
  lost: 3000,
  stalled: 0,
  damage_known: true,
  dealt: { low: 4, likely: 7, high: 11 },
  taken: { low: 3, likely: 5, high: 9 },
  approximate: [{ card_id: "BG25_009", reason: "its numbers grow during the game; the base values are used" }],
  start_of_combat: true,
};

const html = (nodes: VNode[]) => nodes.map(toHtml).join("");
const text = (nodes: VNode[]) => nodes.map(textOf).join(" ");

test("an answer is read whole or not at all", () => {
  const ok = readAnswer(JSON.stringify(ODDS));
  assert.equal(ok.kind, "odds");
  const broken: Record<string, unknown>[] = [
    { ...ODDS, won: 1 },
    { ...ODDS, trials: 0, won: 0, tied: 0, lost: 0 },
    { ...ODDS, dealt: { low: 9, likely: 7, high: 11 } },
    { ...ODDS, approximate: [{ card_id: "<b>", reason: "x" }] },
    { ...ODDS, engine: "zero" },
    { ...ODDS, status: "great" },
    { ...ODDS, damage_known: "yes" },
    { engine: "0.4.0", status: "not_modelled", not_modelled: ["a b"], approximate: [] },
  ];
  for (const b of broken) assert.equal(readAnswer(JSON.stringify(b)).kind, "unreadable", JSON.stringify(b).slice(0, 80));
  assert.equal(readAnswer("not json").kind, "unreadable");
  assert.equal(readAnswer("[]").kind, "unreadable");
  assert.deepEqual(readAnswer(JSON.stringify({ engine: "0.4.0", status: "no_card_data", missing: ["BG_X"] })), { kind: "no_card_data", engine: "0.4.0", missing: ["BG_X"] });
});

test("the odds show the three shares and the likely damage first; ranges, trials and engine in Advanced", () => {
  const nodes = resultView({ result: { kind: "answer", answer: readAnswer(JSON.stringify(ODDS)) }, cards, cap: 15 });
  const out = html(nodes);
  for (const s of ["62%", "8%", "30%", "Likely damage", "You deal 7", "You take 5", "4 to 11", "10,000 combats played", "Engine 0.4.0", "Card data: fixture"]) {
    assert.ok(out.includes(s), s);
  }
  // Advanced-only pieces carry the .adv class.
  assert.match(out, /class="sim-ranges adv"/);
  assert.match(out, /class="sim-meta adv"/);
  assert.match(out, /≈ Approximate/);
  assert.match(out, /Synthetic Grower/);
  assert.match(out, /start of combat/);
  // The cap field keeps its value.
  assert.match(out, /data-field="cap"[^>]*value="15"|value="15"[^>]*data-field="cap"/);
});

test("a card not modelled blocks the run and is named, never a number", () => {
  const a: Answer = { kind: "not_modelled", engine: "0.4.0", notModelled: ["BG_NOT_A_CARD"], reasons: [], approximate: [] };
  const out = text(resultView({ result: { kind: "answer", answer: a }, cards, cap: null }));
  assert.match(out, /does not know these cards/);
  assert.match(out, /Synthetic Unknown/);
  assert.doesNotMatch(out, /%/);
  // The editor marks it too.
  let s = B.addMinion(B.emptyState(), "player", 0, "BG_NOT_A_CARD", cards);
  s = B.addMinion(s, "opponent", 0, "BGS_119", cards);
  const marks = { ...NO_MARKS, notModelled: new Set(["BG_NOT_A_CARD"]) };
  const editor = html(editorView({ state: s, cards, selected: null, adding: null, marks }));
  assert.match(editor, /is-blocked/);
  assert.match(editor, /aria-label="Synthetic Unknown, 6\/6, Not modelled"/);
});

test("a refused card says why it blocks the odds, in English and Spanish (T-616)", () => {
  const why = "its Deathrattle copies the minion it destroys at the start of combat; the board is taken after that, so the copied minion is gone and not known";
  const json = { engine: "0.9.0", status: "not_modelled", not_modelled: ["BG_NOT_A_CARD"], reasons: [{ card_id: "BG_NOT_A_CARD", reason: why }], approximate: [] };
  const a = readAnswer(JSON.stringify(json));
  assert.equal(a.kind === "not_modelled" && a.reasons.length, 1);
  const out = text(resultView({ result: { kind: "answer", answer: a }, cards, cap: null }));
  assert.match(out, /Synthetic Unknown\s*:\s*its Deathrattle copies the minion it destroys/);
  setLang("es");
  const spanish = text(resultView({ result: { kind: "answer", answer: a }, cards, cap: null }));
  assert.match(spanish, /su Último aliento copia al esbirro que destruye/);
  // An engine without reasons still names the cards.
  setLang("en");
  const old = readAnswer(JSON.stringify({ engine: "0.8.0", status: "not_modelled", not_modelled: ["BG_NOT_A_CARD"], approximate: [] }));
  assert.equal(old.kind === "not_modelled" && old.reasons.length, 0);
  assert.match(text(resultView({ result: { kind: "answer", answer: old }, cards, cap: null })), /Synthetic Unknown/);
});

test("waiting, empty and failed states say so in words; a failure offers a retry", () => {
  assert.match(text(resultView({ result: { kind: "empty" }, cards, cap: null })), /Put minions on both sides/);
  assert.match(text(resultView({ result: { kind: "running", previous: null }, cards, cap: null })), /Playing 10,000 combats/);
  const failed = html(resultView({ result: { kind: "failed" }, cards, cap: null }));
  assert.match(failed, /stopped unexpectedly/);
  assert.match(failed, /data-act="retry"/);
  // A run over the last answer keeps it in place, dimmed: nothing jumps.
  const stale = html(resultView({ result: { kind: "running", previous: readAnswer(JSON.stringify(ODDS)) }, cards, cap: null }));
  assert.match(stale, /is-stale/);
  assert.match(stale, /62%/);
});

test("the editor: each minion a button with its words, the picked one's controls, the search", () => {
  let s = B.addMinion(B.emptyState(), "player", 0, "BGS_119", cards);
  const key = s.player[0].minions[0].key;
  s = B.setTier(s, "player", 0, 3);
  const nodes = editorView({ state: s, cards, selected: { side: "player", board: 0, key }, adding: { side: "opponent", board: 0, query: "skull" }, marks: NO_MARKS });
  const out = html(nodes);
  assert.match(out, /aria-label="Synthetic Spark, 2\/1, Divine Shield, Windfury"/);
  assert.match(out, /aria-pressed="true" aria-label="Tier 3"|aria-label="Tier 3"[^>]*aria-pressed="true"|data-tier="3"[^>]*aria-pressed="true"/);
  for (const act of ["golden", "keyword", "move", "remove", "close", "pick", "add-close"]) assert.ok(out.includes(`data-act="${act}"`), act);
  assert.match(out, /data-field="attack"/);
  assert.match(out, /Synthetic Skull/);
  // Picture of every minion, decorative (the button carries the words).
  const imgs = nodes.flatMap(elements).filter((n) => n.tag === "img");
  assert.ok(imgs.length >= 2);
  assert.ok(imgs.every((i) => i.attrs.some(([k, v]) => k === "alt" && v === "")));
  // A full board offers no more "add".
  let full = B.emptyState();
  for (let i = 0; i < 7; i++) full = B.addMinion(full, "player", 0, "BGS_119", cards);
  const fullOut = html(editorView({ state: full, cards, selected: null, adding: null, marks: NO_MARKS }));
  assert.equal(fullOut.match(/data-act="add-open"/g)?.length, 1, "only the opponent's");
});

test("a card shows short numbers and has a tooltip with the exact ones and its buffs (T-619)", () => {
  let s = B.addMinion(B.emptyState(), "player", 0, "BGS_119", cards);
  const key = s.player[0].minions[0].key;
  s = B.setStats(s, "player", 0, key, 12_345, 2_000);
  const nodes = editorView({ state: s, cards, selected: null, adding: null, marks: NO_MARKS });
  const out = html(nodes);
  assert.match(out, /class="sim-atk" aria-hidden="true">12\.3K</);
  assert.match(out, /class="sim-hp" aria-hidden="true">2K</);
  assert.match(out, /aria-label="Synthetic Spark, 12,345\/2,000, /, "the words say every digit");
  const tip = nodes.flatMap(elements).find((n) => n.attrs.some(([k, v]) => k === "role" && v === "tooltip"));
  assert.ok(tip, "a tooltip");
  const id = tip.attrs.find(([k]) => k === "id")?.[1];
  assert.match(out, new RegExp(`aria-describedby="${id}"`));
  assert.match(text([tip]), /Attack 12,345/);
  assert.match(text([tip]), /Health 2,000/);
  assert.match(text([tip]), /No buffs/);
  assert.doesNotMatch(out, /title="Synthetic Spark"/, "no browser title over the plaque");
});

test("Duos shows each team's boards and offers a second one; Solo does not", () => {
  const solo = html(editorView({ state: B.emptyState(), cards, selected: null, adding: null, marks: NO_MARKS }));
  assert.doesNotMatch(solo, /board-add/);
  const duos = html(editorView({ state: B.setMode(B.emptyState(), "duos"), cards, selected: null, adding: null, marks: NO_MARKS }));
  assert.equal(duos.match(/data-act="board-add"/g)?.length, 2);
  assert.match(duos, /Your team/);
  assert.match(duos, /First board/);
});

test("signed out, loading a combat asks to sign in; the list shows only the rows it was given", () => {
  assert.match(html(loadView({ kind: "signed-out" }, null)), /href="\/signin\/"/);
  const out2 = html(loadView({ kind: "signed-out" }, null));
  assert.match(out2, /class="sim-step"[^>]*>2</, "step 2 stays on the page so the seals read 1, 2, 3");
  assert.match(out2, /Pick a game first/);
  const row = { publicId: "3f2b8c1e-5d4a-4b7e-9c6d-1a2b3c4d5e6f", storagePath: null, gameIndex: 2, mode: "solo" as const, status: "ok" as const, playedOn: "2026-10-09", hero: "SYN_HERO_A", place: 3, rounds: 3, combatsWon: 1, combatsLost: 1, combatsInferred: 0, triples: 1, apm: 3, goldSpent: 9 };
  const out = html(loadView({ kind: "ready", rows: [row], total: 1, more: false }, { key: row.publicId, rounds: [{ number: 1, result: "won" }, { number: 2, result: null }], failed: false, loaded: 2 }));
  assert.match(out, /data-act="game"/);
  assert.match(out, /Oct 9, 2026/);
  assert.match(out, /Turn 1/);
  assert.match(out, /class="sim-round is-won"/);
  assert.match(out, /data-round="2"[^>]*aria-pressed="true"/);
  assert.doesNotMatch(out, /games-more/);
});

test("the Spanish page is Spanish: views, keywords and approximation reasons", () => {
  setLang("es");
  const spanish = new CardIndex(extractCards(en, es, "fixture"));
  let s = B.addMinion(B.emptyState(), "player", 0, "BGS_119", spanish);
  s = B.addMinion(s, "opponent", 0, "BG25_009", spanish);
  const key = s.player[0].minions[0].key;
  const marks = { ...NO_MARKS, approximate: new Map([["BG25_009", ODDS.approximate[0].reason]]) };
  const editor = text(editorView({ state: s, cards: spanish, selected: { side: "player", board: 0, key }, adding: null, marks }));
  for (const word of ["Tú", "Rival", "Escudo divino", "Viento furioso", "Dorado", "Quitar"]) assert.ok(editor.includes(word), word);
  const result = text(resultView({ result: { kind: "answer", answer: readAnswer(JSON.stringify(ODDS)) }, cards: spanish, cap: null }));
  for (const word of ["Victoria", "Empate", "Derrota", "Haces 7", "10.000 combates jugados", "sus números crecen", "Crecedor sintético"]) assert.ok(result.includes(word), word);
  for (const english of ["Win", "Lose", "Likely damage", "its numbers grow"]) assert.ok(!result.includes(english), english);
  // A reason the table does not know stays as written, marked English.
  assert.deepEqual(reasonText("a brand new gap"), { text: "a brand new gap", english: true });
});

test("every reason the engine can give has its Spanish", () => {
  const src = join(REPO, "crates", "combat-sim", "src");
  const files = readdirSync(src).filter((f) => /^(cards|cards_round\d+|gifts|trials)\.rs$/.test(f));
  const reasons = new Set<string>();
  for (const f of files) {
    const code = readFileSync(join(src, f), "utf8");
    for (const m of code.matchAll(/const [A-Z_]+: &str =\s*"([^"]+)"/g)) if (m[1].includes(" ")) reasons.add(m[1]);
    for (const m of code.matchAll(/approx\(\s*"([^"]+)"/g)) reasons.add(m[1]);
    for (const m of code.matchAll(/"(may come back[^"]*)"/g)) reasons.add(m[1]);
  }
  assert.ok(reasons.size >= 20, `${reasons.size} reasons found`);
  for (const r of reasons) assert.ok(r in REASONS_ES, `no Spanish for: ${r}`);
  for (const r of Object.keys(REASONS_ES)) assert.ok(reasons.has(r), `a Spanish reason the engine no longer gives: ${r}`);
});

test("the two string tables have the same keys", () => {
  assert.deepEqual(Object.keys(ES).sort(), Object.keys(SIM_STRINGS).sort());
});

test("a card wears its keywords as the combat replays' marks (T-575)", () => {
  const s = B.addMinion(B.emptyState(), "player", 0, "BGS_119", cards);
  const out = html(editorView({ state: s, cards, selected: null, adding: null, marks: NO_MARKS }));
  // Synthetic Spark: Divine Shield and Windfury.
  assert.match(out, /class="sim-card[^"]*\bhas-shield\b[^"]*\bhas-windfury\b/);
  assert.doesNotMatch(out, /class="sim-card[^"]*\bis-taunt\b/);
  for (const mark of ["sim-taunt", "sim-veil", "sim-venom", "sim-wind"]) assert.ok(out.includes(`class="${mark}" aria-hidden="true"`), mark);
});

test("Poisonous wears its own flask, Venomous its vial, the vial when both (T-581)", () => {
  // BG29_611 has Venomous and Poisonous printed.
  let s = B.addMinion(B.emptyState(), "player", 0, "BG29_611", cards);
  const both = html(editorView({ state: s, cards, selected: null, adding: null, marks: NO_MARKS }));
  assert.match(both, /class="sim-card[^"]*\bhas-venomous\b[^"]*\bhas-poisonous\b/);
  const key = s.player[0].minions[0].key;
  s = B.toggleKeyword(s, "player", 0, key, "venomous");
  const poisonous = html(editorView({ state: s, cards, selected: null, adding: null, marks: NO_MARKS }));
  assert.match(poisonous, /class="sim-card[^"]*\bhas-poisonous\b/);
  assert.doesNotMatch(poisonous, /class="sim-card[^"]*\bhas-venomous\b/);
  const css = readFileSync(join(ROOT, "src", "components", "sim", "sim-cards.css"), "utf8");
  assert.match(css, /\n\.sim \.sim-card\.has-poisonous:not\(\.has-venomous\) \.sim-venom \{/, "the flask only without Venomous");
});

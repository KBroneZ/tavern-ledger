// "Why the result differed" on the game page (T-549, D-127): the request
// carries the stored report and its cards' facts, the engine's answer is
// read strictly, and each combat gets its odds and a mark in its turn's row
// and one plain line with its evidence in the row's detail, in English and
// Spanish. The built engine answers a real request.
import { afterEach, test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { loadEngine } from "../../src/lib/sim/engine.ts";
import { cardIdsOf, explainRequest, newBuildOf, readWhy, type WhyCombat } from "../../src/lib/game/why.ts";
import { causeLine, causeMark, evidence, oddsCell, whyDetail } from "../../src/lib/game/whyView.ts";
import { askEngine, fillWhy } from "../../src/lib/game/whyRun.ts";
import { setLang } from "../../src/lib/i18n.ts";
import type { VNode } from "../../src/lib/view/node.ts";
import { WASM_PATH } from "../../tools/simStamp.ts";

afterEach(() => setLang("en"));

/** A card with no combat text, and the facts the page's card data gives it. */
const PLAIN = "BG28_603t";
const FACTS = { tier: 1, tribes: [], attack: 1, health: 1, keywords: [] };

const minion = (atk: number, health: number, position: number) => ({ card_id: PLAIN, golden: false, atk, health, position, keywords: [] });

/** The player's 3/3 and 1/1 against a 1/1: always a win. Turn 3 says it was lost, with a replay where the 1/1 attacks first. */
function report(): Record<string, unknown> {
  const entries = [
    { side: "own", player_id: 1, board: [minion(3, 3, 1), minion(1, 1, 2)] },
    { side: "opponent", player_id: 2, board: [minion(1, 1, 1)] },
  ];
  const replay = {
    complete: true,
    start: 3,
    minions: [
      { side: "own", ...minion(3, 3, 1) },
      { side: "own", ...minion(1, 1, 2) },
      { side: "opponent", ...minion(1, 1, 1) },
    ],
    events: [{ kind: "attack", attacker: 2, defender: 0 }],
  };
  return {
    game_type: "GT_BATTLEGROUNDS",
    rounds: [
      { number: 2, result: "won", entries },
      { number: 3, result: "lost", entries, replay },
      { number: 4, result: "lost", entries },
    ],
  };
}

const answer = (combats: unknown[]) => JSON.stringify({ engine: "0.8.0", status: "explained", combats });
const good = { round: 2, result: "won", odds: { won: 1, tied: 0, lost: 0 }, no_odds: null, cards: [], cause: { id: "matched" } };

function* walk(node: VNode | string): Generator<VNode> {
  if (typeof node === "string") return;
  yield node;
  for (const child of node.children) yield* walk(child);
}
const text = (n: VNode | string): string => (typeof n === "string" ? n : n.children.map(text).join(""));
const nameOf = (id: string) => (id === PLAIN ? "Plain Card" : id);

test("the request carries the report as stored and the facts of every card on its boards and replays", () => {
  const r = report();
  assert.deepEqual(cardIdsOf(r), [PLAIN]);
  assert.deepEqual(cardIdsOf({ rounds: [{ replay: { minions: [{ card_id: "BG_X" }, { card_id: "../bad" }] } }] }), ["BG_X"]);
  assert.deepEqual(cardIdsOf(null), []);
  const req = JSON.parse(explainRequest(r, (id) => (id === PLAIN ? FACTS : null)));
  assert.deepEqual(req.explain.report, r);
  assert.deepEqual(req.explain.facts, { [PLAIN]: FACTS });
  const none = JSON.parse(explainRequest(r, () => null));
  assert.deepEqual(none.explain.facts, { [PLAIN]: null }, "a card the data lacks is sent as null: the engine names it");
});

test("the answer is read strictly: anything out of shape makes it unreadable", () => {
  assert.equal(readWhy(answer([good]))?.[0].cause?.id, "matched");
  const bad: unknown[] = [
    { ...good, cause: { id: "made_up" } },
    { ...good, no_odds: "no_boards" },
    { ...good, odds: null },
    { ...good, odds: { won: 2, tied: 0, lost: 0 } },
    { ...good, result: "draw" },
    { ...good, cards: ["../x"] },
    { ...good, cause: { id: "missing_inputs", inputs: [{ what: "luck", card: PLAIN }] } },
    { ...good, cause: { id: "replay_stats", rule: "chance" } },
  ];
  for (const b of bad) assert.equal(readWhy(answer([b])), null, JSON.stringify(b));
  assert.equal(readWhy(JSON.stringify({ status: "bad_input", error: "x" })), null);
  assert.equal(readWhy("not json"), null);
});

test("each cause gets one plain line and its evidence, in English and Spanish", () => {
  const miss = (cause: Record<string, unknown>): WhyCombat =>
    readWhy(answer([{ ...good, result: "lost", odds: { won: 0.7, tied: 0, lost: 0.3 }, cause }]))![0];
  const lines = {
    chance: miss({ id: "chance", attacks: 9 }),
    missing: miss({ id: "missing_inputs", attack: 4, rule: "replay_stats", card: PLAIN, inputs: [{ what: "blood_gems", card: PLAIN }] }),
    approximate: miss({ id: "approximate_card", attack: 2, rule: "replay_minions", cards: [PLAIN] }),
    rule: miss({ id: "replay_stats", attack: 4, rule: "replay_stats", card: PLAIN }),
    noReplay: miss({ id: "no_replay_exact", replay: "not_saved" }),
  };
  assert.equal(causeLine(lines.chance, nameOf), "Chance: the replay matched until the dice.");
  assert.equal(causeLine(lines.missing, nameOf), "Not seen: Blood Gem upgrades.");
  assert.equal(causeLine(lines.approximate, nameOf), "Approximate: Plain Card.");
  assert.equal(causeLine(lines.rule, nameOf), "Rule differs at attack 4: Plain Card.");
  assert.equal(causeLine(lines.noReplay, nameOf), "No replay saved, so no cause is given.");
  assert.match(evidence(lines.chance, nameOf)[0], /all 9 attacks/);
  assert.match(evidence(lines.missing, nameOf)[0], /attack 4 \(Stats and keywords\), on Plain Card/);
  assert.deepEqual(evidence(readWhy(answer([good]))![0], nameOf), [], "nothing to fold when the favourite happened");
  const noData = readWhy(answer([{ ...good, odds: null, no_odds: "no_card_data", cards: [PLAIN], cause: null }]))![0];
  assert.equal(causeLine(noData, nameOf), "No odds: no card data for Plain Card.");
  // A card the engine refuses says why, in the page's language (T-616).
  const refused = readWhy(
    answer([
      {
        ...good,
        odds: null,
        no_odds: "not_modelled",
        cards: [PLAIN],
        reasons: [{ card_id: PLAIN, reason: "the simulator has not reviewed this card yet" }],
        cause: null,
      },
    ]),
  )![0];
  assert.equal(causeLine(refused, nameOf), "No odds: the simulator does not know Plain Card. Plain Card: the simulator has not reviewed this card yet.");
  setLang("es");
  assert.match(causeLine(refused, nameOf), /Plain Card: el simulador aún no ha revisado esta carta\.$/);
  assert.equal(causeLine(lines.chance, nameOf), "Azar: la repetición coincidió hasta los dados.");
  assert.equal(causeLine(lines.missing, nameOf), "No visto: las mejoras de las Gemas de sangre.");
  assert.match(evidence(lines.missing, nameOf)[0], /ataque 4 \(Estadísticas y palabras clave\), en Plain Card/);
});

test("a row shows the win odds, and a mark only when the likeliest result did not happen (T-559, D-131)", () => {
  const combat = (extra: Record<string, unknown>) => readWhy(answer([{ ...good, ...extra }]))![0];
  const matched = combat({});
  assert.equal(causeMark(matched), null, "nothing said for the expected result");
  assert.equal(oddsCell(matched).map(text).join(""), "100%");
  const chance = combat({ result: "lost", odds: { won: 0.58, tied: 0.04, lost: 0.38 }, cause: { id: "chance", attacks: 9 } });
  assert.equal(oddsCell(chance).map(text).join(""), "58% Chance");
  const noReplay = combat({ result: "lost", odds: { won: 0.58, tied: 0.04, lost: 0.38 }, cause: { id: "no_replay_exact", replay: "not_saved" } });
  assert.equal(text(causeMark(noReplay)!), "—no cause given", "a quiet dash, the words for screen readers only");
  const rule = combat({ result: "lost", odds: { won: 0.7, tied: 0, lost: 0.3 }, cause: { id: "replay_stats", attack: 4, rule: "replay_stats", card: PLAIN } });
  assert.equal(text(causeMark(rule)!), "Rule");
  const noOdds = combat({ odds: null, no_odds: "no_card_data", cards: [PLAIN], cause: null });
  assert.equal(oddsCell(noOdds).map(text).join(""), "—");
  // The detail: the odds in full, then the cause and its evidence.
  const detail = whyDetail(chance, nameOf).map(text);
  assert.equal(detail[0], "Odds 58% win · 4% tie · 38% loss");
  assert.equal(detail[1], "Chance: the replay matched until the dice.");
  assert.match(detail[2], /all 9 attacks/);
  assert.equal(whyDetail(matched, nameOf).length, 1, "the expected result: the odds only");
  setLang("es");
  assert.match(oddsCell(chance).map(text).join(""), /^58\s%\sAzar$/);
});

test("the built engine explains a stored game combat by combat", async () => {
  const engine = await loadEngine(readFileSync(WASM_PATH));
  const combats = readWhy(engine.run(explainRequest(report(), (id) => (id === PLAIN ? FACTS : null))));
  assert.ok(combats, "a readable answer");
  assert.deepEqual(combats.map((c) => [c.round, c.cause?.id]), [
    [2, "matched"],
    [3, "replay_first_attacker"],
    [4, "no_replay_exact"],
  ]);
  assert.equal(combats[1].cause?.card, PLAIN);
});

test("the built engine's missing inputs and approximate cards read as the page expects", async () => {
  const engine = await loadEngine(readFileSync(WASM_PATH));
  // A Blood Gem card with no upgrades saved, and a card that picks minions at random (approximate).
  const withCard = (id: string) => {
    const r = report();
    const rounds = r.rounds as { entries: { board: { card_id: string }[] }[]; replay?: { minions: { card_id: string }[] } }[];
    const round = structuredClone(rounds[1]);
    round.entries[0].board[1].card_id = id;
    round.replay!.minions[1].card_id = id;
    return { ...r, rounds: [round] };
  };
  const facts = () => FACTS;
  const gems = readWhy(engine.run(explainRequest(withCard("BG33_886"), facts)))!;
  assert.equal(gems[0].cause?.id, "missing_inputs");
  assert.deepEqual(gems[0].cause?.inputs, [{ what: "blood_gems", card: "BG33_886" }]);
  assert.equal(gems[0].cause?.rule, "replay_first_attacker");
  const picks = readWhy(engine.run(explainRequest(withCard("BG28_309"), facts)))!;
  assert.equal(picks[0].cause?.id, "approximate_card");
  assert.deepEqual(picks[0].cause?.cards, ["BG28_309"]);
});

/** A stand-in for the simulator's worker: answers (or not) as told, and notes when it is stopped. */
class FakeWorker extends EventTarget {
  stopped = false;
  private readonly reply: (request: string) => unknown;
  constructor(reply: (request: string) => unknown) {
    super();
    this.reply = reply;
  }
  postMessage(message: { id: number; request: string }): void {
    const data = this.reply(message.request);
    if (data !== undefined) queueMicrotask(() => this.dispatchEvent(new MessageEvent("message", { data })));
  }
  terminate(): void {
    this.stopped = true;
  }
}

test("the engine's worker is stopped once it answers, fails or takes too long", async () => {
  const answered = new FakeWorker((request) => ({ id: 1, answer: `echo ${request.length}` }));
  assert.equal(await askEngine("abc", () => answered as unknown as Worker), "echo 3");
  assert.ok(answered.stopped);
  const failed = new FakeWorker(() => ({ id: 1, failed: true }));
  await assert.rejects(askEngine("x", () => failed as unknown as Worker), /failed/);
  assert.ok(failed.stopped);
  const silent = new FakeWorker(() => undefined);
  await assert.rejects(askEngine("x", () => silent as unknown as Worker, 10), /too long/);
  assert.ok(silent.stopped);
  const other = new FakeWorker(() => ({ id: 2, answer: "not ours" }));
  await assert.rejects(askEngine("x", () => other as unknown as Worker, 10), /too long/, "another request's answer is not taken");
});

/** A stand-in for the page: the turn table's odds cells, the detail boxes and the status line. */
function fakePage(rounds: number[]) {
  const text = (n: unknown): string =>
    typeof n === "object" && n !== null && "data" in n ? String((n as { data: string }).data) : ((n as { kids?: unknown[] }).kids ?? []).map(text).join("");
  const doc = {
    createElement: () => {
      const el = { kids: [] as unknown[], setAttribute() {}, appendChild: (c: unknown) => void el.kids.push(c) };
      return el;
    },
    createElementNS: () => doc.createElement(),
    createTextNode: (data: string) => ({ data }),
  };
  const box = (key: string, round: number) => {
    const node = { dataset: { [key]: String(round) }, ownerDocument: doc, kids: [] as unknown[], replaceChildren: (...k: unknown[]) => void (node.kids = k) };
    return node;
  };
  const odds = rounds.map((r) => box("odds", r));
  const why = rounds.map((r) => box("why", r));
  const status = { textContent: "the note" };
  const root = {
    querySelectorAll: (sel: string) => (sel === "[data-odds]" ? odds : why),
    querySelector: () => status,
  } as unknown as ParentNode;
  return { root, status, oddsText: () => odds.map((c) => c.kids.map(text).join("")), whyCount: () => why.map((c) => c.kids.length) };
}

test("a new game build is named under the table, the odds stay; checked or unsaid builds say nothing", () => {
  const withBuild = (b: unknown) => JSON.stringify({ status: "explained", engine: "0.9.1", combats: [good], new_build: b });
  assert.equal(newBuildOf(withBuild(999_999)), 999_999);
  for (const none of [null, undefined, 0, -3, 1.5, "254477", 100_000_000]) assert.equal(newBuildOf(withBuild(none)), null, String(none));
  assert.equal(newBuildOf("not json"), null);
  assert.equal(newBuildOf(answer([good])), null, "an engine without the field says nothing");

  const page = fakePage([1]);
  fillWhy(page.root, readWhy(withBuild(999_999)), 999_999);
  assert.match(page.status.textContent, /^the note New game build \(999999\): the odds are not checked against it yet\.$/);
  assert.equal(page.oddsText().length, 1, "the odds are still filled");
  const calm = fakePage([1]);
  fillWhy(calm.root, readWhy(withBuild(null)), null);
  assert.equal(calm.status.textContent, "the note");
  // No odds at all: the failure line wins, with no build warning on top of it.
  const failed = fakePage([1]);
  fillWhy(failed.root, null, 999_999);
  assert.match(failed.status.textContent, /could not be worked out/);
  assert.doesNotMatch(failed.status.textContent, /New game build/);
});

test("the engine's answer fills each row; a combat it does not answer, or no answer at all, ends as a dash", () => {
  const page = fakePage([1, 3]);
  const chance = { ...good, round: 1, result: "lost", odds: { won: 0.58, tied: 0.04, lost: 0.38 }, cause: { id: "chance", attacks: 9 } };
  fillWhy(page.root, readWhy(answer([chance])));
  assert.deepEqual(page.oddsText(), ["58% Chance", "—"], "never left waiting");
  assert.deepEqual(page.whyCount(), [3, 0]);
  assert.equal(page.status.textContent, "the note");
  const failed = fakePage([1, 3]);
  fillWhy(failed.root, null);
  assert.deepEqual(failed.oddsText(), ["—", "—"]);
  assert.match(failed.status.textContent, /could not be worked out/);
});

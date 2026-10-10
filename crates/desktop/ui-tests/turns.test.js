"use strict";
const test = require("node:test");
const assert = require("node:assert/strict");
const TLTurns = require("../ui/turns.js");

const minion = (card_id, atk, health, extra = {}) => ({ card_id, atk, health, golden: false, keywords: [], ...extra });
const shopTurn = (turn, extra = {}) => ({
  turn, in_log: true, tier: 1, tier_up: false, gold: 3, gold_spent: 3, rolls: 0, free_rolls: 0,
  buys: ["BG_A"], spell_buys: 0, sells: [], freezes: 0, actions: 2, apm: 4.2, seconds: 30, shops: [], ...extra,
});

function recap(over = {}) {
  return {
    is_duos: false,
    health: [{ round: 0, health: 30 }, { round: 1, health: 30 }, { round: 2, health: 25 }, { round: 3, health: null }],
    health_source: "log",
    record: [
      {
        heroes: [{ id: "HERO_B", name: "Bee" }], seats: [5],
        combats: [{ round: 1, outcome: "won", source: "log", reason: null }, { round: 3, outcome: "unknown", source: "unknown", reason: "no result" }],
        boards: [{ round: 1, seat: 5, hero: "HERO_B", minions: [minion("BG_X", 2, 2)] }],
      },
      {
        heroes: [{ id: "HERO_C", name: null }], seats: [6],
        combats: [{ round: 2, outcome: "lost", source: "inferred", reason: null }],
        boards: [],
      },
    ],
    unattributed: [],
    shop: { state: "recorded", turns: [shopTurn(1), shopTurn(2, { tier: 2, tier_up: true }), shopTurn(3, { tier: 2 })] },
    ...over,
  };
}

const report = {
  hero: "HERO_A",
  local_player_id: 2,
  rounds: [
    { number: 1, entries: [{ side: "own", player_id: 2, hero: "HERO_A", board: [minion("BG_A", 1, 2), minion("BG_B", 3, 4, { golden: true })] }] },
    { number: 2, entries: [{ side: "own", player_id: 2, hero: "HERO_A", board: [minion("BG_A", 2, 3)] }] },
  ],
};

test("each turn joins its shop, the combat after it, the health and both boards", () => {
  const rows = TLTurns.rows(recap(), report);
  assert.deepEqual(rows.map((r) => r.turn), [1, 2, 3]);
  const [one, two, three] = rows;
  assert.equal(one.shop.turn, 1);
  assert.equal(one.combat.outcome, "won");
  assert.deepEqual(one.combat.opponents.map((h) => h.id), ["HERO_B"]);
  assert.deepEqual(one.combat.board.map((m) => m.id), ["BG_X"]);
  assert.deepEqual(one.own.minions.map((m) => m.id), ["BG_A", "BG_B"]);
  assert.equal(one.own.minions[1].golden, true);
  assert.deepEqual(one.own.totals, { count: 2, attack: 4, health: 6 });
  assert.deepEqual(one.tier, { value: 1, up: false });
  assert.deepEqual(two.tier, { value: 2, up: true });
  assert.equal(two.combat.outcome, "lost");
  assert.equal(two.combat.source, "inferred");
  assert.equal(two.combat.board, null, "no board of theirs that round");
  assert.equal(three.own, null, "no board of yours in the log that round");
});

test("health: the change over the combat, unknown when either side is missing", () => {
  const [one, two, three] = TLTurns.rows(recap(), report);
  assert.deepEqual(one.health, { before: 30, after: 30, change: 0 });
  assert.deepEqual(two.health, { before: 30, after: 25, change: -5 });
  assert.deepEqual(three.health, { before: 25, after: null, change: null });
});

test("the words for a health change: damage, nothing, healing or unknown", () => {
  assert.equal(TLTurns.healthWords({ change: -5, after: 25 }), "−5");
  assert.equal(TLTurns.healthWords({ change: 0, after: 30 }), "±0");
  assert.equal(TLTurns.healthWords({ change: 3, after: 33 }), "+3");
  assert.equal(TLTurns.healthWords({ change: null, after: null }), "?");
});

test("a combat with no opponent known still shows, with its reason", () => {
  const r = recap({ unattributed: [{ round: 4, outcome: "unknown", source: "unknown", reason: "opponent unknown" }] });
  const four = TLTurns.rows(r, report).find((x) => x.turn === 4);
  assert.equal(four.combat.outcome, "unknown");
  assert.deepEqual(four.combat.opponents, []);
  assert.equal(four.combat.reason, "opponent unknown");
  assert.equal(four.shop, null);
});

test("a game saved without a shop record still lists its combats", () => {
  const rows = TLTurns.rows(recap({ shop: { state: "not_recorded", turns: [] } }), report);
  assert.deepEqual(rows.map((r) => r.turn), [1, 2, 3]);
  assert.equal(rows[0].shop, null);
  assert.equal(rows[0].tier, null);
});

test("a game with no report rounds has no own boards, never a made-up one", () => {
  const rows = TLTurns.rows(recap(), { rounds: null });
  assert.ok(rows.every((r) => r.own === null));
});

test("in Duos each own board says whose it is when it is not your hero", () => {
  const duos = {
    hero: "HERO_A", local_player_id: 2,
    rounds: [{ number: 1, entries: [{ side: "own", player_id: 3, hero: "HERO_T", board: [minion("BG_A", 1, 1)] }] }],
  };
  const [one] = TLTurns.rows(recap({ is_duos: true }), duos);
  assert.equal(one.own.hero, "HERO_T");
  assert.equal(one.own.mine, false);
});

test("a turn's summary line is short and in words, for the screen reader too", () => {
  const [, two] = TLTurns.rows(recap(), report);
  assert.equal(TLTurns.summaryWords(two, (h) => h.name || h.id), "Turn 2, tier 2, up a tier, lost against HERO_C, health −5, your board 1 minion");
});

test("shop facts are counted, never shown as zero when not in the log", () => {
  const [one] = TLTurns.rows(recap(), report);
  assert.deepEqual(TLTurns.shopFacts(one.shop), [
    { label: "Gold", value: "3" },
    { label: "Spent", value: "3" },
    { label: "Rolls", value: "0" },
    { label: "Buys", value: "1" },
    { label: "Sells", value: "0" },
    { label: "APM", value: "4.2" },
  ]);
  assert.deepEqual(TLTurns.shopFacts(shopTurn(1, { gold: null, rolls: 3, free_rolls: 2, apm: null })).map((f) => f.value), ["—", "3", "3 (2 free)", "1", "0", "—"]);
  assert.deepEqual(TLTurns.shopFacts(null), []);
});

test("board totals with a missing attack or health are unknown, never added as 0", () => {
  const r = { hero: "HERO_A", local_player_id: 2, rounds: [{ number: 1, entries: [{ side: "own", player_id: 2, hero: "HERO_A", board: [minion("BG_A", 2, 3), minion("BG_B", null, 4)] }] }] };
  const [one] = TLTurns.rows(recap(), r);
  assert.deepEqual(one.own.totals, { count: 2, attack: null, health: 7 });
  assert.equal(TLTurns.totalsWords(one.own.totals), "?/7");
  assert.equal(TLTurns.totalsWords({ count: 0, attack: 0, health: 0 }), "empty");
});

test("a board the record lacks is unknown, not empty", () => {
  const r = { hero: "HERO_A", local_player_id: 2, rounds: [{ number: 1, entries: [{ side: "own", player_id: 2, hero: "HERO_A" }] }] };
  const [one] = TLTurns.rows(recap(), r);
  assert.equal(one.own, null);
});

test("keywords not recorded (older games) are told apart from no keywords", () => {
  const r = { hero: "HERO_A", local_player_id: 2, rounds: [{ number: 1, entries: [{ side: "own", player_id: 2, hero: "HERO_A", board: [{ card_id: "BG_A", atk: 1, health: 1 }] }] }] };
  const [one] = TLTurns.rows(recap(), r);
  assert.equal(one.own.minions[0].keywords, null);
  assert.equal(TLTurns.keywordsRecorded(one.own.minions), false);
  assert.equal(TLTurns.keywordsRecorded([{ keywords: [] }]), true);
});

test("several opponents keep the order of their names", () => {
  const order = TLTurns.rows(recap({ record: [{ heroes: [{ id: "A" }, { id: "B" }], seats: [1, 2], combats: [{ round: 1, outcome: "won", source: "log" }], boards: [] }] }), report)[0];
  assert.deepEqual(order.combat.opponents.map((h) => h.id), ["A", "B"]);
});

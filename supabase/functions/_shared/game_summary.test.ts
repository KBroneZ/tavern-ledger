// Run: deno test supabase/functions/_shared/
// The per-game summary the website reads (T-502, D-071): numbers worked out
// from a validated report, each null when the report cannot back it.
import { type GameStats, STATS_KEYS, summarize, SUMMARY_VERSION } from "./game_summary.ts";
import {
  dataGames,
  duo,
  type Json,
  realDuos,
  realSolo,
  roundTwoGames,
  shopGames,
  solo,
} from "../upload-game/testdata.ts";

function assertEquals(actual: unknown, expected: unknown, msg = ""): void {
  const a = JSON.stringify(actual);
  const e = JSON.stringify(expected);
  if (a !== e) throw new Error(`${msg}\n  actual:   ${a}\n  expected: ${e}`);
}

const SOLO = "GT_BATTLEGROUNDS";
const DUOS = "GT_BATTLEGROUNDS_DUO";

/** A small Solo report: player 1 against players 2 and 3. */
function report(over: Json = {}): Json {
  return {
    index: 1,
    status: "ok",
    game_type: SOLO,
    local_player_id: 1,
    hero: "BG20_HERO_202",
    final_place: 3,
    lobby: [
      { player_id: 1, hero: "BG20_HERO_202", final_place: 3, final_health: 0 },
      { player_id: 2, hero: "BG22_HERO_000", final_place: 1, final_health: 10 },
      { player_id: 3, hero: "BG23_HERO_201", final_place: 2, final_health: 5 },
    ],
    start_health: { "1": 30, "2": 30, "3": 30 },
    rounds: [],
    ...over,
  };
}

function round(number: number, opponent: number | null, over: Json = {}): Json {
  const entries: Json[] = [{ side: "own", player_id: 1, hero: "BG20_HERO_202", board: [] }];
  if (opponent !== null) {
    entries.push({ side: "opponent", player_id: opponent, hero: "X", board: [] });
  }
  return { number, entries, own_health_after: null, health_after: {}, ...over };
}

function minion(atk: number, health: number, golden = false, card_id: string | null = "BG_A") {
  return { card_id, atk, health, position: 0, golden, keywords: [] };
}

Deno.test("the summary has exactly the game_stats columns", () => {
  const s = summarize(report());
  assertEquals(Object.keys(s).sort(), [...STATS_KEYS].sort());
  assertEquals(SUMMARY_VERSION, 2);
});

Deno.test("a report with no rounds, shop or pick backs nothing: every number is null", () => {
  const s = summarize(report({ start_health: {}, lobby: [] }));
  for (const key of STATS_KEYS) {
    assertEquals(s[key], null, key);
  }
});

Deno.test("results from the log are counted as they are, never inferred", () => {
  const s = summarize(report({
    rounds: [
      round(1, 2, { result: "won" }),
      round(2, 3, { result: "won" }),
      round(3, 2, { result: "lost" }),
      round(4, 3, { result: "tie" }),
      round(5, 2, { result: null }),
    ],
  }));
  assertEquals(
    [s.rounds, s.combats_won, s.combats_lost, s.combats_tied, s.combats_unknown],
    [5, 2, 1, 1, 1],
  );
  assertEquals(s.combats_inferred, 0);
  assertEquals([s.win_streak, s.win_streak_inferred], [2, false]);
});

Deno.test("without a logged result the result is worked out from health and marked inferred", () => {
  const s = summarize(report({
    rounds: [
      // Player 2 lost health, we did not: won (inferred).
      round(1, 2, { health_after: { "1": 30, "2": 25, "3": 30 }, own_health_after: 30 }),
      // We lost health, player 3 did not: lost.
      round(2, 3, { health_after: { "1": 22, "2": 25, "3": 30 }, own_health_after: 22 }),
      // Neither: tie.
      round(3, 2, { health_after: { "1": 22, "2": 25, "3": 30 }, own_health_after: 22 }),
      // Both lost health: unknown.
      round(4, 3, { health_after: { "1": 20, "2": 25, "3": 28 }, own_health_after: 20 }),
      // A side gained health: unknown.
      round(5, 2, { health_after: { "1": 20, "2": 27, "3": 28 }, own_health_after: 20 }),
    ],
  }));
  assertEquals(
    [s.combats_won, s.combats_lost, s.combats_tied, s.combats_unknown, s.combats_inferred],
    [1, 1, 1, 2, 3],
  );
  assertEquals([s.win_streak, s.win_streak_inferred], [1, true]);
});

Deno.test("an unknown opponent leaves an inferred result unknown, a logged one stays", () => {
  const s = summarize(report({
    rounds: [
      round(1, null, { result: "won" }),
      round(2, null, { health_after: { "1": 30, "2": 20 }, own_health_after: 30 }),
    ],
  }));
  assertEquals([s.combats_won, s.combats_unknown, s.combats_inferred], [1, 1, 0]);
});

Deno.test("a win streak needs rounds in a row; a gap or an unknown result ends it", () => {
  const s = summarize(report({
    rounds: [
      round(1, 2, { result: "won" }),
      round(2, 3, { result: "won" }),
      // Round 3 is missing from the log.
      round(4, 2, { result: "won" }),
      round(5, 3, { result: null }),
      round(6, 2, { result: "won" }),
    ],
  }));
  assertEquals(s.win_streak, 2);
});

Deno.test("a game whose results are all unknown has no win streak, not zero", () => {
  const s = summarize(report({ rounds: [round(1, 2, { result: null })] }));
  assertEquals([s.win_streak, s.win_streak_inferred], [null, null]);
  const lost = summarize(report({ rounds: [round(1, 2, { result: "lost" })] }));
  assertEquals([lost.win_streak, lost.win_streak_inferred], [0, false]);
});

Deno.test("damage taken is the own health drop between rounds in a row", () => {
  const s = summarize(report({
    rounds: [
      round(1, 2, { own_health_after: 30, result: "won" }),
      round(2, 3, { own_health_after: 24, result: "lost" }),
      // Healed during the shop: no drop, counts as 0.
      round(3, 2, { own_health_after: 26, result: "tie" }),
      // Round 4 missing: the pair 3 -> 5 is not in a row.
      round(5, 3, { own_health_after: 10, result: "lost" }),
      round(6, 2, { own_health_after: null, result: "lost" }),
    ],
  }));
  // start 30 -> 30 (0), 30 -> 24 (6), 24 -> 26 (0).
  assertEquals([s.damage_taken, s.damage_taken_combats, s.damage_taken_max], [6, 3, 6]);
});

Deno.test("damage dealt is the opponent's health drop in that round", () => {
  const s = summarize(report({
    rounds: [
      round(1, 2, { health_after: { "1": 30, "2": 23, "3": 30 }, own_health_after: 30 }),
      round(2, 3, { health_after: { "1": 30, "2": 23, "3": 21 }, own_health_after: 30 }),
      // Unknown opponent: not counted.
      round(3, null, { health_after: { "1": 30, "2": 20, "3": 21 }, own_health_after: 30 }),
      // No health after: not counted.
      round(4, 2, { own_health_after: 30 }),
    ],
  }));
  assertEquals([s.damage_dealt, s.damage_dealt_combats, s.damage_dealt_max], [16, 2, 9]);
});

/** A combat replay that ends with the game's own hit on a hero (T-215, parser revision 8). */
function withHit(side: string, amount: unknown): Json {
  return {
    replay: {
      start: 0,
      complete: true,
      minions: [],
      events: [{ kind: "attack" }, { kind: "hero_damage", side, amount }],
    },
  };
}

Deno.test("damage is the game's own hit at the end of the combat, not the health drop (D-108)", () => {
  const s = summarize(report({
    rounds: [
      // Lost 5 health, the game hit for 4 (1 health was paid in the shop): 4.
      round(1, 2, {
        own_health_after: 25,
        health_after: { "1": 25, "2": 30 },
        result: "lost",
        ...withHit("own", 4),
      }),
      // Won: the hit is on the opponent, health says 6, the hit says 7.
      round(2, 3, {
        own_health_after: 25,
        health_after: { "1": 25, "2": 30, "3": 24 },
        result: "won",
        ...withHit("opponent", 7),
      }),
    ],
  }));
  assertEquals([s.damage_taken, s.damage_taken_combats, s.damage_taken_max], [4, 2, 4]);
  assertEquals([s.damage_dealt, s.damage_dealt_combats, s.damage_dealt_max], [7, 2, 7]);
});

Deno.test("a combat with a hit counts without health after it; a recorded tie hit nobody", () => {
  const s = summarize(report({
    rounds: [
      round(1, 2, { own_health_after: 30, result: "won", ...withHit("opponent", 6) }),
      // A tie the game recorded, no replay: 0 taken and 0 dealt, from the log.
      round(2, 3, { own_health_after: null, result: "tie" }),
      // The last combat: no health after it, but the game's hit is there.
      round(3, 2, { own_health_after: null, result: "lost", ...withHit("own", 15) }),
    ],
  }));
  assertEquals([s.damage_taken, s.damage_taken_combats, s.damage_taken_max], [15, 3, 15]);
  assertEquals([s.damage_dealt, s.damage_dealt_combats, s.damage_dealt_max], [6, 3, 6]);
});

Deno.test("without the hit the health rule stays, and a combat with neither is left out", () => {
  const s = summarize(report({
    rounds: [
      round(1, 2, { own_health_after: 30, result: "won", ...withHit("opponent", 3) }),
      // No replay: health drop 30 -> 24.
      round(2, 3, {
        own_health_after: 24,
        health_after: { "1": 24, "2": 30, "3": 20 },
        result: "lost",
      }),
      // A replay cut short before the hit and no result: nothing to read.
      round(3, 2, {
        own_health_after: null,
        replay: { start: 0, complete: false, minions: [], events: [] },
      }),
    ],
  }));
  assertEquals([s.damage_taken, s.damage_taken_combats], [6, 2]);
});

Deno.test("a hit that is not a plain number in range is not used (the replay is untrusted)", () => {
  const bad = [
    withHit("own", "9"),
    withHit("own", 0),
    withHit("own", -3),
    withHit("own", 1001),
    withHit("own", 2.5),
    withHit("nobody", 4),
    withHit("constructor", 4),
    { replay: "x" },
    { replay: { events: "x" } },
    { replay: { events: [null, 5, { kind: "hero_damage" }] } },
    { replay: { events: { length: 1 } } },
  ];
  for (const extra of bad) {
    const s = summarize(report({
      rounds: [round(1, 2, { own_health_after: 22, result: "lost", ...extra })],
    }));
    // Falls back to the health drop, 30 -> 22.
    assertEquals([s.damage_taken, s.damage_taken_max], [8, 8], JSON.stringify(extra));
  }
});

Deno.test("the last hit of the replay is the one that counts", () => {
  const events = [
    { kind: "hero_damage", side: "own", amount: 2 },
    { kind: "hero_damage", side: "own", amount: 9 },
  ];
  const s = summarize(report({
    rounds: [
      round(1, 2, {
        own_health_after: 30,
        result: "lost",
        replay: { start: 0, complete: true, minions: [], events },
      }),
    ],
  }));
  assertEquals(s.damage_taken, 9);
});

Deno.test("an opponent already out (0 health) is not counted as damage dealt", () => {
  const s = summarize(report({
    start_health: { "1": 30, "2": 0 },
    rounds: [round(1, 2, { health_after: { "1": 30, "2": 0 }, own_health_after: 30 })],
  }));
  assertEquals([s.damage_dealt, s.damage_dealt_combats], [null, null]);
});

Deno.test("in Duos a team shares one health: the opposing team is one side", () => {
  const duos = {
    game_type: DUOS,
    local_player_id: 1,
    teammate_player_id: 2,
    teammate_hero: "BG22_HERO_000",
    final_place: 2,
    lobby: [
      { player_id: 1, hero: "A", duo_team: 1, final_health: 0 },
      { player_id: 2, hero: "B", duo_team: 1, final_health: 0 },
      { player_id: 3, hero: "C", duo_team: 2, final_health: 0 },
      { player_id: 4, hero: "D", duo_team: 2, final_health: 0 },
    ],
    start_health: { "1": 40, "2": 40, "3": 40, "4": 40 },
    rounds: [{
      number: 1,
      entries: [
        { side: "own", player_id: 1, hero: "A", board: [minion(2, 3)] },
        { side: "opponent", player_id: 3, hero: "C", board: [] },
        { side: "own", player_id: 2, hero: "B", board: [minion(5, 5)] },
        { side: "opponent", player_id: 4, hero: "D", board: [] },
      ],
      own_health_after: 40,
      health_after: { "1": 40, "2": 40, "3": 33, "4": 33 },
    }],
  };
  const s = summarize(report(duos));
  assertEquals([s.combats_won, s.combats_inferred], [1, 1]);
  assertEquals([s.damage_dealt, s.damage_dealt_combats], [7, 1]);
  assertEquals(s.teammate_hero, "BG22_HERO_000");
  // The final board is the local player's own, not the teammate's.
  assertEquals([s.final_board, s.board_stats_max], [["BG_A"], 5]);
});

Deno.test("the final board is the local player's last known board", () => {
  const s = summarize(report({
    rounds: [
      round(1, 2, {
        entries: [
          { side: "own", player_id: 1, board: [minion(1, 1), minion(4, 4, true)] },
          { side: "opponent", player_id: 2, board: [] },
        ],
      }),
      round(2, 3, {
        entries: [
          { side: "own", player_id: 1, board: [minion(3, 2, false, null), minion(10, 10)] },
          { side: "opponent", player_id: 3, board: null },
        ],
      }),
      // The board never played out in the log: unknown, so round 2 stays final.
      round(3, 2, { entries: [{ side: "own", player_id: 1, board: null }] }),
    ],
  }));
  assertEquals([s.final_board, s.final_board_round], [[null, "BG_A"], 2]);
  assertEquals([s.board_stats_max, s.golden_max], [25, 1]);
});

Deno.test("negative attack or health adds nothing to the board's stats", () => {
  const s = summarize(report({
    rounds: [round(1, 2, {
      entries: [{ side: "own", player_id: 1, board: [minion(-5, 3), minion(2, -1)] }],
    })],
  }));
  assertEquals(s.board_stats_max, 5);
});

Deno.test("shop totals sum the turns; gold counts say how many turns back them", () => {
  const turn = (n: number, over: Json = {}) => ({
    turn: n,
    start_ms: n * 1000,
    end_ms: n * 1000 + 500,
    tier: 1,
    gold: 3,
    gold_spent: 3,
    rolls: 2,
    free_rolls: 1,
    buys: ["BG_A", "BG_B"],
    spell_buys: 1,
    sells: ["BG_C"],
    freezes: 1,
    offers: [],
    actions: 4,
    extra_gold: 1,
    sell_gold: 1,
    buy_gold: 3,
    spell_gold: 0,
    free_refreshes: 0,
    passes: [],
    ...over,
  });
  const s = summarize(report({
    shop: {
      turns: [
        turn(1),
        turn(2, { gold_spent: null, extra_gold: null, tier: 2 }),
        // Turn 3 is missing; turn 4 is the last one.
        turn(4, { sell_gold: 2, tier: null }),
      ],
      tier_ups: [{ turn: 2, tier: 2 }, { turn: 4, tier: 3 }],
      actions: Array.from({ length: 30 }, (_, i) => ({ ms: i, turn: 1, kind: "buy" })),
      start_ms: 0,
      end_ms: 600_000,
      ended: true,
    },
  }));
  assertEquals(
    [s.shop_turns, s.rolls, s.free_rolls, s.buys, s.spell_buys, s.sells, s.freezes],
    [4, 6, 3, 6, 3, 3, 3],
  );
  assertEquals([s.gold_spent, s.gold_spent_turns], [6, 2]);
  assertEquals([s.sell_gold, s.sell_gold_turns], [4, 3]);
  // Both extra and sell gold known: turns 1 and 4.
  assertEquals([s.gold_beyond_income, s.gold_beyond_turns], [5, 2]);
  assertEquals([s.actions, s.apm], [30, 3]);
  assertEquals(s.final_tier, 3);
});

Deno.test("a shop with no logged action has no APM, not zero", () => {
  const s = summarize(report({
    shop: { turns: [], tier_ups: [], actions: [], start_ms: null, end_ms: null, ended: false },
  }));
  assertEquals([s.shop_turns, s.rolls, s.actions, s.apm], [null, null, null, null]);
});

Deno.test("APM is rounded to two decimals and needs a time span", () => {
  const shop = (end: number | null) => ({
    turns: [{
      turn: 1,
      start_ms: 0,
      end_ms: 1,
      tier: 1,
      gold: 3,
      gold_spent: 0,
      rolls: 0,
      free_rolls: 0,
      buys: [],
      spell_buys: 0,
      sells: [],
      freezes: 0,
      offers: [],
      actions: 1,
    }],
    tier_ups: [],
    actions: [{ ms: 0, turn: 1, kind: "roll" }],
    start_ms: 0,
    end_ms: end,
    ended: true,
  });
  assertEquals(summarize(report({ shop: shop(180_000) })).apm, 0.33);
  assertEquals(summarize(report({ shop: shop(null) })).apm, null);
  assertEquals(summarize(report({ shop: shop(0) })).apm, null);
  // A turn without the revision 4 gold fields backs no gold beyond income.
  assertEquals(summarize(report({ shop: shop(1) })).gold_beyond_income, null);
});

Deno.test("own triples: the leaderboard's last count in a finished game, else unknown", () => {
  const lobby = (triples: Json) => [
    { player_id: 1, hero: "A", final_health: 0, triples, trinkets: [] },
    { player_id: 2, hero: "B", final_health: 0, triples: [], trinkets: [] },
  ];
  const counted = [{ turn: 3, count: 1 }, { turn: 6, count: 2 }];
  assertEquals(summarize(report({ lobby: lobby(counted) })).triples, 2);
  assertEquals(summarize(report({ lobby: lobby([]) })).triples, 0);
  assertEquals(summarize(report({ status: "incomplete", lobby: lobby(counted) })).triples, null);
  // Saved before revision 5: no triples key at all.
  assertEquals(summarize(report()).triples, null);
});

Deno.test("the hero pick keeps the heroes offered and the rerolls", () => {
  const s = summarize(report({
    hero_select: { offered: ["H1", "H2_SKIN_A"], rerolls: 1, picked: "H1" },
  }));
  assertEquals([s.heroes_offered, s.hero_rerolls], [["H1", "H2_SKIN_A"], 1]);
});

Deno.test("every shared fixture summarizes without throwing and within the columns' bounds", () => {
  const reports: Json[] = [
    ...duo,
    ...solo,
    ...realDuos,
    ...realSolo,
    ...shopGames,
    ...dataGames,
    ...roundTwoGames,
  ];
  for (const r of reports) {
    const s: GameStats = summarize(r);
    for (const [key, value] of Object.entries(s)) {
      if (typeof value === "number") {
        if (!Number.isFinite(value) || value < 0 || value > 2 ** 31 - 1) {
          throw new Error(`${key} out of bounds: ${value}`);
        }
      }
    }
  }
});

Deno.test("the real Duos game: results from the log, own board and triples", () => {
  const s = summarize(realDuos[0]);
  assertEquals(s.teammate_hero, realDuos[0].teammate_hero);
  assertEquals(s.rounds, realDuos[0].rounds.length);
  assertEquals(
    s.combats_won! + s.combats_lost! + s.combats_tied! + s.combats_unknown!,
    realDuos[0].rounds.length,
  );
  assertEquals(s.triples, 1);
  assertEquals(s.heroes_offered, realDuos[0].hero_select.offered);
  assertEquals(s.shop_turns, realDuos[0].shop.turns.length);
});

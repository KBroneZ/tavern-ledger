// Synthetic game records for the game viewer's tests (T-504): made-up heroes,
// card ids and numbers, shaped like the parser's report (revision 7). No real
// player data, no real card names.

export const USER = "0f8fad5b-d9cb-469f-a165-70867728950e";
export const SESSION = "Hearthstone_2026_10_09_20_15_03";
export const INDEX = 2;

type Json = Record<string, unknown>;

const minion = (card_id: string | null, atk: number, health: number, extra: Json = {}): Json => ({
  card_id,
  atk,
  health,
  position: 0,
  golden: false,
  keywords: [],
  ...extra,
});

const turn = (n: number, extra: Json = {}): Json => ({
  turn: n,
  start_ms: n * 60_000,
  end_ms: n * 60_000 + 45_000,
  tier: 1,
  gold: 2 + n,
  gold_spent: 2 + n,
  rolls: 1,
  free_rolls: 0,
  buys: ["SYN_M_01"],
  spell_buys: 0,
  sells: [],
  freezes: 0,
  offers: [
    { card_id: "SYN_M_01", roll: 0, frozen: false },
    { card_id: "SYN_M_02", roll: 0, frozen: true },
    { card_id: "SYN_M_03", roll: 1, frozen: false },
  ],
  actions: 4,
  extra_gold: 0,
  sell_gold: 0,
  buy_gold: 3,
  spell_gold: 0,
  free_refreshes: 0,
  passes: [],
  ...extra,
});

/** A finished Solo game: 3 shop turns, 3 combats (log, inferred, unknown). */
export function soloReport(): Json {
  return {
    index: INDEX,
    status: "ok",
    game_type: "GT_BATTLEGROUNDS",
    build: 250000,
    local_player_id: 2,
    hero: "SYN_HERO_A",
    teammate_player_id: null,
    teammate_hero: null,
    card_names: { SYN_HERO_A: "Lantern Keeper", SYN_HERO_B: "Brass Captain", SYN_HERO_C: "Moss Witch" },
    final_place: 3,
    final_health: 12,
    lobby: [
      { player_id: 2, hero: "SYN_HERO_A", duo_team: null, final_place: 3, final_health: 12, triples: [{ turn: 3, count: 1 }], trinkets: [{ turn: 2, slot: "first", dbf_id: 900001 }] },
      { player_id: 3, hero: "SYN_HERO_B", duo_team: null, final_place: 1, final_health: 20, triples: [], trinkets: [] },
      { player_id: 4, hero: "SYN_HERO_C", duo_team: null, final_place: 5, final_health: 0, triples: [{ turn: 2, count: 1 }, { turn: 3, count: 2 }], trinkets: [] },
    ],
    shop_tribes: { BEAST: 3 },
    rounds: [
      {
        number: 1,
        entries: [
          { side: "own", player_id: 2, hero: "SYN_HERO_A", board: [minion("SYN_M_01", 2, 3)] },
          { side: "opponent", player_id: 3, hero: "SYN_HERO_B", board: [minion("SYN_M_04", 1, 1)] },
        ],
        own_health_after: 30,
        opponents: [3],
        health_after: { "2": 30, "3": 27, "4": 30 },
        result: "won",
      },
      {
        number: 2,
        entries: [
          { side: "own", player_id: 2, hero: "SYN_HERO_A", board: [minion("SYN_M_01", 2, 3), minion("SYN_M_02", 3, 3, { golden: true, keywords: ["taunt", "divine_shield"] })] },
          { side: "opponent", player_id: 4, hero: "SYN_HERO_C", board: [minion("SYN_M_05", 5, 5), minion(null, 1, 2)] },
        ],
        own_health_after: 25,
        opponents: [4],
        health_after: { "2": 25, "3": 27, "4": 30 },
        result: null,
      },
      {
        number: 3,
        entries: [
          { side: "own", player_id: 2, hero: "SYN_HERO_A", board: null },
          { side: "opponent", player_id: 3, hero: "SYN_HERO_B", board: [] },
        ],
        own_health_after: 12,
        opponents: [3],
        health_after: { "2": 12, "4": 30 },
        result: null,
      },
    ],
    start_health: { "2": 30, "3": 30, "4": 30 },
    shop: {
      turns: [turn(1), turn(2, { tier: 2, gold: null, gold_spent: null }), turn(3, { tier: 2, sells: ["SYN_M_01"], sell_gold: 1 })],
      tier_ups: [{ turn: 2, tier: 2 }],
      actions: [{ ms: 61_000, turn: 1, kind: "buy" }, { ms: 125_000, turn: 2, kind: "roll" }],
      start_ms: 60_000,
      end_ms: 240_000,
      ended: true,
    },
    hero_select: { offered: ["SYN_HERO_A", "SYN_HERO_D"], rerolls: 1, picked: "SYN_HERO_A" },
    skin_parents: {},
    discovers: [
      { turn: 2, source: "BG30_Trinket_1st", offered: [{ card_id: "SYN_TRINKET_1", gift: null }, { card_id: "SYN_TRINKET_2", gift: null }], picked: [1] },
      { turn: 3, source: "BG36_MidGameEffect_010", offered: [{ card_id: "SYN_M_06", gift: "SYN_GIFT_1" }], picked: [0] },
      { turn: 3, source: "TB_BaconShop_Triples_01", offered: [{ card_id: "SYN_M_07", gift: null }], picked: [] },
      { turn: 1, source: "SYN_QUEST_SRC", offered: [{ card_id: "SYN_Quest_01", gift: null }], picked: [0] },
      { turn: 3, source: "SYN_SPELL", offered: [{ card_id: "SYN_M_08", gift: null }], picked: [0] },
    ],
    old_god: 900100,
    warnings: ["synthetic warning"],
    problems: [],
    not_in_log: ["MMR", "available tribes (exact list)"],
  };
}

/** A Duos game: the local player (2) and teammate (3) against team 4+5. */
export function duosReport(): Json {
  return {
    index: INDEX,
    status: "ok",
    game_type: "GT_BATTLEGROUNDS_DUO",
    build: 250000,
    local_player_id: 2,
    hero: "SYN_HERO_A",
    teammate_player_id: 3,
    teammate_hero: "SYN_HERO_B",
    card_names: { SYN_HERO_A: "Lantern Keeper", SYN_HERO_B: "Brass Captain" },
    final_place: 2,
    final_health: 10,
    lobby: [
      { player_id: 2, hero: "SYN_HERO_A", duo_team: 1, final_place: 2, final_health: 10, triples: [], trinkets: [] },
      { player_id: 3, hero: "SYN_HERO_B", duo_team: 1, final_place: 2, final_health: 10, triples: [], trinkets: [] },
      { player_id: 4, hero: "SYN_HERO_C", duo_team: 2, final_place: 1, final_health: 20, triples: [], trinkets: [] },
      { player_id: 5, hero: "SYN_HERO_D", duo_team: 2, final_place: 1, final_health: 20, triples: [], trinkets: [] },
    ],
    shop_tribes: {},
    rounds: [
      {
        number: 1,
        entries: [
          { side: "own", player_id: 2, hero: "SYN_HERO_A", board: [] },
          { side: "opponent", player_id: 4, hero: "SYN_HERO_C", board: [] },
        ],
        own_health_after: 35,
        opponents: [4],
        health_after: { "2": 35, "3": 35, "4": 40, "5": 40 },
        result: null,
      },
    ],
    start_health: { "2": 40, "3": 40, "4": 40, "5": 40 },
    shop: null,
    hero_select: null,
    skin_parents: {},
    discovers: [],
    old_god: null,
    warnings: [],
    problems: [],
    not_in_log: [],
  };
}

export function record(report: Json = soloReport(), extra: Json = {}): Json {
  return { session: SESSION, index: INDEX, saved_at: 1791000000, parser: { version: "0.1.0", revision: 7 }, report, ...extra };
}

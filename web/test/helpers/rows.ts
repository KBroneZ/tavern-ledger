// Made-up rows shaped like the data API's views (T-502), for the T-503 tests.
// No player's data: invented numbers and card ids that look like the game's.

export type RawRow = Record<string, unknown>;

export function overviewRow(over: RawRow = {}): RawRow {
  return {
    user_id: "0f8fad5b-d9cb-469f-a165-70867728950e",
    mode: "solo",
    games: 31,
    incomplete_games: 1,
    unsupported_games: 0,
    placed_games: 30,
    avg_place: 3.42,
    top_half: 19,
    top_half_pct: 63.3,
    wins: 4,
    win_pct: 13.3,
    first_played_on: "2026-09-01",
    last_played_on: "2026-10-09",
    summarized_games: 29,
    combats_won: 120,
    combats_lost: 100,
    combats_tied: 10,
    combats_unknown: 3,
    combats_inferred: 4,
    combat_games: 29,
    damage_dealt: 900,
    damage_dealt_games: 29,
    damage_taken: 800,
    damage_taken_games: 29,
    gold_spent: 4060,
    gold_spent_games: 29,
    gold_beyond_income: 290,
    gold_beyond_games: 20,
    sell_gold: 87,
    sell_gold_games: 29,
    rolls: 290,
    free_rolls: 30,
    buys: 400,
    sells: 87,
    shop_games: 29,
    avg_apm: 38.25,
    apm_games: 29,
    triples: 58,
    triples_games: 28,
    ...over,
  };
}

export function gameRaw(over: RawRow = {}): RawRow {
  return {
    session: "Hearthstone_2026_10_09_18_30_00",
    game_index: 2,
    mode: "solo",
    status: "ok",
    played_on: "2026-10-09",
    hero: "BG20_HERO_202",
    counted_place: 3,
    rounds: 13,
    combats_won: 7,
    combats_lost: 5,
    combats_inferred: 0,
    triples: 2,
    apm: 41.2,
    gold_spent: 140,
    ...over,
  };
}

export function heroRaw(over: RawRow = {}): RawRow {
  return { mode: "solo", hero: "BG20_HERO_202", games: 5, placed_games: 5, avg_place: 3.2, top_half: 3, wins: 1, best_place: 1, ...over };
}

export function recordRaw(over: RawRow = {}): RawRow {
  return { mode: "solo", record: "win_streak", value: 6, played_on: "2026-10-02", games: 29, inferred: false, ...over };
}

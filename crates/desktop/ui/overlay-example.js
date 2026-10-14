// Made-up game shown while the layout is being edited and no game is on, so
// there is something to arrange. Invented names and numbers, labelled on the
// page as example data; never mixed with a real game.
"use strict";

const EXAMPLE_GAME = {
  phase: "playing",
  is_duos: false,
  game_type: { value: "GT_BATTLEGROUNDS", source: "log" },
  hero: { value: { id: "EXAMPLE_HERO", name: "Example Hero" }, source: "log" },
  teammate_hero: { value: null, source: "unknown" },
  last_combat: { value: 5, source: "log" },
  turn: { value: 6, source: "log" },
  own_seats: [3],
  own_place: { value: 2, source: "log" },
  leaderboard_slots: 3,
  own_health: { value: 32, source: "log" },
  damage_cap: 15,
  // A few counters (T-612) so the panel can be arranged with no game.
  counters: [
    { kind: "blood_gem", atk: 3, health: 2 },
    { kind: "tavern_spell", atk: 2, health: 0 },
    { kind: "eternal_knights", count: 5 },
    { kind: "free_rolls", count: 2 },
  ],
  odds: {
    seat: 1, gap: null, seen_round: 5, won: 0.58, tied: 0.09, lost: 0.33, trials: 10000,
    dealt: { low: 5, likely: 7, high: 10 }, taken: { low: 4, likely: 6, high: 9 },
    approximate: [], missing: [], start_of_combat: false, new_build: null,
  },
  tribes: [
    { tribe: "BEAST", offers: 6 },
    { tribe: "MURLOC", offers: 4 },
  ],
  tribes_source: "inferred",
  entered_tribes: ["BEAST", "MURLOC", "UNDEAD", "DEMON", "DRAGON"],
  entered_tribes_source: "entered",
  game_tribes: { tribes: ["BEAST", "DEMON", "DRAGON", "MURLOC", "UNDEAD"], status: "ok", checked_build: true, note: null },
  lobby_tribes: { tribes: ["BEAST", "DEMON", "DRAGON", "MURLOC", "UNDEAD"], source: "game", basis: "game", complete: true },
  opponents: [
    {
      heroes: [{ id: "EXAMPLE_A", name: "Example Opponent A" }],
      seats: [1],
      health: { value: 28, source: "log" },
      boards: [
        {
          hero: null,
          round: 5,
          minions: [
            { card_id: "EXAMPLE_1", atk: 4, health: 5, golden: false, keywords: ["taunt", "divine_shield"] },
            { card_id: "EXAMPLE_2", atk: 8, health: 9, golden: true, keywords: ["deathrattle"], gifts: ["BG36_MidGameEffect_000t9"] },
          ],
        },
      ],
      boards_source: "log",
      record: { won: 1, lost: 1, tie: 0, unknown: 0 },
      record_source: "inferred",
      place: { value: 1, source: "log" },
      tiers: [{ value: 4, source: "log" }],
      possible: [],
      shown: [{ seat: 1, triples: 2, trinkets: [{ slot: "first", dbf_id: 1, card_id: "EXAMPLE_TRINKET" }] }],
      shown_source: "log",
      next: true,
    },
    {
      heroes: [{ id: "EXAMPLE_B", name: "Example Opponent B" }],
      seats: [2],
      health: { value: null, source: "unknown" },
      boards: [],
      boards_source: "unknown",
      record: null,
      record_source: "unknown",
      place: { value: 3, source: "log" },
      tiers: [{ value: 2, source: "log" }],
      possible: [],
      shown: [{ seat: 2, triples: null, trinkets: [] }],
      shown_source: "log",
      next: false,
    },
  ],
  warnings: [],
  problems: [],
  legend: [
    { source: "log", label: "from the log" },
    { source: "inferred", label: "inferred" },
    { source: "entered", label: "entered by you" },
    { source: "leaderboard", label: "from the leaderboard" },
    { source: "card_data", label: "card database" },
    { source: "possible", label: "possible" },
    { source: "unknown", label: "unknown" },
  ],
};

// The hero select panel's example (T-365), shown while arranging with no game
// on: four heroes, one of them never played.
const EXAMPLE_HERO_OFFER = {
  rerolls: 0,
  heroes: [
    { hero: { id: "EXAMPLE_HERO_A", name: "Example Hero A" }, own: { games: 6, placed: 6, average_place: 3.5, top_half_share: 4 / 6 } },
    { hero: { id: "EXAMPLE_HERO_B", name: "Example Hero B" }, own: null },
    { hero: { id: "EXAMPLE_HERO_C", name: "Example Hero C" }, own: { games: 2, placed: 2, average_place: 5.5, top_half_share: 0.5 } },
    { hero: { id: "EXAMPLE_HERO_D", name: "Example Hero D" }, own: { games: 11, placed: 10, average_place: 2.9, top_half_share: 0.8 } },
  ],
};

// The lobby panel's example (T-338), shown while arranging with no game on.
const EXAMPLE_LOBBY = {
  in_lobby: true,
  rating: { solo: 6400, duos: 5800, status: "ok" },
  rating_change: { duos: false, delta: 45 },
  today: [{ mode: "GT_BATTLEGROUNDS", top_half: 4, tally: { games: 3, placed: 3, average_place: 3.7, top_half_share: 2 / 3 } }],
};

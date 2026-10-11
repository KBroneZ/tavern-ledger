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
  odds: {
    seat: 1, gap: null, seen_round: 5, won: 0.58, tied: 0.09, lost: 0.33, trials: 10000,
    dealt: { low: 5, likely: 7, high: 10 }, taken: { low: 4, likely: 6, high: 9 },
    approximate: [], missing: [], start_of_combat: false,
  },
  tribes: [
    { tribe: "BEAST", offers: 6 },
    { tribe: "MURLOC", offers: 4 },
  ],
  tribes_source: "inferred",
  entered_tribes: ["BEAST", "MURLOC", "UNDEAD", "DEMON", "DRAGON"],
  entered_tribes_source: "entered",
  lobby_tribes: { tribes: ["BEAST", "DEMON", "DRAGON", "MURLOC", "UNDEAD"], source: "entered", basis: "entered", complete: true },
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

// The lobby panel's example (T-338), shown while arranging with no game on.
const EXAMPLE_LOBBY = {
  in_lobby: true,
  rating: { solo: 6400, duos: 5800, status: "ok" },
  rating_change: { duos: false, delta: 45 },
  today: [{ mode: "GT_BATTLEGROUNDS", top_half: 4, tally: { games: 3, placed: 3, average_place: 3.7, top_half_share: 2 / 3 } }],
};

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
  own_health: { value: 32, source: "log" },
  tribes: [
    { tribe: "BEAST", offers: 6 },
    { tribe: "MURLOC", offers: 4 },
  ],
  tribes_source: "inferred",
  entered_tribes: ["BEAST", "MURLOC", "UNDEAD", "DEMON", "DRAGON"],
  entered_tribes_source: "entered",
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
            { card_id: "EXAMPLE_1", atk: 4, health: 5, golden: false },
            { card_id: "EXAMPLE_2", atk: 8, health: 9, golden: true },
          ],
        },
      ],
      boards_source: "log",
      record: { won: 1, lost: 1, tie: 0, unknown: 0 },
      record_source: "inferred",
    },
    {
      heroes: [{ id: "EXAMPLE_B", name: "Example Opponent B" }],
      seats: [2],
      health: { value: null, source: "unknown" },
      boards: [],
      boards_source: "unknown",
      record: null,
      record_source: "unknown",
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
    { source: "unknown", label: "unknown" },
  ],
};

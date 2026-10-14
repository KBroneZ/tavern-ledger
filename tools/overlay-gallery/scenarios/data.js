// Example data for the gallery: the shapes the app sends the overlay page
// (tracker/src/live.rs `LiveGame`), filled with invented names and numbers
// only, like ui/overlay-example.js. Every card and hero id starts with EX_ so
// it can never be a real card. Builders return new objects; a scenario
// changes a base game with a spread, never in place.
"use strict";

// Invented names, in both languages: id -> { en, es }.
const NAMES = {
  EX_HERO_BRINDLE: { en: "Captain Brindle", es: "Capitana Brindle" },
  EX_HERO_MATE: { en: "Pip the Cook", es: "Pip el Cocinero" },
  EX_HERO_QUILL: { en: "Madam Quill", es: "Madame Quill" },
  EX_HERO_TARN: { en: "Old Tarn", es: "El viejo Tarn" },
  EX_HERO_PELLAM: { en: "Sir Pellam", es: "Sir Pellam" },
  EX_HERO_ORRIN: { en: "Dame Orrin", es: "Dama Orrin" },
  EX_HERO_ROOK: { en: "Rook the Lamplighter", es: "Rook el Farolero" },
  EX_HERO_FENWICK: { en: "Fenwick", es: "Fenwick" },
  EX_HERO_MAUD: { en: "Maud Ashby", es: "Maud Ashby" },
  EX_HERO_GRIST: { en: "Grist the Miller", es: "Grist el Molinero" },
  EX_HERO_VESPER: { en: "Vesper Lane", es: "Vesper Lane" },
  EX_HERO_DOWSE: { en: "Dowse", es: "Dowse" },
  EX_HERO_HALE: { en: "Hale Wick", es: "Hale Wick" },
  EX_M_HOUND: { en: "Cinder Hound", es: "Sabueso de ceniza" },
  EX_M_SPRITE: { en: "Tide Sprite", es: "Duende de marea" },
  EX_M_BRUTE: { en: "Moss Brute", es: "Bruto de musgo" },
  EX_M_WARDEN: { en: "Bone Warden", es: "Guardián de hueso" },
  EX_M_IMP: { en: "Gilded Imp", es: "Diablillo dorado" },
  EX_M_CRAWLER: { en: "Reef Crawler", es: "Reptador del arrecife" },
  EX_M_WHELP: { en: "Lantern Whelp", es: "Cría de farol" },
  EX_M_GOLEM: { en: "Brass Golem", es: "Gólem de latón" },
  EX_M_MOTH: { en: "Dusk Moth", es: "Polilla del ocaso" },
  EX_T_COMPASS: { en: "Cracked Compass", es: "Brújula rota" },
  EX_T_LOCKET: { en: "Silver Locket", es: "Relicario de plata" },
  EX_T_BELL: { en: "Ship's Bell", es: "Campana de barco" },
};

const sourced = (value, source = "log") => ({ value, source });
const hero = (id) => ({ id, name: NAMES[id].en });
const minion = (card_id, atk, health, keywords = [], golden = false) => ({ card_id, atk, health, golden, keywords, gifts: [] });
const board = (round, minions, seat) => ({ ...(seat === undefined ? {} : { seat }), hero: null, round, minions });

const LEGEND = [
  { source: "log", label: "from the log" },
  { source: "inferred", label: "inferred" },
  { source: "entered", label: "entered by you" },
  { source: "leaderboard", label: "from the leaderboard" },
  { source: "card_data", label: "card database" },
  { source: "possible", label: "possible" },
  { source: "unknown", label: "unknown" },
];

// Boards the example opponents were last seen with.
const BOARDS = {
  quill: [
    minion("EX_M_BRUTE", 6, 9, ["taunt"]),
    minion("EX_M_HOUND", 9, 4, ["windfury"]),
    minion("EX_M_WARDEN", 5, 7, ["deathrattle", "reborn"], true),
    minion("EX_M_IMP", 3, 3, []),
    minion("EX_M_SPRITE", 4, 6, ["divine_shield"]),
  ],
  tarn: [
    minion("EX_M_GOLEM", 12, 12, ["taunt", "divine_shield"]),
    minion("EX_M_MOTH", 7, 2, ["stealth", "venomous"]),
    minion("EX_M_WHELP", 5, 5, ["rally"]),
  ],
  pellam: [
    minion("EX_M_CRAWLER", 8, 8, ["deathrattle"]),
    minion("EX_M_SPRITE", 5, 5, []),
    minion("EX_M_HOUND", 7, 3, ["windfury"]),
    minion("EX_M_BRUTE", 4, 11, ["taunt"]),
  ],
};

function opponent({ heroes, seats, health, place, tiers, boards = [], record = null, shown, possible = [], next = false }) {
  return {
    heroes: heroes.map(hero),
    seats,
    health: sourced(health, health === null ? "unknown" : "log"),
    boards,
    boards_source: boards.length ? "log" : "unknown",
    record,
    record_source: record ? "inferred" : "unknown",
    place: sourced(place),
    tiers: tiers.map((t) => sourced(t)),
    possible,
    shown: shown || seats.map((seat) => ({ seat, triples: null, trinkets: [] })),
    shown_source: "log",
    next,
  };
}

const record = (won, lost, tie = 0) => ({ won, lost, tie, unknown: 0 });
const trinket = (card_id, slot, dbf_id) => ({ slot, dbf_id, card_id });

// A Solo game in a shop turn: the player third, seven opponents.
function soloOpponents() {
  return [
    opponent({
      heroes: ["EX_HERO_QUILL"], seats: [1], health: 34, place: 1, tiers: [5], next: true,
      boards: [board(6, BOARDS.quill, 1)], record: record(2, 1),
      shown: [{ seat: 1, triples: 2, trinkets: [trinket("EX_T_COMPASS", "first", 9001), trinket("EX_T_BELL", "second", 9003)] }],
    }),
    opponent({
      heroes: ["EX_HERO_TARN"], seats: [2], health: 29, place: 2, tiers: [5],
      boards: [board(5, BOARDS.tarn, 2)], record: record(1, 1),
      shown: [{ seat: 2, triples: 1, trinkets: [trinket("EX_T_LOCKET", "first", 9002)] }],
    }),
    opponent({
      heroes: ["EX_HERO_PELLAM"], seats: [4], health: 24, place: 4, tiers: [4],
      boards: [board(4, BOARDS.pellam, 4)], record: record(1, 0),
      shown: [{ seat: 4, triples: 0, trinkets: [] }],
    }),
    opponent({ heroes: ["EX_HERO_ORRIN"], seats: [5], health: 21, place: 5, tiers: [4], shown: [{ seat: 5, triples: 1, trinkets: [trinket("EX_T_LOCKET", "first", 9002)] }] }),
    opponent({ heroes: ["EX_HERO_ROOK"], seats: [6], health: 17, place: 6, tiers: [3] }),
    opponent({ heroes: ["EX_HERO_FENWICK"], seats: [7], health: 12, place: 7, tiers: [3], shown: [{ seat: 7, triples: 0, trinkets: [] }] }),
    opponent({ heroes: ["EX_HERO_MAUD"], seats: [8], health: 8, place: 8, tiers: [2] }),
  ];
}

const SOLO_ODDS = {
  seat: 1, gap: null, seen_round: 6, boards: [], won: 0.58, tied: 0.09, lost: 0.33, trials: 10000,
  dealt: { low: 5, likely: 8, high: 12 }, taken: { low: 4, likely: 6, high: 11 },
  approximate: [], missing: [], start_of_combat: false,
};

// The five tribes of the example lobby, as the game's memory gives them.
const GAME_TRIBES = ["BEAST", "DEMON", "DRAGON", "MURLOC", "UNDEAD"];

function soloGame() {
  return {
    phase: "playing",
    is_duos: false,
    game_type: sourced("GT_BATTLEGROUNDS"),
    hero: sourced(hero("EX_HERO_BRINDLE")),
    teammate_hero: sourced(null, "unknown"),
    last_combat: sourced(6),
    turn: sourced(7),
    own_seats: [3],
    own_place: sourced(3),
    leaderboard_slots: 8,
    own_health: sourced(27),
    tribes: [
      { tribe: "BEAST", offers: 9 },
      { tribe: "MURLOC", offers: 6 },
      { tribe: "DEMON", offers: 4 },
    ],
    tribes_source: "inferred",
    entered_tribes: [],
    entered_tribes_source: "unknown",
    game_tribes: { tribes: GAME_TRIBES, status: "ok", checked_build: true, note: null },
    build: 253216,
    lobby_tribes: { tribes: GAME_TRIBES, source: "game", basis: "game", complete: true },
    opponents: soloOpponents(),
    record_basis: "log",
    legend: LEGEND,
    warnings: [],
    problems: [],
    damage_cap: null,
    odds: null,
  };
}

// A Duos game: four teams of two; the player's team is third.
function duosOpponents() {
  const pair = (a, b) => ({ heroes: [a, b] });
  return [
    opponent({
      ...pair("EX_HERO_QUILL", "EX_HERO_GRIST"), seats: [1, 2], health: 31, place: 1, tiers: [5, 4], next: true,
      boards: [board(6, BOARDS.quill, 1), board(6, BOARDS.tarn, 2)], record: record(2, 0),
      shown: [
        { seat: 1, triples: 2, trinkets: [trinket("EX_T_COMPASS", "first", 9001)] },
        { seat: 2, triples: 0, trinkets: [] },
      ],
    }),
    opponent({
      ...pair("EX_HERO_PELLAM", "EX_HERO_VESPER"), seats: [3, 4], health: 26, place: 2, tiers: [4, 4],
      boards: [board(5, BOARDS.pellam, 3), board(5, BOARDS.tarn, 4)], record: record(1, 1),
      shown: [
        { seat: 3, triples: 1, trinkets: [] },
        { seat: 4, triples: 1, trinkets: [trinket("EX_T_LOCKET", "first", 9002)] },
      ],
    }),
    opponent({
      ...pair("EX_HERO_ORRIN", "EX_HERO_DOWSE"), seats: [7, 8], health: 14, place: 4, tiers: [3, 3],
      shown: [{ seat: 7, triples: 0, trinkets: [] }, { seat: 8, triples: 0, trinkets: [] }],
    }),
  ];
}

function duosGame() {
  return {
    ...soloGame(),
    is_duos: true,
    game_type: sourced("GT_BATTLEGROUNDS_DUO"),
    teammate_hero: sourced(hero("EX_HERO_MATE")),
    own_seats: [5, 6],
    own_place: sourced(3),
    leaderboard_slots: 4,
    own_health: sourced(30),
    opponents: duosOpponents(),
  };
}

// A game at hero select: no lobby yet, the heroes on offer each with the
// player's own numbers (null: no game with it yet). The lobby's tribes come
// from the game's memory, as in a played game.
const own = (games, placed, average_place, top_half_share) => ({ games, placed, average_place, top_half_share });
function heroSelectGame({ duos = false, heroes, rerolls = 0 }) {
  return {
    ...soloGame(),
    phase: "starting",
    is_duos: duos,
    game_type: sourced(duos ? "GT_BATTLEGROUNDS_DUO" : "GT_BATTLEGROUNDS"),
    hero: sourced(null, "unknown"),
    last_combat: sourced(null),
    turn: sourced(null),
    own_seats: [],
    own_place: sourced(null),
    leaderboard_slots: null,
    own_health: sourced(null),
    opponents: [],
    hero_offer: { rerolls, heroes: heroes.map(([id, numbers]) => ({ hero: hero(id), own: numbers })) },
  };
}

const DUOS_BOARDS = (rounds) => [
  { role: "teammate", seat: 6, round: rounds[0] },
  { role: "opponent", seat: 1, round: rounds[1] },
  { role: "opponent_teammate", seat: 2, round: rounds[2] },
];

module.exports = {
  NAMES, sourced, hero, minion, board, opponent, record, trinket, soloGame, duosGame, heroSelectGame, own, SOLO_ODDS, DUOS_BOARDS, BOARDS,
};

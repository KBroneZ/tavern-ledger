// The gallery's scenarios: each is one frame of the real overlay page at a
// fixed game-window size, with the example data the stand-in answers with and
// a caption in English and Spanish.
"use strict";

const data = require("./data.js");
const { CAPTIONS } = require("./captions.js");

const { sourced, soloGame, duosGame, SOLO_ODDS, DUOS_BOARDS } = data;

const SCREEN_W = 1920;
const SCREEN_H = 1080;
// Where the app puts the hover card (screen_fit::place_card): centred across
// the top of the game window, as wide as the game's own board (85% of its height).
const CARD_WIDTH = 0.85 * SCREEN_H;
const HOVER_CARD = { x: (SCREEN_W - CARD_WIDTH) / 2, y: 8, width: CARD_WIDTH, max_height: SCREEN_H - 16 };
// A slot of the game's leaderboard, at the left edge of the screen.
const hover = (place, member) => ({ place, member, slot: { x: 24, y: 120 + place * 96, w: 150, h: 92 }, card: HOVER_CARD });

const withOdds = (game, odds) => ({ ...game, odds });
const gap = (name, extra = {}) => ({
  seat: 1, gap: name, seen_round: null, boards: [], won: 0, tied: 0, lost: 0, trials: 0,
  dealt: null, taken: null, approximate: [], missing: [], start_of_combat: false, ...extra,
});

const lobby = {
  in_lobby: true,
  rating: { solo: 6420, duos: 5870, status: "ok" },
  rating_change: { duos: false, delta: 38 },
  today: [
    { mode: "GT_BATTLEGROUNDS", top_half: 4, top_half_games: 2, tally: { games: 3, placed: 3, average_place: 3.7, top_half_share: 2 / 3 } },
    { mode: "GT_BATTLEGROUNDS_DUO", top_half: 2, top_half_games: 1, tally: { games: 1, placed: 1, average_place: 2, top_half_share: 1 } },
  ],
};

const solo = soloGame();
const soloWithOdds = withOdds(solo, SOLO_ODDS);
const duos = duosGame();
const duosOdds = {
  ...SOLO_ODDS, seat: 1, seen_round: 5, boards: DUOS_BOARDS([6, 5, 5]), won: 0.47, tied: 0.12, lost: 0.41,
};

// The game's own mark for the opponent being fought goes with the game.
const gameOver = {
  ...solo,
  phase: "over",
  turn: sourced(12),
  last_combat: sourced(12),
  own_health: sourced(0),
  opponents: solo.opponents.map((o) => ({ ...o, next: false })),
};

const unreadable = {
  ...solo,
  phase: "unreadable",
  hero: sourced(null, "unknown"),
  opponents: [],
  tribes: [],
  lobby_tribes: { tribes: [], source: "unknown", basis: "unknown", complete: false },
};

const countersList = [
  { kind: "blood_gem", atk: 3, health: 2 },
  { kind: "tavern_spell", atk: 2, health: 0 },
  { kind: "undead_attack", atk: 4 },
  { kind: "eternal_knights", count: 5 },
  { kind: "shop_elemental", atk: 4, health: 4 },
  { kind: "free_rolls", count: 2 },
];

// The part of the game window each frame is about, for the zoomed view that
// phones start with: { x, y, w, h } in game-window pixels.
const STACK_SMALL = { x: 1700, y: 90, w: 220, h: 490 };
const STACK_TALL = { x: 1530, y: 90, w: 390, h: 990 };
const STACK_DUOS = { x: 1700, y: 90, w: 220, h: 280 };
const STATUS_BOX = { x: 1500, y: 90, w: 420, h: 150 };
const HOVER_CARD_AREA = { x: 480, y: 0, w: 960, h: 300 };
const LOBBY_AREA = { x: 0, y: 0, w: 290, h: 180 };
const FOCUS = {
  waiting: STATUS_BOX,
  lobby: LOBBY_AREA,
  solo: STACK_SMALL,
  duos: STACK_DUOS,
  "gap-no-board-seen": STACK_SMALL,
  "gap-no-card-data": STACK_SMALL,
  "gap-not-modelled": STACK_SMALL,
  "gap-bad-board": STACK_SMALL,
  "gap-order": STACK_DUOS,
  "gap-missing-board": STACK_DUOS,
  "hover-solo": HOVER_CARD_AREA,
  "hover-duos": HOVER_CARD_AREA,
  "damage-cap": STACK_SMALL,
  unreadable: STATUS_BOX,
  "game-over": STACK_SMALL,
  "high-contrast": STACK_SMALL,
  counters: STACK_SMALL,
};

// [id, game, extra state]
const DEFINITIONS = [
  ["waiting", null, {}],
  ["lobby", null, { lobby }],
  ["solo", soloWithOdds, {}],
  ["duos", withOdds(duos, duosOdds), {}],
  ["gap-no-board-seen", withOdds(solo, gap("no_board_seen")), {}],
  ["gap-no-card-data", withOdds(solo, gap("no_card_data", { seen_round: 6 })), {}],
  ["gap-not-modelled", withOdds(solo, gap("not_modelled", { seen_round: 6, missing: ["EX_M_WARDEN"] })), {}],
  ["gap-bad-board", withOdds(solo, gap("bad_board", { seen_round: 6 })), {}],
  ["gap-order", withOdds(duos, gap("order")), {}],
  ["gap-missing-board", withOdds(duos, gap("missing_board", { boards: DUOS_BOARDS([6, null, null]) })), {}],
  ["hover-solo", soloWithOdds, { hover: hover(1, null) }],
  ["hover-duos", withOdds(duos, duosOdds), { hover: hover(1, 0) }],
  ["damage-cap", { ...solo, damage_cap: 15 }, {}],
  ["unreadable", unreadable, {}],
  ["game-over", gameOver, {}],
  ["high-contrast", soloWithOdds, { settings: { theme: "contrast" } }],
  ["counters", { ...soloWithOdds, counters: countersList }, {}],
];

const SCENARIOS = DEFINITIONS.map(([id, game, extra]) => ({
  id,
  state: {
    game,
    scene: null,
    lobby: null,
    hover: null,
    ...extra,
    settings: { theme: "glass", ...extra.settings },
    names: data.NAMES,
  },
  focus: FOCUS[id],
  caption: CAPTIONS[id],
}));

// The counters panel is a slot of the overlay page (data-panel="counters");
// its scenario is shown once the page has it switched on (the slot is not
// `hidden`: the panel is off until the overlay rework).
const COUNTERS_SLOT = /<div class="slot"(?![^>]*\bhidden\b)[^>]*data-panel="counters"/;

function scenariosFor(overlayHtml) {
  const hasCounters = COUNTERS_SLOT.test(overlayHtml);
  return SCENARIOS.filter((s) => s.id !== "counters" || hasCounters);
}

module.exports = { SCENARIOS, scenariosFor, SCREEN_W, SCREEN_H };

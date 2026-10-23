// The gallery's scenarios: each is one frame of the real overlay page at a
// fixed game-window size, with the example data the stand-in answers with and
// a caption in English and Spanish.
"use strict";

const data = require("./data.js");
const { CAPTIONS } = require("./captions.js");

const { sourced, minion, soloGame, duosGame, heroSelectGame, own, SOLO_ODDS, DUOS_BOARDS } = data;

const SCREEN_W = 1920;
const SCREEN_H = 1080;
// Where the app puts the hover card (screen_fit::place_card): centred across
// the top of the game's picture, as wide as the game's own board (85% of its height).
const HOVER_CARD = require("../default-layout.js").hoverCard(SCREEN_W, SCREEN_H);
// A slot of the game's leaderboard, at the left edge of the screen.
const hover = (place, member) => ({ place, member, slot: { x: 24, y: 120 + place * 96, w: 150, h: 92 }, card: HOVER_CARD });

const withOdds = (game, odds) => ({ ...game, odds });

// The first opponent's last board replaced by `minions` (the hover card at place 1).
const withBoard = (game, minions) => ({
  ...game,
  opponents: game.opponents.map((o) => (o.place && o.place.value === 1 ? { ...o, boards: [{ ...o.boards[0], minions }] } : o)),
});
// Seven minions that between them wear every keyword the game shows (T-372).
const KEYWORD_BOARD = (golden = false) => [
  minion("EX_M_BRUTE", 6, 9, ["taunt", "divine_shield"], golden),
  minion("EX_M_HOUND", 9, 4, ["windfury", "stealth"], golden),
  minion("EX_M_WARDEN", 5, 7, ["deathrattle", "reborn"], golden),
  minion("EX_M_MOTH", 7, 2, ["venomous", "poisonous"], golden),
  minion("EX_M_WHELP", 5, 5, ["rally"], golden),
  minion("EX_M_GOLEM", 1999, 12, ["taunt", "divine_shield", "reborn", "venomous"], golden),
  minion("EX_M_IMP", 3, 3, [], golden),
];
// Seven minions all with Windfury and Stealth: the worst case for the effects.
const EFFECTS_BOARD = ["EX_M_BRUTE", "EX_M_HOUND", "EX_M_WARDEN", "EX_M_MOTH", "EX_M_WHELP", "EX_M_GOLEM", "EX_M_IMP"].map((id, i) =>
  minion(id, 3 + i, 4 + i, ["windfury", "stealth"]),
);
const PLAIN_BOARD = EFFECTS_BOARD.map((m) => ({ ...m, keywords: [] }));
const gap = (name, extra = {}) => ({
  seat: 1, gap: name, round: null, boards: [], won: 0, tied: 0, lost: 0, trials: 0,
  dealt: null, taken: null, approximate: [], missing: [], start_of_combat: false, ...extra,
});

const tile = (hero, place, change, duos = false) => ({ hero: { id: hero, name: data.NAMES[hero].en }, duos, place, change });
const HEROES = ["EX_HERO_BRINDLE", "EX_HERO_QUILL", "EX_HERO_TARN", "EX_HERO_PELLAM", "EX_HERO_ORRIN", "EX_HERO_ROOK", "EX_HERO_FENWICK", "EX_HERO_MAUD"];
// Eight Solo games, the oldest first, the app's own order.
const EIGHT = [
  tile(HEROES[0], 3, 22), tile(HEROES[1], 7, -41), tile(HEROES[2], 1, 74), tile(HEROES[3], 5, -9),
  tile(HEROES[4], 2, 51), tile(HEROES[5], 4, 12), tile(HEROES[6], 8, -63), tile(HEROES[7], 3, 28),
];
const summary = (solo, duos) => [
  { mode: "GT_BATTLEGROUNDS", top_half: 4, top_half_games: solo[0], tally: { games: solo[1], placed: solo[1], average_place: solo[2], top_half_share: solo[0] / solo[1] } },
  ...(duos ? [{ mode: "GT_BATTLEGROUNDS_DUO", top_half: 2, top_half_games: duos[0], tally: { games: duos[1], placed: duos[1], average_place: duos[2], top_half_share: duos[0] / duos[1] } }] : []),
];
const lobby = {
  in_lobby: true,
  rating: { solo: 6420, duos: 5870, status: "ok" },
  today: summary([5, 8, 4.1]),
  recap: { games: 8, starting: { solo: 6366, duos: 5870 }, tiles: EIGHT },
};
// The same games with the read off: games and places, no rating anywhere.
const lobbyNoRating = {
  in_lobby: true,
  rating: { solo: null, duos: null, status: "off" },
  today: summary([5, 8, 4.1]),
  recap: { games: 8, starting: { solo: null, duos: null }, tiles: EIGHT.map((t) => ({ ...t, change: null })) },
};
// A short session: fewer than eight games, one of them not finished in the log.
const lobbyFew = {
  in_lobby: true,
  rating: { solo: 6481, duos: 5870, status: "ok" },
  today: [{ mode: "GT_BATTLEGROUNDS", top_half: 4, top_half_games: 2, tally: { games: 3, placed: 2, average_place: 3.5, top_half_share: 1 } }],
  recap: { games: 3, starting: { solo: 6433, duos: 5870 }, tiles: [tile(HEROES[0], 2, 30), tile(HEROES[1], null, null), tile(HEROES[2], 5, null)] },
};
// Duos games among Solo ones: each tile says its mode and its change is in it.
const lobbyDuos = {
  in_lobby: true,
  rating: { solo: 6420, duos: 5911, status: "ok" },
  today: summary([1, 2, 4.5], [2, 3, 2]),
  recap: {
    games: 5,
    starting: { solo: 6395, duos: 5840 },
    tiles: [tile(HEROES[0], 6, -25), tile("EX_HERO_MATE", 1, 52, true), tile(HEROES[2], 2, 31), tile("EX_HERO_VESPER", 2, 9, true), tile("EX_HERO_GRIST", 3, -20, true)],
  },
};

const solo = soloGame();
const soloWithOdds = withOdds(solo, SOLO_ODDS);
// The same odds as a new game build gives them (the warning line) and with a
// board only approximated: the tallest the bar gets.
const tallestOdds = { ...SOLO_ODDS, approximate: ["EX_M_WARDEN"], new_build: 255026 };
const duos = duosGame();
const duosOdds = {
  ...SOLO_ODDS, seat: 1, round: 6, boards: DUOS_BOARDS([6, 6, 6]), won: 0.47, tied: 0.12, lost: 0.41,
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

// Hero select: four heroes on offer in Solo (one never played), the same offer
// after a reroll (the new hero has no games yet), two in Duos, and a new player.
const heroOffer = heroSelectGame({
  heroes: [
    ["EX_HERO_BRINDLE", own(9, 9, 3.1, 7 / 9)],
    ["EX_HERO_QUILL", null],
    ["EX_HERO_TARN", own(2, 2, 5.5, 0.5)],
    ["EX_HERO_ROOK", own(14, 13, 2.8, 0.85)],
  ],
});
const heroRerolled = heroSelectGame({
  rerolls: 1,
  heroes: [
    ["EX_HERO_BRINDLE", own(9, 9, 3.1, 7 / 9)],
    ["EX_HERO_FENWICK", null],
    ["EX_HERO_TARN", own(2, 2, 5.5, 0.5)],
    ["EX_HERO_ROOK", own(14, 13, 2.8, 0.85)],
  ],
});
const heroOfferDuos = heroSelectGame({
  duos: true,
  heroes: [
    ["EX_HERO_MATE", own(5, 5, 2.4, 0.8)],
    ["EX_HERO_VESPER", null],
  ],
});
const heroOfferNew = heroSelectGame({
  heroes: [["EX_HERO_BRINDLE", null], ["EX_HERO_QUILL", null], ["EX_HERO_TARN", null], ["EX_HERO_ROOK", null]],
});

const unreadable = {
  ...solo,
  phase: "unreadable",
  hero: sourced(null, "unknown"),
  opponents: [],
  tribes: [],
  lobby_tribes: { tribes: [], source: "unknown", basis: "unknown", complete: false },
};

// The same odds with no damage ranges: the app does not know them.
const noRanges = { ...SOLO_ODDS, dealt: null, taken: null };

// The lobby's tribes when the game's memory does not give them (D-148): no
// tribes at all on the overlay, never a guess.
const noTribes = (status) => ({
  ...solo,
  game_tribes: { tribes: [], status, checked_build: false, note: null },
  lobby_tribes: { tribes: [], source: "unknown", basis: "unknown", complete: false },
});

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
const LOBBY_AREA = { x: 0, y: 0, w: 290, h: 330 };
const ODDS_BAR_AREA = { x: 660, y: 0, w: 600, h: 100 };
const LEADERBOARD_AREA = { x: 60, y: 140, w: 300, h: 800 };
const TRIBES_AREA = { x: 1700, y: 90, w: 220, h: 120 };
const HEROES_AREA = { x: 1690, y: 90, w: 230, h: 520 };
const HEROES_DUOS_AREA = { x: 1690, y: 90, w: 230, h: 290 };
const FOCUS = {
  waiting: STATUS_BOX,
  lobby: LOBBY_AREA,
  "lobby-no-rating": LOBBY_AREA,
  "lobby-few-games": LOBBY_AREA,
  "lobby-duos": LOBBY_AREA,
  solo: STACK_SMALL,
  duos: STACK_DUOS,
  "gap-no-board-seen": STACK_SMALL,
  "gap-no-card-data": STACK_SMALL,
  "gap-not-modelled": STACK_SMALL,
  "gap-bad-board": STACK_SMALL,
  "gap-missing-board": STACK_DUOS,
  "hover-solo": HOVER_CARD_AREA,
  crowded: HOVER_CARD_AREA,
  "hover-duos": HOVER_CARD_AREA,
  "hover-keywords": HOVER_CARD_AREA,
  "hover-golden": HOVER_CARD_AREA,
  "hover-effects-seven": HOVER_CARD_AREA,
  "hover-plain-seven": HOVER_CARD_AREA,
  "hover-textures-none": HOVER_CARD_AREA,
  "hover-texture-bad": HOVER_CARD_AREA,
  "hover-no-webgl": HOVER_CARD_AREA,
  "damage-cap": STACK_SMALL,
  unreadable: STATUS_BOX,
  "game-over": STACK_SMALL,
  "high-contrast": STACK_SMALL,
  counters: STACK_SMALL,
  "odds-bar": ODDS_BAR_AREA,
  "odds-bar-no-ranges": ODDS_BAR_AREA,
  "leaderboard-marks": LEADERBOARD_AREA,
  "tribes-game": TRIBES_AREA,
  "tribes-off": TRIBES_AREA,
  "tribes-unsupported": TRIBES_AREA,
  "tribes-no-lobby": TRIBES_AREA,
  "hero-select": HEROES_AREA,
  "hero-select-reroll": HEROES_AREA,
  "hero-select-duos": HEROES_DUOS_AREA,
  "hero-select-new": HEROES_AREA,
};

// [id, game, extra state]
const DEFINITIONS = [
  ["waiting", null, {}],
  ["lobby", null, { lobby }],
  ["lobby-no-rating", null, { lobby: lobbyNoRating }],
  ["lobby-few-games", null, { lobby: lobbyFew }],
  ["lobby-duos", null, { lobby: lobbyDuos }],
  ["solo", soloWithOdds, {}],
  ["duos", withOdds(duos, duosOdds), {}],
  ["gap-no-board-seen", withOdds(solo, gap("no_board_seen")), {}],
  ["gap-no-card-data", withOdds(solo, gap("no_card_data", { round: 6 })), {}],
  ["gap-not-modelled", withOdds(solo, gap("not_modelled", { round: 6, missing: ["EX_M_WARDEN"] })), {}],
  ["gap-bad-board", withOdds(solo, gap("bad_board", { round: 6 })), {}],
  ["gap-missing-board", withOdds(duos, gap("missing_board", { round: 6, boards: DUOS_BOARDS([6, 6, null]) })), {}],
  ["hover-solo", soloWithOdds, { hover: hover(1, null) }],
  ["crowded", withOdds(solo, tallestOdds), { hover: hover(1, null) }],
  ["hover-duos", withOdds(duos, duosOdds), { hover: hover(1, 0) }],
  ["hover-keywords", withBoard(soloWithOdds, KEYWORD_BOARD()), { hover: hover(1, null) }],
  ["hover-golden", withBoard(soloWithOdds, KEYWORD_BOARD(true)), { hover: hover(1, null) }],
  ["hover-effects-seven", withBoard(soloWithOdds, EFFECTS_BOARD), { hover: hover(1, null) }],
  ["hover-plain-seven", withBoard(soloWithOdds, PLAIN_BOARD), { hover: hover(1, null) }],
  ["hover-textures-none", withBoard(soloWithOdds, KEYWORD_BOARD()), { hover: hover(1, null), textures: "none" }],
  ["hover-texture-bad", withBoard(soloWithOdds, KEYWORD_BOARD()), { hover: hover(1, null), textures: { bad: ["skull", "taunt_normal", "StealthDomeMask"] } }],
  ["hover-no-webgl", withBoard(soloWithOdds, KEYWORD_BOARD()), { hover: hover(1, null), noWebgl: true }],
  ["damage-cap", { ...solo, damage_cap: 15 }, {}],
  ["unreadable", unreadable, {}],
  ["game-over", gameOver, {}],
  ["high-contrast", soloWithOdds, { settings: { theme: "contrast" } }],
  ["counters", { ...soloWithOdds, counters: countersList }, {}],
  ["odds-bar", soloWithOdds, {}],
  ["odds-bar-no-ranges", withOdds(solo, noRanges), {}],
  ["leaderboard-marks", soloWithOdds, {}],
  ["tribes-game", solo, {}],
  ["tribes-off", noTribes("off"), {}],
  ["tribes-unsupported", noTribes("unsupported_version"), {}],
  ["tribes-no-lobby", noTribes("no_lobby"), {}],
  ["hero-select", heroOffer, {}],
  ["hero-select-reroll", heroRerolled, {}],
  ["hero-select-duos", heroOfferDuos, {}],
  ["hero-select-new", heroOfferNew, {}],
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

// What an uploaded game may hold (T-104d). The record is untrusted input: it
// is checked field by field against the report the desktop app writes
// (crates/bg-parser/src/report.rs) and refused if anything is outside it. It
// must never carry player names, BattleTags, ratings or MMR (D-012, D-017):
// there is no field for them, a BattleTag-shaped string anywhere is refused,
// and hero names are only accepted for heroes the report shows.
//
// The keys it checks at each level are listed in report_keys.json, which a
// cargo test ties to the parser's report (T-104h, D-047): a parser that
// writes a new key fails CI until the key is listed there and checked here.

import contract from "./report_keys.json" with { type: "json" };

/**
 * Goes up by one each time the validator starts accepting records it used to
 * refuse. Every answer carries it, and the desktop app retries a game this
 * server refused once when it sees a higher number (D-047).
 */
export const VALIDATOR_VERSION = 3;

export const SESSION_PATTERN = /^Hearthstone_(\d{4})_(\d{2})_(\d{2})_\d{2}_\d{2}_\d{2}$/;
const PATH_PATTERN = /\/v1\/games\/(Hearthstone_\d{4}(?:_\d{2}){5})\/([1-9][0-9]{0,3})$/;
const MAX_INDEX = 1000;
const CARD_ID = /^[A-Za-z0-9_]{1,64}$/;
const TRIBE = /^[A-Z0-9_]{1,32}$/;
// Warnings and problems are short sentences the parser writes itself.
const NOTE = /^[A-Za-z0-9 _:.()-]{1,160}$/;
const NOT_IN_LOG = new Set(["MMR", "available tribes (exact list)"]);
const PARSER_VERSION = /^\d{1,4}\.\d{1,4}\.\d{1,4}$/;
// Name#1234 and look-alikes of '#'.
const BATTLETAG = /[#＃﹟]\s*\d{3,6}/;
// Hero names in the game's language: letters, marks, digits, punctuation,
// symbols and spaces; no markup, '@', '#' or control characters.
const NAME = /^[\p{L}\p{M}\p{N}\p{P}\p{S}\p{Zs}]{1,64}$/u;
const NAME_FORBIDDEN = /[<>#＃﹟@＠\\"]/;
const MAX_INT = 2 ** 31 - 1;
// Health maps: player id ("1"-"64") -> health + armor - damage, never below 0.
const PLAYER_KEY = /^[1-9][0-9]?$/;
const MAX_HEALTH = 100000;
const MAX_HEALTH_ENTRIES = 16;
// The shop record (parser revision 3, T-204, T-205, D-046): the parser's own
// bounds (crates/bg-parser/src/shop.rs). Times are ms from the game's first
// log line, never a clock time; a day is far longer than any game.
const MAX_SHOP_TURNS = 100;
const MAX_OFFERS = 400;
const MAX_GAME_OFFERS = 3000;
const MAX_CARDS = 100;
const MAX_ACTIONS = 3000;
const MAX_MS = 24 * 60 * 60 * 1000;
const MAX_COUNT = 10000;
const ACTION_KINDS = new Set([
  "roll",
  "buy",
  "buy_spell",
  "sell",
  "freeze",
  "unfreeze",
  "tier_up",
  "hero_power",
  "play",
  "move",
  "choose",
  "other",
]);

/**
 * The keys checked at each level; must equal report_keys.json. Level names
 * stay quoted: crates/bg-parser/tests/upload_contract.rs reads this table.
 */
const KEYS: Record<string, string[]> = {
  "record": ["session", "index", "saved_at", "parser", "report"],
  "parser": ["version", "revision"],
  "report": [
    "index",
    "status",
    "game_type",
    "build",
    "local_player_id",
    "hero",
    "teammate_player_id",
    "teammate_hero",
    "card_names",
    "final_place",
    "final_health",
    "lobby",
    "shop_tribes",
    "rounds",
    "start_health",
    "shop",
    "warnings",
    "problems",
    "not_in_log",
  ],
  "report.lobby[]": ["player_id", "hero", "duo_team", "final_place", "final_health"],
  "report.rounds[]": ["number", "entries", "own_health_after", "opponents", "health_after"],
  "report.rounds[].entries[]": ["side", "player_id", "hero", "board"],
  "report.rounds[].entries[].board[]": ["card_id", "atk", "health", "position", "golden"],
  "report.shop": ["turns", "tier_ups", "actions", "start_ms", "end_ms", "ended"],
  "report.shop.turns[]": [
    "turn",
    "start_ms",
    "end_ms",
    "tier",
    "gold",
    "gold_spent",
    "rolls",
    "free_rolls",
    "buys",
    "spell_buys",
    "sells",
    "freezes",
    "offers",
    "actions",
  ],
  "report.shop.turns[].offers[]": ["card_id", "roll", "frozen"],
  "report.shop.tier_ups[]": ["turn", "tier"],
  "report.shop.actions[]": ["ms", "turn", "kind"],
};

// A validator out of step with the list stops the function at start: games
// wait in the app (5xx) instead of being refused or stored unchecked.
const sameKeys = (a: string[] | undefined, b: string[]) =>
  a !== undefined && [...a].sort().join() === [...b].sort().join();
const listed: Record<string, string[]> = contract.keys;
if (
  !sameKeys(Object.keys(listed), Object.keys(KEYS)) ||
  Object.entries(KEYS).some(([level, keys]) => !sameKeys(listed[level], keys))
) {
  throw new Error("validate.ts and report_keys.json list different keys");
}

export type GameType = "GT_BATTLEGROUNDS" | "GT_BATTLEGROUNDS_DUO";
const MAX_PLACE: Record<GameType, number> = { GT_BATTLEGROUNDS: 8, GT_BATTLEGROUNDS_DUO: 4 };

/** The summary row written next to the file (public.games). */
export interface Summary {
  game_type: GameType;
  status: "ok" | "incomplete" | "unsupported";
  hero_card_id: string | null;
  final_place: number | null;
  build: number | null;
  played_on: string;
  tribes_offered: Record<string, number>;
  saved_at: number;
  parser_version: string | null;
}

export type Validation =
  | { ok: true; summary: Summary }
  | { ok: false; code: "invalid_report" | "player_name"; field: string };

/** `{session, index}` from `.../v1/games/<session>/<index>`, or null. */
export function parsePath(url: string): { session: string; index: number } | null {
  let path: string;
  try {
    const u = new URL(url);
    if (u.search !== "") return null;
    path = u.pathname;
  } catch (_e) {
    return null;
  }
  const m = PATH_PATTERN.exec(path);
  if (!m) return null;
  const index = Number(m[2]);
  return index <= MAX_INDEX ? { session: m[1], index } : null;
}

class Invalid extends Error {
  constructor(readonly field: string) {
    super(field);
  }
}

type Obj = Record<string, unknown>;

function isObject(v: unknown): v is Obj {
  return typeof v === "object" && v !== null && !Array.isArray(v);
}

/** The object's keys must all be in `allowed`; those in `required` must exist. */
function shape(v: unknown, field: string, allowed: string[], required: string[] = []): Obj {
  if (!isObject(v)) throw new Invalid(field);
  for (const key of Object.keys(v)) {
    if (!allowed.includes(key)) throw new Invalid(field ? `${field}.${key}` : key);
  }
  for (const key of required) {
    if (!(key in v) || v[key] === undefined) throw new Invalid(field ? `${field}.${key}` : key);
  }
  return v;
}

function int(v: unknown, field: string, min: number, max: number): number {
  if (typeof v !== "number" || !Number.isInteger(v) || v < min || v > max) {
    throw new Invalid(field);
  }
  return v;
}

function optInt(v: unknown, field: string, min: number, max: number): number | null {
  return v === null || v === undefined ? null : int(v, field, min, max);
}

function cardId(v: unknown, field: string): string | null {
  if (v === null || v === undefined) return null;
  if (typeof v !== "string" || !CARD_ID.test(v)) throw new Invalid(field);
  return v;
}

function list(v: unknown, field: string, max: number): unknown[] {
  if (!Array.isArray(v) || v.length > max) throw new Invalid(field);
  return v;
}

function notes(v: unknown, field: string): void {
  if (v === undefined) return;
  list(v, field, 16).forEach((s, i) => {
    if (typeof s !== "string" || !NOTE.test(s)) throw new Invalid(`${field}[${i}]`);
  });
}

function minion(v: unknown, field: string): void {
  const m = shape(v, field, KEYS["report.rounds[].entries[].board[]"], [
    "atk",
    "health",
  ]);
  cardId(m.card_id, `${field}.card_id`);
  int(m.atk, `${field}.atk`, -MAX_INT, MAX_INT);
  int(m.health, `${field}.health`, -MAX_INT, MAX_INT);
  optInt(m.position, `${field}.position`, 0, 64);
  if (m.golden !== undefined && typeof m.golden !== "boolean") {
    throw new Invalid(`${field}.golden`);
  }
}

/** Hero card ids and player ids the report shows, outside the health maps. */
interface Seen {
  heroes: Set<string>;
  players: Set<number>;
}

function entry(v: unknown, field: string, seen: Seen): void {
  const e = shape(v, field, KEYS["report.rounds[].entries[]"], ["side"]);
  if (e.side !== "own" && e.side !== "opponent") throw new Invalid(`${field}.side`);
  const pid = optInt(e.player_id, `${field}.player_id`, 1, 64);
  if (pid !== null) seen.players.add(pid);
  const hero = cardId(e.hero, `${field}.hero`);
  if (hero) seen.heroes.add(hero);
  if (e.board !== null && e.board !== undefined) {
    list(e.board, `${field}.board`, 16).forEach((m, i) => minion(m, `${field}.board[${i}]`));
  }
}

function round(v: unknown, field: string, seen: Seen): Obj {
  const r = shape(v, field, KEYS["report.rounds[]"], ["number"]);
  int(r.number, `${field}.number`, 0, 1000);
  if (r.entries !== undefined) {
    list(r.entries, `${field}.entries`, 8).forEach((e, i) =>
      entry(e, `${field}.entries[${i}]`, seen)
    );
  }
  optInt(r.own_health_after, `${field}.own_health_after`, -100000, 100000);
  if (r.opponents !== undefined) {
    list(r.opponents, `${field}.opponents`, 8).forEach((p, i) =>
      optInt(p, `${field}.opponents[${i}]`, 1, 64)
    );
  }
  return r;
}

/**
 * Health by player id (`start_health`, `rounds[].health_after`): only
 * players this report shows elsewhere, so a key can never carry a name.
 */
function healthMap(v: unknown, field: string, players: Set<number>): void {
  if (v === undefined) return;
  if (!isObject(v) || Object.keys(v).length > MAX_HEALTH_ENTRIES) throw new Invalid(field);
  for (const [pid, health] of Object.entries(v)) {
    if (!PLAYER_KEY.test(pid) || !players.has(Number(pid))) throw new Invalid(field);
    int(health, field, 0, MAX_HEALTH);
  }
}

function bool(v: unknown, field: string): void {
  if (typeof v !== "boolean") throw new Invalid(field);
}

function cards(v: unknown, field: string): void {
  list(v, field, MAX_CARDS).forEach((c, i) => cardId(c, `${field}[${i}]`));
}

function shopTurn(v: unknown, field: string): void {
  const t = shape(v, field, KEYS["report.shop.turns[]"], KEYS["report.shop.turns[]"]);
  int(t.turn, `${field}.turn`, 1, MAX_SHOP_TURNS);
  const start = int(t.start_ms, `${field}.start_ms`, 0, MAX_MS);
  int(t.end_ms, `${field}.end_ms`, start, MAX_MS);
  optInt(t.tier, `${field}.tier`, 1, 7);
  optInt(t.gold, `${field}.gold`, 0, MAX_COUNT);
  optInt(t.gold_spent, `${field}.gold_spent`, 0, MAX_COUNT);
  const rolls = int(t.rolls, `${field}.rolls`, 0, MAX_COUNT);
  int(t.free_rolls, `${field}.free_rolls`, 0, rolls);
  cards(t.buys, `${field}.buys`);
  int(t.spell_buys, `${field}.spell_buys`, 0, MAX_COUNT);
  cards(t.sells, `${field}.sells`);
  int(t.freezes, `${field}.freezes`, 0, MAX_COUNT);
  list(t.offers, `${field}.offers`, MAX_OFFERS).forEach((o, i) => {
    const at = `${field}.offers[${i}]`;
    const offer = shape(o, at, KEYS["report.shop.turns[].offers[]"], ["roll", "frozen"]);
    cardId(offer.card_id, `${at}.card_id`);
    int(offer.roll, `${at}.roll`, 0, rolls);
    bool(offer.frozen, `${at}.frozen`);
  });
  int(t.actions, `${field}.actions`, 0, MAX_ACTIONS);
}

/** The player's own shop and logged actions; null when the game was unreadable. */
function shop(v: unknown, field: string): void {
  if (v === undefined || v === null) return;
  const s = shape(v, field, KEYS["report.shop"], ["turns", "tier_ups", "actions", "ended"]);
  const turns = list(s.turns, `${field}.turns`, MAX_SHOP_TURNS);
  turns.forEach((t, i) => shopTurn(t, `${field}.turns[${i}]`));
  const offers = turns.reduce((n: number, t) => n + (t as { offers: unknown[] }).offers.length, 0);
  if (offers > MAX_GAME_OFFERS) throw new Invalid(`${field}.turns`);
  list(s.tier_ups, `${field}.tier_ups`, MAX_SHOP_TURNS).forEach((u, i) => {
    const at = `${field}.tier_ups[${i}]`;
    const up = shape(u, at, KEYS["report.shop.tier_ups[]"], KEYS["report.shop.tier_ups[]"]);
    int(up.turn, `${at}.turn`, 1, MAX_SHOP_TURNS);
    int(up.tier, `${at}.tier`, 2, 7);
  });
  list(s.actions, `${field}.actions`, MAX_ACTIONS).forEach((a, i) => {
    const at = `${field}.actions[${i}]`;
    const action = shape(a, at, KEYS["report.shop.actions[]"], KEYS["report.shop.actions[]"]);
    int(action.ms, `${at}.ms`, 0, MAX_MS);
    int(action.turn, `${at}.turn`, 1, MAX_SHOP_TURNS);
    if (typeof action.kind !== "string" || !ACTION_KINDS.has(action.kind)) {
      throw new Invalid(`${at}.kind`);
    }
  });
  const start = optInt(s.start_ms, `${field}.start_ms`, 0, MAX_MS);
  optInt(s.end_ms, `${field}.end_ms`, start ?? 0, MAX_MS);
  bool(s.ended, `${field}.ended`);
}

function lobbyPlayer(v: unknown, field: string, seen: Seen): void {
  const p = shape(v, field, KEYS["report.lobby[]"], ["player_id"]);
  seen.players.add(int(p.player_id, `${field}.player_id`, 1, 64));
  const hero = cardId(p.hero, `${field}.hero`);
  if (hero) seen.heroes.add(hero);
  optInt(p.duo_team, `${field}.duo_team`, 1, 8);
  optInt(p.final_place, `${field}.final_place`, 1, 8);
  optInt(p.final_health, `${field}.final_health`, -100000, 100000);
}

function tribes(v: unknown, field: string): Record<string, number> {
  if (v === undefined) return {};
  if (!isObject(v) || Object.keys(v).length > 32) throw new Invalid(field);
  const out: Record<string, number> = {};
  for (const [tribe, n] of Object.entries(v)) {
    if (!TRIBE.test(tribe) || typeof n !== "number" || !Number.isInteger(n) || n < 0 || n > 10000) {
      throw new Invalid(field);
    }
    out[tribe] = n;
  }
  return out;
}

function heroNames(v: unknown, field: string, heroes: Set<string>): void {
  if (v === undefined) return;
  if (!isObject(v) || Object.keys(v).length > 32) throw new Invalid(field);
  for (const [id, name] of Object.entries(v)) {
    // Only the heroes this report shows, as the parser writes it.
    if (!CARD_ID.test(id) || !heroes.has(id)) throw new Invalid(field);
    if (
      typeof name !== "string" || !NAME.test(name) || NAME_FORBIDDEN.test(name) ||
      name.trim() !== name
    ) {
      throw new Invalid(field);
    }
  }
}

/** Any string, key or value, shaped like a BattleTag. */
function hasBattleTag(v: unknown, depth = 0): boolean {
  if (depth > 16) return false; // deeper than any report; the shape check refuses it
  if (typeof v === "string") return BATTLETAG.test(v);
  if (Array.isArray(v)) return v.some((x) => hasBattleTag(x, depth + 1));
  if (isObject(v)) {
    return Object.entries(v).some(([k, x]) => BATTLETAG.test(k) || hasBattleTag(x, depth + 1));
  }
  return false;
}

function playedOn(session: string): string {
  const m = SESSION_PATTERN.exec(session);
  if (!m) throw new Invalid("session");
  const iso = `${m[1]}-${m[2]}-${m[3]}`;
  const d = new Date(`${iso}T00:00:00Z`);
  if (Number.isNaN(d.getTime()) || d.toISOString().slice(0, 10) !== iso) {
    throw new Invalid("session");
  }
  return iso;
}

function check(value: unknown, session: string, index: number): Summary {
  if (!isObject(value)) throw new Invalid("record");
  const rec = shape(value, "", KEYS["record"], [
    "session",
    "index",
    "saved_at",
    "report",
  ]);
  if (rec.session !== session) throw new Invalid("session");
  if (rec.index !== index) throw new Invalid("index");
  const played = playedOn(session);
  const savedAt = int(rec.saved_at, "saved_at", 0, 32503680000);
  let parserVersion: string | null = null;
  if (rec.parser !== undefined && rec.parser !== null) {
    const p = shape(rec.parser, "parser", KEYS["parser"], ["version", "revision"]);
    if (typeof p.version !== "string" || !PARSER_VERSION.test(p.version)) {
      throw new Invalid("parser.version");
    }
    const revision = int(p.revision, "parser.revision", 0, 999999);
    parserVersion = `${p.version}+r${revision}`;
  }

  const r = shape(rec.report, "report", KEYS["report"], ["index", "status", "game_type"]);
  if (r.index !== index) throw new Invalid("report.index");
  if (r.status !== "ok" && r.status !== "incomplete" && r.status !== "unsupported") {
    throw new Invalid("report.status");
  }
  if (r.game_type !== "GT_BATTLEGROUNDS" && r.game_type !== "GT_BATTLEGROUNDS_DUO") {
    throw new Invalid("report.game_type");
  }
  const gameType = r.game_type as GameType;
  const build = optInt(r.build, "report.build", 1, 100000000);
  const seen: Seen = { heroes: new Set(), players: new Set() };
  for (const key of ["local_player_id", "teammate_player_id"]) {
    const pid = optInt(r[key], `report.${key}`, 1, 64);
    if (pid !== null) seen.players.add(pid);
  }
  const hero = cardId(r.hero, "report.hero");
  if (hero) seen.heroes.add(hero);
  const mate = cardId(r.teammate_hero, "report.teammate_hero");
  if (mate) seen.heroes.add(mate);
  const finalPlace = optInt(r.final_place, "report.final_place", 1, MAX_PLACE[gameType]);
  optInt(r.final_health, "report.final_health", -100000, 100000);
  if (r.lobby !== undefined) {
    list(r.lobby, "report.lobby", 16).forEach((p, i) => lobbyPlayer(p, `report.lobby[${i}]`, seen));
  }
  const offered = tribes(r.shop_tribes, "report.shop_tribes");
  const rounds = r.rounds === undefined
    ? []
    : list(r.rounds, "report.rounds", 200).map((x, i) => round(x, `report.rounds[${i}]`, seen));
  // After every round, so a player seen in any combat counts.
  healthMap(r.start_health, "report.start_health", seen.players);
  rounds.forEach((x, i) =>
    healthMap(x.health_after, `report.rounds[${i}].health_after`, seen.players)
  );
  shop(r.shop, "report.shop");
  notes(r.warnings, "report.warnings");
  notes(r.problems, "report.problems");
  if (r.not_in_log !== undefined) {
    list(r.not_in_log, "report.not_in_log", 8).forEach((s, i) => {
      if (typeof s !== "string" || !NOT_IN_LOG.has(s)) {
        throw new Invalid(`report.not_in_log[${i}]`);
      }
    });
  }
  heroNames(r.card_names, "report.card_names", seen.heroes);

  return {
    game_type: gameType,
    status: r.status,
    hero_card_id: hero,
    final_place: finalPlace,
    build,
    played_on: played,
    tribes_offered: offered,
    saved_at: savedAt,
    parser_version: parserVersion,
  };
}

/**
 * Checks one uploaded record against the address it was sent to. A refusal
 * names the field, never its value, so the answer cannot echo what was sent.
 */
export function validateRecord(value: unknown, session: string, index: number): Validation {
  if (hasBattleTag(value)) return { ok: false, code: "player_name", field: "record" };
  try {
    return { ok: true, summary: check(value, session, index) };
  } catch (e) {
    if (e instanceof Invalid) return { ok: false, code: "invalid_report", field: e.field };
    throw e;
  }
}

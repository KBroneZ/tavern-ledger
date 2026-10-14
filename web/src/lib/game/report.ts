// Reads one stored game record (T-504) into plain typed data. The file is
// untrusted input even though the server checked it at upload: keys the site
// does not know are ignored, a value outside the server's own bounds
// (supabase/functions/upload-game/validate.ts) becomes unknown (null) and is
// counted in `unreadable`, and nothing here throws on a bad value. Only a
// record that is not an object, or names another game, is refused whole.
//
// No free text passes except hero names (the game's own, checked like the
// server does, BattleTag shapes refused) and the parser's short notes.

import { type Buff, buffsRecorded, MAX_BUFF_COUNT, MAX_BUFF_KINDS, oneEachCard } from "./buffs.ts";
import { KEYWORDS, type Keyword } from "./keywords.ts";
import { readReplay, type ReplayRead } from "./replayRead.ts";
import { playedOn } from "./url.ts";

export { KEYWORDS, type Keyword };

/** The parser revision this site knows (crates/bg-parser PARSER_REVISION). */
export const KNOWN_REVISION = 14;

const CARD_ID = /^[A-Za-z0-9_]{1,64}$/;
const NOTE = /^[A-Za-z0-9 _:.()-]{1,160}$/;
const PARSER_VERSION = /^\d{1,4}\.\d{1,4}\.\d{1,4}$/;
const PLAYER_KEY = /^[1-9][0-9]?$/;
const MAX_INT = 2 ** 31 - 1;
const MAX_HEALTH = 100_000;
const MAX_COUNT = 10_000;
const MAX_DBF_ID = 10_000_000;
// The parser keeps at most 4 Dark Gifts on a minion (crates/bg-parser/src/gifts.rs).
const MAX_GIFTS = 4;
const MAX_TURN = 100;
const MAX_GAME_OFFERS = 3000;

// A board's game-long values (parser revision 10, T-610): the names the
// parser saves (crates/bg-parser/src/report.rs CombatValues), each 1 to 99 999.
export const GAME_VALUES = [
  "blood_gem_atk",
  "blood_gem_health",
  "spell_atk",
  "spell_health",
  "knights_dead",
  "undead_atk",
  "beetle_atk",
  "beetle_health",
] as const;
export type GameValue = (typeof GAME_VALUES)[number];
/** Only the values above zero the record holds; a name left out is not known (the printed numbers). */
export type GameValues = Readonly<Partial<Record<GameValue, number>>>;
const MAX_GAME_VALUE = 99_999;

export type Outcome = "won" | "lost" | "tie";
export type TrinketSlot = "first" | "second" | "hero_power";

export interface Minion {
  cardId: string | null;
  atk: number | null;
  health: number | null;
  golden: boolean;
  keywords: Keyword[];
  /** Dark Gifts as gift card ids (parser revision 9, T-528); [] when none or older. */
  gifts: string[];
  /** The game marks a Dark Gift the log does not name. */
  giftUnknown: boolean;
  /** Buffs the minion has, by card (revision 14, T-616); [] when none; null when the record is older and did not read them. */
  buffs: Buff[] | null;
}

export interface Entry {
  side: "own" | "opponent";
  playerId: number | null;
  hero: string | null;
  /** null: no board in the record (unknown); [] is a real empty board. */
  board: Minion[] | null;
  /** The board's game-long values (revision 10); null when the record holds none. */
  values: GameValues | null;
}

export interface Round {
  number: number;
  entries: Entry[];
  ownHealthAfter: number | null;
  healthAfter: ReadonlyMap<number, number>;
  /** The result the game records; null when the log does not say. */
  result: Outcome | null;
  /** The combat replay (parser revision 8, T-536): null when the round has none; not ok when it cannot be read. */
  replay: ReplayRead | null;
}

export interface Seat {
  playerId: number;
  hero: string | null;
  duoTeam: number | null;
  finalPlace: number | null;
  /** null: saved before the leaderboard was kept (parser revision 5). */
  triples: { turn: number; count: number }[] | null;
  trinkets: { turn: number; slot: TrinketSlot; dbfId: number }[] | null;
}

export interface Offer {
  cardId: string | null;
  roll: number;
  frozen: boolean;
}

export interface ShopTurn {
  turn: number;
  tier: number | null;
  gold: number | null;
  goldSpent: number | null;
  rolls: number | null;
  freeRolls: number | null;
  buys: (string | null)[];
  spellBuys: number | null;
  sells: (string | null)[];
  freezes: number | null;
  offers: Offer[];
  actions: number | null;
  extraGold: number | null;
  sellGold: number | null;
  /** null: not recorded (before parser revision 4). */
  passes: (string | null)[] | null;
}

export interface Shop {
  turns: ShopTurn[];
  tierUps: { turn: number; tier: number }[];
  actions: number;
  startMs: number | null;
  endMs: number | null;
}

export interface HeroSelect {
  offered: string[];
  rerolls: number | null;
  picked: string | null;
}

export interface Choice {
  turn: number;
  source: string | null;
  offered: { cardId: string | null; gift: string | null }[];
  picked: number[];
}

export interface Report {
  status: "ok" | "incomplete" | "unsupported" | null;
  mode: "solo" | "duos" | null;
  localPlayerId: number | null;
  hero: string | null;
  teammatePlayerId: number | null;
  teammateHero: string | null;
  finalPlace: number | null;
  lobby: Seat[];
  rounds: Round[];
  startHealth: ReadonlyMap<number, number>;
  shop: Shop | null;
  heroSelect: HeroSelect | null;
  /** null: saved before choices were kept (parser revision 5). */
  choices: Choice[] | null;
  oldGod: number | null;
  warnings: string[];
  problems: string[];
}

export interface GameRecord {
  parserVersion: string | null;
  revision: number | null;
  /** Written by a parser newer than this site knows: shown as far as known. */
  newer: boolean;
  report: Report;
  /** Values that were there but could not be read (shown as unknown). */
  unreadable: number;
  /** The day the game was played (YYYY-MM-DD); never the time of day. */
  playedOn: string | null;
}

export type ReadResult = { ok: true; record: GameRecord } | { ok: false };

type Obj = Record<string, unknown>;

const isObject = (v: unknown): v is Obj => typeof v === "object" && v !== null && !Array.isArray(v);
const absent = (v: unknown) => v === null || v === undefined;

/** Reading helpers that count every value they have to drop. */
class Reader {
  unreadable = 0;
  /** The record's parser read buffs (revision 14): a minion without them has none, not unknown. */
  buffsRecorded = false;

  bad<T>(fallback: T): T {
    this.unreadable += 1;
    return fallback;
  }

  int(v: unknown, min: number, max: number): number | null {
    if (absent(v)) return null;
    return typeof v === "number" && Number.isInteger(v) && v >= min && v <= max ? v : this.bad(null);
  }

  bool(v: unknown): boolean {
    if (absent(v)) return false;
    return typeof v === "boolean" ? v : this.bad(false);
  }

  card(v: unknown): string | null {
    if (absent(v)) return null;
    return typeof v === "string" && CARD_ID.test(v) ? v : this.bad(null);
  }

  /** A list capped at max items; anything past it is dropped and counted. */
  list(v: unknown, max: number): unknown[] {
    if (absent(v)) return [];
    if (!Array.isArray(v)) return this.bad([]);
    if (v.length > max) {
      this.unreadable += v.length - max;
      return v.slice(0, max);
    }
    return v;
  }

  /** Like list, but null when the key is missing (not recorded). */
  optList(v: unknown, max: number): unknown[] | null {
    return v === undefined || v === null ? null : this.list(v, max);
  }

  obj(v: unknown): Obj | null {
    if (isObject(v)) return v;
    return absent(v) ? null : this.bad(null);
  }

  cards(v: unknown, max: number): (string | null)[] {
    return this.list(v, max).map((c) => this.card(c));
  }

  oneOf<T extends string>(v: unknown, allowed: readonly T[]): T | null {
    if (absent(v)) return null;
    return typeof v === "string" && (allowed as readonly string[]).includes(v) ? (v as T) : this.bad(null);
  }

  health(v: unknown): Map<number, number> {
    const map = new Map<number, number>();
    const o = this.obj(v);
    if (!o) return map;
    const entries = Object.entries(o);
    this.unreadable += Math.max(0, entries.length - 16);
    for (const [key, value] of entries.slice(0, 16)) {
      if (!PLAYER_KEY.test(key) || absent(value)) {
        this.bad(null);
        continue;
      }
      const health = this.int(value, 0, MAX_HEALTH);
      if (health !== null) map.set(Number(key), health);
    }
    return map;
  }
}

function minion(r: Reader, v: unknown): Minion {
  const m = r.obj(v) ?? {};
  const words = r.list(m.keywords, KEYWORDS.length);
  const keywords = [...new Set(words.map((w) => r.oneOf(w, KEYWORDS)).filter((w): w is Keyword => w !== null))];
  return {
    cardId: r.card(m.card_id),
    atk: r.int(m.atk, -MAX_INT, MAX_INT),
    health: r.int(m.health, -MAX_INT, MAX_INT),
    golden: r.bool(m.golden),
    keywords,
    gifts: absent(m.gifts) ? [] : [...new Set(r.list(m.gifts, MAX_GIFTS).map((v) => r.card(v)).filter((v): v is string => v !== null))],
    giftUnknown: absent(m.gift_unknown) ? false : r.bool(m.gift_unknown),
    buffs: r.buffsRecorded ? buffs(r, m.buffs) : null,
  };
}

/** A minion's buffs: a card once, a count from 1 to 999; what is not that is counted and dropped. */
function buffs(r: Reader, v: unknown): Buff[] {
  const out: Buff[] = [];
  for (const item of r.list(v, MAX_BUFF_KINDS)) {
    const o = r.obj(item);
    const cardId = o ? r.card(o.card_id) : null;
    const count = o ? r.int(o.count, 1, MAX_BUFF_COUNT) : null;
    if (cardId !== null && count !== null) out.push({ cardId, count });
  }
  return oneEachCard(out);
}

function gameValues(r: Reader, v: unknown): GameValues | null {
  const o = r.obj(v);
  if (!o) return null;
  const values: Partial<Record<GameValue, number>> = {};
  for (const name of GAME_VALUES) {
    const n = r.int(o[name], 1, MAX_GAME_VALUE);
    if (n !== null) values[name] = n;
  }
  return Object.keys(values).length > 0 ? values : null;
}

function entry(r: Reader, v: unknown): Entry | null {
  const e = r.obj(v);
  const side = e ? r.oneOf(e.side, ["own", "opponent"] as const) : null;
  if (!e || !side) return r.bad(null);
  const board = absent(e.board) ? null : r.list(e.board, 16).map((m) => minion(r, m));
  const values = board === null ? null : gameValues(r, e.values);
  return { side, playerId: r.int(e.player_id, 1, 64), hero: r.card(e.hero), board, values };
}

function round(r: Reader, v: unknown): Round | null {
  const o = r.obj(v);
  const number = o ? r.int(o.number, 0, 1000) : null;
  if (!o || number === null) return r.bad(null);
  return {
    number,
    entries: r.list(o.entries, 8).map((e) => entry(r, e)).filter((e): e is Entry => e !== null),
    ownHealthAfter: r.int(o.own_health_after, -MAX_HEALTH, MAX_HEALTH),
    healthAfter: r.health(o.health_after),
    result: r.oneOf(o.result, ["won", "lost", "tie"] as const),
    replay: replayOf(r, o.replay),
  };
}

/** A replay that cannot be read is one unreadable value, shown as "not available". */
function replayOf(r: Reader, v: unknown): ReplayRead | null {
  const read = readReplay(v, r.buffsRecorded);
  if (read && !read.ok) r.bad(null);
  return read;
}

/** One round per number (a repeated number keeps the last, like the server), in order. */
function rounds(r: Reader, v: unknown): Round[] {
  const byNumber = new Map<number, Round>();
  for (const x of r.list(v, 200)) {
    const one = round(r, x);
    if (one) byNumber.set(one.number, one);
  }
  return [...byNumber.values()].sort((a, b) => a.number - b.number);
}

function seat(r: Reader, v: unknown): Seat | null {
  const p = r.obj(v);
  const playerId = p ? r.int(p.player_id, 1, 64) : null;
  if (!p || playerId === null) return r.bad(null);
  const triples = r.optList(p.triples, 64)?.flatMap((x) => {
    const t = r.obj(x);
    const turn = t ? r.int(t.turn, 0, MAX_TURN) : null;
    const count = t ? r.int(t.count, 0, 1000) : null;
    return turn !== null && count !== null ? [{ turn, count }] : r.bad([]);
  }) ?? null;
  const trinkets = r.optList(p.trinkets, 64)?.flatMap((x) => {
    const t = r.obj(x);
    const turn = t ? r.int(t.turn, 0, MAX_TURN) : null;
    const slot = t ? r.oneOf(t.slot, ["first", "second", "hero_power"] as const) : null;
    const dbfId = t ? r.int(t.dbf_id, 1, MAX_DBF_ID) : null;
    return turn !== null && slot !== null && dbfId !== null ? [{ turn, slot, dbfId }] : r.bad([]);
  }) ?? null;
  return {
    playerId,
    hero: r.card(p.hero),
    duoTeam: r.int(p.duo_team, 1, 8),
    finalPlace: r.int(p.final_place, 1, 8),
    triples,
    trinkets,
  };
}

function shopTurn(r: Reader, v: unknown): ShopTurn | null {
  const t = r.obj(v);
  const turn = t ? r.int(t.turn, 1, MAX_TURN) : null;
  if (!t || turn === null) return r.bad(null);
  const rolls = r.int(t.rolls, 0, MAX_COUNT);
  const offers = r.list(t.offers, 400).flatMap((x) => {
    const o = r.obj(x);
    const roll = o ? r.int(o.roll, 0, rolls ?? MAX_COUNT) : null;
    return o && roll !== null ? [{ cardId: r.card(o.card_id), roll, frozen: r.bool(o.frozen) }] : r.bad([]);
  });
  return {
    turn,
    tier: r.int(t.tier, 1, 7),
    gold: r.int(t.gold, 0, MAX_COUNT),
    goldSpent: r.int(t.gold_spent, 0, MAX_COUNT),
    rolls,
    freeRolls: r.int(t.free_rolls, 0, rolls ?? MAX_COUNT),
    buys: r.cards(t.buys, 100),
    spellBuys: r.int(t.spell_buys, 0, MAX_COUNT),
    sells: r.cards(t.sells, 100),
    freezes: r.int(t.freezes, 0, MAX_COUNT),
    offers,
    actions: r.int(t.actions, 0, 3000),
    extraGold: r.int(t.extra_gold, 0, MAX_COUNT),
    sellGold: r.int(t.sell_gold, 0, MAX_COUNT),
    passes: absent(t.passes) ? null : r.cards(t.passes, 100),
  };
}

/** At most MAX_GAME_OFFERS offers in a game, as the server checks; the rest is dropped and counted. */
function capOffers(r: Reader, turns: ShopTurn[]): ShopTurn[] {
  let left = MAX_GAME_OFFERS;
  return turns.map((t) => {
    const kept = t.offers.slice(0, Math.max(0, left));
    left -= kept.length;
    r.unreadable += t.offers.length - kept.length;
    return kept.length === t.offers.length ? t : { ...t, offers: kept };
  });
}

function shop(r: Reader, v: unknown): Shop | null {
  const s = r.obj(v);
  if (!s) return null;
  const turns = capOffers(r, r.list(s.turns, MAX_TURN).map((t) => shopTurn(r, t)).filter((t): t is ShopTurn => t !== null));
  const tierUps = r.list(s.tier_ups, MAX_TURN).flatMap((x) => {
    const u = r.obj(x);
    const turn = u ? r.int(u.turn, 1, MAX_TURN) : null;
    const tier = u ? r.int(u.tier, 2, 7) : null;
    return turn !== null && tier !== null ? [{ turn, tier }] : r.bad([]);
  });
  const startMs = r.int(s.start_ms, 0, 86_400_000);
  const endMs = r.int(s.end_ms, startMs ?? 0, 86_400_000);
  return { turns, tierUps, actions: r.list(s.actions, 3000).length, startMs, endMs };
}

function heroSelect(r: Reader, v: unknown): HeroSelect | null {
  const h = r.obj(v);
  if (!h) return null;
  const offered = r.cards(h.offered, 32).filter((c): c is string => c !== null);
  const picked = r.card(h.picked);
  return { offered, rerolls: r.int(h.rerolls, 0, 32), picked: picked && offered.includes(picked) ? picked : null };
}

function choice(r: Reader, v: unknown): Choice | null {
  const c = r.obj(v);
  const turn = c ? r.int(c.turn, 0, MAX_TURN) : null;
  if (!c || turn === null) return r.bad(null);
  const offered = r.list(c.offered, 10).map((x) => {
    const o = r.obj(x) ?? {};
    return { cardId: r.card(o.card_id), gift: r.card(o.gift) };
  });
  const picked = [
    ...new Set(r.list(c.picked, offered.length).map((p) => r.int(p, 0, offered.length - 1)).filter((p): p is number => p !== null)),
  ];
  return { turn, source: r.card(c.source), offered, picked };
}

function notes(r: Reader, v: unknown): string[] {
  return r.list(v, 16).flatMap((s) => (typeof s === "string" && NOTE.test(s) ? [s] : r.bad([])));
}

function report(r: Reader, o: Obj): Report {
  const gameType = r.oneOf(o.game_type, ["GT_BATTLEGROUNDS", "GT_BATTLEGROUNDS_DUO"] as const);
  const mode = gameType === "GT_BATTLEGROUNDS" ? "solo" : gameType === "GT_BATTLEGROUNDS_DUO" ? "duos" : null;
  return {
    status: r.oneOf(o.status, ["ok", "incomplete", "unsupported"] as const),
    mode,
    localPlayerId: r.int(o.local_player_id, 1, 64),
    hero: r.card(o.hero),
    teammatePlayerId: r.int(o.teammate_player_id, 1, 64),
    teammateHero: r.card(o.teammate_hero),
    finalPlace: r.int(o.final_place, 1, mode === "duos" ? 4 : 8),
    lobby: r.list(o.lobby, 16).map((p) => seat(r, p)).filter((p): p is Seat => p !== null),
    rounds: rounds(r, o.rounds),
    startHealth: r.health(o.start_health),
    shop: shop(r, o.shop),
    heroSelect: heroSelect(r, o.hero_select),
    choices: r.optList(o.discovers, 200)?.map((c) => choice(r, c)).filter((c): c is Choice => c !== null) ?? null,
    oldGod: r.int(o.old_god, 1, MAX_DBF_ID),
    warnings: notes(r, o.warnings),
    problems: notes(r, o.problems),
  };
}

/**
 * The game record in a file, or not ok when it is not one. The file is the one
 * the address names (its name is the game's public id), so only its own parts
 * must agree; the day comes from its session name and the time is dropped.
 */
export function readRecord(value: unknown): ReadResult {
  if (!isObject(value) || !isObject(value.report)) return { ok: false };
  if (typeof value.index !== "number" || value.index !== value.report.index) return { ok: false };
  const r = new Reader();
  const parser = r.obj(value.parser);
  const version = parser && typeof parser.version === "string" && PARSER_VERSION.test(parser.version)
    ? parser.version
    : parser ? r.bad(null) : null;
  const revision = parser ? r.int(parser.revision, 0, 999_999) : null;
  r.buffsRecorded = buffsRecorded(revision);
  const rep = report(r, value.report);
  return {
    ok: true,
    record: { parserVersion: version, revision, newer: revision !== null && revision > KNOWN_REVISION, report: rep, unreadable: r.unreadable, playedOn: typeof value.session === "string" ? playedOn(value.session) : null },
  };
}

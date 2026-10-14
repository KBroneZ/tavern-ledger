// The combat replay of each round (T-215, D-099, parser revision 8), as
// upload-game checks it: the parser's own bounds
// (crates/bg-parser/src/replay.rs). Older records have none. Parser revision
// 13 (T-575) adds three things, each optional so older apps keep uploading:
// the beats (how many events each log block gave), each damage's source, and
// keyword events (a keyword given or used mid-combat).
import {
  bool,
  cardId,
  buffs,
  gifts,
  int,
  Invalid,
  isObject,
  keywords,
  KEYWORDS,
  list,
  MAX_INT,
  shape,
} from "./checks.ts";

const MAX_REPLAY_MINIONS = 200;
// Written only when the minion has a Dark Gift (parser revision 9) or a buff
// (revision 14).
const OPTIONAL_MINION_KEYS = new Set(["gifts", "gift_unknown", "buffs"]);
const MAX_REPLAY_EVENTS = 2000;
const SIDES = new Set(["own", "opponent"]);
const TRIGGERS = new Set(["start_of_combat", "deathrattle", "avenge", "rally"]);
// How a damage came about (revision 13).
const CAUSES = new Set([
  "attack",
  "start_of_combat",
  "deathrattle",
  "avenge",
  "rally",
  "hero_power",
  "trinket",
  "effect",
]);
// The keys of each kind of replay event, `kind` included; all required but
// those in OPTIONAL_EVENT_KEYS.
const EVENT_KEYS: Record<string, string[]> = {
  attack: ["kind", "attacker", "defender"],
  damage: ["kind", "minion", "amount", "health", "source"],
  shield: ["kind", "minion", "on"],
  stats: ["kind", "minion", "atk", "health"],
  death: ["kind", "minion"],
  summon: ["kind", "minion"],
  trigger: ["kind", "trigger", "side", "card_id", "source"],
  hero_damage: ["kind", "side", "amount"],
  board_in: ["kind", "side", "hero"],
  keyword: ["kind", "minion", "keyword", "on"],
};
// A damage's source is written since revision 13 (null when unknown).
const OPTIONAL_EVENT_KEYS = new Set(["damage.source"]);
// Written since revision 13.
const OPTIONAL_REPLAY_KEYS = new Set(["beats"]);
const MAX_HEALTH = 100000;

function side(v: unknown, field: string): void {
  if (typeof v !== "string" || !SIDES.has(v)) throw new Invalid(field);
}

/**
 * One combat's replay (T-215): both boards and every summon by card id, side
 * and slot, and the events naming them by index. A summon names a minion
 * past the starting boards, once; nothing else names one before it arrives.
 */
export function replay(
  v: unknown,
  field: string,
  keys: Record<string, string[]>,
  addHero: (id: string) => void,
): void {
  if (v === undefined || v === null) return;
  const replayKeys = keys["report.rounds[].replay"];
  const r = shape(v, field, replayKeys, replayKeys.filter((k) => !OPTIONAL_REPLAY_KEYS.has(k)));
  const minions = list(r.minions, `${field}.minions`, MAX_REPLAY_MINIONS);
  const minionKeys = keys["report.rounds[].replay.minions[]"];
  minions.forEach((m, i) => {
    const at = `${field}.minions[${i}]`;
    const one = shape(m, at, minionKeys, minionKeys.filter((k) => !OPTIONAL_MINION_KEYS.has(k)));
    side(one.side, `${at}.side`);
    cardId(one.card_id, `${at}.card_id`);
    int(one.atk, `${at}.atk`, -MAX_INT, MAX_INT);
    int(one.health, `${at}.health`, -MAX_INT, MAX_INT);
    int(one.position, `${at}.position`, 0, 64);
    bool(one.golden, `${at}.golden`);
    keywords(one.keywords, `${at}.keywords`);
    gifts(one, at);
    buffs(one, at);
  });
  const start = int(r.start, `${field}.start`, 0, minions.length);
  bool(r.complete, `${field}.complete`);
  const arrived = new Set<number>();
  const known = (n: unknown, at: string) => {
    const i = int(n, at, 0, minions.length - 1);
    if (i >= start && !arrived.has(i)) throw new Invalid(at);
    return i;
  };
  const events = list(r.events, `${field}.events`, MAX_REPLAY_EVENTS);
  events.forEach((x, i) => {
    const at = `${field}.events[${i}]`;
    // Own keys only: "constructor" or "toString" must not find an inherited value.
    const name = isObject(x) && typeof x.kind === "string" && Object.hasOwn(EVENT_KEYS, x.kind)
      ? x.kind
      : undefined;
    if (!name) throw new Invalid(`${at}.kind`);
    const kind = EVENT_KEYS[name];
    const e = shape(x, at, kind, kind.filter((k) => !OPTIONAL_EVENT_KEYS.has(`${name}.${k}`)));
    switch (e.kind) {
      case "attack":
        known(e.attacker, `${at}.attacker`);
        if (e.defender !== null) known(e.defender, `${at}.defender`);
        break;
      case "damage":
        int(e.amount, `${at}.amount`, 1, MAX_INT);
        int(e.health, `${at}.health`, -MAX_INT, MAX_INT);
        known(e.minion, `${at}.minion`);
        if (e.source !== undefined && e.source !== null) damageSource(e.source, `${at}.source`, known);
        break;
      case "keyword":
        known(e.minion, `${at}.minion`);
        // Divine Shield has its own event (shield).
        if (typeof e.keyword !== "string" || !KEYWORDS.has(e.keyword) || e.keyword === "divine_shield") {
          throw new Invalid(`${at}.keyword`);
        }
        bool(e.on, `${at}.on`);
        break;
      case "shield":
        bool(e.on, `${at}.on`);
        known(e.minion, `${at}.minion`);
        break;
      case "stats":
        int(e.atk, `${at}.atk`, -MAX_INT, MAX_INT);
        int(e.health, `${at}.health`, -MAX_INT, MAX_INT);
        known(e.minion, `${at}.minion`);
        break;
      case "death":
        known(e.minion, `${at}.minion`);
        break;
      case "summon": {
        const m = int(e.minion, `${at}.minion`, start, minions.length - 1);
        if (arrived.has(m)) throw new Invalid(`${at}.minion`);
        arrived.add(m);
        break;
      }
      case "trigger":
        if (typeof e.trigger !== "string" || !TRIGGERS.has(e.trigger)) {
          throw new Invalid(`${at}.trigger`);
        }
        side(e.side, `${at}.side`);
        if (cardId(e.card_id, `${at}.card_id`) === null) throw new Invalid(`${at}.card_id`);
        if (e.source !== null) known(e.source, `${at}.source`);
        break;
      case "hero_damage":
        side(e.side, `${at}.side`);
        int(e.amount, `${at}.amount`, 1, MAX_HEALTH);
        break;
      case "board_in": {
        side(e.side, `${at}.side`);
        const hero = cardId(e.hero, `${at}.hero`);
        if (hero) addHero(hero);
        break;
      }
    }
  });
  if (r.beats !== undefined) beats(r.beats, `${field}.beats`, events.length);
}

/**
 * Where a damage came from: a minion of the replay, or a card with its side
 * (a hero power, a trinket), and how.
 */
function damageSource(v: unknown, field: string, known: (n: unknown, at: string) => number): void {
  const s = shape(v, field, ["how", "minion", "card_id", "side"], ["how"]);
  if (typeof s.how !== "string" || !CAUSES.has(s.how)) throw new Invalid(`${field}.how`);
  if (s.minion !== undefined) {
    if (s.card_id !== undefined || s.side !== undefined) throw new Invalid(field);
    known(s.minion, `${field}.minion`);
    return;
  }
  if (cardId(s.card_id, `${field}.card_id`) === null) throw new Invalid(`${field}.card_id`);
  side(s.side, `${field}.side`);
}

/** The beats: each a run of at least one event, adding up to the events. */
function beats(v: unknown, field: string, events: number): void {
  const runs = list(v, field, MAX_REPLAY_EVENTS);
  let sum = 0;
  runs.forEach((n, i) => (sum += int(n, `${field}[${i}]`, 1, MAX_REPLAY_EVENTS)));
  if (sum !== events) throw new Invalid(field);
}

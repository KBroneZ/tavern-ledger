// The combat replay of each round (T-215, D-099, parser revision 8), as
// upload-game checks it: the parser's own bounds
// (crates/bg-parser/src/replay.rs). Older records have none.
import { bool, cardId, int, Invalid, isObject, keywords, list, MAX_INT, shape } from "./checks.ts";

const MAX_REPLAY_MINIONS = 200;
const MAX_REPLAY_EVENTS = 2000;
const SIDES = new Set(["own", "opponent"]);
const TRIGGERS = new Set(["start_of_combat", "deathrattle", "avenge", "rally"]);
// The keys of each kind of replay event, `kind` included.
const EVENT_KEYS: Record<string, string[]> = {
  attack: ["kind", "attacker", "defender"],
  damage: ["kind", "minion", "amount", "health"],
  shield: ["kind", "minion", "on"],
  stats: ["kind", "minion", "atk", "health"],
  death: ["kind", "minion"],
  summon: ["kind", "minion"],
  trigger: ["kind", "trigger", "side", "card_id", "source"],
  hero_damage: ["kind", "side", "amount"],
  board_in: ["kind", "side", "hero"],
};
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
  const r = shape(v, field, keys["report.rounds[].replay"], keys["report.rounds[].replay"]);
  const minions = list(r.minions, `${field}.minions`, MAX_REPLAY_MINIONS);
  const minionKeys = keys["report.rounds[].replay.minions[]"];
  minions.forEach((m, i) => {
    const at = `${field}.minions[${i}]`;
    const one = shape(m, at, minionKeys, minionKeys);
    side(one.side, `${at}.side`);
    cardId(one.card_id, `${at}.card_id`);
    int(one.atk, `${at}.atk`, -MAX_INT, MAX_INT);
    int(one.health, `${at}.health`, -MAX_INT, MAX_INT);
    int(one.position, `${at}.position`, 0, 64);
    bool(one.golden, `${at}.golden`);
    keywords(one.keywords, `${at}.keywords`);
  });
  const start = int(r.start, `${field}.start`, 0, minions.length);
  bool(r.complete, `${field}.complete`);
  const arrived = new Set<number>();
  const known = (n: unknown, at: string) => {
    const i = int(n, at, 0, minions.length - 1);
    if (i >= start && !arrived.has(i)) throw new Invalid(at);
    return i;
  };
  list(r.events, `${field}.events`, MAX_REPLAY_EVENTS).forEach((x, i) => {
    const at = `${field}.events[${i}]`;
    // Own keys only: "constructor" or "toString" must not find an inherited value.
    const kind = isObject(x) && typeof x.kind === "string" && Object.hasOwn(EVENT_KEYS, x.kind)
      ? EVENT_KEYS[x.kind]
      : undefined;
    if (!kind) throw new Invalid(`${at}.kind`);
    const e = shape(x, at, kind, kind);
    switch (e.kind) {
      case "attack":
        known(e.attacker, `${at}.attacker`);
        if (e.defender !== null) known(e.defender, `${at}.defender`);
        break;
      case "damage":
        int(e.amount, `${at}.amount`, 1, MAX_INT);
        int(e.health, `${at}.health`, -MAX_INT, MAX_INT);
        known(e.minion, `${at}.minion`);
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
}

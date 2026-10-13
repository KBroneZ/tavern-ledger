// The combat replay's step model (T-536): every board state of a replay, the
// start then one per step, and each step in words. A port of the plain data
// handling in the app's recap (crates/desktop/ui/replay.js: startState,
// applyEvent, frames, stepWords); the app's drawing code stays in the app.
// Rules that differ from the app:
// * the replay was read strictly first (replayRead.ts), so only a step that
//   names a minion no longer on a board, or a kind this site does not know,
//   is left out here (and counted), never a malformed one;
// * a Duos board coming in with no hero named shows "unknown hero" instead
//   of keeping the round's first hero;
// * a trigger with no minion behind it (a hero power or trinket) is named by
//   its card id, as the site names every card (it has no card names yet).
// What the log says happened, never a simulation.
import type { Keyword } from "./keywords.ts";
import { type Replay, type ReplayEvent, type Side, SIDES } from "./replayRead.ts";
import { g, type GameStringKey } from "./strings.ts";
import { lang } from "../i18n.ts";

/** Playback speeds; at 1× each step stays for its beat (BEAT_MS). */
export const SPEEDS = [0.5, 1, 2, 4] as const;

/**
 * How long each kind of step stays at 1× before the next one plays (T-560,
 * D-132), the app's too. Set from the game's own playback on the owner's 73
 * combats: about 1.2 s from one attack to the next with no death, 1.3 s with
 * one, 2.1 s with two or more. An attack's beat ends as it hits, so its
 * damage draws on the impact; "start" is the boards before the first step.
 */
export const BEAT_MS: Readonly<Record<ReplayEvent["kind"] | "start", number>> = {
  start: 900,
  attack: 600,
  damage: 260,
  shield: 120,
  stats: 220,
  death: 300,
  summon: 380,
  trigger: 480,
  hero_damage: 1200,
  board_in: 800,
  unknown: 0,
};

/** What the log writes between an attack and its first damage (a buff, a shield) plays at the impact. */
const AT_IMPACT_MS = 60;

/** How long step `e` (null: the start) stays at `speed` before the next; a speed not offered is 1×. */
export function stepDelay(e: ReplayEvent | null, speed: number): number {
  const s = Number.isFinite(speed) && speed > 0 ? speed : 1;
  return BEAT_MS[e === null ? "start" : e.kind] / s;
}

/** Each step's beat at 1× (the steps' events, null for the start), with the attack's damage on its impact. */
export function beats(events: (ReplayEvent | null)[]): number[] {
  let landing = false;
  return events.map((e) => {
    if (e?.kind === "attack") landing = true;
    else if (e?.kind === "damage" || e?.kind === "death" || e?.kind === "hero_damage") landing = false;
    else if (landing) return AT_IMPACT_MS;
    return stepDelay(e, 1);
  });
}

/** What happened to a minion at this step. */
export interface Marks {
  attacker?: true;
  defender?: true;
  /** Damage taken at this step. */
  hurt?: number;
  /** Lost its Divine Shield. */
  popped?: true;
  buffed?: true;
  dying?: true;
  summoned?: true;
  triggered?: true;
}

/** A minion on a board at one step. */
export interface Tile {
  /** Its index in the replay's minions. */
  ref: number;
  cardId: string | null;
  atk: number;
  /** The log's health: below 0 after overkill (the page draws 0). */
  health: number;
  golden: boolean;
  shield: boolean;
  keywords: Keyword[];
  gifts: string[];
  giftUnknown: boolean;
  marks: Marks;
}

export interface Frame {
  /** null: the start. */
  event: ReplayEvent | null;
  /** Hero damage so far, per side. */
  hero: Record<Side, number>;
  /** Each side's hero (a Duos board coming in brings its own). */
  heroes: Record<Side, string | null>;
  /** Left to right. */
  own: Tile[];
  opponent: Tile[];
}

export interface Playback {
  frames: Frame[];
  /** Steps left out: they name a minion no longer on a board, or a kind this site does not know. */
  skipped: number;
}

type Base = Omit<Tile, "marks">;

interface State {
  replay: Replay;
  tiles: Map<number, Base>;
  boards: Record<Side, number[]>;
  hero: Record<Side, number>;
  heroes: Record<Side, string | null>;
  dying: number[];
}

function tileOf(replay: Replay, ref: number): Base {
  const m = replay.minions[ref];
  return {
    ref,
    cardId: m.cardId,
    atk: m.atk,
    health: m.health,
    golden: m.golden,
    shield: m.keywords.includes("divine_shield"),
    keywords: m.keywords,
    gifts: m.gifts,
    giftUnknown: m.giftUnknown,
  };
}

function startState(replay: Replay, heroes: Record<Side, string | null>): State {
  const st: State = { replay, tiles: new Map(), boards: { own: [], opponent: [] }, hero: { own: 0, opponent: 0 }, heroes: { ...heroes }, dying: [] };
  for (let ref = 0; ref < replay.start; ref++) {
    st.tiles.set(ref, tileOf(replay, ref));
    st.boards[replay.minions[ref].side].push(ref);
  }
  return st;
}

/** Can this step be played now? Every minion it names must be on a board (a summon: not yet). */
function playable(st: State, e: ReplayEvent): boolean {
  const on = (n: number) => st.tiles.has(n);
  switch (e.kind) {
    case "attack":
      return on(e.attacker) && (e.defender === null || on(e.defender));
    case "damage":
    case "shield":
    case "stats":
    case "death":
      return on(e.minion);
    case "summon":
      return !on(e.minion);
    case "trigger":
    case "hero_damage":
    case "board_in":
      return true;
    case "unknown":
      return false;
  }
}

/** Plays the step on the state; returns its marks per minion. */
function apply(st: State, e: ReplayEvent): Map<number, Marks> {
  const marks = new Map<number, Marks>();
  const mark = (ref: number, what: Marks) => marks.set(ref, { ...marks.get(ref), ...what });
  const set = (ref: number, what: Partial<Base>) => st.tiles.set(ref, { ...(st.tiles.get(ref) as Base), ...what });
  switch (e.kind) {
    case "attack":
      mark(e.attacker, { attacker: true });
      if (e.defender !== null) mark(e.defender, { defender: true });
      break;
    case "damage":
      set(e.minion, { health: e.health });
      mark(e.minion, { hurt: e.amount });
      break;
    case "shield":
      set(e.minion, { shield: e.on });
      if (!e.on) mark(e.minion, { popped: true });
      break;
    case "stats":
      set(e.minion, { atk: e.atk, health: e.health });
      mark(e.minion, { buffed: true });
      break;
    case "death":
      mark(e.minion, { dying: true });
      st.dying.push(e.minion);
      break;
    case "summon": {
      const m = st.replay.minions[e.minion];
      const board = st.boards[m.side];
      // The app's rule: slot 1 is the left-most; a slot of 0 (none in the log) goes left too.
      const at = Math.min(Math.max(m.position - 1, 0), board.length);
      st.tiles.set(e.minion, tileOf(st.replay, e.minion));
      board.splice(at, 0, e.minion);
      mark(e.minion, { summoned: true });
      break;
    }
    case "trigger":
      if (e.source !== null && st.tiles.has(e.source)) mark(e.source, { triggered: true });
      break;
    case "hero_damage":
      st.hero[e.side] += e.amount;
      break;
    case "board_in":
      st.heroes[e.side] = e.hero;
      break;
    case "unknown":
      break;
  }
  return marks;
}

function snapshot(st: State, event: ReplayEvent | null, marks: Map<number, Marks>): Frame {
  const board = (side: Side) => st.boards[side].map((ref) => ({ ...(st.tiles.get(ref) as Base), marks: marks.get(ref) ?? {} }));
  return { event, hero: { ...st.hero }, heroes: { ...st.heroes }, own: board("own"), opponent: board("opponent") };
}

/**
 * Every board state of a replay: the start, then one per step. `heroes` are
 * the round's heroes as the combat starts.
 */
export function playback(replay: Replay, heroes: Record<Side, string | null>): Playback {
  const st = startState(replay, heroes);
  const frames = [snapshot(st, null, new Map())];
  let skipped = 0;
  for (const e of replay.events) {
    // Who died at the last step leaves the board now.
    for (const ref of st.dying) {
      for (const side of SIDES) st.boards[side] = st.boards[side].filter((x) => x !== ref);
      st.tiles.delete(ref);
    }
    st.dying = [];
    if (!playable(st, e)) {
      skipped += 1;
      continue;
    }
    frames.push(snapshot(st, e, apply(st, e)));
  }
  return { frames, skipped };
}

/** How the words name things: a minion of the replay by its index, a card, a hero. */
export interface Namer {
  minion(ref: number): string;
  card(id: string): string;
  hero(id: string | null): string;
}

const capital = (s: string) => s.charAt(0).toLocaleUpperCase(lang()) + s.slice(1);

/** A step in words; null is the start. */
export function stepWords(e: ReplayEvent | null, replay: Replay, namer: Namer): string {
  const whose = (side: Side, name: string) => g(side === "own" ? "replay.your" : "replay.their", { name });
  const m = (ref: number) => whose(replay.minions[ref].side, namer.minion(ref));
  if (e === null) return g("replay.start");
  switch (e.kind) {
    case "attack":
      return capital(e.defender === null ? g("replay.ev.attackAlone", { a: m(e.attacker) }) : g("replay.ev.attack", { a: m(e.attacker), b: m(e.defender) }));
    case "damage":
      return capital(e.health > 0
        ? g("replay.ev.damage", { m: m(e.minion), n: e.amount, left: e.health })
        : g("replay.ev.damageLethal", { m: m(e.minion), n: e.amount }));
    case "shield":
      return capital(g(e.on ? "replay.ev.shieldOn" : "replay.ev.shieldOff", { m: m(e.minion) }));
    case "stats":
      return capital(g("replay.ev.stats", { m: m(e.minion), atk: e.atk, health: e.health }));
    case "death":
      return capital(g("replay.ev.death", { m: m(e.minion) }));
    case "summon":
      return capital(g("replay.ev.summon", { m: m(e.minion) }));
    case "trigger": {
      const who = e.source !== null ? m(e.source) : whose(e.side, namer.card(e.cardId));
      return capital(g(`replay.ev.trigger.${e.trigger}` as GameStringKey, { m: who }));
    }
    case "hero_damage":
      return g(e.side === "own" ? "replay.ev.heroOwn" : "replay.ev.heroOpponent", { n: e.amount });
    case "board_in":
      return g(e.side === "own" ? "replay.ev.boardInOwn" : "replay.ev.boardInOpponent", { hero: namer.hero(e.hero) });
    case "unknown":
      return g("replay.ev.unknown");
  }
}

// ---------- Playback control (the page's buttons and keys, as plain data) ----------

export interface PlayState {
  step: number;
  playing: boolean;
}

export type PlayAction = "play" | "back" | "forward" | "restart" | "end" | "tick" | { goto: number };

/**
 * The next state after an action, for a replay whose last step is `last`.
 * Nothing plays with reduced motion, nor past the end; play at the end
 * starts over (the app's rule).
 */
export function control(state: PlayState, action: PlayAction, last: number, reduced: boolean): PlayState {
  const at = (step: number, playing: boolean): PlayState => {
    const clamped = Math.min(Math.max(step, 0), last);
    return { step: clamped, playing: playing && !reduced && clamped < last };
  };
  if (typeof action === "object") return at(action.goto, false);
  switch (action) {
    case "play":
      return state.playing ? at(state.step, false) : at(state.step >= last ? 0 : state.step, true);
    case "tick":
      return state.playing ? at(state.step + 1, true) : state;
    case "back":
      return at(state.step - 1, false);
    case "forward":
      return at(state.step + 1, false);
    case "restart":
      return at(0, false);
    case "end":
      return at(last, false);
  }
}

/** The step a key asks for on the controls; none with a modifier (Alt+Left is the browser's Back). */
export function keyAction(e: { key: string; altKey: boolean; ctrlKey: boolean; metaKey: boolean; shiftKey: boolean }): PlayAction | null {
  if (e.altKey || e.ctrlKey || e.metaKey || e.shiftKey) return null;
  const keys: Record<string, PlayAction> = { ArrowLeft: "back", ArrowRight: "forward", Home: "restart", End: "end" };
  return Object.prototype.hasOwnProperty.call(keys, e.key) ? keys[e.key] : null;
}

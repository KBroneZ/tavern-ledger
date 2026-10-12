// Per-game summary for the website (T-502, D-071): the numbers the pages
// need, worked out once from a report that passed validateRecord() and
// stored in public.game_stats, so the site never downloads every report.
//
// Unknown is not zero: every value is null when the report cannot back it
// (an older parser revision, a log that lacks it, an unfinished game). The
// rules follow the desktop app's (crates/tracker: recap.rs, shop.rs,
// choices.rs, stats/curious.rs) so the site and the app agree:
// * a combat's result is the one the game records (from the log); without
//   it, it is worked out from health and counted as inferred; when neither
//   can say, it is unknown;
// * damage taken and dealt are the game's own hit at the end of each combat,
//   which the combat's replay keeps (crates/tracker/src/damage.rs, D-108), or
//   0 after a tie the game recorded; a combat with neither falls back to the
//   health drop between rounds in a row (inferred: health paid or healed in
//   the shop counts too);
// * shop gold totals are sums over the turns that have the number, with the
//   count of those turns;
// * triples are the leaderboard's own count at the end of a finished game.
//
// Goes up by one each time a value changes meaning or a value is added; the
// daily sweep then fills the summary again from the stored reports.
//
// 2: damage from the game's own hit in each combat's replay (T-545, D-108).
// 3: the damage records say whether the max came from the game's own hit.
// 4: how many combats behind each damage total are the game's own hit.
export const SUMMARY_VERSION = 4;

// deno-lint-ignore no-explicit-any
type Json = any;

export interface GameStats {
  teammate_hero: string | null;
  rounds: number | null;
  combats_won: number | null;
  combats_lost: number | null;
  combats_tied: number | null;
  combats_unknown: number | null;
  combats_inferred: number | null;
  win_streak: number | null;
  win_streak_inferred: boolean | null;
  damage_taken: number | null;
  damage_taken_combats: number | null;
  damage_taken_max: number | null;
  damage_taken_max_inferred: boolean | null;
  damage_dealt: number | null;
  damage_dealt_combats: number | null;
  damage_dealt_max: number | null;
  damage_dealt_max_inferred: boolean | null;
  shop_turns: number | null;
  final_tier: number | null;
  gold_spent: number | null;
  gold_spent_turns: number | null;
  gold_beyond_income: number | null;
  gold_beyond_turns: number | null;
  sell_gold: number | null;
  sell_gold_turns: number | null;
  rolls: number | null;
  free_rolls: number | null;
  buys: number | null;
  spell_buys: number | null;
  sells: number | null;
  freezes: number | null;
  actions: number | null;
  apm: number | null;
  triples: number | null;
  final_board: (string | null)[] | null;
  final_board_round: number | null;
  board_stats_max: number | null;
  golden_max: number | null;
  heroes_offered: string[] | null;
  hero_rerolls: number | null;
  /** Of damage_taken_combats, how many are the game's own hit (the rest worked out from health). */
  damage_taken_logged: number | null;
  /** Of damage_dealt_combats, how many are the game's own hit. */
  damage_dealt_logged: number | null;
}

/** The columns of public.game_stats this summary fills, in table order. */
export const STATS_KEYS: readonly (keyof GameStats)[] = [
  "teammate_hero",
  "rounds",
  "combats_won",
  "combats_lost",
  "combats_tied",
  "combats_unknown",
  "combats_inferred",
  "win_streak",
  "win_streak_inferred",
  "damage_taken",
  "damage_taken_combats",
  "damage_taken_max",
  "damage_taken_max_inferred",
  "damage_dealt",
  "damage_dealt_combats",
  "damage_dealt_max",
  "damage_dealt_max_inferred",
  "shop_turns",
  "final_tier",
  "gold_spent",
  "gold_spent_turns",
  "gold_beyond_income",
  "gold_beyond_turns",
  "sell_gold",
  "sell_gold_turns",
  "rolls",
  "free_rolls",
  "buys",
  "spell_buys",
  "sells",
  "freezes",
  "actions",
  "apm",
  "triples",
  "final_board",
  "final_board_round",
  "board_stats_max",
  "golden_max",
  "heroes_offered",
  "hero_rerolls",
  "damage_taken_logged",
  "damage_dealt_logged",
];

const DUOS = "GT_BATTLEGROUNDS_DUO";
const MS_PER_MINUTE = 60_000;
// The parser's own cap on shop turns (crates/bg-parser/src/shop.rs).
const MAX_TURN = 100;
const MAX_INT = 2 ** 31 - 1;
// No hero attack in the game comes near this; a larger number is a
// hand-edited record, not a hit (crates/tracker/src/damage.rs).
const MAX_HIT = 1_000;

type Outcome = "won" | "lost" | "tie" | "unknown";
interface Combat {
  round: number;
  outcome: Outcome;
  inferred: boolean;
}

const isInt = (v: unknown): v is number => typeof v === "number" && Number.isInteger(v);
const arr = (v: unknown): Json[] => (Array.isArray(v) ? v : []);
const cap = (n: number) => Math.min(n, MAX_INT);

/** The rounds by number (a repeated number keeps the last), in order. */
function roundsByNumber(report: Json): Map<number, Json> {
  const byNumber = new Map<number, Json>();
  for (const r of arr(report.rounds)) {
    if (isInt(r?.number)) byNumber.set(r.number, r);
  }
  return new Map([...byNumber.entries()].sort((a, b) => a[0] - b[0]));
}

/** Who shares health in one game: a player alone, or their Duos team. */
class Sides {
  private readonly teams = new Map<number, number>();
  private readonly duos: boolean;

  constructor(private readonly report: Json, private readonly rounds: Map<number, Json>) {
    this.duos = report.game_type === DUOS;
    for (const seat of arr(report.lobby)) {
      if (isInt(seat?.player_id) && isInt(seat?.duo_team)) {
        this.teams.set(seat.player_id, seat.duo_team);
      }
    }
  }

  sideOf(pid: number): number[] {
    const side = new Set([pid]);
    const team = this.duos ? this.teams.get(pid) : undefined;
    if (team !== undefined) {
      for (const [other, t] of this.teams) if (t === team) side.add(other);
    }
    return [...side].sort((a, b) => a - b);
  }

  own(): number[] | null {
    const local = this.report.local_player_id;
    if (!isInt(local)) return null;
    const side = new Set(this.sideOf(local));
    if (this.duos && isInt(this.report.teammate_player_id)) {
      side.add(this.report.teammate_player_id);
    }
    return [...side].sort((a, b) => a - b);
  }

  /** The health a side shares after `round` (0: before the first combat). */
  healthOf(side: number[], round: number): number | null {
    const map = round === 0 ? this.report.start_health : this.rounds.get(round)?.health_after;
    if (map === null || typeof map !== "object" || Array.isArray(map)) return null;
    const values = new Set<number>();
    for (const pid of side) {
      const h = map[String(pid)];
      if (isInt(h)) values.add(h);
    }
    return values.size === 1 ? [...values][0] : null;
  }

  /** The opposing sides a round names. */
  opposing(round: Json): number[][] {
    const keys = new Map<string, number[]>();
    for (const e of arr(round.entries)) {
      if (e?.side === "opponent" && isInt(e.player_id)) {
        const side = this.sideOf(e.player_id);
        keys.set(side.join(","), side);
      }
    }
    return [...keys.values()];
  }
}

/** Worked out from health alone, as recap.rs judges it. */
function judge(own: [number | null, number | null], opp: [number | null, number | null]) {
  const [ownBefore, ownAfter] = own;
  const [oppBefore, oppAfter] = opp;
  if (ownBefore === null || ownAfter === null || oppBefore === null || oppAfter === null) {
    return "unknown";
  }
  if (ownBefore === 0 || oppBefore === 0) return "unknown";
  if (ownAfter > ownBefore || oppAfter > oppBefore) return "unknown";
  const ownLost = ownAfter < ownBefore;
  const oppLost = oppAfter < oppBefore;
  if (ownLost && !oppLost) return "lost";
  if (!ownLost && oppLost) return "won";
  if (!ownLost && !oppLost) return "tie";
  return "unknown";
}

function logged(round: Json): Outcome | null {
  return round.result === "won" || round.result === "lost" || round.result === "tie"
    ? round.result
    : null;
}

/** One result per round, in round order. */
function combats(sides: Sides, rounds: Map<number, Json>): Combat[] {
  const own = sides.own();
  const out: Combat[] = [];
  for (const [number, round] of rounds) {
    const fromLog = logged(round);
    if (fromLog !== null) {
      out.push({ round: number, outcome: fromLog, inferred: false });
      continue;
    }
    const opposing = sides.opposing(round);
    let outcome: Outcome = "unknown";
    if (opposing.length === 1 && own !== null) {
      const opp = opposing[0];
      outcome = judge(
        [sides.healthOf(own, number - 1), sides.healthOf(own, number)],
        [sides.healthOf(opp, number - 1), sides.healthOf(opp, number)],
      );
    }
    out.push({ round: number, outcome, inferred: outcome !== "unknown" });
  }
  return out;
}

function results(list: Combat[]) {
  if (list.length === 0) {
    return {
      combats_won: null,
      combats_lost: null,
      combats_tied: null,
      combats_unknown: null,
      combats_inferred: null,
    };
  }
  const count = (o: Outcome) => list.filter((c) => c.outcome === o).length;
  return {
    combats_won: count("won"),
    combats_lost: count("lost"),
    combats_tied: count("tie"),
    combats_unknown: count("unknown"),
    combats_inferred: list.filter((c) => c.inferred).length,
  };
}

/** Most combats won in rounds in a row; a log-only streak beats an inferred one of equal length. */
function streak(list: Combat[]) {
  if (list.every((c) => c.outcome === "unknown")) {
    return { win_streak: null, win_streak_inferred: null };
  }
  let best = { len: 0, log: false };
  let run = { len: 0, log: false };
  let last: number | null = null;
  for (const c of list) {
    const follows = last !== null && c.round === last + 1;
    last = c.round;
    if (c.outcome !== "won") {
      run = { len: 0, log: false };
      continue;
    }
    run = follows && run.len > 0
      ? { len: run.len + 1, log: run.log && !c.inferred }
      : { len: 1, log: !c.inferred };
    if (run.len > best.len || (run.len === best.len && run.log && !best.log)) best = run;
  }
  return { win_streak: best.len, win_streak_inferred: best.len > 0 && !best.log };
}

interface Drops {
  total: number;
  combats: number;
  max: number;
  /** No combat at the max value has the game's own hit: the max is worked out from health. */
  maxInferred: boolean;
  /** How many of the combats are the game's own hit. */
  logged: number;
}

/** One combat's damage, and whether it is the game's own hit (not worked out from health). */
interface Damage {
  value: number;
  logged: boolean;
}

function drops(damages: Damage[]): Drops | null {
  if (damages.length === 0) return null;
  const max = Math.max(...damages.map((d) => d.value));
  return {
    total: cap(damages.reduce((a, d) => a + d.value, 0)),
    combats: damages.length,
    max,
    maxInferred: !damages.some((d) => d.value === max && d.logged),
    logged: damages.filter((d) => d.logged).length,
  };
}

interface Hit {
  taken: number;
  dealt: number;
}

/**
 * One combat's damage as the game dealt it: the last hero hit in the replay
 * (the winner's hit as the combat ends), or nothing after a tie the game
 * recorded. null when the record does not have it. The replay is untrusted
 * (D-079): anything but a plain whole number in range on a known side is not
 * a hit.
 */
function loggedHit(round: Json): Hit | null {
  const events = round.replay?.events;
  let hit: Json = undefined;
  if (Array.isArray(events)) {
    for (let i = events.length - 1; i >= 0; i--) {
      if (events[i]?.kind === "hero_damage") {
        hit = events[i];
        break;
      }
    }
  }
  if (hit === undefined) return round.result === "tie" ? { taken: 0, dealt: 0 } : null;
  if (!isInt(hit.amount) || hit.amount < 1 || hit.amount > MAX_HIT) return null;
  if (hit.side === "own") return { taken: hit.amount, dealt: 0 };
  if (hit.side === "opponent") return { taken: 0, dealt: hit.amount };
  return null;
}

/** Each combat's logged damage by round number; rounds the record lacks it for are left out. */
function loggedHits(rounds: Map<number, Json>): Map<number, Hit> {
  const hits = new Map<number, Hit>();
  for (const [number, r] of rounds) {
    const hit = number > 0 ? loggedHit(r) : null;
    if (hit !== null) hits.set(number, hit);
  }
  return hits;
}

/** Own health lost in each combat: the game's hit, else the drop where health before and after is known. */
function damageTaken(sides: Sides, rounds: Map<number, Json>, hits: Map<number, Hit>): Drops | null {
  const points = new Map<number, number | null>();
  const own = sides.own();
  const start = own === null ? null : sides.healthOf(own, 0);
  if (start !== null) points.set(0, start);
  for (const [number, r] of rounds) {
    points.set(number, isInt(r.own_health_after) ? r.own_health_after : null);
  }
  const values: Damage[] = [];
  for (const number of rounds.keys()) {
    const hit = hits.get(number);
    if (hit !== undefined) {
      values.push({ value: hit.taken, logged: true });
      continue;
    }
    const after = points.get(number);
    const before = points.get(number - 1);
    if (number > 0 && after != null && before != null) {
      values.push({ value: Math.max(0, before - after), logged: false });
    }
  }
  return drops(values);
}

/** Health the one opposing side lost in each round: the game's hit, else the drop where known before and after. */
function damageDealt(sides: Sides, rounds: Map<number, Json>, hits: Map<number, Hit>): Drops | null {
  const values: Damage[] = [];
  for (const [number, r] of rounds) {
    const hit = hits.get(number);
    if (hit !== undefined) {
      values.push({ value: hit.dealt, logged: true });
      continue;
    }
    const opposing = sides.opposing(r);
    if (opposing.length !== 1) continue;
    const before = sides.healthOf(opposing[0], number - 1);
    const after = sides.healthOf(opposing[0], number);
    // A side already out comes back as a ghost: nothing to read.
    if (before === null || after === null || before === 0) continue;
    values.push({ value: Math.max(0, before - after), logged: false });
  }
  return drops(values);
}

function boards(report: Json, rounds: Map<number, Json>) {
  const local = report.local_player_id;
  let finalBoard: (string | null)[] | null = null;
  let finalRound: number | null = null;
  let statsMax: number | null = null;
  let goldenMax: number | null = null;
  for (const [number, r] of rounds) {
    const entry = arr(r.entries).find((e) =>
      e?.side === "own" && e.player_id === local && Array.isArray(e.board)
    );
    if (entry === undefined || !isInt(local)) continue;
    const board: Json[] = entry.board;
    finalBoard = board.map((m) => (typeof m?.card_id === "string" ? m.card_id : null));
    finalRound = number;
    const stats = cap(
      board.reduce(
        (sum, m) =>
          sum + Math.max(0, isInt(m?.atk) ? m.atk : 0) +
          Math.max(0, isInt(m?.health) ? m.health : 0),
        0,
      ),
    );
    const golden = board.filter((m) => m?.golden === true).length;
    statsMax = Math.max(statsMax ?? 0, stats);
    goldenMax = Math.max(goldenMax ?? 0, golden);
  }
  return {
    final_board: finalBoard,
    final_board_round: finalRound,
    board_stats_max: statsMax,
    golden_max: goldenMax,
  };
}

/** A number summed over the turns that have it, and how many turns that is. */
function sumKnown(turns: Json[], f: (t: Json) => number | null): [number | null, number | null] {
  let sum = 0;
  let known = 0;
  for (const t of turns) {
    const v = f(t);
    if (v !== null) {
      sum += v;
      known += 1;
    }
  }
  return known > 0 ? [cap(sum), known] : [null, null];
}

const intOrNull = (v: unknown) => (isInt(v) ? v : null);

function shop(report: Json) {
  const s = report.shop;
  const turns = s !== null && typeof s === "object" ? arr(s.turns) : [];
  if (turns.length === 0) {
    return {
      shop_turns: null,
      final_tier: null,
      gold_spent: null,
      gold_spent_turns: null,
      gold_beyond_income: null,
      gold_beyond_turns: null,
      sell_gold: null,
      sell_gold_turns: null,
      rolls: null,
      free_rolls: null,
      buys: null,
      spell_buys: null,
      sells: null,
      freezes: null,
      actions: null,
      apm: null,
    };
  }
  const total = (f: (t: Json) => number) => cap(turns.reduce((n, t) => n + f(t), 0));
  const lastTurn = Math.max(0, ...turns.map((t) => (isInt(t.turn) ? t.turn : 0)));
  const tiers = [
    ...turns.map((t) => t.tier),
    ...arr(s.tier_ups).map((u) => u?.tier),
  ].filter(isInt);
  const [goldSpent, goldSpentTurns] = sumKnown(turns, (t) => intOrNull(t.gold_spent));
  const [sellGold, sellGoldTurns] = sumKnown(turns, (t) => intOrNull(t.sell_gold));
  const [beyond, beyondTurns] = sumKnown(
    turns,
    (t) => (isInt(t.extra_gold) && isInt(t.sell_gold) ? t.extra_gold + t.sell_gold : null),
  );
  const actions = arr(s.actions).length;
  const minutes = isInt(s.start_ms) && isInt(s.end_ms)
    ? (s.end_ms - s.start_ms) / MS_PER_MINUTE
    : 0;
  // No logged action at all: the log has no actions for this game (unknown, not 0).
  const apm = actions > 0 && minutes > 0 ? Math.round((actions / minutes) * 100) / 100 : null;
  return {
    shop_turns: Math.max(Math.min(lastTurn, MAX_TURN), turns.length),
    final_tier: tiers.length > 0 ? Math.max(...tiers) : null,
    gold_spent: goldSpent,
    gold_spent_turns: goldSpentTurns,
    gold_beyond_income: beyond,
    gold_beyond_turns: beyondTurns,
    sell_gold: sellGold,
    sell_gold_turns: sellGoldTurns,
    rolls: total((t) => intOrNull(t.rolls) ?? 0),
    free_rolls: total((t) => intOrNull(t.free_rolls) ?? 0),
    buys: total((t) => arr(t.buys).length),
    spell_buys: total((t) => intOrNull(t.spell_buys) ?? 0),
    sells: total((t) => arr(t.sells).length),
    freezes: total((t) => intOrNull(t.freezes) ?? 0),
    actions: actions > 0 ? actions : null,
    apm: apm !== null && apm <= 99999 ? apm : null,
  };
}

/** The player's own triple count at the end of a finished game (choices.rs). */
function triples(report: Json): number | null {
  if (report.status !== "ok") return null;
  const lobby = arr(report.lobby);
  // Saved before revision 5: the seats have no triples key.
  if (lobby.length === 0 || lobby.some((p) => !Array.isArray(p?.triples))) return null;
  const own = lobby.find((p) => p.player_id === report.local_player_id);
  if (own === undefined) return null;
  const last = own.triples[own.triples.length - 1];
  return isInt(last?.count) ? last.count : 0;
}

function heroPick(report: Json) {
  const h = report.hero_select;
  if (h === null || typeof h !== "object" || !Array.isArray(h.offered)) {
    return { heroes_offered: null, hero_rerolls: null };
  }
  return {
    heroes_offered: h.offered.filter((id: unknown) => typeof id === "string"),
    hero_rerolls: intOrNull(h.rerolls),
  };
}

/** The summary of one report. The report must have passed validateRecord(). */
export function summarize(report: Json): GameStats {
  const rounds = roundsByNumber(report);
  const sides = new Sides(report, rounds);
  const list = combats(sides, rounds);
  const hits = loggedHits(rounds);
  const taken = damageTaken(sides, rounds, hits);
  const dealt = damageDealt(sides, rounds, hits);
  const stats: GameStats = {
    teammate_hero: typeof report.teammate_hero === "string" ? report.teammate_hero : null,
    rounds: rounds.size > 0 ? rounds.size : null,
    ...results(list),
    ...streak(list),
    damage_taken: taken?.total ?? null,
    damage_taken_combats: taken?.combats ?? null,
    damage_taken_max: taken?.max ?? null,
    damage_taken_max_inferred: taken?.maxInferred ?? null,
    damage_dealt: dealt?.total ?? null,
    damage_dealt_combats: dealt?.combats ?? null,
    damage_dealt_max: dealt?.max ?? null,
    damage_dealt_max_inferred: dealt?.maxInferred ?? null,
    ...shop(report),
    triples: triples(report),
    ...boards(report, rounds),
    ...heroPick(report),
    damage_taken_logged: taken?.logged ?? null,
    damage_dealt_logged: dealt?.logged ?? null,
  };
  // Table order, so the JSON sent to the database is stable.
  return Object.fromEntries(STATS_KEYS.map((k) => [k, stats[k]])) as unknown as GameStats;
}

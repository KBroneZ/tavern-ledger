// Shapes one read report into what the game viewer shows (T-504). The rules
// are the desktop app's and the server summary's (crates/tracker recap.rs and
// choices.rs, supabase/functions/_shared/game_summary.ts), so the site, the
// app and the games list agree:
// * a combat's result is the one the game records; without it, it is worked
//   out from health and marked inferred; when neither can say, unknown;
// * damage is the game's own hit at the end of the combat, which the combat's
//   replay keeps (crates/tracker damage.rs, D-108), or 0 after a tie the game
//   recorded: marked "from the log". Without it, it is a health drop between
//   rounds in a row, marked inferred. The replay is untrusted input (D-079,
//   D-111): a hit outside the bounds is not a hit;
// * a combat follows the shop turn of the same number (D-060);
// * totals over turns say how many turns back them; unknown is never zero.
// Opponents are shown by hero only: the report has no player names.
import type { Result } from "../data/format.ts";
import type { Choice, Entry, GameRecord, Minion, Report, Round, Seat, ShopTurn, TrinketSlot } from "./report.ts";
import { heroName } from "../heroes/names.ts";
import type { Replay, Side } from "./replayRead.ts";

export interface HeroRef {
  id: string | null;
  /** The game's own name when the record has it, else null (the page shows the id). */
  name: string | null;
}

export interface BoardView {
  minions: Minion[];
  /** Totals are unknown when any minion's stat is. */
  attack: number | null;
  health: number | null;
}

export interface Known {
  value: number | null;
  /** How many turns (or combats) the value comes from, of `total`. */
  known: number;
  total: number;
  /** Of `known`, how many are the game's own number from the log (damage only; 0 for the rest). */
  logged: number;
}

/** Where a combat's damage comes from: the game's own hit, or health worked out; null when neither. */
export type DamageSource = "log" | "inferred" | null;

export interface TurnRow {
  turn: number;
  tier: number | null;
  tierUp: boolean;
  gold: number | null;
  goldSpent: number | null;
  /** null: no combat in the record for this turn. */
  result: Result | null;
  inferred: boolean;
  opponents: HeroRef[];
  /** Health the opposing side lost in this combat. */
  dealt: number | null;
  /** Own (or the team's) health lost in this combat. */
  taken: number | null;
  damageSource: DamageSource;
  healthAfter: number | null;
  ownBoard: BoardView | null;
  /** Whose board ownBoard is, in Duos when it is the teammate's. */
  ownBoardHero: HeroRef | null;
  /** In Duos, the other own-side board of the turn (the teammate's) beside ownBoard; null when the record has none. */
  partnerBoard: { hero: HeroRef; board: BoardView } | null;
  /** Each opposing hero's board the record has (two in Duos); empty: none recorded. */
  opponentBoards: { hero: HeroRef; board: BoardView }[];
  shop: ShopTurn | null;
}

export interface Summary {
  mode: "solo" | "duos" | null;
  status: Report["status"];
  place: number | null;
  hero: HeroRef;
  teammate: HeroRef | null;
  rounds: number;
  results: { won: number; lost: number; tie: number; unknown: number; inferred: number };
  dealt: Known;
  taken: Known;
  goldSpent: Known;
  rolls: number | null;
  buys: number | null;
  sells: number | null;
  finalTier: number | null;
  apm: number | null;
  triples: number | null;
}

export interface OpponentView {
  heroes: HeroRef[];
  record: { won: number; lost: number; tie: number; unknown: number };
  boards: { round: number; hero: HeroRef; board: BoardView }[];
}

export interface SeatView {
  hero: HeroRef;
  own: boolean;
  teammate: boolean;
  place: number | null;
  team: number | null;
  /** The leaderboard's last count; null when not recorded for this game. */
  triples: number | null;
  trinkets: { turn: number; slot: TrinketSlot; dbfId: number }[] | null;
}

export type ChoiceKind = "trinket" | "dark_gift" | "quest" | "triple_reward" | "discover";

export interface ChoiceView {
  turn: number;
  kind: ChoiceKind;
  options: { cardId: string | null; gift: string | null; taken: boolean }[];
  /** False when the log does not say which was taken. */
  takenKnown: boolean;
}

/** One round's combat replay (T-536), with what the turns table says of that combat. */
export interface ReplayCombat {
  round: number;
  result: Result | null;
  inferred: boolean;
  opponents: HeroRef[];
  /** Each side's hero as the combat starts. */
  heroes: Record<Side, string | null>;
  /** Every hero the replay names (the start and each Duos board coming in), by id. */
  heroNames: ReadonlyMap<string, HeroRef>;
  /** null: the record has one this site cannot read ("not available"). */
  replay: Replay | null;
}

export interface GameView {
  summary: Summary;
  turns: TurnRow[];
  opponents: OpponentView[];
  lobby: SeatView[];
  /** null: not recorded (saved before parser revision 5). */
  choices: ChoiceView[] | null;
  heroPick: { offered: HeroRef[]; picked: string | null; rerolls: number | null } | null;
  oldGod: number | null;
  notes: string[];
  /** The rounds that have a replay, in order; a round without one is not listed. */
  replays: ReplayCombat[];
}

const TRINKET_SOURCES = new Set(["BG30_Trinket_1st", "BG30_Trinket_2nd"]);
const DARK_GIFT_SOURCE = "BG36_MidGameEffect_010";
const TRIPLE_REWARD = "TB_BaconShop_Triples_01";
const MS_PER_MINUTE = 60_000;
// The server summary's cap: more is a log the clock cannot back.
const MAX_APM = 99_999;
// No hero attack in the game comes near this; a larger number is a
// hand-edited record, not a hit (crates/tracker damage.rs).
const MAX_HIT = 1_000;

/**
 * The hero's name in the page's language from the site's own name table
 * (T-510). A name the file carries is never used: the file is the uploader's.
 */
export function heroRef(id: string | null): HeroRef {
  return { id, name: heroName(id) };
}

function boardView(minions: Minion[] | null): BoardView | null {
  if (minions === null) return null;
  const sum = (key: "atk" | "health") =>
    minions.every((m) => m[key] !== null) ? minions.reduce((n, m) => n + (m[key] as number), 0) : null;
  return { minions, attack: sum("atk"), health: sum("health") };
}

/** Who shares health: a player alone, or their Duos team. */
class Sides {
  private readonly teams = new Map<number, number>();
  private readonly report: Report;

  constructor(report: Report) {
    this.report = report;
    if (report.mode === "duos") {
      for (const seat of report.lobby) if (seat.duoTeam !== null) this.teams.set(seat.playerId, seat.duoTeam);
    }
  }

  sideOf(pid: number): number[] {
    const team = this.teams.get(pid);
    const side = new Set([pid]);
    if (team !== undefined) for (const [other, t] of this.teams) if (t === team) side.add(other);
    return [...side].sort((a, b) => a - b);
  }

  own(): number[] | null {
    const local = this.report.localPlayerId;
    if (local === null) return null;
    const side = new Set(this.sideOf(local));
    if (this.report.mode === "duos" && this.report.teammatePlayerId !== null) side.add(this.report.teammatePlayerId);
    return [...side].sort((a, b) => a - b);
  }

  healthOf(side: number[], round: number): number | null {
    const map = round === 0 ? this.report.startHealth : this.report.rounds.find((r) => r.number === round)?.healthAfter;
    if (!map) return null;
    const values = new Set(side.map((pid) => map.get(pid)).filter((h): h is number => h !== undefined));
    return values.size === 1 ? [...values][0] : null;
  }

  opposing(round: Round): number[][] {
    const keys = new Map<string, number[]>();
    for (const e of round.entries) {
      if (e.side === "opponent" && e.playerId !== null) {
        const side = this.sideOf(e.playerId);
        keys.set(side.join(","), side);
      }
    }
    return [...keys.values()];
  }
}

type Outcome = "won" | "lost" | "tie" | "unknown";

/** Worked out from health alone, as recap.rs judges it. */
function judge(own: [number | null, number | null], opp: [number | null, number | null]): Outcome {
  const [ownBefore, ownAfter] = own;
  const [oppBefore, oppAfter] = opp;
  if (ownBefore === null || ownAfter === null || oppBefore === null || oppAfter === null) return "unknown";
  if (ownBefore === 0 || oppBefore === 0) return "unknown";
  if (ownAfter > ownBefore || oppAfter > oppBefore) return "unknown";
  const ownLost = ownAfter < ownBefore;
  const oppLost = oppAfter < oppBefore;
  if (ownLost && !oppLost) return "lost";
  if (!ownLost && oppLost) return "won";
  if (!ownLost && !oppLost) return "tie";
  return "unknown";
}

const RESULT: Record<Outcome, Result> = { won: "win", lost: "loss", tie: "tie", unknown: "unknown" };

interface Combat {
  round: Round;
  outcome: Outcome;
  inferred: boolean;
  dealt: number | null;
  taken: number | null;
  /** The damage is the game's own hit (or a recorded tie), not worked out from health. */
  logged: boolean;
  side: number[] | null;
}

/**
 * The damage the game dealt in one combat: the last hero hit in a replay the
 * site can read, or nothing after a tie the game recorded; null when the
 * record has neither. A hit with a side or amount out of range is not one.
 */
function loggedDamage(round: Round): { taken: number; dealt: number } | null {
  const events = round.replay?.ok ? round.replay.replay.events : [];
  const hit = events.findLast((e) => e.kind === "hero_damage");
  if (hit === undefined) return round.result === "tie" ? { taken: 0, dealt: 0 } : null;
  if (hit.kind !== "hero_damage" || hit.amount < 1 || hit.amount > MAX_HIT) return null;
  return hit.side === "own" ? { taken: hit.amount, dealt: 0 } : { taken: 0, dealt: hit.amount };
}

function combats(report: Report, sides: Sides): Combat[] {
  const own = sides.own();
  const ownHealth = new Map<number, number | null>([[0, own === null ? null : sides.healthOf(own, 0)]]);
  for (const r of report.rounds) ownHealth.set(r.number, r.ownHealthAfter);
  return report.rounds.map((round) => {
    const opposing = sides.opposing(round);
    const side = opposing.length === 1 ? opposing[0] : null;
    const oppBefore = side ? sides.healthOf(side, round.number - 1) : null;
    const oppAfter = side ? sides.healthOf(side, round.number) : null;
    // A side already out comes back as a ghost: nothing to read.
    const dealt = oppBefore !== null && oppAfter !== null && oppBefore > 0 ? Math.max(0, oppBefore - oppAfter) : null;
    const before = ownHealth.get(round.number - 1);
    const after = ownHealth.get(round.number);
    const taken = round.number > 0 && before != null && after != null ? Math.max(0, before - after) : null;
    const hit = round.number > 0 ? loggedDamage(round) : null;
    const damage = hit ? { dealt: hit.dealt, taken: hit.taken, logged: true } : { dealt, taken, logged: false };
    if (round.result !== null) return { round, outcome: round.result, inferred: false, ...damage, side };
    const outcome = side && own
      ? judge([sides.healthOf(own, round.number - 1), sides.healthOf(own, round.number)], [oppBefore, oppAfter])
      : "unknown";
    return { round, outcome, inferred: outcome !== "unknown", ...damage, side };
  });
}

function ownEntry(report: Report, round: Round) {
  const own = round.entries.filter((e) => e.side === "own");
  return own.find((e) => e.playerId === report.localPlayerId) ?? own[0] ?? null;
}

/** The own-side entry that is not the one shown as yours: the teammate's, with a board. */
function partnerEntry(round: Round, own: ReturnType<typeof ownEntry>) {
  return round.entries.find((e) => e.side === "own" && e !== own && e.playerId !== own?.playerId && e.board !== null) ?? null;
}

function turnRows(report: Report, list: Combat[]): TurnRow[] {
  const shopTurns = new Map((report.shop?.turns ?? []).map((t) => [t.turn, t]));
  const tierUps = new Set((report.shop?.tierUps ?? []).map((u) => u.turn));
  const byRound = new Map(list.map((c) => [c.round.number, c]));
  const numbers = [...new Set([...shopTurns.keys(), ...byRound.keys()])].sort((a, b) => a - b);
  return numbers.map((turn) => {
    const shop = shopTurns.get(turn) ?? null;
    const combat = byRound.get(turn) ?? null;
    const own = combat ? ownEntry(report, combat.round) : null;
    const partner = combat ? partnerEntry(combat.round, own) : null;
    // The game can log one opponent twice (once without a board): each hero once, with the board it has.
    const opponents = new Map<string, Entry>();
    for (const e of combat?.round.entries ?? []) {
      if (e.side !== "opponent") continue;
      const key = e.playerId !== null ? `p${e.playerId}` : `h${e.hero ?? ""}`;
      const seen = opponents.get(key);
      if (!seen || (seen.board === null && e.board !== null)) opponents.set(key, e);
    }
    return {
      turn,
      tier: shop?.tier ?? null,
      tierUp: tierUps.has(turn),
      gold: shop?.gold ?? null,
      goldSpent: shop?.goldSpent ?? null,
      result: combat ? RESULT[combat.outcome] : null,
      inferred: combat?.inferred ?? false,
      opponents: [...opponents.values()].map((e) => heroRef(e.hero)),
      dealt: combat?.dealt ?? null,
      taken: combat?.taken ?? null,
      damageSource: !combat || (combat.dealt === null && combat.taken === null) ? null : combat.logged ? "log" : "inferred",
      healthAfter: combat?.round.ownHealthAfter ?? null,
      ownBoard: boardView(own?.board ?? null),
      ownBoardHero: own && own.playerId !== report.localPlayerId && own.hero !== report.hero ? heroRef(own.hero) : null,
      partnerBoard: partner && boardView(partner.board) ? { hero: heroRef(partner.hero), board: boardView(partner.board)! } : null,
      opponentBoards: [...opponents.values()].flatMap((e) => {
        const board = boardView(e.board);
        return board ? [{ hero: heroRef(e.hero), board }] : [];
      }),
      shop,
    };
  });
}

function known(values: (number | null)[], total: number, logged = 0): Known {
  const have = values.filter((v): v is number => v !== null);
  return { value: have.length > 0 ? have.reduce((a, b) => a + b, 0) : null, known: have.length, total, logged };
}

function summary(report: Report, list: Combat[]): Summary {
  const turns = report.shop?.turns ?? [];
  const count = (o: Outcome) => list.filter((c) => c.outcome === o).length;
  const sum = (f: (t: ShopTurn) => number | null) =>
    turns.length === 0 || turns.some((t) => f(t) === null) ? null : turns.reduce((n, t) => n + (f(t) as number), 0);
  const tiers = [...turns.map((t) => t.tier), ...(report.shop?.tierUps ?? []).map((u) => u.tier)].filter(
    (t): t is number => t !== null,
  );
  const shop = report.shop;
  const minutes = shop && shop.startMs !== null && shop.endMs !== null ? (shop.endMs - shop.startMs) / MS_PER_MINUTE : 0;
  const apm = shop && shop.actions > 0 && minutes > 0 ? Math.round((shop.actions / minutes) * 100) / 100 : null;
  return {
    mode: report.mode,
    status: report.status,
    place: report.finalPlace,
    hero: heroRef(report.hero),
    teammate: report.mode === "duos" ? heroRef(report.teammateHero) : null,
    rounds: list.length,
    results: { won: count("won"), lost: count("lost"), tie: count("tie"), unknown: count("unknown"), inferred: list.filter((c) => c.inferred).length },
    dealt: known(list.map((c) => c.dealt), list.length, list.filter((c) => c.logged).length),
    taken: known(list.map((c) => c.taken), list.length, list.filter((c) => c.logged).length),
    goldSpent: known(turns.map((t) => t.goldSpent), turns.length),
    rolls: sum((t) => t.rolls),
    buys: turns.length === 0 ? null : turns.reduce((n, t) => n + t.buys.length, 0),
    sells: turns.length === 0 ? null : turns.reduce((n, t) => n + t.sells.length, 0),
    finalTier: tiers.length > 0 ? Math.max(...tiers) : null,
    apm: apm !== null && apm <= MAX_APM ? apm : null,
    triples: triples(report),
  };
}

/** The leaderboard's own count at the end of a finished game; unknown unless every seat has the record (choices.rs). */
function triples(report: Report): number | null {
  if (report.status !== "ok" || report.lobby.length === 0 || report.lobby.some((s) => s.triples === null)) return null;
  const own = report.lobby.find((s) => s.playerId === report.localPlayerId);
  return own?.triples ? (own.triples.at(-1)?.count ?? 0) : null;
}

function opponentsMet(report: Report, list: Combat[]): OpponentView[] {
  const groups = new Map<string, OpponentView>();
  for (const c of list) {
    if (!c.side) continue;
    const key = c.side.join(",");
    const group = groups.get(key) ?? { heroes: [], record: { won: 0, lost: 0, tie: 0, unknown: 0 }, boards: [] };
    groups.set(key, group);
    group.record[c.outcome] += 1;
    for (const e of c.round.entries) {
      if (e.side !== "opponent" || e.playerId === null || !c.side.includes(e.playerId)) continue;
      if (!group.heroes.some((h) => h.id === e.hero)) group.heroes.push(heroRef(e.hero));
      const board = boardView(e.board);
      if (board) group.boards.push({ round: c.round.number, hero: heroRef(e.hero), board });
    }
  }
  return [...groups.values()];
}

function seatViews(report: Report): SeatView[] {
  const own = report.localPlayerId;
  const byPlace = (s: Seat) => s.finalPlace ?? Number.MAX_SAFE_INTEGER;
  return [...report.lobby]
    .sort((a, b) => byPlace(a) - byPlace(b) || a.playerId - b.playerId)
    .map((s) => ({
      hero: heroRef(s.hero),
      own: s.playerId === own,
      teammate: report.mode === "duos" && s.playerId === report.teammatePlayerId,
      place: s.finalPlace,
      team: report.mode === "duos" ? s.duoTeam : null,
      triples: s.triples === null ? null : (s.triples.at(-1)?.count ?? 0),
      trinkets: s.trinkets,
    }));
}

function choiceKind(choice: Choice): ChoiceKind {
  const source = choice.source ?? "";
  if (TRINKET_SOURCES.has(source)) return "trinket";
  if (source === DARK_GIFT_SOURCE) return "dark_gift";
  if (source === TRIPLE_REWARD) return "triple_reward";
  if (choice.offered.some((o) => o.cardId?.includes("_Quest_"))) return "quest";
  return "discover";
}

function choiceViews(choices: Choice[]): ChoiceView[] {
  return choices
    .map((c, i) => ({ c, i }))
    .sort((a, b) => a.c.turn - b.c.turn || a.i - b.i)
    .map(({ c }) => ({
      turn: c.turn,
      kind: choiceKind(c),
      // Every choice in the user's logs takes one card; only the first is shown (choices.rs).
      options: c.offered.map((o, i) => ({ cardId: o.cardId, gift: o.gift, taken: c.picked[0] === i })),
      takenKnown: c.picked.length > 0,
    }));
}

/** The heroes as the combat starts: the first of each side in the round (the app's rule). */
function startHeroes(round: Round): Record<Side, string | null> {
  const first = (side: Side) => round.entries.find((e) => e.side === side && e.hero !== null)?.hero ?? null;
  return { own: first("own"), opponent: first("opponent") };
}

function replayCombats(report: Report, turns: TurnRow[]): ReplayCombat[] {
  const byTurn = new Map(turns.map((x) => [x.turn, x]));
  return report.rounds.flatMap((round) => {
    if (round.replay === null) return [];
    const row = byTurn.get(round.number);
    const heroes = startHeroes(round);
    const replay = round.replay.ok ? round.replay.replay : null;
    const ids = [heroes.own, heroes.opponent, ...(replay?.events ?? []).map((e) => (e.kind === "board_in" ? e.hero : null))];
    const heroNames = new Map(ids.filter((id): id is string => id !== null).map((id) => [id, heroRef(id)]));
    return [{
      round: round.number,
      result: row?.result ?? null,
      inferred: row?.inferred ?? false,
      opponents: row?.opponents ?? [],
      heroes,
      heroNames,
      replay,
    }];
  });
}

export function shapeGame(record: GameRecord): GameView {
  const report = record.report;
  const sides = new Sides(report);
  const list = combats(report, sides);
  const pick = report.heroSelect;
  const turns = turnRows(report, list);
  return {
    summary: summary(report, list),
    turns,
    opponents: opponentsMet(report, list),
    lobby: seatViews(report),
    choices: report.choices === null ? null : choiceViews(report.choices),
    heroPick: pick ? { offered: pick.offered.map((id) => heroRef(id)), picked: pick.picked, rerolls: pick.rerolls } : null,
    oldGod: report.oldGod,
    notes: [...report.problems, ...report.warnings],
    replays: replayCombats(report, turns),
  };
}

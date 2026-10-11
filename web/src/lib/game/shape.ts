// Shapes one read report into what the game viewer shows (T-504). The rules
// are the desktop app's and the server summary's (crates/tracker recap.rs and
// choices.rs, supabase/functions/_shared/game_summary.ts), so the site, the
// app and the games list agree:
// * a combat's result is the one the game records; without it, it is worked
//   out from health and marked inferred; when neither can say, unknown;
// * damage is a health drop between rounds in a row, so always inferred;
// * a combat follows the shop turn of the same number (D-060);
// * totals over turns say how many turns back them; unknown is never zero.
// Opponents are shown by hero only: the report has no player names.
import type { Result } from "../data/format.ts";
import type { Choice, GameRecord, Minion, Report, Round, Seat, ShopTurn } from "./report.ts";

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
}

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
  /** Health the opposing side lost in this combat (inferred). */
  dealt: number | null;
  /** Own (or the team's) health lost in this combat (inferred). */
  taken: number | null;
  healthAfter: number | null;
  ownBoard: BoardView | null;
  /** Whose board ownBoard is, in Duos when it is the teammate's. */
  ownBoardHero: HeroRef | null;
  opponentBoard: BoardView | null;
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
  shopTurns: number;
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
  trinkets: { turn: number; slot: string; dbfId: number }[] | null;
}

export type ChoiceKind = "trinket" | "dark_gift" | "quest" | "triple_reward" | "discover";

export interface ChoiceView {
  turn: number;
  kind: ChoiceKind;
  options: { cardId: string | null; gift: string | null; taken: boolean }[];
  /** False when the log does not say which was taken. */
  takenKnown: boolean;
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
}

const TRINKET_SOURCES = new Set(["BG30_Trinket_1st", "BG30_Trinket_2nd"]);
const DARK_GIFT_SOURCE = "BG36_MidGameEffect_010";
const TRIPLE_REWARD = "TB_BaconShop_Triples_01";
const MS_PER_MINUTE = 60_000;

export function heroRef(report: Report, id: string | null): HeroRef {
  return { id, name: id ? (report.heroNames.get(id) ?? null) : null };
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
  side: number[] | null;
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
    if (round.result !== null) return { round, outcome: round.result, inferred: false, dealt, taken, side };
    const outcome = side && own
      ? judge([sides.healthOf(own, round.number - 1), sides.healthOf(own, round.number)], [oppBefore, oppAfter])
      : "unknown";
    return { round, outcome, inferred: outcome !== "unknown", dealt, taken, side };
  });
}

function ownEntry(report: Report, round: Round) {
  const own = round.entries.filter((e) => e.side === "own");
  return own.find((e) => e.playerId === report.localPlayerId) ?? own[0] ?? null;
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
    const opponents = combat?.round.entries.filter((e) => e.side === "opponent") ?? [];
    const opponentBoards = opponents.filter((e) => e.board !== null);
    return {
      turn,
      tier: shop?.tier ?? null,
      tierUp: tierUps.has(turn),
      gold: shop?.gold ?? null,
      goldSpent: shop?.goldSpent ?? null,
      result: combat ? RESULT[combat.outcome] : null,
      inferred: combat?.inferred ?? false,
      opponents: opponents.map((e) => heroRef(report, e.hero)),
      dealt: combat?.dealt ?? null,
      taken: combat?.taken ?? null,
      healthAfter: combat?.round.ownHealthAfter ?? null,
      ownBoard: boardView(own?.board ?? null),
      ownBoardHero: own && own.playerId !== report.localPlayerId && own.hero !== report.hero ? heroRef(report, own.hero) : null,
      opponentBoard: opponentBoards.length === 1 ? boardView(opponentBoards[0].board) : null,
      shop,
    };
  });
}

function known(values: (number | null)[], total: number): Known {
  const have = values.filter((v): v is number => v !== null);
  return { value: have.length > 0 ? have.reduce((a, b) => a + b, 0) : null, known: have.length, total };
}

function summary(record: GameRecord, list: Combat[]): Summary {
  const report = record.report;
  const turns = report.shop?.turns ?? [];
  const count = (o: Outcome) => list.filter((c) => c.outcome === o).length;
  const sum = (f: (t: ShopTurn) => number | null) =>
    turns.length === 0 || turns.some((t) => f(t) === null) ? null : turns.reduce((n, t) => n + (f(t) as number), 0);
  const tiers = [...turns.map((t) => t.tier), ...(report.shop?.tierUps ?? []).map((u) => u.tier)].filter(
    (t): t is number => t !== null,
  );
  const shop = report.shop;
  const minutes = shop && shop.startMs !== null && shop.endMs !== null ? (shop.endMs - shop.startMs) / MS_PER_MINUTE : 0;
  const own = report.lobby.find((s) => s.playerId === report.localPlayerId);
  return {
    mode: report.mode,
    status: report.status,
    place: report.finalPlace,
    hero: heroRef(report, report.hero),
    teammate: report.mode === "duos" ? heroRef(report, report.teammateHero) : null,
    rounds: list.length,
    results: { won: count("won"), lost: count("lost"), tie: count("tie"), unknown: count("unknown"), inferred: list.filter((c) => c.inferred).length },
    dealt: known(list.map((c) => c.dealt), list.length),
    taken: known(list.map((c) => c.taken), list.length),
    goldSpent: known(turns.map((t) => t.goldSpent), turns.length),
    rolls: sum((t) => t.rolls),
    buys: turns.length === 0 ? null : turns.reduce((n, t) => n + t.buys.length, 0),
    sells: turns.length === 0 ? null : turns.reduce((n, t) => n + t.sells.length, 0),
    shopTurns: turns.length,
    finalTier: tiers.length > 0 ? Math.max(...tiers) : null,
    apm: shop && shop.actions > 0 && minutes > 0 ? Math.round((shop.actions / minutes) * 100) / 100 : null,
    // The leaderboard's own count at the end of a finished game (choices.rs).
    triples: report.status === "ok" && own?.triples ? (own.triples.at(-1)?.count ?? 0) : null,
  };
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
      if (!group.heroes.some((h) => h.id === e.hero)) group.heroes.push(heroRef(report, e.hero));
      const board = boardView(e.board);
      if (board) group.boards.push({ round: c.round.number, hero: heroRef(report, e.hero), board });
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
      hero: heroRef(report, s.hero),
      own: s.playerId === own,
      teammate: report.mode === "duos" && s.playerId === report.teammatePlayerId,
      place: s.finalPlace,
      team: report.mode === "duos" ? s.duoTeam : null,
      triples: s.triples === null ? null : (s.triples.at(-1)?.count ?? 0),
      trinkets: s.trinkets,
    }));
}

export function choiceKind(choice: Choice): ChoiceKind {
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

export function shapeGame(record: GameRecord): GameView {
  const report = record.report;
  const sides = new Sides(report);
  const list = combats(report, sides);
  const pick = report.heroSelect;
  return {
    summary: summary(record, list),
    turns: turnRows(report, list),
    opponents: opponentsMet(report, list),
    lobby: seatViews(report),
    choices: report.choices === null ? null : choiceViews(report.choices),
    heroPick: pick ? { offered: pick.offered.map((id) => heroRef(report, id)), picked: pick.picked, rerolls: pick.rerolls } : null,
    oldGod: report.oldGod,
    notes: [...report.problems, ...report.warnings],
  };
}

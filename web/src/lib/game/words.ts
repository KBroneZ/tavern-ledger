// The game viewer's values as words (T-504): plain data in, text out, so the
// page and its tests say the same. Unknown is a dash or "unknown", never 0.
import { backing, compact, average, ordinal } from "../data/format.ts";
import { t } from "../data/strings.ts";
import type { Keyword, Minion } from "./report.ts";
import type { BoardView, HeroRef, Known, Summary, TurnRow } from "./shape.ts";
import { g } from "./strings.ts";

const DASH = t("value.none");
const num = (v: number | null) => (v === null ? DASH : compact(v));

/** The game's own hero name, else the card id, else "Unknown hero". */
export function heroLabel(hero: HeroRef | null): string {
  if (!hero || !hero.id) return g("hero.unknown");
  return hero.name ?? hero.id;
}

export function pageHeading(summary: Summary): string {
  const hero = heroLabel(summary.hero);
  return summary.place === null ? g("page.heading.noPlace", { hero }) : g("page.heading", { hero, place: ordinal(summary.place) });
}

export function knownValue(k: Known): string {
  return num(k.value);
}

/** "in 10 of 11 turns" under a total over turns. */
export function turnsBacking(k: Known): string {
  return backing(k.known, k.total, "turns");
}

export function combatsBacking(k: Known): string {
  return g("summary.backing.combats", { known: k.known, total: k.total });
}

export function resultsLine(s: Summary): string {
  return g("summary.results.value", { won: s.results.won, lost: s.results.lost, tie: s.results.tie });
}

export function apmText(apm: number | null): string {
  return apm === null ? DASH : average(apm);
}

export function damageText(turn: TurnRow): string {
  const parts: string[] = [];
  if (turn.dealt !== null && turn.dealt > 0) parts.push(g("turns.dealt", { n: turn.dealt }));
  if (turn.taken !== null && turn.taken > 0) parts.push(g("turns.taken", { n: turn.taken }));
  if (parts.length > 0) return parts.join(" · ");
  return turn.dealt === 0 || turn.taken === 0 ? "0" : DASH;
}

export function goldText(turn: TurnRow): string {
  if (turn.gold === null && turn.goldSpent === null) return DASH;
  return g("turns.gold", { spent: num(turn.goldSpent), gold: num(turn.gold) });
}

export function tierText(turn: TurnRow): string {
  return turn.tier === null ? DASH : String(turn.tier);
}

export function keywordWords(keywords: readonly Keyword[]): string {
  return keywords.map((k) => g(`keyword.${k}`)).join(", ");
}

/** "3/3", with "?" for a stat the record lacks. */
export function statsText(m: Minion): string {
  return g("minion.stats", { attack: m.atk === null ? "?" : m.atk, health: m.health === null ? "?" : m.health });
}

/** "SYN_M_02, 3/3, Golden, Taunt, Divine Shield": what a picture's alt and the list say. */
export function minionWords(m: Minion): string {
  return [m.cardId ?? g("minion.unknownCard"), statsText(m), m.golden ? g("minion.golden") : "", keywordWords(m.keywords)]
    .filter(Boolean)
    .join(", ");
}

export function boardTotals(board: BoardView): string {
  if (board.minions.length === 0) return g("boards.empty");
  return g("boards.totals", { count: board.minions.length, attack: num(board.attack), health: num(board.health) });
}

export function plural(one: Parameters<typeof g>[0], other: Parameters<typeof g>[0], count: number): string {
  return count === 1 ? g(one) : g(other, { count });
}

// The game viewer's values as words (T-504): plain data in, text out, so the
// page and its tests say the same. Unknown is a dash or "unknown", never 0.
import { backing, compact, average, ordinal } from "../data/format.ts";
import { t } from "../data/strings.ts";
import { cardLabel } from "./names.ts";
import type { Keyword, Minion, ShopTurn } from "./report.ts";
import type { BoardView, HeroRef, Known, Summary, TurnRow } from "./shape.ts";
import { g } from "./strings.ts";

// The same in every language (test/unit/i18n.test.ts), so it can be a constant.
const DASH = "—";
const num = (v: number | null) => (v === null ? DASH : compact(v));

/** The hero's name, else "Unknown hero": never the bare card id (T-510). */
export function heroLabel(hero: HeroRef | null): string {
  return hero?.name ?? g("hero.unknown");
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
  const base = g("summary.backing.combats", { known: k.known, total: k.total });
  return k.logged > 0 && k.logged < k.known ? `${base} · ${g("summary.backing.logged", { n: k.logged })}` : base;
}

/** A total over only some of its combats: its backing line stays in view, so it does not read as the whole game. */
export function partial(k: Known): boolean {
  return k.value !== null && k.known < k.total;
}

/** A damage total is marked inferred when any combat behind it was worked out from health (T-559: only inferred is marked). */
export function damageMark(k: Known): "inferred" | undefined {
  return k.known > 0 && k.logged < k.known ? "inferred" : undefined;
}

/** "11–4", or "11–4–1" with ties: the tile's label says which is which. */
export function resultsShort(s: Summary): string {
  const { won, lost, tie } = s.results;
  return tie > 0 ? `${won}–${lost}–${tie}` : `${won}–${lost}`;
}

export function apmText(apm: number | null): string {
  return apm === null ? DASH : average(apm);
}

export function damageText(turn: TurnRow): string {
  const parts: string[] = [];
  if (turn.dealt !== null && turn.dealt > 0) parts.push(g("turns.dealt", { n: turn.dealt }));
  if (turn.taken !== null && turn.taken > 0) parts.push(g("turns.taken", { n: turn.taken }));
  if (parts.length > 0) return parts.join(" · ");
  // No damage either way: 0 only when the figure that result turns on is known.
  const zero = turn.result === "win" ? turn.dealt === 0 : turn.result === "loss" ? turn.taken === 0 : turn.dealt === 0 && turn.taken === 0;
  return zero ? "0" : DASH;
}

/** The damage as signed numbers ("+15", "−5"), for narrow screens; the same dash or 0 as damageText otherwise. */
export function damageShort(turn: TurnRow): string {
  const parts: string[] = [];
  if (turn.dealt !== null && turn.dealt > 0) parts.push(`+${turn.dealt}`);
  if (turn.taken !== null && turn.taken > 0) parts.push(`−${turn.taken}`);
  return parts.length > 0 ? parts.join(" ") : damageText(turn);
}

/**
 * A turn's shop in one line (T-559): the tier, the gold the turn gave and
 * what selling and other effects added, what was spent, the rolls and the
 * freezes. Spending can pass the turn's gold only by the parts named here.
 */
export function shopLine(shop: ShopTurn, turn: Pick<TurnRow, "tierUp">): string {
  const parts: string[] = [];
  if (shop.tier !== null) parts.push(turn.tierUp ? g("shop.line.tierUp", { tier: shop.tier }) : g("shop.line.tier", { tier: shop.tier }));
  const gold = [
    shop.gold === null ? null : g("shop.line.gold", { gold: shop.gold }),
    shop.sellGold ? g("shop.line.sell", { n: shop.sellGold }) : null,
    shop.extraGold ? g("shop.line.extra", { n: shop.extraGold }) : null,
  ].filter((p): p is string => p !== null);
  if (gold.length > 0) parts.push(gold.join(" "));
  parts.push(g("shop.line.spent", { n: num(shop.goldSpent) }));
  if (shop.rolls !== null) {
    const rolls = plural("shop.line.rolls.one", "shop.line.rolls.other", shop.rolls);
    parts.push(shop.freeRolls ? `${rolls} ${g("shop.line.free", { n: shop.freeRolls })}` : rolls);
  }
  if (shop.freezes) parts.push(plural("shop.line.freezes.one", "shop.line.freezes.other", shop.freezes));
  return parts.join(" · ");
}

export function tierText(turn: TurnRow): string {
  return turn.tier === null ? DASH : String(turn.tier);
}

export function keywordWords(keywords: readonly Keyword[]): string {
  return keywords.map((k) => g(`keyword.${k}`)).join(", ");
}

/** "3/3", with "?" for a stat the record lacks. */
export function statsText(m: Pick<Minion, "atk" | "health">): string {
  return g("minion.stats", { attack: m.atk === null ? "?" : m.atk, health: m.health === null ? "?" : m.health });
}

/** "Lantern Moth, 3/3, Golden, Taunt, Divine Shield" (the id when the card data lacks it): a minion's alt and tooltip. */
export function minionWords(m: Pick<Minion, "cardId" | "atk" | "health" | "golden" | "keywords">): string {
  return [cardLabel(m.cardId, g("minion.unknownCard")), statsText(m), m.golden ? g("minion.golden") : "", keywordWords(m.keywords)]
    .filter(Boolean)
    .join(", ");
}

export function boardTotals(board: BoardView): string {
  const count = board.minions.length;
  if (count === 0) return g("boards.empty");
  const stats = { attack: num(board.attack), health: num(board.health) };
  return count === 1 ? g("boards.totals.one", stats) : g("boards.totals", { count, ...stats });
}

export function plural(one: Parameters<typeof g>[0], other: Parameters<typeof g>[0], count: number): string {
  return count === 1 ? g(one) : g(other, { count });
}

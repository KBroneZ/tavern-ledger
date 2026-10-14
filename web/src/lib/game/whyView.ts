// "Why the result differed" on the game page (T-549, D-127; one turn table
// since T-559, D-131) as element trees: each combat's win odds in its turn's
// row, with a short mark only when the likeliest result did not happen, and
// the odds in full, the cause and its evidence in the row's detail. The cause
// list on the accuracy page explains every cause. whyRun.ts fills the cells
// once the engine answers.
import { type AccKey, at } from "../accuracy/strings.ts";
import { reasonText } from "../sim/reasons.ts";
import { share } from "../data/format.ts";
import { h, type VNode } from "../view/node.ts";
import { g, type GameStringKey } from "./strings.ts";
import type { WhyCause, WhyCombat } from "./why.ts";

/** A card's name in the page's language (the card data, else the id). */
export type NameOf = (id: string) => string;

function names(ids: readonly string[], nameOf: NameOf): string {
  return ids.map(nameOf).join(", ");
}

function oddsText(c: WhyCombat): string {
  if (!c.odds) return g("why.noOddsShort");
  return g("why.odds", { won: share(c.odds.won, 1), tied: share(c.odds.tied, 1), lost: share(c.odds.lost, 1) });
}

/** What was not seen, in words: "Blood Gem upgrades", "a minion from the hand (Card)". */
function unseenText(cause: WhyCause, nameOf: NameOf): string {
  const kinds = cause.inputs.map((i) => g(`why.unseen.${i.what}` as GameStringKey, { card: nameOf(i.card) }));
  return [...new Set(kinds)].join("; ");
}

/** The one short line: the cause in plain words. */
export function causeLine(c: WhyCombat, nameOf: NameOf): string {
  if (c.noOdds) {
    const line = g(`why.noOdds.${c.noOdds}` as GameStringKey, { cards: names(c.cards, nameOf) });
    // A refused card says why: "Name: reason".
    const why = c.reasons.map((r) => `${nameOf(r.cardId)}: ${reasonText(r.reason).text}`);
    return why.length > 0 ? `${line} ${why.join("; ")}.` : line;
  }
  const cause = c.cause;
  if (!cause) return g("why.noOdds.bad_board");
  switch (cause.id) {
    case "matched":
      return g("why.matched");
    case "chance":
      return g("why.chance");
    case "missing_inputs":
      return g("why.missing", { what: unseenText(cause, nameOf) });
    case "approximate_card":
      return g("why.approximate", { cards: names(cause.cards, nameOf) });
    default:
      if (cause.replay !== null) return g(`why.replay.${cause.replay}` as GameStringKey);
      return cause.card
        ? g("why.rule", { attack: cause.attack ?? 0, card: nameOf(cause.card) })
        : g("why.ruleNoCard", { attack: cause.attack ?? 0 });
  }
}

/** The evidence behind the line, for the fold; empty when the line says it all. */
export function evidence(c: WhyCombat, nameOf: NameOf): string[] {
  const cause = c.cause;
  if (!cause || cause.id === "matched") return [];
  const out: string[] = [];
  if (cause.id === "chance") out.push(g("why.ev.chance", { attacks: cause.attacks ?? 0 }));
  if (cause.attack !== null && cause.rule !== null) {
    const where = cause.card ? g("why.ev.partsAtCard", { card: nameOf(cause.card) }) : "";
    out.push(g("why.ev.parts", { attack: cause.attack, rule: at(`cause.${cause.rule}` as AccKey), where }));
  }
  if (cause.replay === "not_saved") out.push(g("why.ev.notSaved"));
  out.push(`${at(`cause.${cause.id}` as AccKey)}: ${at(`cause.${cause.id}.why` as AccKey)}`);
  return out;
}

/** The mark in a row: the cause in a word; a quiet dash when no replay could give one; none when the likeliest result happened. */
export function causeMark(c: WhyCombat): VNode | null {
  const cause = c.cause;
  if (c.noOdds || !cause || cause.id === "matched") return null;
  if (cause.replay !== null) {
    return h("span", { class: "gv-cause is-quiet" }, h("span", { "aria-hidden": "true" }, "—"), h("span", { class: "visually-hidden" }, g("why.mark.none")));
  }
  const kind = cause.id === "chance" || cause.id === "missing_inputs" || cause.id === "approximate_card" ? cause.id : "rule";
  return h("span", { class: `gv-cause is-${kind}` }, g(`why.mark.${kind}` as GameStringKey));
}

/** What a turn's odds cell shows: the chance to win, then the mark. */
export function oddsCell(c: WhyCombat): VNode[] {
  const mark = causeMark(c);
  const odds = c.odds ? share(c.odds.won, 1) : "—";
  return [h("span", {}, odds), mark ? " " : null, mark].filter((n): n is VNode => n !== null);
}

/** A turn's detail: the odds in full, and the cause with its evidence when the likeliest result did not happen. */
export function whyDetail(c: WhyCombat, nameOf: NameOf): VNode[] {
  const differed = c.noOdds !== null || (c.cause !== null && c.cause.id !== "matched");
  const more = evidence(c, nameOf);
  return [
    h("p", {}, h("span", { class: "tile-label" }, g("why.oddsTitle")), " ", oddsText(c)),
    differed ? h("p", { class: "gv-why-line" }, causeLine(c, nameOf)) : null,
    more.length > 0 ? h("ul", { class: "gv-why-ev" }, more.map((m) => h("li", {}, m))) : null,
  ].filter((n): n is VNode => n !== null);
}

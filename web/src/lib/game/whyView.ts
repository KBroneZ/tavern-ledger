// "Why the result differed" on the game page (T-549, D-127) as element
// trees: one short line per combat (the turn, the odds the engine gives on
// the boards as they fought, the result and the cause), and a fold with the
// evidence. Simple by default; the cause list on the accuracy page explains
// every cause. The page's script (main.ts) fills the section once the
// engine answers.
import { type AccKey, at } from "../accuracy/strings.ts";
import { share } from "../data/format.ts";
import { t } from "../data/strings.ts";
import { href } from "../i18n.ts";
import { h, type VNode } from "../view/node.ts";
import { fold, stateBlock } from "../view/pieces.ts";
import { g, type GameStringKey } from "./strings.ts";
import type { WhyCause, WhyCombat } from "./why.ts";

export const WHY_ID = "gv-why";

/** The engine's result words as the site's (lib/data/format.ts). */
const RESULT = { won: "win", lost: "loss", tie: "tie" } as const;

/** A card's name in the page's language (the card data, else the id). */
export type NameOf = (id: string) => string;

/** The section as the page first draws it: the heading and a loading line. */
export function whyShell(): VNode {
  return h(
    "section",
    { class: "data-section", "aria-labelledby": `${WHY_ID}-title` },
    h("h2", { id: `${WHY_ID}-title` }, g("why.title")),
    h("p", { class: "data-note" }, g("why.desc")),
    h("div", { id: WHY_ID, "aria-live": "polite" }, stateBlock({ kind: "loading", title: g("why.loading"), lines: 3 })),
  );
}

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
  if (c.noOdds) return g(`why.noOdds.${c.noOdds}` as GameStringKey, { cards: names(c.cards, nameOf) });
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

function combatRow(c: WhyCombat, nameOf: NameOf): VNode {
  const word = RESULT[c.result];
  const result = h("span", { class: `res-${word}` }, t(`result.${word}`));
  const line = h(
    "p",
    { class: "gv-why-line" },
    h("span", { class: "gv-why-turn" }, g("why.turn", { turn: c.round })),
    " ",
    result,
    " ",
    h("span", { class: "gv-why-odds" }, oddsText(c)),
    " · ",
    h("span", { class: "gv-why-cause" }, causeLine(c, nameOf)),
  );
  const more = evidence(c, nameOf);
  if (more.length === 0) return h("li", { "data-round": String(c.round) }, line);
  return h(
    "li",
    { "data-round": String(c.round) },
    line,
    fold({ summary: g("why.more"), context: g("why.turn", { turn: c.round }), compact: true }, h("ul", {}, more.map((m) => h("li", {}, m)))),
  );
}

/** The lines once the engine has answered (`null`: it could not). */
export function whyLines(combats: WhyCombat[] | null, nameOf: NameOf): VNode[] {
  if (combats === null) return [stateBlock({ kind: "error", title: g("why.failed.title"), body: g("why.failed.body") })];
  if (combats.length === 0) return [h("p", { class: "data-note" }, g("why.none"))];
  const missed = combats.filter((c) => c.cause !== null && c.cause.id !== "matched").length;
  return [
    h("p", { class: "data-note" }, g("why.count", { missed, total: combats.length })),
    h("ul", { class: "gv-why" }, combats.map((c) => combatRow(c, nameOf))),
    h("p", { class: "data-note" }, h("a", { href: `${href("/accuracy/")}#acc-causes` }, g("why.link"))),
  ];
}

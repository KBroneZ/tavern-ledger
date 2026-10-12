// Boards and card rows on the game page (T-559) as element trees: a board
// reads like the game's, one portrait per minion with its Attack and Health,
// a gold ring when golden, the Divine Shield halo and the Dark Gift marks of
// the replay (replay.css), and the card's name under it. The keywords are in
// the minion's words (the tooltip and what a screen reader hears).
import { cls, h, type VNode } from "../view/node.ts";
import { picture } from "../view/pieces.ts";
import { giftList } from "./gifts.ts";
import { cardLabel } from "./names.ts";
import type { Minion } from "./report.ts";
import type { BoardView, HeroRef } from "./shape.ts";
import { g } from "./strings.ts";
import * as w from "./words.ts";

function minionTile(m: Minion): VNode {
  const words = w.minionWords(m);
  const stat = (v: number | null) => (v === null ? "?" : String(v));
  return h(
    "li",
    { class: cls("rp-minion", "gv-minion", { "is-golden": m.golden, "has-shield": m.keywords.includes("divine_shield") }), title: words },
    h("span", { class: "rp-art" }, picture("card", m.cardId, words, 56, true)),
    h(
      "span",
      { class: "rp-info" },
      h("span", { class: "rp-stats", "aria-hidden": "true" }, h("span", { class: "rp-atk" }, stat(m.atk)), h("span", { class: "rp-hp" }, stat(m.health))),
      h("span", { class: "visually-hidden" }, words),
    ),
    h("span", { class: "gv-minion-name", "aria-hidden": "true" }, cardLabel(m.cardId, g("minion.unknownCard"))),
    giftList(m),
  );
}

/** One side's board as the combat began: whose it is, its totals, and its minions. */
export function boardBlock(id: string, title: string, hero: HeroRef | null, view: BoardView | null): VNode {
  return h(
    "div",
    { class: "gv-board" },
    h(
      "p",
      { class: "gv-board-title", id },
      hero ? picture("hero", hero.id, w.heroLabel(hero), 24, true) : null,
      h("span", {}, title),
      " ",
      h("span", { class: "gv-dim" }, view ? w.boardTotals(view) : g("boards.unknown")),
    ),
    view && view.minions.length > 0 ? h("ul", { class: "rp-board gv-minions", role: "list", "aria-labelledby": id }, view.minions.map(minionTile)) : null,
  );
}

/** Cards in a row (bought, sold, offered), each a small picture with its name. */
export function cardRow(label: string, cards: (string | null)[], extra: (index: number) => VNode | null = () => null): VNode {
  return h(
    "ul",
    { class: "gv-cardrow", role: "list", "aria-label": label },
    cards.map((c, i) => {
      const name = cardLabel(c, g("minion.unknownCard"));
      return h("li", { title: name }, picture("card", c, name, 32, true), h("span", {}, name), extra(i));
    }),
  );
}

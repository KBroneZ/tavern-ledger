// The tooltip of a minion (T-616, D-141): its exact Attack and Health and the
// buffs it has, each card with its picture, name and how many times. It is
// real markup next to the minion (not a title), shown by the stylesheet on
// hover and on keyboard focus or a tap (the minion takes focus); the minion
// points to it with aria-describedby, so a screen reader hears it too.
// "Empty is not unknown": a game saved before the buffs were read says so.
import { exactStat } from "../stat.ts";
import { h, type VNode } from "../view/node.ts";
import { picture } from "../view/pieces.ts";
import type { Buff } from "./buffs.ts";
import { cardLabel } from "./names.ts";
import { g } from "./strings.ts";

export interface TipMinion {
  atk: number | null;
  health: number | null;
  /** null: the record is older and did not read them. */
  buffs: readonly Buff[] | null;
}

/** The attributes a minion needs to own a tooltip: focusable, and described by it. */
export function tipHost(id: string): { tabindex: 0; "aria-describedby": string } {
  return { tabindex: 0, "aria-describedby": id };
}

function buffRow(b: Buff): VNode {
  const name = cardLabel(b.cardId, b.cardId);
  return h(
    "li",
    { class: "tip-buff" },
    picture("card", b.cardId, name, 24, true),
    h("span", {}, g("tip.buff", { name, count: b.count })),
  );
}

function buffPart(buffs: readonly Buff[] | null): VNode {
  if (buffs === null) return h("p", { class: "tip-note" }, g("tip.buffs.unknown"));
  if (buffs.length === 0) return h("p", { class: "tip-note" }, g("tip.buffs.none"));
  return h("div", { class: "tip-buffs" }, h("p", { class: "tip-head" }, g("tip.buffs")), h("ul", {}, buffs.map(buffRow)));
}

export function minionTip(id: string, m: TipMinion): VNode {
  return h(
    "span",
    { class: "minion-tip", id, role: "tooltip" },
    h(
      "span",
      { class: "tip-stats" },
      h("span", { class: "tip-atk" }, g("tip.attack", { value: exactStat(m.atk) })),
      h("span", { class: "tip-hp" }, g("tip.health", { value: exactStat(m.health) })),
    ),
    buffPart(m.buffs),
  );
}

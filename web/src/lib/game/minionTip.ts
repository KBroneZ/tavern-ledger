// The tooltip of a minion (T-619, D-142): its exact Attack and Health and the
// buffs it has, each card with its picture, name and how many times. It is
// real markup next to the minion (not a title), shown by the stylesheet on
// hover and on keyboard focus or a tap (the minion takes focus); the minion
// points to it with aria-describedby, so a screen reader hears it too.
// "Empty is not unknown": a game saved before the buffs were read says so.
import { exactStat } from "../stat.ts";
import { h, type VNode } from "../view/node.ts";
import { picture } from "../view/pieces.ts";
import type { Buff } from "./buffs.ts";
import type { GiftMark } from "./gifts.ts";
import { cardLabel } from "./names.ts";
import { g } from "./strings.ts";

export interface TipMinion {
  /** The card, for the plaque's heading (the minion's own title is gone: the plaque is its tooltip). */
  cardId?: string | null;
  atk: number | null;
  health: number | null;
  /** null: the record is older and did not read them. */
  buffs: readonly Buff[] | null;
  /** The minion's Dark Gifts, as words: a screen that cannot hover reads them here (the star's tooltip is a title). */
  gifts?: readonly GiftMark[];
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

/** What the list of buffs is as of: a combat replay's start, or the game a board was loaded from. */
export type TipWhen = "start" | "loaded";

function buffPart(buffs: readonly Buff[] | null, when?: TipWhen): VNode {
  if (buffs === null) return h("p", { class: "tip-note" }, g("tip.buffs.unknown"));
  if (buffs.length === 0) return h("p", { class: "tip-note" }, g("tip.buffs.none"));
  const head = when === "start" ? g("tip.buffs.start") : when === "loaded" ? g("tip.buffs.loaded") : g("tip.buffs");
  return h("div", { class: "tip-buffs" }, h("p", { class: "tip-head" }, head), h("ul", {}, buffs.map(buffRow)));
}

/**
 * Escape hides the plaques (WCAG 1.4.13: dismissible) until the pointer or
 * the focus moves again: one attribute on the page, which tip.css reads.
 */
export function watchTipEscape(doc: Pick<Document, "documentElement" | "addEventListener">): void {
  const root = doc.documentElement;
  const off = () => root.setAttribute("data-tips-off", "");
  const back = () => root.removeAttribute("data-tips-off");
  doc.addEventListener("keydown", (event) => {
    if ((event as KeyboardEvent).key === "Escape") off();
  });
  doc.addEventListener("pointermove", back);
  doc.addEventListener("focusin", back);
}

/** The room a plaque keeps from the screen's edge, in pixels. */
const TIP_MARGIN = 8;

/**
 * How far to move a plaque sideways to be inside the viewport: 0 when it is,
 * else enough to put the edge it crossed at the margin. A plaque wider than
 * the viewport lines up with the left margin.
 */
export function tipShift(rect: { left: number; right: number }, viewport: number): number {
  if (rect.right - rect.left > viewport - 2 * TIP_MARGIN) return TIP_MARGIN - rect.left;
  if (rect.left < TIP_MARGIN) return TIP_MARGIN - rect.left;
  if (rect.right > viewport - TIP_MARGIN) return viewport - TIP_MARGIN - rect.right;
  return 0;
}

/**
 * On a narrow screen a plaque centred on a minion near an end runs off the
 * page: when the pointer or the focus (a tap focuses the minion) reaches a
 * minion, its plaque is moved back in (--tip-dx, read by tip.css).
 */
export function watchTipFit(doc: Pick<Document, "documentElement" | "addEventListener">): void {
  const fit = (event: Event) => {
    const target = event.target as Element | null;
    const host = target?.closest?.("[aria-describedby]");
    const tip = host?.querySelector<HTMLElement>(".minion-tip");
    if (!tip) return;
    // Measured where the stylesheet puts it, then moved.
    tip.style.setProperty("--tip-dx", "0px");
    const rect = tip.getBoundingClientRect();
    // Not laid out (display: none until the hover or focus lands): the next event measures it.
    if (rect.width === 0) return;
    tip.style.setProperty("--tip-dx", `${Math.round(tipShift(rect, doc.documentElement.clientWidth))}px`);
  };
  doc.addEventListener("pointerover", fit);
  doc.addEventListener("focusin", fit);
}

export function minionTip(id: string, m: TipMinion, when?: TipWhen): VNode {
  return h(
    "span",
    { class: "minion-tip", id, role: "tooltip" },
    m.cardId === undefined ? null : h("span", { class: "tip-name" }, cardLabel(m.cardId, g("minion.unknownCard"))),
    h(
      "span",
      { class: "tip-stats" },
      h("span", { class: "tip-atk" }, g("tip.attack", { value: exactStat(m.atk) })),
      h("span", { class: "tip-hp" }, g("tip.health", { value: exactStat(m.health) })),
    ),
    buffPart(m.buffs, when),
    ...(m.gifts ?? []).map((k) => h("p", { class: "tip-gift" }, k.title)),
  );
}

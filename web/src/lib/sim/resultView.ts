// The simulator's answer as element trees (T-606): key numbers first (the
// three shares and the likely damage), details in Advanced (ranges, trials,
// engine, card data, the damage cap) and the gaps folded.
import { compact, share } from "../data/format.ts";
import { listFormat, numberFormat } from "../i18n.ts";
import { cls, h, s, type VNode } from "../view/node.ts";
import { fold } from "../view/pieces.ts";
import type { Answer, Approximate, Spread } from "./answer.ts";
import type { CardIndex } from "./cards.ts";
import { reasonText } from "./reasons.ts";
import { sm } from "./strings.ts";

export type ResultState =
  | { kind: "empty" }
  | { kind: "running"; previous: Answer | null }
  | { kind: "answer"; answer: Answer }
  | { kind: "failed" };

export interface ResultProps {
  result: ResultState;
  cards: CardIndex;
  cap: number | null;
}

/** A count in full ("10,000"), not compact ("10K"). */
const whole = (n: number) => numberFormat({ maximumFractionDigits: 0 }).format(n);

const names = (ids: string[], cards: CardIndex) => listFormat(ids.map((id) => cards.name(id)));

function bar(won: number, tied: number, lost: number, trials: number): VNode {
  const pct = (n: number) => (100 * n) / trials;
  const parts: [string, number][] = [["win", pct(won)], ["tie", pct(tied)], ["lose", pct(lost)]];
  let x = 0;
  return s(
    "svg",
    { class: "sim-bar", viewBox: "0 0 100 6", preserveAspectRatio: "none", "aria-hidden": "true", focusable: "false" },
    parts.map(([kind, width]) => {
      const rect = s("rect", { class: `sim-bar-${kind}`, x: x.toFixed(3), y: 0, width: width.toFixed(3), height: 6 });
      x += width;
      return rect;
    }),
  );
}

function shareCell(kind: "win" | "tie" | "lose", n: number, trials: number): VNode {
  return h(
    "div",
    { class: `sim-share sim-share-${kind}` },
    h("span", { class: "sim-share-value" }, share(n, trials)),
    h("span", { class: "sim-share-label" }, sm(`result.${kind}`)),
  );
}

function rangeRow(label: string, spread: Spread | null): VNode {
  return h(
    "div",
    { class: "sim-range" },
    h("dt", {}, label),
    h(
      "dd",
      {},
      spread ? h("strong", {}, compact(spread.likely)) : sm("result.noDamage"),
      spread && spread.low !== spread.high ? [" ", h("span", { class: "sim-dim" }, sm("result.range", { low: compact(spread.low), high: compact(spread.high) }))] : null,
    ),
  );
}

function approxFold(list: Approximate[], cards: CardIndex): VNode | null {
  if (list.length === 0) return null;
  const summary = list.length === 1 ? sm("result.approx.one") : sm("result.approx.other", { count: list.length });
  return fold(
    { summary: sm("result.approx"), hint: summary, compact: true },
    h(
      "ul",
      { class: "sim-approx", role: "list" },
      list.map((a) => {
        const said = reasonText(a.reason);
        return h("li", {}, h("strong", {}, cards.name(a.cardId)), ": ", h("span", { lang: said.english ? "en" : undefined }, said.text));
      }),
    ),
  );
}

function capInput(cap: number | null): VNode {
  return h(
    "label",
    { class: "sim-cap" },
    h("span", {}, sm("result.cap")),
    h("input", { type: "number", inputmode: "numeric", min: 1, max: 1000, step: 1, value: cap ?? "", placeholder: sm("result.capNone"), "data-field": "cap", "data-focus": "cap" }),
  );
}

function odds(a: Extract<Answer, { kind: "odds" }>, props: ResultProps): VNode[] {
  const { cards } = props;
  const dealt = a.dealt?.likely;
  const taken = a.taken?.likely;
  const likely: string[] = [];
  if (dealt !== undefined) likely.push(sm("result.dealt", { n: compact(dealt) }));
  if (taken !== undefined) likely.push(sm("result.taken", { n: compact(taken) }));
  return [
    h(
      "div",
      { class: "sim-shares", role: "img", "aria-label": sm("result.bar", { win: share(a.won, a.trials), tie: share(a.tied, a.trials), lose: share(a.lost, a.trials) }) },
      shareCell("win", a.won, a.trials),
      shareCell("tie", a.tied, a.trials),
      shareCell("lose", a.lost, a.trials),
    ),
    bar(a.won, a.tied, a.lost, a.trials),
    a.damageKnown
      ? likely.length > 0
        ? h("p", { class: "sim-likely" }, h("span", { class: "sim-dim" }, `${sm("result.likely")} · `), likely.join(" · "))
        : null
      : h("p", { class: "sim-likely sim-dim" }, sm("result.damageUnknown")),
    a.damageKnown
      ? h(
          "dl",
          { class: "sim-ranges adv" },
          rangeRow(sm("result.dealtRange"), a.dealt),
          rangeRow(sm("result.takenRange"), a.taken),
          h("p", { class: "sim-dim" }, sm("result.rangeNote")),
        )
      : null,
    approxFold(a.approximate, cards),
    a.newBuild !== null ? h("p", { class: "sim-note sim-warning", role: "note" }, sm("result.newBuild", { build: a.newBuild })) : null,
    a.startOfCombat ? h("p", { class: "sim-note" }, "≈ ", sm("result.soc")) : null,
    h(
      "p",
      { class: "sim-meta adv" },
      sm("result.trials", { n: whole(a.trials) }),
      a.stalled > 0 ? ` · ${sm("result.stalled", { n: whole(a.stalled) })}` : null,
    ),
  ];
}

/** Each card with the reason the engine gave for refusing it (T-616). */
function whyList(reasons: Approximate[], cards: CardIndex): VNode {
  return h(
    "ul",
    { class: "sim-approx", role: "list" },
    reasons.map((a) => {
      const said = reasonText(a.reason);
      return h("li", {}, h("strong", {}, cards.name(a.cardId)), ": ", h("span", { lang: said.english ? "en" : undefined }, said.text));
    }),
  );
}

function blocked(title: string, ids: string[], cards: CardIndex, hint?: string, reasons: Approximate[] = []): VNode[] {
  return [
    h(
      "div",
      { class: "state state-error sim-blocked" },
      h("p", { class: "state-title" }, title),
      reasons.length > 0 ? whyList(reasons, cards) : h("p", {}, names(ids, cards)),
      hint ? h("p", { class: "sim-dim" }, hint) : null,
    ),
  ];
}

function answerView(a: Answer, props: ResultProps): VNode[] {
  switch (a.kind) {
    case "odds":
      return odds(a, props);
    case "not_modelled":
      return [...blocked(sm("result.notModelled"), a.notModelled, props.cards, sm("result.notModelledHint"), a.reasons), approxFold(a.approximate, props.cards)].filter(
        (n): n is VNode => n !== null,
      );
    case "no_card_data":
      return blocked(sm("result.noCardData"), a.missing, props.cards);
    case "bad_board":
    case "bad_input":
      return [h("div", { class: "state state-error" }, h("p", { class: "state-title" }, sm("result.badBoard")), h("p", { class: "sim-dim adv", lang: "en" }, a.error))];
    case "unreadable":
      return failed();
  }
}

function failed(): VNode[] {
  return [
    h(
      "div",
      { class: "state state-error" },
      h("p", { class: "state-title" }, sm("result.failed")),
      h("button", { type: "button", "data-act": "retry", "data-focus": "retry" }, sm("result.retry")),
    ),
  ];
}

const engineOf = (r: ResultState): string | null => {
  const a = r.kind === "answer" ? r.answer : r.kind === "running" ? r.previous : null;
  return a && a.kind !== "unreadable" ? a.engine : null;
};

/** The result panel's content. */
export function resultView(props: ResultProps): VNode[] {
  const { result, cards } = props;
  let body: VNode[];
  if (result.kind === "empty") body = [h("p", { class: "sim-dim sim-wait" }, sm("result.empty"))];
  else if (result.kind === "failed") body = failed();
  else if (result.kind === "running") body = result.previous ? answerView(result.previous, props) : [h("p", { class: "sim-dim sim-wait" }, sm("result.running"))];
  else body = answerView(result.answer, props);
  const engine = engineOf(result);
  return [
    h("div", { class: cls("sim-result-body", { "is-stale": result.kind === "running" && result.previous !== null }) }, body),
    h(
      "div",
      { class: "sim-settings adv" },
      capInput(props.cap),
      h("p", { class: "sim-meta" }, engine ? `${sm("result.engine", { version: engine })} · ` : "", sm("result.cards", { source: cards.source })),
    ),
  ];
}

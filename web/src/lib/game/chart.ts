// The turns chart as an element tree (T-504), the same marks as the T-501
// TurnBars piece: a won turn is a bar up (damage dealt), a lost turn a bar
// down (damage taken), a tie a flat tick, an unknown result a dashed box. The
// biggest hit each way is labelled; every value is in the turns table that
// follows it on the page.
import { barScale, columnPath, type Result } from "../data/format.ts";
import { t } from "../data/strings.ts";
import { h, s, type VNode } from "../view/node.ts";
import { g } from "./strings.ts";

export interface ChartTurn {
  turn: number;
  result: Result;
  /** Dealt on a win, taken on a loss; null when unknown. */
  damage: number | null;
}

const SLOT = 24;
const BAR = 14;
const HALF = 52;
const PAD = 14;
const MID = PAD + HALF;
const HEIGHT = MID + HALF + PAD + 16;

function peakOf(turns: ChartTurn[], result: Result): ChartTurn | null {
  return turns
    .filter((x) => x.result === result && x.damage !== null)
    .reduce<ChartTurn | null>((a, b) => (!a || (b.damage as number) > (a.damage as number) ? b : a), null);
}

function mark(x: ChartTurn, i: number, scale: (v: number) => number, peaks: Set<ChartTurn | null>): VNode {
  const left = i * SLOT + (SLOT - BAR) / 2;
  const length = x.damage === null ? 0 : scale(x.damage);
  const up = x.result === "win";
  const title = `${t("chart.turns.turn", { turn: x.turn })}: ${t(`result.${x.result}`)}${x.damage === null ? "" : `, ${x.damage}`}`;
  return s(
    "g",
    { class: "mark" },
    s("title", {}, title),
    s("rect", { class: "hit", x: i * SLOT, y: 0, width: SLOT, height: MID + HALF + PAD }),
    (x.result === "win" || x.result === "loss") && length > 0
      ? s("path", { class: `fill-${x.result}`, d: columnPath(left, MID, BAR, length, up ? "up" : "down") })
      : null,
    (x.result === "tie" || length === 0) && x.result !== "unknown" && x.damage !== null
      ? s("rect", { class: `fill-${x.result}`, x: left, y: MID - 2, width: BAR, height: 4, rx: 2 })
      : null,
    x.result === "unknown" || x.damage === null
      ? s("rect", { class: "mark-none", x: left + 0.75, y: MID - 6, width: BAR - 1.5, height: 12, rx: 3 })
      : null,
    peaks.has(x)
      ? s("text", { x: left + BAR / 2, y: up ? MID - length - 4 : MID + length + 12, "text-anchor": "middle", class: "label-strong" }, up ? String(x.damage) : `−${x.damage}`)
      : null,
    s("text", { x: left + BAR / 2, y: HEIGHT - 4, "text-anchor": "middle" }, String(x.turn)),
  );
}

export function turnChart(turns: ChartTurn[], id: string): VNode {
  const width = Math.max(1, turns.length * SLOT);
  const scale = barScale(turns.map((x) => x.damage ?? 0), HALF);
  const peaks = new Set([peakOf(turns, "win"), peakOf(turns, "loss")].filter(Boolean));
  const key = (cls: string, text: string) => h("li", {}, h("i", { class: cls, "aria-hidden": "true" }), text);
  return h(
    "figure",
    { class: "chart chart-turns" },
    h(
      "figcaption",
      {},
      h("p", { class: "chart-title", id: `${id}-title` }, t("chart.turns.title")),
      h("p", { class: "chart-desc", id: `${id}-desc` }, t("chart.turns.desc")),
    ),
    h("ul", { class: "chart-key", "aria-hidden": "true" }, key("key-win", t("result.win")), key("key-loss", t("result.loss")), key("key-tie", t("result.tie")), key("key-unknown", t("result.unknown"))),
    h(
      "div",
      { class: "scroll-x", role: "region", "aria-label": g("turns.chart"), tabindex: 0 },
      s(
        "svg",
        { viewBox: `0 0 ${width} ${HEIGHT}`, width, height: HEIGHT, role: "img", "aria-labelledby": `${id}-title`, "aria-describedby": `${id}-desc` },
        s("line", { class: "axis", x1: 0, x2: width, y1: MID, y2: MID }),
        turns.map((x, i) => mark(x, i, scale, peaks)),
      ),
    ),
  );
}

// The game's turns (T-559, D-131) as one table: turn, tier, result,
// opponent, damage, gold spent and the combat's win odds, with a short mark
// when the likeliest result did not happen. A row opens on its boards, shop,
// the odds in full and the cause. The chart above it is the summary. The
// odds are worked out in the browser after the page is drawn (whyRun.ts
// fills the cells marked data-odds and data-why); until then a cell waits.
import { t } from "../data/strings.ts";
import { href } from "../i18n.ts";
import { h, type VNode } from "../view/node.ts";
import { fold, more, picture, sourceMark, unknownValue } from "../view/pieces.ts";
import { boardBlock, cardRow } from "./boards.ts";
import { turnChart } from "./chart.ts";
import type { ShopTurn } from "./report.ts";
import type { GameView, HeroRef, TurnRow } from "./shape.ts";
import { g } from "./strings.ts";
import * as w from "./words.ts";

export const TURNS_ID = "gv-turns";
export const WHY_STATUS_ID = "gv-why-status";
const COLUMNS = 7;

/** The detail row of a turn: the toggle's aria-controls. */
export const detailId = (turn: number): string => `gv-t${turn}-more`;

function opponentCell(opponents: HeroRef[]): VNode {
  if (opponents.length === 0) return h("td", { class: "gv-opp-col" }, unknownValue());
  return h(
    "td",
    { class: "gv-opp-col" },
    h(
      "span",
      { class: "gv-opps" },
      opponents.map((o) => h("span", { class: "gv-hero", title: w.heroTip(o) }, picture("hero", o.id, w.heroLabel(o), 24, true), h("span", { class: "gv-opp-name" }, w.heroLabel(o)))),
    ),
  );
}

function resultCell(x: TurnRow): VNode {
  if (!x.result) return h("td", {}, h("span", { class: "gv-dim" }, g("turns.noCombat")));
  return h("td", {}, h("span", { class: `res-${x.result}` }, t(`result.${x.result}`)), x.inferred ? " " : null, x.inferred ? sourceMark("inferred") : null);
}

/** One damage splat: the signed number is drawn (the sign carries it, not the colour), the words are read. */
function hitBadge(kind: "dealt" | "taken", n: number, inferred: boolean): VNode {
  return h(
    "span",
    { class: `gv-hit gv-hit-${kind}` },
    h("span", { "aria-hidden": "true" }, `${kind === "dealt" ? "+" : "−"}${n}`),
    h("span", { class: "visually-hidden" }, g(`turns.${kind}`, { n })),
    inferred ? " " : null,
    inferred ? sourceMark("inferred") : null,
  );
}

function damageCell(x: TurnRow): VNode {
  const inferred = x.damageSource === "inferred";
  const badges: VNode[] = [];
  if (x.dealt !== null && x.dealt > 0) badges.push(hitBadge("dealt", x.dealt, inferred));
  if (x.taken !== null && x.taken > 0) badges.push(hitBadge("taken", x.taken, inferred));
  if (badges.length > 0) return h("td", { class: "num" }, h("span", { class: "gv-hits" }, badges));
  // No damage either way: a quiet 0 or dash, with its mark when worked out.
  return h("td", { class: "num" }, w.damageText(x), inferred ? " " : null, inferred ? sourceMark("inferred") : null);
}

/** Waits for the engine: a quiet ellipsis, and words for screen readers. No combat: empty; a result not known: no odds. */
function oddsCell(x: TurnRow): VNode {
  if (!x.result) return h("td", { class: "num gv-odds" });
  if (x.result === "unknown") return h("td", { class: "num gv-odds" }, unknownValue());
  return h(
    "td",
    { class: "num gv-odds", "data-odds": String(x.turn) },
    h("span", { class: "gv-dim", "aria-hidden": "true" }, "…"),
    h("span", { class: "visually-hidden" }, g("turns.odds.loading")),
  );
}

function turnRow(x: TurnRow): VNode {
  return h(
    "tr",
    { class: "gv-turn", "data-turn": String(x.turn) },
    h(
      "th",
      { scope: "row" },
      h(
        "button",
        { type: "button", class: "gv-toggle", "aria-expanded": "false", "aria-controls": detailId(x.turn), "data-toggle": String(x.turn) },
        h("span", { class: "gv-chev", "aria-hidden": "true" }),
        h("span", { "aria-hidden": "true" }, String(x.turn)),
        h("span", { class: "visually-hidden" }, g("turns.toggle", { turn: x.turn })),
      ),
    ),
    h("td", { class: "num gv-wide" }, w.tierText(x), x.tierUp ? h("span", { class: "gv-tierup", title: g("turns.tierUp") }, " ▲") : null),
    resultCell(x),
    opponentCell(x.opponents),
    damageCell(x),
    h("td", { class: "num gv-wide" }, x.goldSpent === null ? "—" : String(x.goldSpent)),
    oddsCell(x),
  );
}

/** The shop in one or two lines: gold in and out, rolls, freezes; then what was bought, sold and passed. */
function shopBlock(shop: ShopTurn, x: TurnRow): VNode {
  const offers = shopsFold(shop, x.turn);
  return h(
    "div",
    { class: "gv-shop" },
    h("p", { class: "gv-board-title" }, g("boards.shopFacts"), " ", h("span", { class: "gv-dim" }, w.shopLine(shop, x))),
    shop.buys.length > 0 ? h("div", { class: "gv-shop-row" }, h("p", { class: "tile-label" }, g("boards.bought")), cardRow(g("boards.bought"), shop.buys)) : null,
    shop.sells.length > 0 ? h("div", { class: "gv-shop-row" }, h("p", { class: "tile-label" }, g("boards.sold")), cardRow(g("boards.sold"), shop.sells)) : null,
    shop.passes && shop.passes.length > 0
      ? h("div", { class: "gv-shop-row" }, h("p", { class: "tile-label" }, g("boards.passed")), cardRow(g("boards.passed"), shop.passes))
      : null,
    offers,
  );
}

function shopsFold(shop: ShopTurn, turn: number): VNode | null {
  const rolls = [...new Set(shop.offers.map((o) => o.roll))].sort((a, b) => a - b);
  if (rolls.length === 0) return null;
  const title = (roll: number) => (roll === 0 ? g("boards.roll.first") : g("boards.roll", { roll }));
  return fold(
    { summary: g("boards.seeShops"), context: g("boards.turn", { turn }), compact: true },
    ...rolls.map((roll) => {
      const offers = shop.offers.filter((o) => o.roll === roll);
      return h(
        "div",
        { class: "gv-roll" },
        h("p", { class: "gv-board-title" }, title(roll)),
        cardRow(title(roll), offers.map((o) => o.cardId), (i) => (offers[i].frozen ? h("span", { class: "gv-frozen" }, g("boards.frozen")) : null)),
      );
    }),
  );
}

function detailBody(x: TurnRow, replay: boolean): VNode[] {
  const ownTitle = x.ownBoardHero ? g("boards.teammates", { hero: w.heroLabel(x.ownBoardHero) }) : g("boards.yours");
  const theirs = x.opponentBoards.length > 0
    ? x.opponentBoards.map((b, i) => boardBlock(`gv-t${x.turn}-opp${i}`, g("boards.theirs", { hero: w.heroLabel(b.hero) }), b.hero, b.board))
    : [boardBlock(`gv-t${x.turn}-opp`, g("boards.theirs", { hero: x.opponents.map(w.heroLabel).join(" + ") || g("hero.unknown") }), null, null)];
  const partner = x.partnerBoard
    ? boardBlock(`gv-t${x.turn}-mate`, g("boards.teammates", { hero: w.heroLabel(x.partnerBoard.hero) }), x.partnerBoard.hero, x.partnerBoard.board)
    : null;
  return [
    h("div", { class: "gv-why-more", "data-why": String(x.turn) }),
    x.result ? h("div", { class: "gv-boards rp" }, ...theirs, boardBlock(`gv-t${x.turn}-own`, ownTitle, x.ownBoardHero, x.ownBoard), partner) : null,
    x.shop ? shopBlock(x.shop, x) : h("p", { class: "data-note" }, g("boards.noShops")),
    replay
      ? h("p", {}, h("button", { type: "button", class: "tb tb-small", "data-replay": String(x.turn) }, g("turns.replay")))
      : null,
  ].filter((n): n is VNode => n !== null);
}

function detailRow(x: TurnRow, replay: boolean): VNode {
  return h(
    "tr",
    { class: "gv-turn-more", id: detailId(x.turn), hidden: true },
    h("td", { colspan: COLUMNS }, h("div", { class: "gv-detail" }, ...detailBody(x, replay))),
  );
}

function turnsTable(view: GameView): VNode {
  const head: [string, string | undefined][] = [
    [g("turns.col.turn"), undefined],
    [g("turns.col.tier"), "num gv-wide"],
    [g("turns.col.result"), undefined],
    [g("turns.col.opponent"), "gv-opp-col"],
    [g("turns.col.damage"), "num"],
    [g("turns.col.gold"), "num gv-wide"],
    [g("turns.col.odds"), "num"],
  ];
  const replays = new Set(view.replays.filter((r) => r.replay !== null).map((r) => r.round));
  return h(
    "div",
    { class: "scroll-x gv-turns-wrap", role: "region", "aria-label": g("turns.table"), tabindex: 0 },
    h(
      "table",
      { class: "dtable gv-turns" },
      h("caption", { class: "visually-hidden" }, g("turns.caption")),
      h("thead", {}, h("tr", {}, head.map(([label, c]) => h("th", { scope: "col", class: c }, label)))),
      h("tbody", {}, view.turns.flatMap((x) => [turnRow(x), detailRow(x, replays.has(x.turn))])),
    ),
  );
}

function notes(view: GameView): VNode[] {
  const inferred = view.turns.some((x) => x.damageSource === "inferred" || x.inferred);
  return [
    inferred ? h("p", { class: "data-note" }, sourceMark("inferred"), " ", g("turns.inferred.note")) : null,
    h("p", { class: "data-note", id: WHY_STATUS_ID, "aria-live": "polite" }, g("turns.odds.note")),
    more(
      { context: g("turns.col.odds") },
      h("p", {}, g("why.desc")),
      h("p", { class: "gv-link" }, h("a", { href: `${href("/accuracy/")}#acc-causes` }, g("why.link"))),
    ),
  ].filter((n): n is VNode => n !== null);
}

export function turnsSection(view: GameView): VNode {
  const turns = view.turns;
  const heading = h("h2", { id: `${TURNS_ID}-title` }, g("turns.title"));
  const attrs = { class: "data-section", "aria-labelledby": `${TURNS_ID}-title` };
  if (turns.length === 0) return h("section", attrs, heading, h("p", { class: "data-note" }, g("turns.empty")));
  const chartTurns = turns.flatMap((x) =>
    x.result === null
      ? []
      : [{ turn: x.turn, result: x.result, damage: x.result === "win" ? x.dealt : x.result === "loss" ? x.taken : x.result === "tie" ? 0 : null }],
  );
  return h(
    "section",
    attrs,
    heading,
    chartTurns.length > 0 ? turnChart(chartTurns, "gv-chart") : null,
    // Expected place per combat (T-654): no trained model ships yet, so every
    // turn would be a dash; one quiet line says why instead of an empty chart.
    chartTurns.length > 0 ? h("p", { class: "data-note gv-place-dash" }, g("turns.place.dash")) : null,
    turnsTable(view),
    ...notes(view),
  );
}

/** Opens or closes a turn's detail row; the page's script calls it on a toggle's click. */
export function toggleTurn(button: HTMLElement): void {
  const row = document.getElementById(button.getAttribute("aria-controls") ?? "");
  if (!row) return;
  const open = button.getAttribute("aria-expanded") !== "true";
  button.setAttribute("aria-expanded", String(open));
  row.hidden = !open;
  button.closest("tr")?.classList.toggle("is-open", open);
}

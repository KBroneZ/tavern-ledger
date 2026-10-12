// One shaped game (T-504) as the page's element trees, on the T-501 pieces
// (lib/view). Simple shows the summary and the turns; Advanced (the T-501
// switch, `.adv`) adds the boards and shops by turn, the boards met, the
// choices, the lobby and the parser's notes. Every value from the record goes
// in as text (lib/view/node.ts never parses HTML).
import { compact, type Mode, type Result } from "../data/format.ts";
import { t } from "../data/strings.ts";
import { cls, h, type VNode } from "../view/node.ts";
import { fold, picture, placeBadge, sourceMark, statTile, type TileProps, unknownValue } from "../view/pieces.ts";
import { turnChart } from "./chart.ts";
import { replaySection } from "./replayView.ts";
import { whyShell } from "./whyView.ts";
import { giftList, giftName } from "./gifts.ts";
import type { Minion, ShopTurn } from "./report.ts";
import type { BoardView, ChoiceView, GameView, HeroRef, OpponentView, SeatView, TurnRow } from "./shape.ts";
import { g } from "./strings.ts";
import * as w from "./words.ts";

// The same in every language (test/unit/i18n.test.ts), so it can be a constant.
const DASH = "—";
const num = (v: number | null) => (v === null ? DASH : compact(v));

/** The same tree, shown in Advanced only. */
function advanced(node: VNode): VNode {
  const has = node.attrs.some(([k]) => k === "class");
  const attrs = node.attrs.map(([k, v]): [string, string] => (k === "class" ? [k, cls(v, "adv")] : [k, v]));
  return { ...node, attrs: has ? attrs : [...attrs, ["class", "adv"]] };
}

function section(id: string, title: string, children: (VNode | null)[], adv = false): VNode {
  return h("section", { class: cls("data-section", { adv }), "aria-labelledby": `${id}-title` }, h("h2", { id: `${id}-title` }, title), children);
}

function note(text: string): VNode {
  return h("p", { class: "data-note" }, text);
}

/** A card id as text: an identifier, not words to translate. */
function cardId(id: string | null): VNode {
  return id ? h("span", { class: "gv-id", translate: "no" }, id) : h("span", { class: "gv-id" }, g("minion.unknownCard"));
}

function heroCell(hero: HeroRef, size: 24 | 32 = 24): VNode {
  const label = w.heroLabel(hero);
  return h("span", { class: "gv-hero" }, picture("hero", hero.id, label, size, true), h("span", {}, label));
}

function resultChip(result: Result, inferred: boolean): VNode {
  return h("span", {}, h("span", { class: `res-${result}` }, t(`result.${result}`)), inferred ? " " : null, inferred ? sourceMark("inferred") : null);
}

function tile(props: TileProps, adv = false): VNode {
  return adv ? advanced(statTile(props)) : statTile(props);
}

function summaryBar(view: GameView, date: string | null): VNode {
  const s = view.summary;
  const mode: Mode = s.mode ?? "solo";
  const fact = (label: string, value: VNode | string) => h("div", {}, h("dt", {}, label), h("dd", {}, value));
  const results = s.results;
  const resultsBacking = [
    results.unknown > 0 ? g("summary.results.unknown", { count: results.unknown }) : "",
    results.inferred > 0 ? g("summary.results.inferred", { count: results.inferred }) : "",
  ].filter(Boolean).join(" · ");
  return h(
    "div",
    { class: "gv-summary", role: "group", "aria-label": g("summary.label") },
    h(
      "div",
      { class: "gv-who" },
      picture("hero", s.hero.id, w.heroLabel(s.hero), 56, true),
      h("div", {}, h("p", { class: "tile-label" }, g("summary.hero")), h("p", { class: "gv-hero-name" }, w.heroLabel(s.hero))),
      h("div", { class: "gv-place" }, h("p", { class: "tile-label" }, g("summary.place")), placeBadge(s.place, mode, "lg")),
    ),
    h(
      "dl",
      { class: "gv-facts" },
      fact(g("summary.mode"), s.mode ? t(`mode.${s.mode}`) : DASH),
      fact(g("summary.date"), date ?? DASH),
      fact(g("summary.rounds"), s.rounds > 0 ? String(s.rounds) : DASH),
      s.teammate ? fact(g("summary.teammate"), heroCell(s.teammate)) : null,
    ),
    s.status && s.status !== "ok" ? h("p", { class: "gv-status" }, g(`status.${s.status}`)) : null,
    h(
      "div",
      { class: "tiles" },
      tile({ label: g("summary.results"), value: results.won + results.lost + results.tie > 0 ? w.resultsLine(s) : DASH, backing: resultsBacking || undefined, source: results.inferred > 0 ? "inferred" : undefined }),
      tile({ label: g("summary.dealt"), value: w.knownValue(s.dealt), backing: w.combatsBacking(s.dealt), source: w.damageSource(s.dealt) }),
      tile({ label: g("summary.taken"), value: w.knownValue(s.taken), backing: w.combatsBacking(s.taken), source: w.damageSource(s.taken) }),
      tile({ label: g("summary.gold"), value: w.knownValue(s.goldSpent), backing: w.turnsBacking(s.goldSpent) }),
      tile({ label: g("summary.tier"), value: num(s.finalTier) }),
      tile({ label: g("summary.triples"), value: num(s.triples) }),
      tile({ label: g("summary.rolls"), value: num(s.rolls) }, true),
      tile({ label: g("summary.buys"), value: num(s.buys) }, true),
      tile({ label: g("summary.sells"), value: num(s.sells) }, true),
      tile({ label: g("summary.apm"), value: w.apmText(s.apm) }, true),
    ),
    note(g("summary.source")),
  );
}

function turnsTable(turns: TurnRow[]): VNode {
  const head = ["turns.col.turn", "turns.col.tier", "turns.col.result", "turns.col.opponent", "turns.col.damage", "turns.col.gold"] as const;
  const rows = turns.map((x) =>
    h(
      "tr",
      {},
      h("th", { scope: "row" }, String(x.turn)),
      h("td", {}, w.tierText(x), x.tierUp ? h("span", { class: "gv-tierup" }, ` ${g("turns.tierUp")}`) : null),
      h("td", {}, x.result ? resultChip(x.result, x.inferred) : h("span", { class: "gv-dim" }, g("turns.noCombat"))),
      h("td", {}, x.opponents.length === 0 ? unknownValue() : x.opponents.flatMap((o, i) => (i === 0 ? [heroCell(o)] : [", ", heroCell(o)]))),
      h("td", { class: "num" }, w.damageText(x), x.damageSource === "log" ? " " : null, x.damageSource === "log" ? sourceMark("log") : null),
      h("td", { class: "num" }, w.goldText(x)),
    ),
  );
  return h(
    "div",
    { class: "scroll-x", role: "region", "aria-label": g("turns.table"), tabindex: 0 },
    h(
      "table",
      { class: "dtable gv-turns" },
      h("caption", { class: "visually-hidden" }, g("turns.caption")),
      h("thead", {}, h("tr", {}, head.map((k, i) => h("th", { scope: "col", class: i >= 4 ? "num" : undefined }, g(k))))),
      h("tbody", {}, rows),
    ),
  );
}

function turnsSection(view: GameView): VNode {
  const turns = view.turns;
  if (turns.length === 0) return section("gv-turns", g("turns.title"), [note(g("turns.empty"))]);
  const chartTurns = turns.flatMap((x) =>
    x.result === null
      ? []
      : [{ turn: x.turn, result: x.result, damage: x.result === "win" ? x.dealt : x.result === "loss" ? x.taken : x.result === "tie" ? 0 : null }],
  );
  return section("gv-turns", g("turns.title"), [
    chartTurns.length > 0 ? turnChart(chartTurns, "gv-chart") : null,
    turnsTable(turns),
    h("p", { class: "data-note" }, sourceMark("log"), " ", sourceMark("inferred"), " ", g("turns.damage.note")),
  ]);
}

function minionItem(m: Minion): VNode {
  return h(
    "li",
    { class: cls("gv-minion", { "is-golden": m.golden }) },
    picture("card", m.cardId, w.minionWords(m), 40, true),
    h(
      "span",
      { class: "gv-minion-text" },
      h("span", { class: "gv-stats" }, h("span", { "aria-hidden": "true" }, w.statsText(m)), h("span", { class: "visually-hidden" }, w.statsWords(m))),
      cardId(m.cardId),
      m.golden ? h("span", { class: "gv-golden" }, g("minion.golden")) : null,
      m.keywords.length > 0 ? h("span", { class: "gv-keywords" }, w.keywordWords(m.keywords)) : null,
      giftList(m),
    ),
  );
}

function board(id: string, title: string, view: BoardView | null): VNode {
  return h(
    "div",
    { class: "gv-board" },
    h("p", { class: "gv-board-title", id }, title, " ", h("span", { class: "gv-dim" }, view ? w.boardTotals(view) : g("boards.unknown"))),
    view && view.minions.length > 0 ? h("ul", { class: "gv-minions", role: "list", "aria-labelledby": id }, view.minions.map(minionItem)) : null,
  );
}

function cardList(label: string, cards: (string | null)[]): VNode {
  return h(
    "div",
    { class: "gv-cards" },
    h("dt", {}, label),
    h("dd", {}, cards.length === 0 ? g("boards.none") : h("ul", { class: "pic-row gv-pics", role: "list" }, cards.map((c) => h("li", {}, picture("card", c, c ?? g("minion.unknownCard"), 32))))),
  );
}

function shopFacts(shop: ShopTurn): VNode {
  const fact = (label: string, value: string) => h("div", {}, h("dt", {}, label), h("dd", {}, value));
  const rolls = shop.rolls === null ? DASH : `${shop.rolls}${shop.freeRolls ? ` (${shop.freeRolls} ${g("shop.freeRolls")})` : ""}`;
  return h(
    "dl",
    { class: "gv-shop" },
    fact(g("shop.gold"), num(shop.gold)),
    fact(g("shop.spent"), num(shop.goldSpent)),
    fact(g("shop.rolls"), rolls),
    fact(g("shop.freezes"), num(shop.freezes)),
    shop.extraGold !== null ? fact(g("shop.extra"), num(shop.extraGold)) : null,
    shop.sellGold !== null ? fact(g("shop.sellGold"), num(shop.sellGold)) : null,
    cardList(g("boards.bought"), shop.buys),
    cardList(g("boards.sold"), shop.sells),
    shop.passes && shop.passes.length > 0 ? cardList(g("boards.passed"), shop.passes) : null,
  );
}

function shops(shop: ShopTurn, turn: number): VNode {
  const rolls = [...new Set(shop.offers.map((o) => o.roll))].sort((a, b) => a - b);
  if (rolls.length === 0) return note(g("boards.noShops"));
  const offers = (roll: number) =>
    shop.offers
      .filter((o) => o.roll === roll)
      .map((o) => h("li", {}, picture("card", o.cardId, o.cardId ?? g("minion.unknownCard"), 40), o.frozen ? h("span", { class: "gv-frozen" }, g("boards.frozen")) : null));
  return fold(
    { summary: g("boards.seeShops"), context: g("boards.turn", { turn }), compact: true },
    ...rolls.map((roll) =>
      h(
        "div",
        { class: "gv-roll" },
        h("p", { class: "gv-board-title" }, roll === 0 ? g("boards.roll.first") : g("boards.roll", { roll })),
        h("ul", { class: "gv-offers", role: "list", "aria-label": roll === 0 ? g("boards.roll.first") : g("boards.roll", { roll }) }, offers(roll)),
      ),
    ),
  );
}

function turnFold(x: TurnRow): VNode {
  const ownTitle = x.ownBoardHero ? g("boards.teammates", { hero: w.heroLabel(x.ownBoardHero) }) : g("boards.yours");
  const theirs = (hero: string) => g("boards.theirs", { hero });
  const opponentBoards = x.opponentBoards.length > 0
    ? x.opponentBoards.map((b, i) => board(`gv-t${x.turn}-opp${i}`, theirs(w.heroLabel(b.hero)), b.board))
    : [board(`gv-t${x.turn}-opp`, theirs(x.opponents.map(w.heroLabel).join(" + ") || g("hero.unknown")), null)];
  const body = [
    x.result ? board(`gv-t${x.turn}-own`, ownTitle, x.ownBoard) : null,
    ...(x.result ? opponentBoards : []),
    x.shop ? h("p", { class: "gv-board-title" }, g("boards.shopFacts")) : null,
    x.shop ? shopFacts(x.shop) : null,
    x.shop ? shops(x.shop, x.turn) : null,
  ].filter((n): n is VNode => n !== null);
  return fold({ summary: g("boards.turn", { turn: x.turn }), hint: x.result ? t(`result.${x.result}`) : g("turns.noCombat") }, ...body);
}

function boardsSection(view: GameView): VNode {
  const hasShop = view.turns.some((x) => x.shop !== null);
  return section("gv-boards", g("boards.title"), [hasShop ? null : note(g("boards.noShopRecord")), ...view.turns.map(turnFold)], true);
}

function opponentFold(o: OpponentView, index: number): VNode {
  const title = o.heroes.map(w.heroLabel).join(" + ") || g("hero.unknown");
  return h(
    "details",
    { class: "fold" },
    h(
      "summary",
      {},
      h("span", { class: "gv-opp" }, o.heroes.map((hero) => picture("hero", hero.id, w.heroLabel(hero), 32, true)), h("span", {}, title)),
      h("span", { class: "fold-hint" }, g("opponents.record", o.record)),
    ),
    h("div", { class: "fold-body" }, o.boards.map((b, j) => board(`gv-o${index}-${j}`, `${g("opponents.round", { turn: b.round })} · ${w.heroLabel(b.hero)}`, b.board))),
  );
}

function opponentsSection(view: GameView): VNode {
  const list = view.opponents;
  return section("gv-opponents", g("opponents.title"), [note(g("opponents.desc")), ...(list.length > 0 ? list.map((o, i) => opponentFold(o, i)) : [note(g("opponents.empty"))])], true);
}

function choiceItem(c: ChoiceView, index: number): VNode {
  return h(
    "li",
    { class: "gv-choice" },
    h("p", { class: "gv-board-title", id: `gv-c${index}` }, g(`choices.kind.${c.kind}`), " ", h("span", { class: "gv-dim" }, g("choices.turn", { turn: c.turn }))),
    h(
      "ul",
      { class: "gv-options", role: "list", "aria-labelledby": `gv-c${index}` },
      c.options.map((o) =>
        h(
          "li",
          { class: o.taken ? "is-taken" : undefined },
          picture("card", o.cardId, o.cardId ?? g("minion.unknownCard"), 40, true),
          cardId(o.cardId),
          o.gift ? h("span", { class: "gv-dim" }, g("choices.gift", { gift: giftName(o.gift) })) : null,
          o.taken ? h("span", { class: "gv-taken" }, g("choices.taken")) : null,
        ),
      ),
    ),
    c.takenKnown ? null : h("p", { class: "gv-dim" }, g("choices.takenUnknown")),
  );
}

function heroPick(pick: NonNullable<GameView["heroPick"]>): VNode {
  const rerolls = pick.rerolls === null ? "" : w.plural("choices.rerolls.one", "choices.rerolls.other", pick.rerolls);
  return h(
    "div",
    { class: "gv-choice" },
    h("p", { class: "gv-board-title", id: "gv-pick" }, g("choices.heroPick"), " ", h("span", { class: "gv-dim" }, rerolls)),
    h(
      "ul",
      { class: "gv-options", role: "list", "aria-labelledby": "gv-pick" },
      pick.offered.map((hero) =>
        h("li", { class: hero.id === pick.picked ? "is-taken" : undefined }, heroCell(hero, 32), hero.id === pick.picked ? h("span", { class: "gv-taken" }, g("choices.taken")) : null),
      ),
    ),
  );
}

function choicesSection(view: GameView): VNode {
  const choices = view.choices;
  const list = choices === null
    ? note(g("choices.notRecorded"))
    : choices.length === 0
      ? note(g("choices.empty"))
      : h("ol", { class: "gv-choices", role: "list" }, choices.map((c, i) => choiceItem(c, i)));
  return section("gv-choices", g("choices.title"), [
    view.heroPick ? heroPick(view.heroPick) : null,
    view.oldGod !== null ? note(g("choices.oldGod", { id: view.oldGod })) : null,
    list,
  ], true);
}

const SLOTS = { first: "lobby.slot.first", second: "lobby.slot.second", hero_power: "lobby.slot.hero_power" } as const;

function seatRow(seat: SeatView, mode: Mode, duos: boolean): VNode {
  const who = seat.own ? g("lobby.you") : seat.teammate ? g("lobby.teammate") : null;
  const trinkets = seat.trinkets === null
    ? g("lobby.notRecorded")
    : seat.trinkets.length === 0
      ? DASH
      : seat.trinkets.map((x) => g("lobby.trinket", { id: x.dbfId, slot: g(SLOTS[x.slot]), turn: x.turn })).join("; ");
  return h(
    "tr",
    { class: seat.own ? "is-own" : undefined },
    h("td", {}, placeBadge(seat.place, mode)),
    h("th", { scope: "row" }, heroCell(seat.hero), who ? h("span", { class: "gv-dim" }, ` (${who})`) : null),
    duos ? h("td", { class: "num" }, seat.team === null ? DASH : String(seat.team)) : null,
    h("td", { class: "num" }, seat.triples === null ? g("lobby.notRecorded") : String(seat.triples)),
    h("td", {}, trinkets),
  );
}

function lobbySection(view: GameView): VNode {
  const mode: Mode = view.summary.mode ?? "solo";
  const duos = mode === "duos";
  if (view.lobby.length === 0) return section("gv-lobby", g("lobby.title"), [note(g("lobby.empty"))], true);
  const cols: [string, boolean][] = [
    [g("lobby.col.place"), false],
    [g("lobby.col.hero"), false],
    ...(duos ? [[g("lobby.col.team"), true] as [string, boolean]] : []),
    [g("lobby.col.triples"), true],
    [g("lobby.col.trinkets"), false],
  ];
  return section("gv-lobby", g("lobby.title"), [
    note(g("lobby.desc")),
    h(
      "div",
      { class: "scroll-x", role: "region", "aria-label": g("lobby.table"), tabindex: 0 },
      h(
        "table",
        { class: "dtable" },
        h("caption", { class: "visually-hidden" }, g("lobby.title")),
        h("thead", {}, h("tr", {}, cols.map(([label, isNum]) => h("th", { scope: "col", class: isNum ? "num" : undefined }, label)))),
        h("tbody", {}, view.lobby.map((seat) => seatRow(seat, mode, duos))),
      ),
    ),
  ], true);
}

function notesSection(view: GameView): VNode | null {
  if (view.notes.length === 0) return null;
  return section("gv-notes", g("notes.title"), [h("ul", { class: "gv-notes" }, view.notes.map((n) => h("li", {}, n)))], true);
}

/** Every section of one game, in page order. */
export function renderGame(view: GameView, date: string | null): VNode[] {
  return [
    summaryBar(view, date),
    turnsSection(view),
    replaySection(view),
    whyShell(),
    boardsSection(view),
    opponentsSection(view),
    choicesSection(view),
    lobbySection(view),
    notesSection(view),
    note(g("credit")),
  ].filter((n): n is VNode => n !== null);
}

// Draws one shaped game (T-504) as the page's sections. Simple shows the
// summary and the turns; Advanced (the T-501 switch, `.adv`) adds the boards
// and shops by turn, the boards met, the choices, the lobby and the parser's
// notes. Every value from the record goes in as text.
import { compact, type Mode } from "../data/format.ts";
import { t } from "../data/strings.ts";
import { turnChart } from "./chart.ts";
import { el, fold, note, picture, placeBadge, resultChip, section, sourceMark, tile } from "./dom.ts";
import type { Minion, ShopTurn } from "./report.ts";
import type { BoardView, ChoiceView, GameView, HeroRef, OpponentView, SeatView, TurnRow } from "./shape.ts";
import { g } from "./strings.ts";
import * as w from "./words.ts";

const DASH = t("value.none");
const num = (v: number | null) => (v === null ? DASH : compact(v));

function heroCell(hero: HeroRef, size: 24 | 32 = 24): HTMLElement {
  const label = w.heroLabel(hero);
  return el("span", { class: "gv-hero" }, picture("hero", hero.id, label, size, true), el("span", {}, label));
}

function summaryBar(view: GameView, date: string | null): HTMLElement {
  const s = view.summary;
  const mode: Mode = s.mode ?? "solo";
  const facts = el(
    "dl",
    { class: "gv-facts" },
    el("div", {}, el("dt", {}, g("summary.mode")), el("dd", {}, s.mode ? t(`mode.${s.mode}`) : DASH)),
    el("div", {}, el("dt", {}, g("summary.date")), el("dd", {}, date ?? DASH)),
    el("div", {}, el("dt", {}, g("summary.rounds")), el("dd", {}, String(s.rounds))),
    s.teammate ? el("div", {}, el("dt", {}, g("summary.teammate")), el("dd", {}, heroCell(s.teammate))) : null,
  );
  const status = s.status && s.status !== "ok" ? el("p", { class: "gv-status" }, g(`status.${s.status}`)) : null;
  const results = s.results;
  const resultsBacking = [
    results.unknown > 0 ? g("summary.results.unknown", { count: results.unknown }) : "",
    results.inferred > 0 ? g("summary.results.inferred", { count: results.inferred }) : "",
  ].filter(Boolean).join(" · ");
  return el(
    "div",
    { class: "gv-summary", role: "group", "aria-label": g("summary.label") },
    el(
      "div",
      { class: "gv-who" },
      picture("hero", s.hero.id, w.heroLabel(s.hero), 56),
      el("div", {}, el("p", { class: "tile-label" }, g("summary.hero")), el("p", { class: "gv-hero-name" }, w.heroLabel(s.hero))),
      el("div", { class: "gv-place" }, el("p", { class: "tile-label" }, g("summary.place")), placeBadge(s.place, mode, true)),
    ),
    facts,
    status,
    el(
      "div",
      { class: "tiles" },
      tile(g("summary.results"), s.rounds > 0 ? w.resultsLine(s) : DASH, { backing: resultsBacking || undefined, source: results.inferred > 0 ? "inferred" : undefined }),
      tile(g("summary.dealt"), w.knownValue(s.dealt), { backing: w.combatsBacking(s.dealt), source: "inferred" }),
      tile(g("summary.taken"), w.knownValue(s.taken), { backing: w.combatsBacking(s.taken), source: "inferred" }),
      tile(g("summary.gold"), w.knownValue(s.goldSpent), { backing: w.turnsBacking(s.goldSpent) }),
      tile(g("summary.tier"), num(s.finalTier)),
      tile(g("summary.triples"), num(s.triples)),
      tile(g("summary.rolls"), num(s.rolls), { cls: "adv" }),
      tile(g("summary.buys"), num(s.buys), { cls: "adv" }),
      tile(g("summary.sells"), num(s.sells), { cls: "adv" }),
      tile(g("summary.apm"), w.apmText(s.apm), { cls: "adv" }),
    ),
    note(g("summary.source")),
  );
}

function turnsTable(turns: TurnRow[]): HTMLElement {
  const head = ["turns.col.turn", "turns.col.tier", "turns.col.result", "turns.col.opponent", "turns.col.damage", "turns.col.gold"] as const;
  const rows = turns.map((x) =>
    el(
      "tr",
      {},
      el("th", { scope: "row" }, String(x.turn)),
      el("td", {}, w.tierText(x), x.tierUp ? el("span", { class: "gv-tierup" }, ` ${g("turns.tierUp")}`) : null),
      el("td", {}, x.result ? resultChip(x.result, x.inferred) : el("span", { class: "gv-dim" }, g("turns.noCombat"))),
      el("td", {}, ...x.opponents.map((h) => heroCell(h))),
      el("td", { class: "num" }, w.damageText(x)),
      el("td", { class: "num" }, w.goldText(x)),
    ),
  );
  return el(
    "div",
    { class: "scroll-x", role: "region", "aria-label": g("turns.title"), tabindex: 0 },
    el(
      "table",
      { class: "dtable gv-turns" },
      el("caption", { class: "visually-hidden" }, g("turns.title")),
      el("thead", {}, el("tr", {}, ...head.map((k, i) => el("th", { scope: "col", class: i >= 4 ? "num" : null }, g(k))))),
      el("tbody", {}, ...rows),
    ),
  );
}

function turnsSection(view: GameView): HTMLElement {
  const turns = view.turns;
  if (turns.length === 0) return section("gv-turns", g("turns.title"), [note(g("turns.empty"))]);
  const chartTurns = turns
    .filter((x) => x.result !== null)
    .map((x) => ({
      turn: x.turn,
      result: x.result ?? "unknown",
      damage: x.result === "win" ? x.dealt : x.result === "loss" ? x.taken : x.result === "tie" ? 0 : null,
      inferred: x.inferred,
    }));
  return section("gv-turns", g("turns.title"), [
    chartTurns.length > 0 ? turnChart(chartTurns, "gv-chart") : null,
    turnsTable(turns),
    el("p", { class: "data-note" }, sourceMark("inferred"), " ", g("turns.damage.note")),
  ]);
}

function minionItem(m: Minion): HTMLElement {
  const words = w.minionWords(m);
  return el(
    "li",
    { class: `gv-minion${m.golden ? " is-golden" : ""}` },
    picture("card", m.cardId, words, 40, true),
    el(
      "span",
      { class: "gv-minion-text" },
      el("span", { class: "gv-stats" }, w.statsText(m)),
      el("span", { class: "gv-id" }, m.cardId ?? g("minion.unknownCard")),
      m.golden ? el("span", { class: "gv-golden" }, g("minion.golden")) : null,
      m.keywords.length > 0 ? el("span", { class: "gv-keywords" }, w.keywordWords(m.keywords)) : null,
    ),
  );
}

function board(title: string, view: BoardView | null): HTMLElement {
  return el(
    "div",
    { class: "gv-board" },
    el("p", { class: "gv-board-title" }, title, " ", el("span", { class: "gv-dim" }, view ? w.boardTotals(view) : g("boards.unknown"))),
    view && view.minions.length > 0 ? el("ul", { class: "gv-minions" }, ...view.minions.map(minionItem)) : null,
  );
}

function cardList(label: string, cards: (string | null)[] | null): HTMLElement | null {
  if (cards === null) return null;
  return el(
    "div",
    { class: "gv-cards" },
    el("dt", {}, label),
    el("dd", {}, cards.length === 0 ? g("boards.none") : el("span", { class: "pic-row gv-pics" }, ...cards.map((c) => picture("card", c, c ?? g("minion.unknownCard"), 32)))),
  );
}

function shopFacts(shop: ShopTurn): HTMLElement {
  const fact = (label: string, value: string) => el("div", {}, el("dt", {}, label), el("dd", {}, value));
  const rolls = shop.rolls === null ? DASH : `${shop.rolls}${shop.freeRolls ? ` (${shop.freeRolls} ${g("shop.freeRolls")})` : ""}`;
  return el(
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

function shops(shop: ShopTurn): HTMLElement {
  const rolls = [...new Set(shop.offers.map((o) => o.roll))].sort((a, b) => a - b);
  if (rolls.length === 0) return note(g("boards.noShops"));
  return fold(
    g("boards.seeShops"),
    rolls.map((roll) =>
      el(
        "div",
        { class: "gv-roll" },
        el("p", { class: "gv-board-title" }, roll === 0 ? g("boards.roll.first") : g("boards.roll", { roll })),
        el(
          "ul",
          { class: "gv-offers" },
          ...shop.offers
            .filter((o) => o.roll === roll)
            .map((o) => el("li", {}, picture("card", o.cardId, o.cardId ?? g("minion.unknownCard"), 40), o.frozen ? el("span", { class: "gv-frozen" }, g("boards.frozen")) : null)),
        ),
      ),
    ),
    { compact: true },
  );
}

function turnFold(x: TurnRow): HTMLElement {
  const ownTitle = x.ownBoardHero ? g("boards.teammates", { hero: w.heroLabel(x.ownBoardHero) }) : g("boards.yours");
  const hint = x.result ? t(`result.${x.result}`) : g("turns.noCombat");
  const body = [
    x.result ? board(ownTitle, x.ownBoard) : null,
    x.result ? board(g("boards.theirs", { hero: x.opponents.map(w.heroLabel).join(" + ") || g("hero.unknown") }), x.opponentBoard) : null,
    x.shop ? el("p", { class: "gv-board-title" }, g("boards.shopFacts")) : null,
    x.shop ? shopFacts(x.shop) : null,
    x.shop ? shops(x.shop) : null,
  ];
  return fold(g("boards.turn", { turn: x.turn }), body, { hint });
}

function boardsSection(view: GameView, hasShop: boolean): HTMLElement {
  return section("gv-boards", g("boards.title"), [hasShop ? null : note(g("boards.noShopRecord")), ...view.turns.map(turnFold)], true);
}

function opponentFold(o: OpponentView): HTMLElement {
  const title = o.heroes.map(w.heroLabel).join(" + ") || g("hero.unknown");
  const record = g("opponents.record", o.record);
  return el(
    "details",
    { class: "fold" },
    el("summary", {}, el("span", { class: "gv-opp" }, ...o.heroes.map((h) => picture("hero", h.id, w.heroLabel(h), 32, true)), el("span", {}, title)), el("span", { class: "fold-hint" }, record)),
    el(
      "div",
      { class: "fold-body" },
      ...o.boards.map((b) => board(`${g("opponents.round", { turn: b.round })} · ${w.heroLabel(b.hero)}`, b.board)),
    ),
  );
}

function opponentsSection(view: GameView): HTMLElement {
  const list = view.opponents;
  return section("gv-opponents", g("opponents.title"), [note(g("opponents.desc")), ...(list.length > 0 ? list.map(opponentFold) : [note(g("opponents.empty"))])], true);
}

function choiceItem(c: ChoiceView): HTMLElement {
  return el(
    "li",
    { class: "gv-choice" },
    el("p", { class: "gv-board-title" }, g(`choices.kind.${c.kind}`), " ", el("span", { class: "gv-dim" }, g("choices.turn", { turn: c.turn }))),
    el(
      "ul",
      { class: "gv-options" },
      ...c.options.map((o) =>
        el(
          "li",
          { class: o.taken ? "is-taken" : null },
          picture("card", o.cardId, o.cardId ?? g("minion.unknownCard"), 40, true),
          el("span", { class: "gv-id" }, o.cardId ?? g("minion.unknownCard")),
          o.gift ? el("span", { class: "gv-dim" }, g("choices.gift", { gift: o.gift })) : null,
          o.taken ? el("span", { class: "gv-taken" }, g("choices.taken")) : null,
        ),
      ),
    ),
    c.takenKnown ? null : el("p", { class: "gv-dim" }, g("choices.takenUnknown")),
  );
}

function choicesSection(view: GameView): HTMLElement {
  const pick = view.heroPick;
  const body = [
    pick
      ? el(
          "div",
          { class: "gv-choice" },
          el("p", { class: "gv-board-title" }, g("choices.heroPick"), " ", el("span", { class: "gv-dim" }, pick.rerolls === null ? "" : w.plural("choices.rerolls.one", "choices.rerolls.other", pick.rerolls))),
          el("ul", { class: "gv-options" }, ...pick.offered.map((h) => el("li", { class: h.id === pick.picked ? "is-taken" : null }, heroCell(h, 32), h.id === pick.picked ? el("span", { class: "gv-taken" }, g("choices.taken")) : null))),
        )
      : null,
    view.oldGod !== null ? note(g("choices.oldGod", { id: view.oldGod })) : null,
    view.choices === null
      ? note(g("choices.notRecorded"))
      : view.choices.length === 0
        ? note(g("choices.empty"))
        : el("ol", { class: "gv-choices" }, ...view.choices.map(choiceItem)),
  ];
  return section("gv-choices", g("choices.title"), body, true);
}

function seatRow(s: SeatView, mode: Mode, duos: boolean): HTMLElement {
  const who = s.own ? g("lobby.you") : s.teammate ? g("lobby.teammate") : null;
  const trinkets = s.trinkets === null
    ? g("lobby.notRecorded")
    : s.trinkets.length === 0
      ? DASH
      : s.trinkets.map((x) => g("lobby.trinket", { id: x.dbfId, slot: g(`lobby.slot.${x.slot as "first" | "second" | "hero_power"}`), turn: x.turn })).join("; ");
  return el(
    "tr",
    { class: s.own ? "is-own" : null },
    el("td", {}, placeBadge(s.place, mode)),
    el("th", { scope: "row" }, heroCell(s.hero), who ? el("span", { class: "gv-dim" }, ` (${who})`) : null),
    duos ? el("td", { class: "num" }, s.team === null ? DASH : String(s.team)) : null,
    el("td", { class: "num" }, s.triples === null ? g("lobby.notRecorded") : String(s.triples)),
    el("td", {}, trinkets),
  );
}

function lobbySection(view: GameView): HTMLElement {
  const mode: Mode = view.summary.mode ?? "solo";
  const duos = mode === "duos";
  if (view.lobby.length === 0) return section("gv-lobby", g("lobby.title"), [note(g("lobby.empty"))], true);
  const cols = ["lobby.col.place", "lobby.col.hero", ...(duos ? ["lobby.col.team"] : []), "lobby.col.triples", "lobby.col.trinkets"] as const;
  return section(
    "gv-lobby",
    g("lobby.title"),
    [
      note(g("lobby.desc")),
      el(
        "div",
        { class: "scroll-x", role: "region", "aria-label": g("lobby.title"), tabindex: 0 },
        el(
          "table",
          { class: "dtable" },
          el("caption", { class: "visually-hidden" }, g("lobby.title")),
          el("thead", {}, el("tr", {}, ...cols.map((k) => el("th", { scope: "col", class: k === "lobby.col.triples" || k === "lobby.col.team" ? "num" : null }, g(k as Parameters<typeof g>[0]))))),
          el("tbody", {}, ...view.lobby.map((s) => seatRow(s, mode, duos))),
        ),
      ),
    ],
    true,
  );
}

function notesSection(view: GameView): HTMLElement | null {
  if (view.notes.length === 0) return null;
  return section("gv-notes", g("notes.title"), [el("ul", { class: "gv-notes" }, ...view.notes.map((n) => el("li", {}, n)))], true);
}

/** Every section of one game, in page order. */
export function renderGame(view: GameView, date: string | null): HTMLElement[] {
  const hasShop = view.turns.some((x) => x.shop !== null);
  return [
    summaryBar(view, date),
    turnsSection(view),
    boardsSection(view, hasShop),
    opponentsSection(view),
    choicesSection(view),
    lobbySection(view),
    notesSection(view),
    note(g("credit")),
  ].filter((n): n is HTMLElement => n !== null);
}

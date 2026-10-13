// One shaped game (T-504) as the page's element trees, on the T-501 pieces
// (lib/view). Simple shows the summary's key numbers, the turns (one table
// whose rows open on the boards, shop and odds, T-559) and the replays;
// Advanced (the T-501 switch, `.adv`) adds the other numbers, the boards
// met, the choices, the lobby and the parser's notes. Every value from the
// record goes in as text (lib/view/node.ts never parses HTML).
import { compact, type Mode } from "../data/format.ts";
import { t } from "../data/strings.ts";
import { cls, h, type VNode } from "../view/node.ts";
import { picture, placeBadge, statTile, type TileProps } from "../view/pieces.ts";
import { boardBlock } from "./boards.ts";
import { replaySection } from "./replayView.ts";
import { turnsSection } from "./turnsView.ts";
import { giftName } from "./gifts.ts";
import { cardLabel, trinketName } from "./names.ts";
import type { ChoiceView, GameView, HeroRef, OpponentView, SeatView } from "./shape.ts";
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

/** A card's name; its id (an identifier, not words to translate) when the card data lacks it. */
function cardText(id: string | null): VNode {
  const label = cardLabel(id, g("minion.unknownCard"));
  return h("span", { class: label === id ? "gv-id" : undefined, translate: label === id ? "no" : undefined }, label);
}

function heroCell(hero: HeroRef, size: 24 | 32 = 24): VNode {
  const label = w.heroLabel(hero);
  return h("span", { class: "gv-hero", title: w.heroTip(hero) }, picture("hero", hero.id, label, size, true), h("span", {}, label));
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
  const fought = results.won + results.lost + results.tie > 0;
  return h(
    "div",
    { class: "gv-summary", role: "group", "aria-label": g("summary.label") },
    h(
      "div",
      { class: "gv-who" },
      picture("hero", s.hero.id, w.heroLabel(s.hero), 56, true),
      h("div", {}, h("p", { class: "tile-label" }, g("summary.hero")), h("p", { class: "gv-hero-name", title: w.heroTip(s.hero) }, w.heroLabel(s.hero))),
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
      { class: "tiles gv-tiles" },
      tile({
        label: results.tie > 0 ? g("summary.results.wlt") : g("summary.results.wl"),
        value: fought ? w.resultsShort(s) : DASH,
        backing: resultsBacking || undefined,
        // Unknown results stay in view: "11-4" alone would read as every combat.
        backingAlways: results.unknown > 0,
        source: results.inferred > 0 ? "inferred" : undefined,
      }),
      tile({ label: g("summary.dealt"), value: w.knownValue(s.dealt), backing: w.combatsBacking(s.dealt), backingAlways: w.partial(s.dealt), source: w.damageMark(s.dealt) }),
      tile({ label: g("summary.taken"), value: w.knownValue(s.taken), backing: w.combatsBacking(s.taken), backingAlways: w.partial(s.taken), source: w.damageMark(s.taken) }),
      tile({ label: g("summary.tier"), value: num(s.finalTier) }),
      tile({ label: g("summary.gold"), value: w.knownValue(s.goldSpent), backing: w.turnsBacking(s.goldSpent) }, true),
      tile({ label: g("summary.triples"), value: num(s.triples) }, true),
      tile({ label: g("summary.rolls"), value: num(s.rolls) }, true),
      tile({ label: g("summary.buys"), value: num(s.buys) }, true),
      tile({ label: g("summary.sells"), value: num(s.sells) }, true),
      tile({ label: g("summary.apm"), value: w.apmText(s.apm) }, true),
    ),
  );
}

function opponentFold(o: OpponentView, index: number): VNode {
  const title = o.heroes.map(w.heroLabel).join(" + ") || g("hero.unknown");
  return h(
    "details",
    { class: "fold" },
    h(
      "summary",
      {},
      h("span", { class: "gv-opp", title: o.heroes.map(w.heroTip).filter(Boolean).join(", ") || undefined }, o.heroes.map((hero) => picture("hero", hero.id, w.heroLabel(hero), 32, true)), h("span", {}, title)),
      h("span", { class: "fold-hint" }, g("opponents.record", o.record)),
    ),
    h("div", { class: "fold-body" }, o.boards.map((b, j) => boardBlock(`gv-o${index}-${j}`, `${g("opponents.round", { turn: b.round })} · ${w.heroLabel(b.hero)}`, b.hero, b.board))),
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
          picture("card", o.cardId, cardLabel(o.cardId, g("minion.unknownCard")), 40, true),
          cardText(o.cardId),
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
      : seat.trinkets
        .map((x) => {
          const name = trinketName(x.dbfId);
          const where = { slot: g(SLOTS[x.slot]), turn: x.turn };
          return name ? g("lobby.trinketNamed", { name, ...where }) : g("lobby.trinket", { id: x.dbfId, ...where });
        })
        .join("; ");
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
    // The replays come after the turn table: a link to them near the top.
    view.replays.some((c) => c.replay !== null) ? h("p", { class: "gv-jump" }, h("a", { href: "#gv-replays" }, g("replay.jump"))) : null,
    turnsSection(view),
    replaySection(view),
    opponentsSection(view),
    choicesSection(view),
    lobbySection(view),
    notesSection(view),
    note(`${g("summary.source")} ${g("credit")}`),
  ].filter((n): n is VNode => n !== null);
}

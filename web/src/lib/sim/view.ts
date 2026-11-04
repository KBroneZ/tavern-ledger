// The simulator's board editor as element trees (T-606, T-526): plain data
// in, markup out, so tests can render it without a browser. The page script
// (main.ts) reads the data-act and data-field attributes to edit the state.
// Each minion is drawn as a card (its art, Attack, Health, keywords and Dark
// Gifts, D-107) in one of a board's seven places; the page draws each side
// on its half of the table, the opponent's across from the player's.
// Text goes in as text nodes only (lib/view/node.ts).
import giftTable from "../../data/gifts.json" with { type: "json" };
import { exactStat, shortStat } from "../stat.ts";
import { lang } from "../i18n.ts";
import { giftMarks, giftName } from "../game/gifts.ts";
import { minionTip, tipHost } from "../game/minionTip.ts";
import { cls, h, type VNode } from "../view/node.ts";
import { drawnAttrs } from "../art/drawn.ts";
import { picture } from "../view/pieces.ts";
import { MAX_GIFTS, MAX_MINIONS, MAX_STAT, MAX_TIER, SIM_KEYWORDS, type Side, type SimBoard, type SimKeyword, type SimMinion, type SimState } from "./board.ts";
import type { KitLists } from "./cardFile.ts";
import type { CardIndex, SimCard } from "./cards.ts";
import { NO_KIT_LISTS } from "../game/names.ts";
import { kitCards } from "./kit.ts";
import { type Holding, kitView } from "./kitView.ts";
import { reasonText } from "./reasons.ts";
import { sm } from "./strings.ts";

/** Short marks on a minion (the overlay's letters, D-061), spelled out in its label. */
const SHORT: Record<SimKeyword, string> = {
  taunt: "T",
  divine_shield: "DS",
  reborn: "R",
  venomous: "V",
  poisonous: "P",
  windfury: "W",
  mega_windfury: "MW",
  stealth: "S",
};

export interface Selection {
  side: Side;
  board: number;
  key: number;
}

export interface Adding {
  side: Side;
  board: number;
  query: string;
}

/** What the last answer said about cards on the boards. */
export interface Marks {
  notModelled: ReadonlySet<string>;
  missing: ReadonlySet<string>;
  /** Card id → the engine's reason. */
  approximate: ReadonlyMap<string, string>;
  /** Blocked card id → why (a hero power's missing input, a card not reviewed). */
  reasons?: ReadonlyMap<string, string>;
}

export const NO_MARKS: Marks = { notModelled: new Set(), missing: new Set(), approximate: new Map() };

export interface EditorProps {
  state: SimState;
  cards: CardIndex;
  selected: Selection | null;
  adding: Adding | null;
  marks: Marks;
  /** The hero powers and trinkets to choose from (T-628); none when the names did not load. */
  lists?: KitLists;
  /** The held-minion search open on a hero power, if any. */
  holding?: Holding | null;
}

const where = (side: Side, board: number) => ({ "data-side": side, "data-board": board });

export const keywordName = (k: SimKeyword): string => sm(`keyword.${k}`);

/** Every Dark Gift the shared table names (src/data/gifts.json), by card id. */
const GIFT_IDS = Object.keys(giftTable.gifts);

function minionLabel(m: SimMinion, cards: CardIndex, marks: Marks): string {
  const parts = [`${cards.name(m.cardId)}, ${exactStat(m.attack)}/${exactStat(m.health)}`];
  if (m.golden) parts.push(sm("minion.golden"));
  parts.push(...m.keywords.map(keywordName));
  parts.push(...m.gifts.map((id) => sm("minion.gift", { name: giftName(id) })));
  if (marks.notModelled.has(m.cardId)) parts.push(sm("minion.notModelled"));
  if (marks.missing.has(m.cardId)) parts.push(sm("minion.noData"));
  if (marks.approximate.has(m.cardId)) parts.push(sm("minion.approximate"));
  return parts.join(", ");
}

/** A star per Dark Gift, its name and our summary as the tooltip (the card's label says it too). */
function giftStars(m: SimMinion): VNode | null {
  if (m.gifts.length === 0) return null;
  const marks = giftMarks({ gifts: m.gifts, giftUnknown: false });
  return h("span", { class: "sim-gifts", "aria-hidden": "true" }, marks.map((k) => h("span", { class: "sim-gift", title: k.title }, "✦")));
}

function card(m: SimMinion, side: Side, board: number, props: EditorProps): VNode {
  const { cards, marks, selected } = props;
  const isSelected = selected?.side === side && selected.board === board && selected.key === m.key;
  const blocked = marks.notModelled.has(m.cardId) || marks.missing.has(m.cardId);
  const approx = marks.approximate.has(m.cardId);
  const name = cards.name(m.cardId);
  const tipId = `sim-tip-${side}-${board}-${m.key}`;
  return h(
    "li",
    { class: "sim-slot" },
    h(
      "button",
      {
        type: "button",
        class: cls("sim-card", {
          "is-golden": m.golden,
          "is-selected": isSelected,
          "is-blocked": blocked,
          "is-approx": approx,
          "is-taunt": m.keywords.includes("taunt"),
          "has-shield": m.keywords.includes("divine_shield"),
          // The marks a combat replay shows too (T-575, T-581); Poisonous
          // keeps the game's older flask, Venomous its vial.
          "has-reborn": m.keywords.includes("reborn"),
          "has-venomous": m.keywords.includes("venomous"),
          "has-poisonous": m.keywords.includes("poisonous"),
          "has-windfury": m.keywords.includes("windfury") || m.keywords.includes("mega_windfury"),
          "has-stealth": m.keywords.includes("stealth"),
        }),
        ...drawnAttrs({ cardId: m.cardId, atk: m.attack, health: m.health, golden: m.golden, keywords: m.keywords }),
        "data-act": "select",
        "data-key": m.key,
        "data-focus": `m${m.key}`,
        ...where(side, board),
        "aria-pressed": isSelected ? "true" : "false",
        "aria-label": minionLabel(m, cards, marks),
        "aria-describedby": tipHost(tipId)["aria-describedby"],
      },
      // The slot is where the game's drawing goes once its textures are loaded (art/runtime.ts); empty, it takes no room.
      h("span", { class: "tlm-slot", "aria-hidden": "true" }),
      h("span", { class: "sim-taunt", "aria-hidden": "true" }),
      h("span", { class: "sim-card-art" }, picture("card", m.cardId, name, 56, true), h("span", { class: "sim-veil", "aria-hidden": "true" })),
      h("span", { class: "sim-venom", "aria-hidden": "true" }),
      h("span", { class: "sim-wind", "aria-hidden": "true" }),
      m.keywords.length > 0 ? h("span", { class: "sim-kw", "aria-hidden": "true" }, m.keywords.map((k) => SHORT[k]).join(" ")) : null,
      giftStars(m),
      blocked ? h("span", { class: "sim-flag sim-flag-blocked", "aria-hidden": "true" }, "!") : approx ? h("span", { class: "sim-flag", "aria-hidden": "true" }, "≈") : null,
      h("span", { class: "sim-atk", "aria-hidden": "true" }, shortStat(m.attack)),
      h("span", { class: "sim-hp", "aria-hidden": "true" }, shortStat(m.health)),
    ),
    minionTip(tipId, { cardId: m.cardId, atk: m.attack, health: m.health, buffs: m.buffs }, "loaded"),
  );
}

function tierPicker(b: SimBoard, side: Side, board: number, id: string): VNode {
  const values: (number | null)[] = [null, ...Array.from({ length: MAX_TIER }, (_, i) => i + 1)];
  return h(
    "div",
    { class: "sim-tier", role: "group", "aria-label": sm("tier.label"), id },
    h("span", { class: "sim-tier-label", "aria-hidden": "true" }, sm("tier.label")),
    values.map((v) =>
      h(
        "button",
        {
          type: "button",
          "data-act": "tier",
          "data-tier": v ?? "",
          "data-focus": `t-${side}-${board}-${v ?? 0}`,
          ...where(side, board),
          "aria-pressed": b.tier === v ? "true" : "false",
          "aria-label": v === null ? sm("tier.unknownLong") : sm("tier.value", { n: v }),
        },
        v === null ? sm("tier.unknown") : String(v),
      ),
    ),
  );
}

function statInput(field: "attack" | "health", m: SimMinion, side: Side, board: number): VNode {
  return h(
    "label",
    { class: `sim-stat sim-stat-${field}` },
    h("span", {}, sm(field === "attack" ? "minion.attack" : "minion.health")),
    h("input", {
      type: "number",
      inputmode: "numeric",
      min: field === "attack" ? 0 : 1,
      max: MAX_STAT,
      step: 1,
      value: m[field],
      "data-field": field,
      "data-key": m.key,
      "data-focus": `${field}${m.key}`,
      ...where(side, board),
    }),
  );
}

function chip(label: string, pressed: boolean, attrs: Record<string, string | number>): VNode {
  return h("button", { type: "button", class: "sim-chip", "aria-pressed": pressed ? "true" : "false", ...attrs }, label);
}

function inspector(m: SimMinion, side: Side, board: number, count: number, props: EditorProps): VNode {
  const { cards, marks } = props;
  const at = { "data-key": m.key, ...where(side, board) };
  const index = props.state[side][board].minions.findIndex((x) => x.key === m.key);
  const reason = marks.approximate.get(m.cardId);
  const said = reason ? reasonText(reason) : null;
  const name = cards.name(m.cardId);
  return h(
    "div",
    { class: "sim-inspect", role: "group", "aria-label": name },
    h(
      "p",
      { class: "sim-inspect-name" },
      picture("card", m.cardId, name, 40, true),
      h("span", {}, name),
      marks.notModelled.has(m.cardId) ? h("span", { class: "sim-badge sim-badge-blocked" }, sm("minion.notModelled")) : null,
      marks.missing.has(m.cardId) ? h("span", { class: "sim-badge sim-badge-blocked" }, sm("minion.noData")) : null,
    ),
    said ? h("p", { class: "sim-reason" }, "≈ ", h("span", { lang: said.english ? "en" : undefined }, said.text)) : null,
    h("div", { class: "sim-stats" }, statInput("attack", m, side, board), statInput("health", m, side, board)),
    h(
      "div",
      { class: "sim-chips", role: "group", "aria-label": sm("minion.keywords") },
      chip(sm("minion.golden"), m.golden, { "data-act": "golden", "data-focus": `g${m.key}`, ...at }),
      SIM_KEYWORDS.map((k) => chip(keywordName(k), m.keywords.includes(k), { "data-act": "keyword", "data-keyword": k, "data-focus": `k${m.key}-${k}`, ...at })),
    ),
    giftEditor(m, side, board),
    h(
      "div",
      { class: "sim-actions" },
      h("button", { type: "button", class: "sim-btn", "data-act": "move", "data-step": -1, "data-focus": `l${m.key}`, disabled: index <= 0, ...at }, "←", h("span", { class: "visually-hidden" }, ` ${sm("minion.left")}`)),
      h("button", { type: "button", class: "sim-btn", "data-act": "move", "data-step": 1, "data-focus": `r${m.key}`, disabled: index >= count - 1, ...at }, "→", h("span", { class: "visually-hidden" }, ` ${sm("minion.right")}`)),
      h("button", { type: "button", class: "sim-btn sim-btn-danger", "data-act": "remove", ...at }, sm("minion.remove")),
      h("button", { type: "button", class: "sim-btn", "data-act": "close", ...at }, sm("minion.close")),
    ),
  );
}

/** The picked minion's Dark Gifts: each one can be removed; a list adds one, up to four. */
function giftEditor(m: SimMinion, side: Side, board: number): VNode {
  const at = { "data-key": m.key, ...where(side, board) };
  const id = `sim-gift-${side}-${board}-${m.key}`;
  const others = GIFT_IDS.filter((g) => !m.gifts.includes(g))
    .map((g) => ({ id: g, name: giftName(g) }))
    .sort((x, y) => x.name.localeCompare(y.name, lang()));
  const marks = giftMarks({ gifts: m.gifts, giftUnknown: false });
  return h(
    "div",
    { class: "sim-gift-edit", role: "group", "aria-label": sm("gift.label") },
    marks.length > 0
      ? h(
          "ul",
          { class: "sim-chips", role: "list" },
          marks.map((k) =>
            h(
              "li",
              {},
              h(
                "button",
                {
                  type: "button",
                  class: "sim-chip sim-gift-chip",
                  "data-act": "gift-remove",
                  "data-gift": k.id ?? "",
                  "data-focus": `gr${m.key}-${k.id}`,
                  ...at,
                  title: k.title,
                  "aria-label": sm("gift.remove", { name: k.name }),
                },
                h("span", { "aria-hidden": "true" }, "✦ "),
                k.name,
                h("span", { "aria-hidden": "true" }, " ×"),
              ),
            ),
          ),
        )
      : null,
    m.gifts.length >= MAX_GIFTS
      ? h("p", { class: "sim-dim" }, sm("gift.full"))
      : // The list only chooses: a closed list changes on every arrow key, so the button adds.
        h(
          "div",
          { class: "sim-gift-add" },
          h("label", { for: id }, sm("gift.add")),
          h(
            "div",
            { class: "sim-gift-pick" },
            h(
              "select",
              { id, "data-field": "gift", "data-focus": `ga${m.key}`, ...at },
              h("option", { value: "" }, sm("gift.choose")),
              others.map((g) => h("option", { value: g.id }, g.name)),
            ),
            h("button", { type: "button", class: "sim-btn", "data-act": "gift-add", "data-focus": `gb${m.key}`, ...at }, sm("gift.addButton")),
          ),
        ),
  );
}

function result(card: SimCard, side: Side, board: number): VNode {
  return h(
    "li",
    {},
    h(
      "button",
      { type: "button", class: "sim-result-row", "data-act": "pick", "data-card": card.i, ...where(side, board) },
      picture("card", card.i, card.n, 32, true),
      h("span", { class: "sim-result-name" }, card.n),
      card.t === null ? null : h("span", { class: "sim-dim" }, sm("add.tier", { n: card.t })),
    ),
  );
}

function adder(a: Adding, props: EditorProps): VNode {
  const found = props.cards.search(a.query);
  const id = `sim-add-${a.side}-${a.board}`;
  return h(
    "div",
    { class: "sim-add" },
    h(
      "div",
      { class: "sim-add-head" },
      h("label", { for: id }, sm("add.label")),
      h("button", { type: "button", class: "sim-btn", "data-act": "add-close", ...where(a.side, a.board) }, sm("add.close")),
    ),
    h("input", {
      id,
      type: "search",
      autocomplete: "off",
      spellcheck: "false",
      maxlength: 40,
      placeholder: sm("add.placeholder"),
      value: a.query,
      "data-field": "query",
      "data-focus": `q-${a.side}-${a.board}`,
      ...where(a.side, a.board),
    }),
    a.query.trim() === "" ? null : found.length === 0
      ? h("p", { class: "sim-dim" }, sm("add.none"))
      : h("ul", { class: "sim-results", role: "list" }, found.map((c) => result(c, a.side, a.board))),
  );
}

function boardView(side: Side, index: number, props: EditorProps): VNode {
  const { state, selected, adding } = props;
  const b = state[side][index];
  const duos = state.mode === "duos";
  const id = `sim-${side}-${index}`;
  const title = duos ? sm(index === 0 ? "board.first" : "board.second") : sm(side === "player" ? "side.player" : "side.opponent");
  const chosen = selected && selected.side === side && selected.board === index ? b.minions.find((m) => m.key === selected.key) : undefined;
  const isAdding = adding?.side === side && adding.board === index;
  const full = b.minions.length >= MAX_MINIONS;
  return h(
    "section",
    { class: "sim-board", "aria-labelledby": `${id}-title` },
    h(
      "div",
      { class: "sim-board-head" },
      h(duos ? "h3" : "h2", { class: "sim-board-title", id: `${id}-title` }, title),
      tierPicker(b, side, index, `${id}-tier`),
      duos && index === 1 ? h("button", { type: "button", class: "sim-btn sim-btn-quiet", "data-act": "board-remove", ...where(side, index) }, sm("board.remove")) : null,
    ),
    kitView(side, index, { state, cards: props.cards, lists: props.lists ?? NO_KIT_LISTS, marks: props.marks, holding: props.holding ?? null }),
    // The request leaves an empty second board out (B.toRequest), so what is set on it is not played.
    duos && index === 1 && b.minions.length === 0 && kitCards(b.kit ?? null).length > 0
      ? h("p", { class: "sim-dim sim-board-left-out" }, sm("board.leftOut"))
      : null,
    h(
      "ul",
      { class: "sim-row", role: "list", "aria-label": title },
      b.minions.map((m) => card(m, side, index, props)),
      full
        ? null
        : h(
            "li",
            { class: "sim-slot" },
            h(
              "button",
              { type: "button", class: "sim-card sim-card-add", "data-act": "add-open", "data-focus": `add-${side}-${index}`, ...where(side, index), "aria-expanded": isAdding ? "true" : "false" },
              h("span", { class: "sim-card-plus", "aria-hidden": "true" }, "+"),
              h("span", { class: "sim-card-add-label" }, sm("add.button")),
            ),
          ),
      // The places still free, so every board reads as seven.
      Array.from({ length: Math.max(0, MAX_MINIONS - b.minions.length - 1) }, () => h("li", { class: "sim-slot is-empty", "aria-hidden": "true" })),
    ),
    full ? h("p", { class: "sim-dim" }, sm("board.full")) : null,
    chosen ? inspector(chosen, side, index, b.minions.length, props) : null,
    isAdding && adding ? adder(adding, props) : null,
  );
}

/** One side of the table: its board, or its two boards in Duos. */
export function sideView(side: Side, props: EditorProps): VNode[] {
  const { state } = props;
  const duos = state.mode === "duos";
  const view = h(
    "div",
    { class: cls("sim-side", `sim-side-${side}`) },
    duos ? h("h2", { class: "sim-side-title" }, sm(side === "player" ? "side.player.duos" : "side.opponent.duos")) : null,
    state[side].map((_, i) => boardView(side, i, props)),
    duos && state[side].length < 2
      ? h("button", { type: "button", class: "sim-btn sim-btn-quiet", "data-act": "board-add", "data-side": side }, sm("board.addSecond"))
      : null,
  );
  return [view];
}

export function modeSwitch(state: SimState, solo: string, duos: string): VNode {
  return h(
    "div",
    { class: "seg sim-mode", role: "group", "aria-label": sm("mode.label") },
    (["solo", "duos"] as const).map((m) =>
      h("button", { type: "button", "data-act": "mode", "data-mode": m, "data-focus": `mode-${m}`, "aria-pressed": state.mode === m ? "true" : "false" }, m === "solo" ? solo : duos),
    ),
  );
}

/** Both sides as the table shows them: the opponent's across from the player's. */
export function editorView(props: EditorProps): VNode[] {
  return [...sideView("opponent", props), ...sideView("player", props)];
}

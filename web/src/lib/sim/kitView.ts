// A board's hero powers and trinkets in the editor (T-628, D-157): a list
// per slot (the hero's power, the lesser, greater and third trinkets), and
// what a played power needs beside the board: the minion it chooses, the
// minion it holds, or the turn. Plain data in, markup out (main.ts reads
// the data-act and data-field attributes). Names come from the card data's
// names table (game/names.ts), as text nodes only.
import { cardName, cardNames, type CardNames } from "../game/names.ts";
import { exactStat } from "../stat.ts";
import { lang } from "../i18n.ts";
import type { KitLists } from "./cardFile.ts";
import type { CardIndex, SimCard } from "./cards.ts";
import { inSlot, type KitKind, needs, type SimKit, type SimKitCard, UNLOCK_TURN } from "./kit.ts";
import { reasonText } from "./reasons.ts";
import { sm, type SimStringKey } from "./strings.ts";
import { cls, h, type VNode } from "../view/node.ts";
import { picture } from "../view/pieces.ts";
import type { Side, SimBoard, SimState } from "./board.ts";

/** The held-minion search open on a board's hero power. */
export interface Holding {
  side: Side;
  board: number;
  slot: number;
  query: string;
}

/** What the last answer said about cards (the editor's Marks). */
export interface KitMarks {
  notModelled: ReadonlySet<string>;
  missing: ReadonlySet<string>;
  approximate: ReadonlyMap<string, string>;
  reasons?: ReadonlyMap<string, string>;
}

export interface KitProps {
  state: SimState;
  cards: CardIndex;
  lists: KitLists;
  marks: KitMarks;
  holding: Holding | null;
}

interface Slot {
  kind: KitKind;
  slot: number;
  label: SimStringKey;
  /** The card ids to choose from. */
  choices: (lists: KitLists) => string[];
}

/** Both trinket lists, once per lists table (the third slot offers both). */
const bothTrinkets = new WeakMap<KitLists, string[]>();
const allTrinkets = (l: KitLists): string[] => {
  const known = bothTrinkets.get(l);
  if (known) return known;
  const both = [...l.lesser, ...l.greater];
  bothTrinkets.set(l, both);
  return both;
};

const SLOTS: Slot[] = [
  { kind: "power", slot: 0, label: "kit.power", choices: (l) => l.power },
  { kind: "power", slot: 1, label: "kit.power2", choices: (l) => l.power },
  { kind: "trinket", slot: 1, label: "kit.lesser", choices: (l) => l.lesser },
  { kind: "trinket", slot: 2, label: "kit.greater", choices: (l) => l.greater },
  { kind: "trinket", slot: 3, label: "kit.third", choices: allTrinkets },
];

const where = (side: Side, board: number) => ({ "data-side": side, "data-board": board });
const nameOf = (id: string): string => cardName(id) ?? id;

interface Choice {
  id: string;
  label: string;
}

/** A list's choices by name, sorted once per list, language and names table; a name two cards share also shows the id. */
const sorted = new WeakMap<readonly string[], { lang: string; table: CardNames; choices: Choice[] }>();
function choicesOf(ids: readonly string[]): Choice[] {
  const known = sorted.get(ids);
  if (known && known.lang === lang() && known.table === cardNames()) return known.choices;
  const named = [...new Set(ids)].map((id) => ({ id, name: nameOf(id) }));
  const count = new Map<string, number>();
  for (const n of named) count.set(n.name, (count.get(n.name) ?? 0) + 1);
  const collator = new Intl.Collator(lang());
  const choices = named
    .sort((a, b) => collator.compare(a.name, b.name) || (a.id < b.id ? -1 : 1))
    .map((n) => ({ id: n.id, label: (count.get(n.name) ?? 0) > 1 ? `${n.name} (${n.id})` : n.name }));
  sorted.set(ids, { lang: lang(), table: cardNames(), choices });
  return choices;
}

/** The choices as options, the chosen card first among them when the list lacks it (a loaded one). */
function options(ids: readonly string[], chosen: string | undefined): VNode[] {
  const choices = choicesOf(ids);
  const extra = chosen && !choices.some((c) => c.id === chosen) ? [{ id: chosen, label: nameOf(chosen) }] : [];
  return [...extra, ...choices].map((c) => h("option", { value: c.id, selected: c.id === chosen }, c.label));
}

/** Why the card blocks the odds (the engine's reason when it gave one), or "≈" when it plays with a known gap. */
function markOf(id: string, marks: KitMarks): VNode[] {
  if (marks.notModelled.has(id) || marks.missing.has(id)) {
    const why = marks.reasons?.get(id);
    if (why && !marks.missing.has(id)) {
      const said = reasonText(why);
      return [h("p", { class: "sim-reason sim-reason-blocked" }, "! ", h("span", { lang: said.english ? "en" : undefined }, said.text))];
    }
    return [h("span", { class: "sim-badge sim-badge-blocked" }, sm(marks.missing.has(id) ? "minion.noData" : "minion.notModelled"))];
  }
  const reason = marks.approximate.get(id);
  if (!reason) return [];
  const said = reasonText(reason);
  return [h("p", { class: "sim-reason" }, "≈ ", h("span", { lang: said.english ? "en" : undefined }, said.text))];
}

function targetPicker(c: SimKitCard, b: SimBoard, side: Side, board: number): VNode {
  const id = `sim-kit-target-${side}-${board}-${c.slot}`;
  return h(
    "div",
    { class: "sim-kit-need" },
    h("label", { for: id }, sm("kit.target")),
    h(
      "select",
      { id, "data-field": "kit-target", "data-slot": c.slot, "data-focus": id, ...where(side, board) },
      h("option", { value: "", selected: c.target === null }, sm("kit.targetNone")),
      b.minions.map((m, i) => h("option", { value: m.key, selected: m.key === c.target }, `${i + 1}. ${nameOf(m.cardId)}`)),
    ),
  );
}

function turnPicker(state: SimState, side: Side, board: number, slot: number): VNode {
  const at = state.round;
  return h(
    "div",
    { class: "seg sim-kit-need", role: "group", "aria-label": sm("kit.turn") },
    ([false, true] as const).map((late) =>
      h(
        "button",
        {
          type: "button",
          "data-act": "kit-turn",
          "data-late": late ? "1" : "0",
          "data-focus": `kit-turn-${side}-${board}-${slot}-${late ? 1 : 0}`,
          "aria-pressed": at !== null && at >= UNLOCK_TURN === late ? "true" : "false",
        },
        sm(late ? "kit.after" : "kit.before", { turn: UNLOCK_TURN }),
      ),
    ),
  );
}

function heldResult(card: SimCard, side: Side, board: number, slot: number): VNode {
  return h(
    "li",
    {},
    h(
      "button",
      { type: "button", class: "sim-result-row", "data-act": "held-pick", "data-card": card.i, "data-slot": slot, ...where(side, board) },
      picture("card", card.i, card.n, 32, true),
      h("span", { class: "sim-result-name" }, card.n),
    ),
  );
}

function heldPicker(c: SimKitCard, side: Side, board: number, props: KitProps): VNode {
  const at = { "data-slot": c.slot, ...where(side, board) };
  const held = c.held;
  if (held) {
    const name = nameOf(held.cardId);
    const stat = (field: "attack" | "health") =>
      h(
        "label",
        { class: `sim-stat sim-stat-${field}` },
        h("span", {}, sm(field === "attack" ? "minion.attack" : "minion.health")),
        h("input", { type: "number", inputmode: "numeric", min: field === "attack" ? 0 : 1, step: 1, value: held[field], "data-field": `held-${field}`, "data-focus": `held-${field}-${side}-${board}-${c.slot}`, ...at }),
      );
    return h(
      "div",
      { class: "sim-kit-need sim-kit-held", role: "group", "aria-label": sm("kit.held") },
      h("span", { class: "sim-kit-held-name" }, picture("card", held.cardId, name, 32, true), `${sm("kit.held")}: ${name}, ${exactStat(held.attack)}/${exactStat(held.health)}`),
      h("div", { class: "sim-stats" }, stat("attack"), stat("health")),
      h("button", { type: "button", class: "sim-btn", "data-act": "held-remove", "data-focus": `held-x-${side}-${board}-${c.slot}`, ...at }, sm("kit.heldRemove", { name })),
    );
  }
  const open = props.holding?.side === side && props.holding.board === board && props.holding.slot === c.slot;
  const query = open && props.holding ? props.holding.query : "";
  const found = query.trim() === "" ? [] : props.cards.search(query);
  const id = `sim-held-${side}-${board}-${c.slot}`;
  return h(
    "div",
    { class: "sim-kit-need sim-kit-held" },
    h("label", { for: id }, sm("kit.heldFind")),
    h("input", { id, type: "search", autocomplete: "off", spellcheck: "false", maxlength: 40, placeholder: sm("add.placeholder"), value: query, "data-field": "held-query", "data-focus": id, ...at }),
    query.trim() === "" ? null : found.length === 0
      ? h("p", { class: "sim-dim" }, sm("add.none"))
      : h("ul", { class: "sim-results", role: "list" }, found.map((card) => heldResult(card, side, board, c.slot))),
  );
}

/** What a played hero power needs beside the board, as controls; with start of combat off they wait. */
function needsOf(c: SimKitCard, b: SimBoard, side: Side, board: number, props: KitProps): VNode[] {
  const n = needs(c.cardId);
  const controls = [
    n.includes("target") ? targetPicker(c, b, side, board) : null,
    n.includes("held") ? heldPicker(c, side, board, props) : null,
    n.includes("round") ? turnPicker(props.state, side, board, c.slot) : null,
  ].filter((v): v is VNode => v !== null);
  if (controls.length === 0 || props.state.startOfCombat) return controls;
  return [...controls, h("p", { class: "sim-dim" }, sm("kit.needsSoc"))];
}

function slotView(s: Slot, kit: SimKit | null, b: SimBoard, side: Side, board: number, props: KitProps): VNode | null {
  const card = inSlot(kit, s.kind, s.slot);
  // A second hero power is rare: its list shows only when the game had one.
  if (s.kind === "power" && s.slot === 1 && !card) return null;
  const id = `sim-kit-${side}-${board}-${s.kind}-${s.slot}`;
  return h(
    "div",
    { class: cls("sim-kit-slot", { "is-set": card !== undefined }) },
    h("label", { for: id }, sm(s.label)),
    h(
      "select",
      { id, "data-field": "kit", "data-kind": s.kind, "data-slot": s.slot, "data-focus": id, ...where(side, board) },
      h("option", { value: "", selected: card === undefined }, sm("kit.none")),
      options(s.choices(props.lists), card?.cardId),
    ),
    card ? markOf(card.cardId, props.marks) : null,
    card && s.kind === "power" ? needsOf(card, b, side, board, props) : null,
  );
}

/** A board's hero powers and trinkets. */
export function kitView(side: Side, board: number, props: KitProps): VNode {
  const b = props.state[side][board];
  const kit = b.kit ?? null;
  const known = props.lists.power.length > 0 || props.lists.lesser.length > 0 || props.lists.greater.length > 0;
  return h(
    "div",
    { class: "sim-kit", role: "group", "aria-label": sm("kit.label") },
    known ? null : h("p", { class: "sim-dim" }, sm("kit.noNames")),
    SLOTS.map((s) => slotView(s, kit, b, side, board, props)),
  );
}

/** The table's start-of-combat switch. */
export function socSwitch(state: SimState): VNode {
  return h(
    "div",
    { class: "sim-soc" },
    h(
      "button",
      {
        type: "button",
        class: "sim-chip",
        "data-act": "soc",
        "data-focus": "soc",
        "aria-pressed": state.startOfCombat ? "true" : "false",
        "aria-describedby": "sim-soc-hint",
      },
      sm("soc.label"),
    ),
    h("p", { class: "sim-dim", id: "sim-soc-hint" }, sm("soc.hint")),
  );
}

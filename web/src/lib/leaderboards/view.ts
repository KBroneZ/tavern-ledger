// The leaderboards page's body (T-506): one panel per mode and period, each
// with the main boards and, for Advanced, the economy and fun boards. Every
// board is a compact table (top 10, "Show all" up to 50) with a fold that
// says how it is counted. Pure: the page script mounts what this returns.
import type { Mode } from "../data/format.ts";
import { t } from "../data/strings.ts";
import { publicProfilePath } from "../profile.ts";
import { h, type VNode } from "../view/node.ts";
import { fold, HERO_PIC, picture, stateBlock, topTable, type Cell, type Column } from "../view/pieces.ts";
import { heroLabel } from "../heroes/names.ts";
import type { Boards, Leaderboards } from "./api.ts";
import {
  boardHow,
  boardTitle,
  BOARDS,
  FEW_GAMES,
  FEW_HERO_GAMES,
  fewGames,
  formatValue,
  periodName,
  PERIODS,
  refreshedText,
  TOP_ROWS,
  valueLabel,
  type BoardDef,
  type BoardRow,
  type Period,
} from "./boards.ts";
import { lt } from "./strings.ts";
import { href } from "../i18n.ts";

const MODES: readonly Mode[] = ["solo", "duos"];

function columns(def: BoardDef, m: Mode): Column[] {
  return [
    { label: lt("col.standing"), numeric: true },
    { label: lt("col.player") },
    ...(def.key === "hero_avg_place" ? [{ label: lt("col.hero") }] : []),
    { label: valueLabel(def, m), numeric: true },
    // The games board's value is the games: a second "Games" column would repeat it.
    ...(def.key === "games" ? [] : [{ label: lt("col.games"), numeric: true }]),
  ];
}

function heroCell(hero: string | null): Cell {
  const { name, unknownId } = heroLabel(hero, lt("hero.unknown"), lt("hero.unknown"));
  const id = unknownId ? h("span", { class: "adv cell-note hero-id" }, unknownId) : null;
  return { node: h("span", { class: "cell-hero" }, picture("hero", hero, name, HERO_PIC, true), h("span", { class: "hero-name" }, h("span", {}, name), id)) };
}

/** The "few games" mark after a value that rests on fewer games than its board's line. */
export function fewMark(def: BoardDef): VNode {
  const tip = def.key === "hero_avg_place" ? lt("few.tip.hero", { min: FEW_HERO_GAMES }) : lt("few.tip", { min: FEW_GAMES });
  return h("span", { class: "mark-src mark-few", title: tip }, lt("few"));
}

/** The value, with the "few games" mark when it rests on few games. */
function valueCell(def: BoardDef, row: BoardRow): Cell {
  const value = formatValue(def, row.value);
  return fewGames(row) ? { node: h("span", {}, value, fewMark(def)) } : value;
}

/** One table row: standing, the player linked to their public profile, (hero), value, (games). */
export function rowCells(def: BoardDef, row: BoardRow): Cell[] {
  const player: Cell = { node: h("a", { href: publicProfilePath(row.userId) }, row.name) };
  return [
    row.standing,
    player,
    ...(def.key === "hero_avg_place" ? [heroCell(row.hero)] : []),
    valueCell(def, row),
    ...(def.key === "games" ? [] : [row.games]),
  ];
}

/** What a board counts and when a row is marked "few games" (the hero board counts games with the hero). */
function counted(def: BoardDef, p: Period): string {
  const vars = { period: periodName(p, true), min: FEW_GAMES, hero: FEW_HERO_GAMES };
  return lt(def.key === "hero_avg_place" ? "board.how.count.hero" : "board.how.count", vars);
}

function how(def: BoardDef, title: string, p: Period): VNode {
  return fold(
    { summary: lt("board.how"), context: title, compact: true },
    h("p", {}, boardHow(def)),
    h("p", {}, counted(def, p)),
    h("p", {}, lt("board.how.ties")),
    h("p", {}, lt("board.how.source")),
  );
}

/** One board: its title, the table or an empty state, and how it is counted. */
export function boardSection(def: BoardDef, rows: readonly BoardRow[], m: Mode, p: Period): VNode {
  const id = `lb-${m}-${p}-${def.key}`;
  const title = boardTitle(def, m);
  const body =
    rows.length === 0
      ? stateBlock({ kind: "empty", title: lt("board.empty"), body: lt("board.empty.body", { period: periodName(p, true) }) })
      : topTable({
          caption: lt("board.caption", { board: title, mode: t(`mode.${m}`), period: periodName(p, true) }),
          columns: columns(def, m),
          rows: rows.map((r) => rowCells(def, r)),
          top: TOP_ROWS,
        });
  return h("section", { class: "lb-board", "aria-labelledby": id }, h("h3", { id }, title), body, how(def, title, p));
}

function group(boards: Boards, m: Mode, p: Period, simple: boolean): VNode {
  const id = `lb-${m}-${p}-${simple ? "main" : "more"}`;
  const defs = BOARDS.filter((d) => d.simple === simple);
  return h(
    "section",
    { class: simple ? "lb-group" : "lb-group adv", "aria-labelledby": id },
    h("h2", { id }, lt(simple ? "section.simple" : "section.advanced")),
    h(
      "div",
      { class: "lb-grid" },
      defs.map((d) => boardSection(d, boards.get(d.key) ?? [], m, p)),
    ),
  );
}

/** The boards of one mode and period. */
export function periodPanel(boards: Boards, m: Mode, p: Period): VNode {
  return h("div", { class: "lb-period", "data-period-panel": p }, group(boards, m, p, true), group(boards, m, p, false));
}

/** Both modes, each with its three periods; the switches pick what shows. */
export function modePanels(data: Leaderboards): VNode[] {
  return MODES.map((m) =>
    h(
      "div",
      { "data-mode-panel": m },
      PERIODS.map((p) => periodPanel(data.boards[m][p], m, p)),
    ),
  );
}

/** "Updated 11 Oct 2026, 14:20 UTC." or "Not built yet." */
export function updatedLine(refreshedAt: string | null): string {
  const time = refreshedText(refreshedAt);
  return time ? lt("page.updated", { time }) : lt("page.updated.never");
}

/** Who is listed, the "few games" mark, the rebuild and where the data comes from. */
export function aboutFold(): VNode {
  return fold(
    { summary: lt("about.summary") },
    h("p", {}, lt("about.who")),
    h("p", {}, lt("about.min", { min: FEW_GAMES, hero: FEW_HERO_GAMES })),
    h("p", {}, lt("about.refresh")),
    h("p", {}, lt("about.data"), " ", h("a", { href: href("/privacy/#public-profiles-and-leaderboards") }, lt("about.privacy")), "."),
  );
}

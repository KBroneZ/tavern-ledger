// The heroes and curious stats pages' bodies (T-505): one panel per mode
// (the mode switch shows one), built with the T-501 pieces. Pure: the page
// script mounts what these return.
import type { Mode } from "../data/format.ts";
import { modeName } from "../data/dashboard.ts";
import { RECORDS } from "../data/rows.ts";
import { t } from "../data/strings.ts";
import { cls, h, type VNode } from "../view/node.ts";
import { fold, stateBlock, statTile, topTable } from "../view/pieces.ts";
import type { Curious } from "./load.ts";
import { CURIOUS_RECORD_COLUMNS_VIEW, gapItems, modeRecords, recordCellsOf, totalTiles } from "./curious.ts";
import {
  fewMark,
  hasHeroes,
  heroColumns,
  heroStatCells,
  MIN_GAMES,
  OFFERED_COLUMNS,
  offeredCells,
  offeredOnly,
  sortHeroes,
  type HeroSort,
  type HeroStat,
} from "./heroes.ts";

const MODES: Mode[] = ["solo", "duos"];
/** Heroes shown before "Show all". */
export const TOP_HEROES = 10;

function section(id: string, title: string, ...body: (VNode | null)[]): VNode {
  return h("section", { class: "data-section", "aria-labelledby": id }, h("h2", { id }, title), ...body);
}

function emptyPanel(m: Mode): VNode {
  return h("div", { "data-mode-panel": m }, stateBlock({ kind: "empty", title: t("dash.mode.empty", { mode: modeName(m) }), body: t("dash.mode.empty.body") }));
}

export interface HeroesOptions {
  /** A public profile's id for the games links; null on the owner's own page. */
  profileId: string | null;
  sort: HeroSort;
}

/** Two buttons with aria-pressed; the page script re-sorts on a click. */
function sortSwitch(m: Mode, sort: HeroSort): VNode {
  const button = (value: HeroSort, label: string) =>
    h("button", { type: "button", "data-sort": value, "aria-pressed": String(sort === value), "aria-controls": `heroes-${m}-table` }, label);
  return h(
    "div",
    { class: "hstats-sort" },
    h("span", { class: "hstats-sort-label", "aria-hidden": "true" }, t("hstats.sort")),
    h("div", { class: "seg", role: "group", "aria-label": t("hstats.sort.label", { mode: modeName(m) }) }, button("games", t("hstats.sort.games")), button("avg", t("hstats.sort.avg"))),
  );
}

function offeredFold(m: Mode, rows: readonly HeroStat[]): VNode | null {
  const only = offeredOnly(rows, m);
  if (only.length === 0) return null;
  return h(
    "div",
    { class: "adv" },
    fold(
      { summary: t("hstats.offered", { count: only.length }), context: modeName(m) },
      topTable({ caption: t("hstats.offered.caption", { mode: modeName(m) }), columns: OFFERED_COLUMNS, rows: only.map(offeredCells), top: TOP_HEROES }),
    ),
  );
}

export function heroesPanel(m: Mode, rows: readonly HeroStat[], opts: HeroesOptions): VNode {
  if (!hasHeroes(rows, m)) return emptyPanel(m);
  const sorted = sortHeroes(rows, m, opts.sort);
  const caption = t(opts.sort === "avg" ? "hstats.caption.avg" : "hstats.caption.games", { mode: modeName(m) });
  return h(
    "div",
    { "data-mode-panel": m, class: "dash-panel" },
    section(
      `heroes-${m}-title`,
      t("hstats.section", { mode: modeName(m) }),
      sortSwitch(m, opts.sort),
      h(
        "div",
        { id: `heroes-${m}-table` },
        topTable({ caption, columns: heroColumns(m), rows: sorted.map((r) => heroStatCells(r, opts.profileId)), top: TOP_HEROES }),
      ),
      h("p", { class: "data-note" }, fewMark(), ` ${t("hstats.few.note", { min: MIN_GAMES })}`),
      h("p", { class: "data-note simple-only" }, t("hstats.simple.more")),
      h("p", { class: "data-note adv" }, t("hstats.picked.note")),
      offeredFold(m, rows),
    ),
  );
}

export function heroesPanels(rows: readonly HeroStat[], opts: HeroesOptions): VNode[] {
  return MODES.map((m) => heroesPanel(m, rows, opts));
}

export function heroesEmpty(rows: readonly HeroStat[]): boolean {
  return MODES.every((m) => !hasHeroes(rows, m));
}

export function heroesAbout(): VNode {
  return fold({ summary: t("dash.about") }, h("p", { class: "data-note" }, t("hstats.about.body")));
}

export function curiousPanel(m: Mode, data: Curious, userId: string): VNode {
  const ov = data.overviews.find((o) => o.mode === m) ?? null;
  if (!ov || ov.games === 0) return emptyPanel(m);
  const records = modeRecords(data.records, m);
  const tiles = totalTiles(ov);
  return h(
    "div",
    { "data-mode-panel": m, class: "dash-panel" },
    section(
      `curious-${m}-records`,
      t("dash.section.records", { mode: modeName(m) }),
      records.length === 0
        ? stateBlock({ kind: "unknown", title: t("records.empty"), body: t("records.empty.body") })
        : topTable({
            caption: t("curious.records.caption", { mode: modeName(m) }),
            columns: CURIOUS_RECORD_COLUMNS_VIEW,
            rows: records.map((r) => recordCellsOf(r, userId, ov.games)),
            top: RECORDS.length,
          }),
    ),
    section(
      `curious-${m}-totals`,
      t("curious.section.totals", { mode: modeName(m) }),
      h("div", { class: "tiles" }, tiles.simple.map(statTile)),
      h("p", { class: "data-note simple-only" }, t("curious.simple.more")),
      h("div", { class: cls("tiles", "adv"), role: "group", "aria-label": t("curious.section.more", { mode: modeName(m) }) }, tiles.advanced.map(statTile)),
    ),
  );
}

export function curiousPanels(data: Curious, userId: string): VNode[] {
  return MODES.map((m) => curiousPanel(m, data, userId));
}

export function curiousEmpty(data: Curious): boolean {
  return MODES.every((m) => (data.overviews.find((o) => o.mode === m)?.games ?? 0) === 0);
}

export function curiousAbout(): VNode[] {
  return [
    fold({ summary: t("curious.gaps") }, h("p", { class: "data-note" }, t("curious.gaps.intro")), h("ul", { class: "data-note" }, gapItems())),
    fold({ summary: t("dash.about") }, h("p", { class: "data-note" }, t("curious.about.body"))),
  ];
}

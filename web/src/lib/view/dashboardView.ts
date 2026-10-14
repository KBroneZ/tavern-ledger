// The dashboard's and the public profile's body (T-503): one panel per mode,
// key numbers first and the rest for Advanced, built from the loaded data
// with the T-501 pieces. Pure: the page script mounts what this returns.
import { PLACES, type Mode } from "../data/format.ts";
import {
  gameCells,
  gameColumns,
  HERO_COLUMNS,
  heroCells,
  modeName,
  overviewTiles,
  RECORD_COLUMNS,
  recentPlaces,
  recordCells,
} from "../data/dashboard.ts";
import { t } from "../data/strings.ts";
import { CHART_GAMES, type Dashboard, type ModeData } from "../dataApi.ts";
import { h, type VNode } from "./node.ts";
import { fold, placeChart, placeTrend, stateBlock, statTile, topTable } from "./pieces.ts";

/** Games in the trend chart and the average place tile's line. */
export const TREND_GAMES = 20;
const TOP_ROWS = 5;
const TOP_RECORDS = 4;

export interface DashboardOptions {
  /** The games list's address for a mode (own or the public profile's). */
  gamesHref: (m: Mode) => string;
  owner: boolean;
  /** Whose games these are, for each game's link. */
  userId: string;
  /** The heroes page's address for a mode (T-505). */
  heroesHref?: (m: Mode) => string;
  /** The curious stats page's address for a mode (T-505). */
  curiousHref?: (m: Mode) => string;
}

function moreLink(href: string | undefined, text: string): VNode | null {
  return href ? h("p", {}, h("a", { class: "dash-more", href }, text)) : null;
}

function section(id: string, title: string, ...body: (VNode | null)[]): VNode {
  return h("section", { class: "data-section", "aria-labelledby": id }, h("h2", { id }, title), ...body);
}

function overviewSection(m: Mode, data: ModeData): VNode | null {
  if (!data.overview) return null;
  const tiles = overviewTiles(data.overview, recentPlaces(data.places, TREND_GAMES));
  return section(
    `dash-${m}-overview`,
    t("dash.section.overview", { mode: modeName(m) }),
    h("div", { class: "tiles" }, tiles.simple.map(statTile)),
    h("p", { class: "data-note simple-only" }, t("dash.simple.more")),
    h("div", { class: "tiles adv", role: "group", "aria-label": t("dash.section.play", { mode: modeName(m) }) }, tiles.advanced.map(statTile)),
  );
}

function chartsSection(m: Mode, data: ModeData): VNode {
  return section(
    `dash-${m}-charts`,
    t("dash.section.charts", { mode: modeName(m) }),
    h(
      "div",
      { class: "chart-grid" },
      placeChart(data.places, m, `places-${m}`),
      placeTrend(recentPlaces(data.places, TREND_GAMES), PLACES[m], `trend-${m}`),
    ),
    data.total > CHART_GAMES ? h("p", { class: "data-note" }, t("dash.trend.capped", { count: CHART_GAMES })) : null,
  );
}

function lastGamesSection(m: Mode, data: ModeData, opts: DashboardOptions): VNode {
  const caption = data.lastGames.length === 1 ? t("games.caption.one", { mode: modeName(m) }) : t("games.caption", { count: data.lastGames.length, mode: modeName(m) });
  return section(
    `dash-${m}-last`,
    t("dash.section.last", { mode: modeName(m) }),
    topTable({ caption, columns: gameColumns(false), rows: data.lastGames.map((g) => gameCells(g, false, opts.userId)), top: TOP_ROWS }),
    h("p", {}, h("a", { class: "dash-more", href: opts.gamesHref(m) }, t("dash.allGames", { mode: modeName(m) }))),
  );
}

function heroesSection(m: Mode, dash: Dashboard, opts: DashboardOptions): VNode {
  const rows = heroCells(dash.heroes, m);
  return section(
    `dash-${m}-heroes`,
    t("dash.section.heroes", { mode: modeName(m) }),
    rows.length === 0
      ? stateBlock({ kind: "unknown", title: t("heroes.empty") })
      : topTable({ caption: t("heroes.caption", { mode: modeName(m) }), columns: HERO_COLUMNS(m), rows, top: TOP_ROWS }),
    rows.length === 0 ? null : moreLink(opts.heroesHref?.(m), t("player.link.heroes", { mode: modeName(m) })),
  );
}

function recordsSection(m: Mode, dash: Dashboard, games: number, opts: DashboardOptions): VNode {
  const rows = recordCells(dash.records, m, games);
  return section(
    `dash-${m}-records`,
    t("dash.section.records", { mode: modeName(m) }),
    rows.length === 0
      ? stateBlock({ kind: "unknown", title: t("records.empty"), body: t("records.empty.body") })
      : topTable({ caption: t("records.caption", { mode: modeName(m) }), columns: RECORD_COLUMNS, rows, top: TOP_RECORDS }),
    moreLink(opts.curiousHref?.(m), t("player.link.curious")),
  );
}

export function modePanel(m: Mode, dash: Dashboard, opts: DashboardOptions): VNode {
  const data = dash.modes[m];
  if (!data.overview || data.overview.games === 0) {
    return h(
      "div",
      { "data-mode-panel": m },
      stateBlock({ kind: "empty", title: t("dash.mode.empty", { mode: modeName(m) }), body: t("dash.mode.empty.body") }),
    );
  }
  return h(
    "div",
    { "data-mode-panel": m, class: "dash-panel" },
    overviewSection(m, data),
    chartsSection(m, data),
    lastGamesSection(m, data, opts),
    heroesSection(m, dash, opts),
    recordsSection(m, dash, data.overview.games, opts),
  );
}

/** True when the player has no game in either mode. */
export function isEmpty(dash: Dashboard): boolean {
  return (["solo", "duos"] as const).every((m) => (dash.modes[m].overview?.games ?? 0) === 0);
}

export function dashboardPanels(dash: Dashboard, opts: DashboardOptions): VNode[] {
  return [modePanel("solo", dash, opts), modePanel("duos", dash, opts)];
}

export function aboutFold(): VNode {
  return fold({ summary: t("dash.about") }, h("p", { class: "data-note" }, t("dash.about.body")));
}

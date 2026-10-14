// The community hero stats page's body (T-507): one panel per mode and
// period (the switches pick what shows), each with the sort switch and one
// compact table, or an honest empty state while too few players share their
// games. Pure: the page script mounts what this returns.
import type { Mode } from "../data/format.ts";
import { t } from "../data/strings.ts";
import { refreshedText } from "../leaderboards/boards.ts";
import { h, type VNode } from "../view/node.ts";
import { fold, more, stateBlock, topTable } from "../view/pieces.ts";
import type { CommunityStats } from "./api.ts";
import {
  heroCells,
  heroColumns,
  MIN_GAMES,
  MIN_PLAYERS,
  MODES,
  periodName,
  PERIODS,
  sortHeroes,
  SORTS,
  TOP_HEROES,
  type CommunityHero,
  type CommunityState,
  type HeroSort,
  type Period,
} from "./heroes.ts";
import { ct, type ChKey } from "./strings.ts";
import { dateFormat, href } from "../i18n.ts";

/** Where the account page's switch is. */
export const JOIN_PATH = "/account/#community-heading";
export const POLICY_PATH = "/privacy/#community-hero-stats";

/** The four sort buttons with aria-pressed, named by the visible "Sort by"; the page script re-sorts every panel on a click. */
function sortSwitch(id: string, sort: HeroSort): VNode {
  const label = `${id}-sort`;
  return h(
    "div",
    { class: "hstats-sort" },
    h("span", { class: "hstats-sort-label", id: label }, ct("sort.label")),
    h(
      "div",
      { class: "seg", role: "group", "aria-labelledby": label },
      SORTS.map((s) => h("button", { type: "button", "data-sort": s, "aria-pressed": String(sort === s) }, ct(`sort.${s}` as ChKey))),
    ),
  );
}

function emptyState(p: Period): VNode {
  return stateBlock(
    {
      kind: "empty",
      title: ct("empty.title"),
      body: ct("empty.body", { min: MIN_GAMES, players: MIN_PLAYERS, period: periodName(p, "in") }),
    },
    h("p", {}, `${ct("empty.join")} `, h("a", { href: href(JOIN_PATH) }, ct("empty.join.link")), "."),
  );
}

/** One mode and period: the table of heroes above the thresholds, or the empty state. */
export function periodPanel(heroes: readonly CommunityHero[], m: Mode, p: Period, sort: HeroSort): VNode {
  const id = `ch-${m}-${p}`;
  const attrs = { class: "data-section", "data-period-panel": p, "aria-labelledby": `${id}-title` };
  const title = h("h2", { id: `${id}-title` }, `${t(`mode.${m}`)} · ${periodName(p)}`);
  if (heroes.length === 0) return h("section", attrs, title, emptyState(p));
  const caption = ct("caption", { mode: t(`mode.${m}`), period: periodName(p, "in"), sort: ct(`sort.name.${sort}` as ChKey) });
  return h(
    "section",
    attrs,
    title,
    sortSwitch(id, sort),
    topTable({ caption, columns: heroColumns(m), rows: sortHeroes(heroes, sort).map(heroCells), top: TOP_HEROES }),
    h("p", { class: "data-note" }, ct("rounding.note")),
    more({ context: caption }, h("p", {}, ct("rounding.more")), h("p", {}, ct("pick.note"))),
    h("p", { class: "data-note simple-only" }, ct("simple.more")),
  );
}

/** Both modes, each with both periods; the switches pick what shows. While the snapshot is too old, one notice instead. */
export function modePanels(stats: CommunityStats, sort: HeroSort): VNode[] {
  if (!stats.state.available && stats.state.refreshedAt !== null) {
    return [stateBlock({ kind: "unknown", title: ct("unavailable.title"), body: ct("unavailable.body") })];
  }
  return MODES.map((m) => h("div", { "data-mode-panel": m }, PERIODS.map((p) => periodPanel(stats.heroes[m][p], m, p, sort))));
}


/** "Updated 11 Oct 2026, 04:05 UTC, with games up to 10 Oct 2026." or "Not built yet." */
export function updatedLine(state: CommunityState): string {
  const time = refreshedText(state.refreshedAt);
  if (!time || !state.countedUntil) return ct("page.updated.never");
  return ct("page.updated", { time, day: dateFormat({ day: "numeric", month: "short", year: "numeric", timeZone: "UTC" }, "en-GB").format(new Date(`${state.countedUntil}T00:00:00Z`)) });
}

/** Who is counted, the thresholds, what is counted, the rounding, where it comes from and how it is built. */
export function howFold(): VNode {
  return fold(
    { summary: ct("how.summary") },
    h("p", {}, ct("how.who"), " ", h("a", { href: href(JOIN_PATH) }, ct("empty.join.link")), "."),
    h("p", {}, ct("how.min", { min: MIN_GAMES, players: MIN_PLAYERS })),
    h("p", {}, ct("how.what")),
    h("p", {}, ct("how.rounding")),
    h("p", {}, ct("how.source")),
    h("p", {}, ct("how.refresh")),
    h("p", {}, `${ct("how.privacy.lead")} `, h("a", { href: href(POLICY_PATH) }, ct("how.privacy")), "."),
  );
}

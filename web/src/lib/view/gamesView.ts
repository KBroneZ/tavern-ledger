// The games list's body (T-503): how many games match, one compact row per
// game and the pages. Pure: the page script mounts what this returns.
import { gameCells, gameColumns } from "../data/dashboard.ts";
import { filtersQuery, hasFilters, pageCount, PAGE_SIZE, type GameFilters } from "../data/filters.ts";
import type { GameRow } from "../data/rows.ts";
import { t } from "../data/strings.ts";
import { h, type VNode } from "./node.ts";
import { stateBlock, topTable } from "./pieces.ts";
import { href } from "../i18n.ts";

export interface ListPage {
  rows: GameRow[];
  total: number;
  filters: GameFilters;
  /** The public profile's id; null on the owner's own list. */
  profileId: string | null;
  /** Whose games these are, for each game's link. */
  userId: string;
  /** Filters from the address left out as not valid. */
  invalid: string[];
}

function pager(page: ListPage): VNode | null {
  const pages = pageCount(page.total);
  const current = page.filters.page;
  if (pages <= 1 && current === 1) return null;
  const link = (to: number, text: string, rel: string) =>
    h("a", { class: "tb tb-small", href: href(`/games/${filtersQuery({ ...page.filters, page: to }, page.profileId)}`), rel }, text);
  return h(
    "nav",
    { class: "pager", "aria-label": t("list.pages") },
    current > 1 ? link(Math.min(current - 1, pages), t("list.prev"), "prev") : h("span"),
    h("p", { class: "pager-at" }, t("list.page", { page: current, pages: Math.max(pages, current) })),
    current < pages ? link(current + 1, t("list.next"), "next") : h("span"),
  );
}

/** The list for one page of filtered games, or the state that says why there is none. */
export function listBody(page: ListPage): VNode[] {
  const invalid = page.invalid.length > 0 ? h("p", { class: "message error" }, t("list.invalid", { names: [...new Set(page.invalid)].join(", ") })) : null;
  if (page.total === 0) {
    // No games at all is not "no games match": say so, and where the app is (the owner's list only).
    if (!hasFilters(page.filters)) {
      const own = page.profileId === null;
      const empty = stateBlock(
        { kind: "empty", title: t("list.empty"), body: t(own ? "list.empty.own" : "list.empty.public") },
        ...(own ? [h("a", { href: href("/download/") }, t("list.empty.own.link"))] : []),
      );
      return [invalid, empty].filter((n): n is VNode => n !== null);
    }
    return [invalid, stateBlock({ kind: "empty", title: t("list.none"), body: t("list.none.body") })].filter((n): n is VNode => n !== null);
  }
  if (page.rows.length === 0) {
    const first = href(`/games/${filtersQuery({ ...page.filters, page: 1 }, page.profileId)}`);
    return [invalid, stateBlock({ kind: "empty", title: t("list.pageEmpty"), body: "" }, h("a", { href: first }, t("list.pageEmpty.link")))].filter(
      (n): n is VNode => n !== null,
    );
  }
  const from = (page.filters.page - 1) * PAGE_SIZE + 1;
  const withMode = page.filters.mode === "all";
  return [
    invalid,
    h(
      "p",
      { class: "data-note list-count" },
      page.rows.length === 1 ? t("list.count.one", { from, total: page.total }) : t("list.count", { from, to: from + page.rows.length - 1, total: page.total }),
    ),
    topTable({ caption: t("list.caption"), columns: gameColumns(withMode), rows: page.rows.map((g) => gameCells(g, withMode, page.userId)), top: PAGE_SIZE }),
    pager(page),
  ].filter((n): n is VNode => n !== null);
}

// "Today and this week" on the dashboard (T-361): two columns, one block per
// mode played, drawn from lib/data/period.ts. Pure: the page script mounts what
// this returns. The numbers are the player's own synced games; the rating is
// never uploaded, so the block says so instead of showing a change.
import { dateText, modeName } from "../data/dashboard.ts";
import { placeAverage } from "../data/format.ts";
import type { ModePeriod, PeriodSummary, Revisit, RevisitReason } from "../data/period.ts";
import { t } from "../data/strings.ts";
import { gamePath } from "../game/url.ts";
import { heroLabel } from "../heroes/names.ts";
import { pluralForm } from "../i18n.ts";
import { h, type VNode } from "./node.ts";
import { fold, stateBlock } from "./pieces.ts";

const REASON: Record<RevisitReason, "period.reason.best" | "period.reason.close" | "period.reason.swing"> = {
  best: "period.reason.best",
  close: "period.reason.close",
  swing: "period.reason.swing",
};

function gamesText(games: number): string {
  return t(pluralForm(games) === "one" ? "period.games.one" : "period.games.other", { n: games });
}

function revisitItem(r: Revisit, userId: string): VNode {
  const hero = heroLabel(r.game.hero, t("hero.unknown"), t("hero.unknownId")).name;
  const label = `${t(REASON[r.reason])} · ${hero} · ${t("period.revisit.place", { n: r.place })}`;
  const publicId = r.game.publicId;
  return h(
    "li",
    {},
    publicId === null ? label : h("a", { href: gamePath(userId, publicId), title: t("period.revisit.open") }, label),
  );
}

function modeBlock(p: ModePeriod, userId: string): VNode {
  const counts = [gamesText(p.games)];
  if (p.placed > 0 && p.placed < p.games) counts.push(t("period.placed", { n: p.placed }));
  return h(
    "div",
    { class: "period-mode" },
    h("h4", {}, modeName(p.mode)),
    h("p", {}, counts.join(" · ")),
    h("p", {}, p.avgPlace === null ? t("period.avg.none") : t("period.avg", { value: placeAverage(p.avgPlace) })),
    h(
      "p",
      {},
      p.placed === 0 ? t("period.record.none") : t("period.record", { top: p.topHalfGames, bottom: p.placed - p.topHalfGames, half: p.topHalf }),
    ),
    h("p", { class: "data-note" }, t("period.rating")),
    p.revisit.length === 0
      ? null
      : h("ul", { class: "period-revisit", "aria-label": t("period.revisit.title") }, p.revisit.map((r) => revisitItem(r, userId))),
  );
}

function column(id: string, title: string, modes: ModePeriod[], empty: string, userId: string): VNode {
  return h(
    "section",
    { class: "period-col", "aria-labelledby": id },
    h("h3", { id }, title),
    modes.length === 0 ? h("p", { class: "data-note" }, empty) : modes.map((p) => modeBlock(p, userId)),
  );
}

/** The summary, in the frame the page reserved for it. */
export function periodSection(s: PeriodSummary, userId: string): VNode[] {
  return [
    h("h2", { id: "dash-period-title" }, t("period.title")),
    h("p", { class: "data-note" }, t("period.weekNote", { date: dateText(s.bounds.weekStart) })),
    h(
      "div",
      { class: "period-grid" },
      column("dash-period-today", t("period.today"), s.today, t("period.empty.today"), userId),
      column("dash-period-week", t("period.week"), s.week, t("period.empty.week"), userId),
    ),
    fold({ summary: t("period.about") }, h("p", { class: "data-note" }, t("period.about.body"))),
  ];
}

/** The summary could not be read: said in its own place, the rest of the page stands. */
export function periodFailure(retry: VNode): VNode[] {
  return [
    h("h2", { id: "dash-period-title" }, t("period.title")),
    stateBlock({ kind: "error", title: t("period.error"), body: t("period.error.body") }, retry),
  ];
}

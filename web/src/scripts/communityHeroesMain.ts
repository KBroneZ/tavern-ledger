// Community hero stats (/heroes/) (T-507), loaded after the first paint by
// communityHeroes.ts (T-554): reads both modes and periods once,
// draws them into the reserved #ch-modes, and runs the period and sort
// switches (both kept in the address, like Solo / Duos). The mode and detail
// switches are the data pages' own (switches.ts).

// First: the page's language, before any module draws text (T-508).
import "../lib/i18n/boot.ts";
import { watchPictureFallback } from "../lib/data/pictures.ts";
import { loadCommunityStats, type CommunityStats } from "../lib/community/api.ts";
import { parsePeriod, parseSort, PERIODS, SORTS, withParam, type HeroSort, type Period } from "../lib/community/heroes.ts";
import { ct, type ChKey } from "../lib/community/strings.ts";
import { howFold, modePanels, updatedLine } from "../lib/community/view.ts";
import { h, mount } from "../lib/view/node.ts";
import { stateBlock } from "../lib/view/pieces.ts";
import { supabase } from "./client.ts";
import { byId } from "./dom.ts";

const root = byId("ch");
const status = byId("ch-status");
const body = byId("ch-body");
const panels = byId("ch-modes");
const periods = byId("ch-period-switch");

// Hero pictures fall back to the local placeholder (D-074).
watchPictureFallback(document);

// Not busy first: some screen readers skip a live region's change inside a busy element.
function settle(text: string): void {
  root.setAttribute("aria-busy", "false");
  status.textContent = text;
}

function setPeriod(period: Period): void {
  body.dataset.period = period;
  for (const button of periods.querySelectorAll<HTMLButtonElement>("button[data-value]")) {
    button.setAttribute("aria-pressed", String(button.dataset.value === period));
  }
}

function setupPeriod(): void {
  setPeriod(parsePeriod(location.search));
  periods.addEventListener("click", (event) => {
    const value = (event.target as HTMLElement).closest<HTMLButtonElement>("button[data-value]")?.dataset.value;
    const period = PERIODS.find((p) => p === value);
    if (!period) return;
    setPeriod(period);
    history.replaceState(history.state, "", withParam(location.href, "period", period));
  });
  periods.hidden = false;
}

/** Re-sorts every panel on a sort button's click; keeps the open folds, the sort in the address and focus on the button. */
function watchSort(stats: CommunityStats): void {
  panels.addEventListener("click", (event) => {
    const button = (event.target as HTMLElement).closest<HTMLButtonElement>("button[data-sort]");
    const sort = SORTS.find((s) => s === button?.dataset.sort);
    if (!button || !sort || button.getAttribute("aria-pressed") === "true") return;
    const panel = button.closest<HTMLElement>("[data-period-panel]");
    const where = `[data-mode-panel="${panel?.closest<HTMLElement>("[data-mode-panel]")?.dataset.modePanel}"] [data-period-panel="${panel?.dataset.periodPanel}"]`;
    history.replaceState(history.state, "", withParam(location.href, "sort", sort));
    const open = [...panels.querySelectorAll<HTMLDetailsElement>("details")].map((d) => d.open);
    mount(panels, ...modePanels(stats, sort));
    panels.querySelectorAll<HTMLDetailsElement>("details").forEach((d, i) => (d.open = open[i] ?? false));
    const target =
      panels.querySelector<HTMLButtonElement>(`${where} button[data-sort="${sort}"]`) ??
      panels.querySelector<HTMLButtonElement>('button[data-sort][aria-pressed="true"]');
    target?.focus();
    status.textContent = ct(`sorted.${sort}` as ChKey);
  });
}

async function start(): Promise<void> {
  root.setAttribute("aria-busy", "true");
  status.textContent = ct("page.loading");
  setupPeriod();
  const stats = await loadCommunityStats(supabase);
  const sort: HeroSort = parseSort(location.search);
  byId("ch-updated").textContent = updatedLine(stats.state);
  mount(panels, ...modePanels(stats, sort));
  mount(byId("ch-about"), howFold());
  byId("ch-switches").classList.remove("is-off");
  watchSort(stats);
  settle(ct("page.loaded"));
}

function failed(): void {
  const retry = h("button", { type: "button", class: "secondary", id: "ch-retry" }, ct("page.retry"));
  mount(panels, stateBlock({ kind: "error", title: ct("page.error"), body: ct("page.error.body") }, retry));
  settle(ct("page.error"));
  byId("ch-retry").addEventListener("click", () => location.reload());
}

start().catch(failed);

// Public leaderboards (/leaderboards/) (T-506): reads every board once,
// draws both modes and the three periods into the reserved #lb-modes, and
// runs the period switch (kept in the address, like Solo / Duos). The mode
// and detail switches are the data pages' own (switches.ts).
import { parsePeriod, PERIODS, withPeriod, type Period } from "../lib/leaderboards/boards.ts";
import { loadLeaderboards } from "../lib/leaderboards/api.ts";
import { lt } from "../lib/leaderboards/strings.ts";
import { aboutFold, modePanels, updatedLine } from "../lib/leaderboards/view.ts";
import { h, mount } from "../lib/view/node.ts";
import { stateBlock } from "../lib/view/pieces.ts";
import { supabase } from "./client.ts";
import { byId, moveFocus } from "./dom.ts";

const root = byId("lb");
const status = byId("lb-status");
const body = byId("lb-body");
const panels = byId("lb-modes");
const periods = byId("lb-period");

function settle(text: string): void {
  status.textContent = text;
  root.setAttribute("aria-busy", "false");
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
    history.replaceState(history.state, "", withPeriod(location.href, period));
  });
  periods.hidden = false;
}

async function start(): Promise<void> {
  root.setAttribute("aria-busy", "true");
  status.textContent = lt("page.loading");
  setupPeriod();
  const data = await loadLeaderboards(supabase);
  byId("lb-updated").textContent = updatedLine(data.refreshedAt);
  mount(panels, ...modePanels(data));
  mount(byId("lb-about"), aboutFold());
  byId("lb-switches").classList.remove("is-off");
  settle(lt("page.loaded"));
}

function failed(): void {
  const retry = h("button", { type: "button", class: "secondary", id: "lb-retry" }, lt("page.retry"));
  mount(panels, stateBlock({ kind: "error", title: lt("page.error"), body: lt("page.error.body") }, retry));
  settle(lt("page.error"));
  byId("lb-retry").addEventListener("click", () => {
    moveFocus(byId("lb"));
    location.reload();
  });
}

start().catch(failed);

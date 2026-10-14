// Simple / Advanced and Solo / Duos switches on the data pages (T-501).
// The switches ship hidden; this shows them, so a page without scripts
// simply stays on Simple and Solo. A page that loads the player's games
// may name the mode with more games (MODE_DEFAULT_EVENT, T-559): the switch
// takes it unless the address or the viewer already chose one.
import { MODE_DEFAULT_EVENT, modeInAddress, parseMode, readDetail, saveDetail, withMode, type Detail, type ModeChoice } from "../lib/data/prefs.ts";

function storage(): Storage | null {
  try {
    return window.localStorage;
  } catch {
    return null;
  }
}

function press(group: HTMLElement, value: string): void {
  for (const button of group.querySelectorAll<HTMLButtonElement>("button[data-value]")) {
    button.setAttribute("aria-pressed", String(button.dataset.value === value));
  }
}

function setupDetail(groups: HTMLElement[]): void {
  const apply = (detail: Detail) => {
    document.documentElement.dataset.detail = detail;
    for (const group of groups) press(group, detail);
  };
  apply(readDetail(storage()));
  for (const group of groups) {
    group.addEventListener("click", (event) => {
      const value = (event.target as HTMLElement).closest<HTMLButtonElement>("button[data-value]")?.dataset.value;
      if (value !== "simple" && value !== "advanced") return;
      apply(value);
      saveDetail(storage(), value);
    });
  }
}

/** False when the panels it drives are missing: the switch then stays hidden. */
function setupMode(group: HTMLElement): boolean {
  const panels = group.dataset.controls ? document.getElementById(group.dataset.controls) : null;
  if (!panels) return false;
  const apply = (mode: ModeChoice) => {
    panels.dataset.mode = mode;
    press(group, mode);
  };
  apply(parseMode(location.search));
  let chosen = modeInAddress(location.search) !== null;
  group.addEventListener(MODE_DEFAULT_EVENT, (event) => {
    const mode = (event as CustomEvent<unknown>).detail;
    if (!chosen && (mode === "solo" || mode === "duos")) apply(mode);
  });
  group.addEventListener("click", (event) => {
    const value = (event.target as HTMLElement).closest<HTMLButtonElement>("button[data-value]")?.dataset.value;
    if (value !== "solo" && value !== "duos") return;
    chosen = true;
    apply(value);
    history.replaceState(history.state, "", withMode(location.href, value));
  });
  return true;
}

const groups = [...document.querySelectorAll<HTMLElement>("[data-switch]")];
const detail = groups.filter((g) => g.dataset.switch === "detail");
setupDetail(detail);
for (const group of detail) group.hidden = false;
for (const group of groups.filter((g) => g.dataset.switch === "mode")) group.hidden = !setupMode(group);

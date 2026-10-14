// Runs "why the result differed" for the game page (T-549): the card data
// the page loaded (names.ts), the simulator's own Web Worker (it carries the
// engine and its own script policy, so the game page's policy is unchanged,
// D-090) and one request with the stored report. Fills the turn table's
// odds cells and each turn's detail (T-559); a failure says so under the
// table and every waiting cell shows a dash.
import { mount } from "../view/node.ts";
import { cardLabel, cardNames } from "./names.ts";
import { g } from "./strings.ts";
import { WHY_STATUS_ID } from "./turnsView.ts";
import { explainRequest, newBuildOf, readWhy, type WhyCombat } from "./why.ts";
import { oddsCell, whyDetail } from "./whyView.ts";

/**
 * How long the engine may take: a real game takes a few seconds; past this
 * the worker is stopped and the section says the odds could not be worked out.
 */
export const ENGINE_TIMEOUT_MS = 60_000;

/** The engine's answer to one request (JSON text); `makeWorker` is the simulator's worker, or a stand-in in tests. */
export function askEngine(request: string, makeWorker: () => Worker = simWorker, timeoutMs = ENGINE_TIMEOUT_MS): Promise<string> {
  return new Promise((resolve, reject) => {
    const worker = makeWorker();
    const stop = (error: Error | null, answer = "") => {
      clearTimeout(timer);
      worker.terminate();
      if (error) reject(error);
      else resolve(answer);
    };
    const timer = setTimeout(() => stop(new Error("the engine took too long")), timeoutMs);
    worker.addEventListener("message", (event: MessageEvent<unknown>) => {
      const data = event.data as { id?: unknown; answer?: unknown } | null;
      if (data?.id !== 1) return;
      if (typeof data.answer === "string") stop(null, data.answer);
      else stop(new Error("the engine failed"));
    });
    worker.addEventListener("error", () => stop(new Error("the engine did not start")));
    worker.addEventListener("messageerror", () => stop(new Error("the answer could not be read")));
    worker.postMessage({ id: 1, request });
  });
}

function simWorker(): Worker {
  return new Worker(new URL("../sim/simWorker.ts", import.meta.url), { type: "module" });
}

/** Puts each combat's odds in its row and its cause in the row's detail; with no answer, dashes and a line saying why. */
export function fillWhy(root: ParentNode, combats: WhyCombat[] | null, newBuild: number | null = null): void {
  const nameOf = (id: string) => cardLabel(id, id);
  const byRound = new Map((combats ?? []).map((c) => [c.round, c]));
  for (const cell of root.querySelectorAll<HTMLElement>("[data-odds]")) {
    const combat = byRound.get(Number(cell.dataset.odds));
    if (combat) mount(cell, ...oddsCell(combat));
    else mount(cell, "—");
  }
  for (const box of root.querySelectorAll<HTMLElement>("[data-why]")) {
    const combat = byRound.get(Number(box.dataset.why));
    if (combat) mount(box, ...whyDetail(combat, nameOf));
  }
  const status = root.querySelector<HTMLElement>(`#${WHY_STATUS_ID}`);
  if (combats === null && status) status.textContent = g("turns.odds.failed");
  // The odds come back for a build the rules were not checked against, with a warning.
  else if (newBuild !== null && status) status.textContent = `${status.textContent} ${g("turns.odds.newBuild", { build: newBuild })}`;
}

/** Works out every combat of `report` (the stored report as read) and fills the table. */
export async function explainCombats(report: unknown): Promise<void> {
  let combats: WhyCombat[] | null = null;
  let newBuild: number | null = null;
  const cards = cardNames().cards;
  try {
    if (!cards) throw new Error("no card data");
    const answer = await askEngine(explainRequest(report, (id) => cards.facts(id)));
    combats = readWhy(answer);
    newBuild = newBuildOf(answer);
  } catch {
    combats = null;
  }
  fillWhy(document, combats, newBuild);
}

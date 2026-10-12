// Runs "why the result differed" for the game page (T-549): the card data
// in the page's language, the simulator's own Web Worker (it carries the
// engine and its own script policy, so the game page's policy is unchanged,
// D-090) and one request with the stored report. Fills the section the page
// drew with `whyShell()`; a failure says so there and nothing else changes.
import { lang } from "../i18n.ts";
import { mount } from "../view/node.ts";
import { CardIndex, readCardFile } from "../sim/cards.ts";
import { giftName } from "./gifts.ts";
import { explainRequest, readWhy, type WhyCombat } from "./why.ts";
import { type NameOf, WHY_ID, whyLines } from "./whyView.ts";

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

async function cardIndex(): Promise<CardIndex> {
  const file = lang() === "es" ? await import("../sim/generated/cards.es.json") : await import("../sim/generated/cards.en.json");
  return new CardIndex(readCardFile(file.default));
}

/** Works out every combat of `report` (the stored report as read) and draws the lines. */
export async function explainCombats(report: unknown): Promise<void> {
  const node = document.getElementById(WHY_ID);
  if (!node) return;
  let combats: WhyCombat[] | null = null;
  let nameOf: NameOf = giftName;
  try {
    const cards = await cardIndex();
    // A minion by the card data's name; a Dark Gift by the gift table's.
    nameOf = (id) => cards.get(id)?.n ?? giftName(id);
    combats = readWhy(await askEngine(explainRequest(report, (id) => cards.facts(id))));
  } catch {
    combats = null;
  }
  mount(node, ...whyLines(combats, nameOf));
}

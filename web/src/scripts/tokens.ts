// Fills the scene's blurred hero tokens (components/home/SceneDepth.astro)
// with hero portraits from HearthstoneJSON's host at view time (D-074). Each
// token shows only once its picture has loaded; a failed one stays hidden.
// It waits for the first scroll after the page has loaded, then for an idle
// moment, so the tokens never compete with the first screen, and loads
// nothing when the visitor asked to save data.
import { pictureFor } from "../lib/data/pictures.ts";
import { pickHeroes, tokenDay } from "../lib/home/tokens.ts";

interface SaveData {
  connection?: { saveData?: boolean };
}

const slots = [...document.querySelectorAll<HTMLElement>(".pin-depth .token")];
const saveData = (navigator as Navigator & SaveData).connection?.saveData === true;

function fill(ids: readonly string[]): void {
  // Only tokens the layout shows (narrow screens and high contrast hide some or all).
  const shown = slots.filter((slot) => getComputedStyle(slot).display !== "none" && slot.offsetParent !== null);
  pickHeroes(ids, shown.length, tokenDay(Date.now())).forEach((id, i) => {
    const slot = shown[i];
    const img = document.createElement("img");
    img.fetchPriority = "low";
    img.alt = "";
    img.width = 256;
    img.height = 256;
    img.decoding = "async";
    img.addEventListener("load", () => slot.classList.add("ready"), { once: true });
    img.addEventListener("error", () => img.remove(), { once: true });
    img.src = pictureFor("hero", id).src;
    slot.append(img);
  });
}

async function start(): Promise<void> {
  try {
    // The hero list is its own file, fetched only now.
    const { HERO_NAMES } = await import("../lib/heroes/names.data.ts");
    fill(Object.keys(HERO_NAMES));
  } catch {
    // Decoration only: without the list the tokens simply stay hidden.
  }
}

function whenIdle(run: () => void): void {
  if ("requestIdleCallback" in window) window.requestIdleCallback(run, { timeout: 3000 });
  else setTimeout(run, 1500);
}

// The tokens live in the scene, below the first screen: start on the first
// scroll after the page has loaded.
if (slots.length > 0 && !saveData) {
  const onScroll = () => whenIdle(() => void start());
  const armed = () => window.addEventListener("scroll", onScroll, { once: true, passive: true });
  if (document.readyState === "complete") armed();
  else window.addEventListener("load", armed, { once: true });
}

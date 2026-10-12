// Moves the home page's painting as the page scrolls (T-530, D-105), slower
// while the five-step scene is pinned. Without this script (accounts closed)
// home.css moves it at an even speed on the scroll timeline; with reduced
// motion it stays still at the barrel door. Only a transform changes; the sizes are read again
// when the page or the screen changes size, never while scrolling.
import { backdropOffset, type Descent } from "../lib/home/descent.ts";

const art = document.querySelector<HTMLElement>("[data-descent]");
// The backdrop is 100lvh tall: a steady screen height while a phone's toolbar
// slides in and out (innerHeight changes then, and the painting would jump).
const frameBox = art?.parentElement ?? null;
const pin = document.querySelector<HTMLElement>("#scene");
const stage = pin?.querySelector<HTMLElement>(".pin-stage") ?? null;
const still = window.matchMedia("(prefers-reduced-motion: reduce)");

let descent: Descent = { maxScroll: 0, travel: 0, pinStart: null, pinEnd: null };
let frame = 0;

/** The scroll span over which the scene's stage is stuck to the screen, or null when it never sticks. */
function pinnedSpan(): [number, number] | null {
  if (!pin || !stage || getComputedStyle(stage).position !== "sticky") return null;
  const top = pin.getBoundingClientRect().top + window.scrollY;
  const stuckAt = parseFloat(getComputedStyle(stage).top) || 0;
  return [top - stuckAt, top + pin.offsetHeight - stage.offsetHeight - stuckAt];
}

function measure(): void {
  if (!art) return;
  const screen = frameBox?.offsetHeight || window.innerHeight;
  const span = pinnedSpan();
  descent = {
    maxScroll: document.documentElement.scrollHeight - screen,
    travel: art.offsetHeight - screen,
    pinStart: span?.[0] ?? null,
    pinEnd: span?.[1] ?? null,
  };
}

function draw(): void {
  frame = 0;
  if (!art) return;
  const y = still.matches ? 0 : backdropOffset(window.scrollY, descent);
  art.style.transform = `translate3d(0, ${-Math.round(y * 10) / 10}px, 0)`;
}

function schedule(): void {
  if (frame === 0) frame = requestAnimationFrame(draw);
}

function remeasure(): void {
  measure();
  schedule();
}

if (art) {
  // Takes over from the CSS descent.
  art.dataset.descent = "script";
  remeasure();
  window.addEventListener("scroll", schedule, { passive: true });
  window.addEventListener("resize", remeasure);
  still.addEventListener("change", remeasure);
  // Fonts, the painting itself and late layout change the page's height.
  new ResizeObserver(remeasure).observe(document.body);
}

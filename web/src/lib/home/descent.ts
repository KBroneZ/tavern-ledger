// The home page's backdrop (T-530, D-105): one tall painting behind the whole
// page, from the barrel door at the top to the cellar at the bottom. It moves
// evenly with the page, except while the five-step scene is pinned (D-052),
// when it moves at a tenth of that speed: just enough to feel the descent.

/** The painting's speed during the pinned scene, against its speed outside it. */
export const PIN_SPEED = 0.1;

export interface Descent {
  /** How far the page scrolls, in px (its height minus the screen's). */
  maxScroll: number;
  /** How far the painting moves, in px (its height minus the screen's). */
  travel: number;
  /** Where the pinned scene starts and ends, in px of scroll; null when it is not pinned. */
  pinStart: number | null;
  pinEnd: number | null;
}

const clamp = (v: number, lo: number, hi: number) => Math.min(hi, Math.max(lo, v));

/** The scroll as the painting feels it: the pinned part counts a tenth. */
function felt(scroll: number, d: Descent): number {
  const { pinStart, pinEnd } = d;
  if (pinStart === null || pinEnd === null || !Number.isFinite(pinStart) || !Number.isFinite(pinEnd) || pinEnd <= pinStart) {
    return scroll;
  }
  const pinned = clamp(scroll, pinStart, pinEnd) - clamp(0, pinStart, pinEnd);
  return scroll - pinned * (1 - PIN_SPEED);
}

/** How far up the painting has moved, in px, at this scroll position. */
export function backdropOffset(scroll: number, d: Descent): number {
  if (!Number.isFinite(scroll) || !(d.maxScroll > 0) || !(d.travel > 0)) return 0;
  const total = felt(d.maxScroll, d);
  if (!(total > 0)) return 0;
  return (d.travel * felt(clamp(scroll, 0, d.maxScroll), d)) / total;
}

// Combat replay motion on the page (T-560, D-132): the attacker winds up,
// flies to its target and back, the target shakes as it is hit, and a board
// that loses or gains a minion slides its others into place. Web Animations
// on `transform` only, so nothing is laid out again while it moves; the page
// measures the boards once before a step and once after it (replayPlayer.ts),
// never in between. The page calls none of this with reduced motion.
import { BEAT_MS } from "./replay.ts";

/** An element's box on the page, as getBoundingClientRect gives it. */
export interface Box {
  x: number;
  y: number;
  width: number;
  height: number;
}

/** An attack at 1×: wind-up and flight until the hit (the attack's beat), then back. */
export const IMPACT_MS = BEAT_MS.attack;
export const ATTACK_MS = IMPACT_MS + 300;
/** The target's shake as it is hit. */
const HIT_MS = 260;
/** A board closing a gap or making room. */
const SLIDE_MS = 260;
/** How far towards the target's centre the attacker flies: it stops at its edge. */
const REACH = 0.78;
const WIND_UP_PX = 8;
const EASE_OUT = "cubic-bezier(0.23, 1, 0.32, 1)";
const EASE_IN_OUT = "cubic-bezier(0.77, 0, 0.175, 1)";

// One decimal, and never "-0px".
const px = (v: number) => `${Math.round(v * 10) / 10 + 0}px`;
const translate = (x: number, y: number) => `translate(${px(x)}, ${px(y)})`;
const centre = (b: Box) => ({ x: b.x + b.width / 2, y: b.y + b.height / 2 });

/** The attacker's flight from `from` to `to`, as keyframes over ATTACK_MS. */
export function lungeFrames(from: Box, to: Box): Keyframe[] {
  const a = centre(from);
  const b = centre(to);
  const dx = b.x - a.x;
  const dy = b.y - a.y;
  const length = Math.hypot(dx, dy) || 1;
  const hit = IMPACT_MS / ATTACK_MS;
  return [
    { offset: 0, transform: translate(0, 0), easing: EASE_OUT },
    { offset: hit * 0.3, transform: `${translate((-dx / length) * WIND_UP_PX, (-dy / length) * WIND_UP_PX)} scale(1.08)`, easing: EASE_IN_OUT },
    { offset: hit, transform: `${translate(dx * REACH, dy * REACH)} scale(1.08)`, easing: EASE_OUT },
    { offset: 1, transform: translate(0, 0) },
  ];
}

/** How far a minion moved between two boxes; null when it did not (under a pixel). */
export function flipDelta(before: Box, after: Box): { dx: number; dy: number } | null {
  const dx = before.x - after.x;
  const dy = before.y - after.y;
  return Math.abs(dx) < 1 && Math.abs(dy) < 1 ? null : { dx, dy };
}

// ---------- On the page ----------

function boxOf(el: Element): Box {
  const r = el.getBoundingClientRect();
  return { x: r.left, y: r.top, width: r.width, height: r.height };
}

/** Where each element is now: read before a step changes the boards. */
export function measure(els: Iterable<HTMLElement>): Map<HTMLElement, Box> {
  const out = new Map<HTMLElement, Box>();
  for (const el of els) if (el.isConnected) out.set(el, boxOf(el));
  return out;
}

const MOTIONS = ["rp-lunge", "rp-hit", "rp-slide"];

/** Stops the motion of this kind already running on `el`, so the new one starts from where it is. */
function restart(el: HTMLElement, id: string): void {
  for (const a of el.getAnimations()) if (a.id === id) a.cancel();
}

/** Stops every motion on `els`: a jump or a step back draws the step as it is. */
export function still(els: Iterable<HTMLElement>): void {
  for (const el of els) {
    for (const a of el.getAnimations()) if (MOTIONS.includes(a.id)) a.cancel();
    delete el.dataset.lunging;
  }
}

/** The lunge running on each element: only its own end clears the mark. */
const lunging = new WeakMap<HTMLElement, Animation>();

/**
 * `attacker` flies to `target` and back, drawn above its neighbours and the
 * other board meanwhile (`data-lunging`: an attribute, so redrawing the
 * minion's classes keeps it).
 */
export function lunge(attacker: HTMLElement, target: Element, speed: number): void {
  restart(attacker, "rp-lunge");
  attacker.dataset.lunging = "";
  const motion = attacker.animate(lungeFrames(boxOf(attacker), boxOf(target)), { duration: ATTACK_MS / speed, id: "rp-lunge" });
  lunging.set(attacker, motion);
  const done = () => {
    if (lunging.get(attacker) === motion) delete attacker.dataset.lunging;
  };
  motion.addEventListener("finish", done);
  motion.addEventListener("cancel", done);
}

/** `el` shakes as the attack lands (IMPACT_MS after the attack's step). */
export function hit(el: HTMLElement, speed: number): void {
  restart(el, "rp-hit");
  el.animate(
    [
      { transform: translate(0, 0) },
      { transform: translate(-4, 0) },
      { transform: translate(4, 0) },
      { transform: translate(-2, 0) },
      { transform: translate(0, 0) },
    ],
    { duration: HIT_MS / speed, delay: IMPACT_MS / speed, easing: "ease-in-out", id: "rp-hit" },
  );
}

/** Each minion of `els` that moved since `before` slides from where it was. */
export function slide(before: Map<HTMLElement, Box>, els: HTMLElement[], speed: number): void {
  for (const el of els) {
    const was = before.get(el);
    const delta = was ? flipDelta(was, boxOf(el)) : null;
    if (!delta) continue;
    restart(el, "rp-slide");
    el.animate([{ transform: translate(delta.dx, delta.dy) }, { transform: translate(0, 0) }], {
      duration: SLIDE_MS / speed,
      easing: EASE_OUT,
      id: "rp-slide",
    });
  }
}

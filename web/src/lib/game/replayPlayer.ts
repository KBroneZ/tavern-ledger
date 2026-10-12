// The combat replays' controls on the page (T-536): picks a combat, plays,
// pauses and steps through it. It redraws only what a step changes (each
// minion's numbers and marks, the hero damage, the words), so a picture
// never loads twice and a button keeps focus; buttons at either end are
// aria-disabled rather than disabled for the same reason. The step's words
// are a polite live region, quiet while playing and read on a step or a
// pause. With reduced motion nothing plays by itself: step by step only.
// Everything drawn comes from replayView.ts (text only, lib/view/node.ts).
import { mount, toDom } from "../view/node.ts";
import { type Frame, type Playback, SPEEDS, STEP_MS, stepWords } from "./replay.ts";
import type { Side } from "./replayRead.ts";
import {
  counterText,
  DEFAULT_SPEED,
  emptyBoard,
  firstCombat,
  framesOf,
  heroAt,
  heroFace,
  heroHit,
  namerFor,
  playerView,
  playText,
  tileClass,
  tileInfo,
  tileView,
} from "./replayView.ts";
import type { ReplayCombat } from "./shape.ts";

const SIDES: Side[] = ["opponent", "own"];
const REDUCED = "(prefers-reduced-motion: reduce)";

interface Player {
  combat: ReplayCombat;
  pb: Playback;
  box: HTMLElement;
  /** Each minion's element, by its index in the replay. */
  tiles: Map<number, HTMLElement>;
  step: number;
  playing: boolean;
}

function one<T extends Element>(root: ParentNode, selector: string): T {
  const el = root.querySelector<T>(selector);
  if (!el) throw new Error(`${selector} is missing`);
  return el;
}

/** Scrolls the step list (not the page) so the current step shows. */
function keepInView(box: HTMLElement, step: number): void {
  const list = box.querySelector<HTMLElement>(".rp-list");
  const item = list?.children[step] as HTMLElement | undefined;
  if (!list || !item || list.clientHeight === 0) return;
  const top = item.offsetTop;
  if (top < list.scrollTop || top + item.offsetHeight > list.scrollTop + list.clientHeight) {
    list.scrollTop = Math.max(0, top - list.clientHeight / 2);
  }
}

/** Starts the section's controls; the page calls it once the game is drawn. */
export function startReplays(section: HTMLElement, combats: ReplayCombat[]): void {
  const holder = section.querySelector<HTMLElement>(".rp-holder");
  const first = firstCombat(combats);
  if (!holder || !first) return;
  const motion = window.matchMedia(REDUCED);
  const cache = new Map<number, Playback>();
  const steps = new Map<number, number>();
  let speed: number = DEFAULT_SPEED;
  let timer: number | null = null;
  let player = adopt(first, 0);

  function playbackOf(combat: ReplayCombat): Playback {
    const known = cache.get(combat.round);
    if (known) return known;
    const pb = framesOf(combat);
    cache.set(combat.round, pb);
    return pb;
  }

  /** Takes over the player drawn in the holder. */
  function adopt(combat: ReplayCombat, step: number): Player {
    const box = one<HTMLElement>(holder as HTMLElement, ".rp-player");
    const tiles = new Map<number, HTMLElement>();
    for (const li of box.querySelectorAll<HTMLElement>(".rp-minion[data-ref]")) tiles.set(Number(li.dataset.ref), li);
    return { combat, pb: playbackOf(combat), box, tiles, step, playing: false };
  }

  function stop(): void {
    if (timer !== null) window.clearTimeout(timer);
    timer = null;
  }

  function drawSide(frame: Frame, side: Side): void {
    const row = one<HTMLElement>(player.box, `.rp-side[data-side="${side}"]`);
    const face = one<HTMLElement>(row, ".rp-hero-face");
    const hero = heroAt(player.combat, frame, side);
    if (face.dataset.hero !== (hero.id ?? "")) {
      face.dataset.hero = hero.id ?? "";
      mount(face, ...heroFace(hero));
    }
    mount(one(row, ".rp-hit"), ...heroHit(frame.hero[side]));
    const nodes = frame[side].map((tile) => {
      let li = player.tiles.get(tile.ref);
      if (!li) {
        li = toDom(tileView(tile)) as HTMLElement;
        player.tiles.set(tile.ref, li);
      } else {
        li.className = tileClass(tile);
        mount(one(li, ".rp-info"), ...tileInfo(tile));
      }
      return li;
    });
    one<HTMLElement>(row, ".rp-board").replaceChildren(...(nodes.length > 0 ? nodes : [toDom(emptyBoard())]));
  }

  function draw(): void {
    const { box, pb, step, playing, combat } = player;
    const last = pb.frames.length - 1;
    const frame = pb.frames[step];
    for (const side of SIDES) drawSide(frame, side);
    const caption = one<HTMLElement>(box, ".rp-caption");
    caption.setAttribute("aria-live", playing ? "off" : "polite");
    caption.textContent = combat.replay ? stepWords(frame.event, combat.replay, namerFor(combat)) : "";
    one<HTMLElement>(box, ".rp-counter").textContent = counterText(step, last);
    const play = one<HTMLElement>(box, '[data-act="play"]');
    const words = playText(playing);
    play.textContent = words.text;
    play.setAttribute("aria-label", words.label);
    play.title = words.label;
    for (const act of ["restart", "back"]) one(box, `[data-act="${act}"]`).setAttribute("aria-disabled", String(step === 0));
    one(box, '[data-act="forward"]').setAttribute("aria-disabled", String(step === last));
    play.setAttribute("aria-disabled", String(last === 0));
    for (const b of box.querySelectorAll<HTMLElement>(".rp-step")) {
      if (Number(b.dataset.step) === step) b.setAttribute("aria-current", "step");
      else b.removeAttribute("aria-current");
    }
    keepInView(box, step);
    steps.set(combat.round, step);
  }

  function go(step: number, playing: boolean): void {
    stop();
    const last = player.pb.frames.length - 1;
    player.step = Math.min(Math.max(step, 0), last);
    player.playing = playing && !motion.matches && player.step < last;
    draw();
    if (player.playing) timer = window.setTimeout(tick, STEP_MS / speed);
  }

  function tick(): void {
    timer = null;
    // Stops by itself once the game is no longer on the page.
    if (!section.isConnected) return;
    go(player.step + 1, true);
  }

  function open(round: number): void {
    const combat = combats.find((c) => c.round === round && c.replay !== null);
    if (!combat || combat.round === player.combat.round) return;
    stop();
    const step = steps.get(round) ?? 0;
    mount(holder as HTMLElement, playerView(combat, playbackOf(combat), step));
    player = adopt(combat, step);
    one<HTMLSelectElement>(player.box, 'select[data-act="speed"]').value = String(speed);
    for (const chip of section.querySelectorAll<HTMLElement>(".rp-chip")) {
      chip.setAttribute("aria-pressed", String(chip.dataset.round === String(round)));
    }
    go(step, false);
  }

  function act(name: string | undefined): void {
    const { step, playing } = player;
    const last = player.pb.frames.length - 1;
    if (name === "play") {
      // At the end, play starts over.
      if (playing) go(step, false);
      else go(step >= last ? 0 : step, true);
    } else if (name === "back") go(step - 1, false);
    else if (name === "forward") go(step + 1, false);
    else if (name === "restart") go(0, false);
    else if (name === "end") go(last, false);
  }

  section.addEventListener("click", (event) => {
    const target = event.target as HTMLElement;
    const chip = target.closest<HTMLElement>(".rp-chip");
    if (chip) return open(Number(chip.dataset.round));
    const stepButton = target.closest<HTMLElement>(".rp-step");
    if (stepButton) return go(Number(stepButton.dataset.step), false);
    const control = target.closest<HTMLElement>("button[data-act]");
    if (control && control.getAttribute("aria-disabled") !== "true") act(control.dataset.act);
  });

  section.addEventListener("change", (event) => {
    const select = (event.target as HTMLElement).closest<HTMLSelectElement>('select[data-act="speed"]');
    if (!select) return;
    const value = Number(select.value);
    speed = (SPEEDS as readonly number[]).includes(value) ? value : DEFAULT_SPEED;
    if (player.playing) go(player.step, true);
  });

  // The arrow keys, Home and End on the controls step through the combat.
  section.addEventListener("keydown", (event) => {
    const target = event.target as HTMLElement;
    if (!target.closest(".rp-controls") || target.tagName === "SELECT") return;
    const keys: Record<string, string> = { ArrowLeft: "back", ArrowRight: "forward", Home: "restart", End: "end" };
    const name = keys[event.key];
    if (!name) return;
    event.preventDefault();
    act(name);
  });

  // Opening the step list shows the current step ("toggle" does not bubble).
  section.addEventListener("toggle", () => keepInView(player.box, player.step), true);

  motion.addEventListener("change", () => {
    if (motion.matches && player.playing) go(player.step, false);
  });

  go(0, false);
}

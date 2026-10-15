// The combat replays' controls on the page (T-536): picks a combat, plays,
// pauses and steps through it. It redraws only what a step changes (each
// minion's numbers and marks, the hero damage, the words), so a picture
// never loads twice and a button keeps focus; buttons at either end are
// aria-disabled rather than disabled for the same reason. A hidden polite
// live region reads the step's words on a step or a pause, never while
// playing. Nothing plays by itself; with reduced motion Play still plays, a
// step at a time at the same pace, without movement.
// What each button and key does is plain data (replay.ts control, keyAction).
// Everything drawn comes from replayView.ts (text only, lib/view/node.ts).
// Each step stays for its own beat, close to the game's pace (replay.ts
// beats), and moving one step on plays it (replayMotion.ts): the attacker
// flies to its target, the board slides into place; a jump does not move.
import { carryMinion } from "../art/paint.ts";
import { paintMinions } from "../art/runtime.ts";
import { mount, toDom } from "../view/node.ts";
import { control, type Frame, frameBeats, frameWords, keyAction, type PlayAction, type Playback, SPEEDS, stepWords } from "./replay.ts";
import { hit, lunge, measure, slide, still } from "./replayMotion.ts";
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
  tileDrawn,
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
  /** Each step's beat at 1× (replay.ts frameBeats). */
  beats: number[];
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
    const pb = playbackOf(combat);
    return { combat, pb, box, tiles, beats: frameBeats(pb), step, playing: false };
  }

  function stop(): void {
    if (timer !== null) window.clearTimeout(timer);
    timer = null;
  }

  /** Draws one side; returns its minions' elements, left to right. */
  function drawSide(frame: Frame, side: Side): HTMLElement[] {
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
        carryMinion(li, tileDrawn(tile));
        mount(one(li, ".rp-info"), ...tileInfo(tile));
      }
      return li;
    });
    const board = one<HTMLElement>(row, ".rp-board");
    // The board is rebuilt only when its minions change: a running motion
    // or a mark's animation is not restarted by a step that leaves it be.
    const same = nodes.length === board.children.length && nodes.every((n, i) => board.children[i] === n);
    const stillEmpty = nodes.length === 0 && board.children.length === 1 && board.firstElementChild?.classList.contains("rp-empty");
    if (!same && !stillEmpty) board.replaceChildren(...(nodes.length > 0 ? nodes : [toDom(emptyBoard())]));
    return nodes;
  }

  /** The step's motion, once the boards are drawn: an attack, the hero's hit, the board sliding into place. */
  function move(frame: Frame, before: ReturnType<typeof measure>, nodes: HTMLElement[]): void {
    slide(before, nodes, speed);
    const e = frame.event;
    if (e?.kind === "attack" && e.defender !== null) {
      const attacker = player.tiles.get(e.attacker);
      const defender = player.tiles.get(e.defender);
      if (attacker?.isConnected && defender?.isConnected) {
        lunge(attacker, defender, speed);
        hit(defender, speed);
      }
    } else if (e?.kind === "hero_damage") {
      // The winner's hero hits the loser's.
      const face = (side: Side) => one<HTMLElement>(player.box, `.rp-side[data-side="${side}"] .rp-hero-face`);
      const winner = e.side === "own" ? "opponent" : "own";
      lunge(face(winner), face(e.side), speed);
      hit(face(e.side), speed);
    }
  }

  /** Each hit's tooltip says what dealt it ("… from their Cleave Hound (attack)"), as the step's words do. */
  function hitTips(frame: Frame): void {
    const replay = player.combat.replay;
    if (!replay) return;
    const tips = new Map<number, string[]>();
    for (const e of frame.events) {
      if (e.kind === "damage") tips.set(e.minion, [...(tips.get(e.minion) ?? []), stepWords(e, replay, namerFor(player.combat))]);
    }
    for (const [ref, lines] of tips) {
      const hurt = player.tiles.get(ref)?.querySelector<HTMLElement>(".rp-hurt");
      if (hurt) hurt.title = lines.join(". ");
    }
  }

  /** Says the step's words once, in the hidden live region; quiet while playing. */
  function announce(text: string): void {
    const live = one<HTMLElement>(player.box, ".rp-live");
    live.textContent = "";
    // A new text a frame later, so a repeated one is read too.
    window.requestAnimationFrame(() => {
      live.textContent = text;
    });
  }

  /** The minions and both heroes' faces: everything that can move. */
  function movers(): HTMLElement[] {
    return [...player.tiles.values(), ...player.box.querySelectorAll<HTMLElement>(".rp-hero-face")];
  }

  /**
   * Draws the step. "move": one step on, played; "still": a jump, a step back
   * or another combat, drawn as it is, every motion and mark animation
   * stopped; "keep": the same step (a pause), left as it is.
   */
  function draw(speak: boolean, how: "move" | "still" | "keep"): void {
    const { box, pb, step, playing, combat } = player;
    const last = pb.frames.length - 1;
    const frame = pb.frames[step];
    if (how === "still") still(movers());
    if (how !== "keep") one(box, ".rp-stage").classList.toggle("is-still", how === "still");
    // Measured before the boards change and once after, never in between.
    const before = how === "move" ? measure(player.tiles.values()) : null;
    const nodes = SIDES.flatMap((side) => drawSide(frame, side));
    // Each token's drawing follows the step: its numbers and keywords (art/runtime.ts).
    paintMinions(box);
    if (before) move(frame, before, nodes);
    const words = combat.replay ? frameWords(frame, combat.replay, namerFor(combat)) : "";
    one<HTMLElement>(box, ".rp-caption").textContent = words;
    hitTips(frame);
    if (speak && !playing) announce(words);
    one<HTMLElement>(box, ".rp-counter").textContent = counterText(step, last);
    const play = one<HTMLElement>(box, '[data-act="play"]');
    const label = playText(playing);
    play.textContent = label.text;
    play.setAttribute("aria-label", label.label);
    play.title = label.label;
    for (const name of ["restart", "back"]) one(box, `[data-act="${name}"]`).setAttribute("aria-disabled", String(step === 0));
    one(box, '[data-act="forward"]').setAttribute("aria-disabled", String(step === last));
    play.setAttribute("aria-disabled", String(last === 0));
    for (const b of box.querySelectorAll<HTMLElement>(".rp-step")) {
      if (Number(b.dataset.step) === step) b.setAttribute("aria-current", "step");
      else b.removeAttribute("aria-current");
    }
    keepInView(box, step);
    steps.set(combat.round, step);
  }

  /** Applies an action (replay.ts control) and draws the result; `fresh`: a combat just drawn. */
  function act(action: PlayAction, speak = true, fresh = false): void {
    stop();
    const next = control(player, action, player.pb.frames.length - 1);
    // Only one step on moves; a jump, a step back or reduced motion draws the step as it is.
    const how = !fresh && !motion.matches && next.step === player.step + 1 ? "move" : fresh || next.step !== player.step ? "still" : "keep";
    player.step = next.step;
    player.playing = next.playing;
    draw(speak, how);
    schedule();
  }

  /** The next step plays after this one's beat, at the chosen speed. */
  function schedule(): void {
    if (player.playing) timer = window.setTimeout(tick, player.beats[player.step] / speed);
  }

  function tick(): void {
    timer = null;
    // Stops by itself once the game is no longer on the page.
    if (!section.isConnected) return;
    act("tick");
  }

  function open(round: number): void {
    const combat = combats.find((c) => c.round === round && c.replay !== null);
    if (!combat || combat.round === player.combat.round) return;
    stop();
    // The step list stays open or closed as it was, and each combat keeps its step.
    const listOpen = player.box.querySelector<HTMLDetailsElement>(".rp-steps")?.open ?? false;
    const step = steps.get(round) ?? 0;
    mount(holder as HTMLElement, playerView(combat, playbackOf(combat), step));
    player = adopt(combat, step);
    one<HTMLSelectElement>(player.box, 'select[data-act="speed"]').value = String(speed);
    one<HTMLDetailsElement>(player.box, ".rp-steps").open = listOpen;
    for (const chip of section.querySelectorAll<HTMLElement>(".rp-chip")) {
      chip.setAttribute("aria-pressed", String(chip.dataset.round === String(round)));
    }
    // The chip's pressed state says which combat; its first words need no reading.
    act({ goto: step }, false, true);
  }

  section.addEventListener("click", (event) => {
    const target = event.target as HTMLElement;
    const chip = target.closest<HTMLElement>(".rp-chip");
    if (chip) return open(Number(chip.dataset.round));
    const stepButton = target.closest<HTMLElement>(".rp-step");
    if (stepButton) return act({ goto: Number(stepButton.dataset.step) });
    const button = target.closest<HTMLElement>("button[data-act]");
    const name = button?.dataset.act;
    if (!button || button.getAttribute("aria-disabled") === "true") return;
    if (name === "play" || name === "back" || name === "forward" || name === "restart") act(name);
  });

  section.addEventListener("change", (event) => {
    const select = (event.target as HTMLElement).closest<HTMLSelectElement>('select[data-act="speed"]');
    if (!select) return;
    const value = Number(select.value);
    speed = (SPEEDS as readonly number[]).includes(value) ? value : DEFAULT_SPEED;
    if (player.playing) {
      stop();
      schedule();
    }
  });

  // The arrow keys, Home and End on the controls step through the combat.
  section.addEventListener("keydown", (event) => {
    const target = event.target as HTMLElement;
    if (!target.closest(".rp-controls") || target.tagName === "SELECT") return;
    const action = keyAction(event);
    if (!action) return;
    event.preventDefault();
    act(action);
  });

  // Opening the step list shows the current step ("toggle" does not bubble).
  section.addEventListener("toggle", () => keepInView(player.box, player.step), true);

  motion.addEventListener("change", () => {
    if (motion.matches) act({ goto: player.step }, false, true);
  });

  act({ goto: 0 }, false, true);
}

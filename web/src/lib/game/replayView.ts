// Combat replays on the game viewer (T-536, D-099) as element trees. Simple
// shows the two boards, play and pause, and the result; Advanced (`.adv`)
// adds the start, step back and forward, the speed and every step in words.
// With reduced motion there is no play: step back and forward show in Simple
// too (replay.css). Theirs on top, yours below, as in the game. The page's
// script (replayPlayer.ts) moves through the steps; these trees are what it
// draws, so the tests see the same markup.
import { t } from "../data/strings.ts";
import { cls, h, type VNode } from "../view/node.ts";
import { picture, sourceMark } from "../view/pieces.ts";
import { giftList } from "./gifts.ts";
import { KEYWORDS } from "./keywords.ts";
import { type Frame, type Namer, type Playback, playback, SPEEDS, stepWords, type Tile } from "./replay.ts";
import type { Side } from "./replayRead.ts";
import type { GameView, HeroRef, ReplayCombat } from "./shape.ts";
import { g } from "./strings.ts";
import { heroLabel, keywordWords } from "./words.ts";

const SECTION_ID = "gv-replays";
/** The speed a page starts at. */
export const DEFAULT_SPEED = 1;

/** How a combat's words name its minions (by card id: the site has no card names yet), cards and heroes. */
export function namerFor(combat: ReplayCombat): Namer {
  return {
    minion: (ref) => combat.replay?.minions[ref]?.cardId ?? g("minion.unknownCard"),
    card: (id) => id,
    hero: (id) => heroLabel(id === null ? null : (combat.heroNames.get(id) ?? { id, name: null })),
  };
}

export function framesOf(combat: ReplayCombat): Playback {
  return combat.replay ? playback(combat.replay, combat.heroes) : { frames: [], skipped: 0 };
}

export function heroAt(combat: ReplayCombat, frame: Frame, side: Side): HeroRef {
  const id = frame.heroes[side];
  return id === null ? { id: null, name: null } : (combat.heroNames.get(id) ?? { id, name: null });
}

// ---------- A minion ----------

export function tileClass(tile: Tile): string {
  const m = tile.marks;
  return cls("rp-minion", {
    "is-golden": tile.golden,
    "has-shield": tile.shield,
    "is-attacker": !!m.attacker,
    "is-defender": !!m.defender,
    "is-hurt": !!m.hurt,
    "is-popped": !!m.popped,
    "is-buffed": !!m.buffed,
    "is-dying": !!m.dying,
    "is-summoned": !!m.summoned,
    "is-triggered": !!m.triggered,
  });
}

/** What a screen reader hears for a minion: "SYN_M_02, 3/3, Golden, Taunt, Divine Shield". Its Dark Gifts follow (giftList). */
export function tileWords(tile: Tile): string {
  // Divine Shield as it is at this step: it can pop, or be given.
  const keywords = KEYWORDS.filter((k) => (k === "divine_shield" ? tile.shield : tile.keywords.includes(k)));
  return [
    tile.cardId ?? g("minion.unknownCard"),
    g("minion.stats", { attack: tile.atk, health: Math.max(tile.health, 0) }),
    tile.golden ? g("minion.golden") : "",
    keywordWords(keywords),
  ].filter(Boolean).join(", ");
}

/** The parts of a tile that change from step to step. Damage past 0 (overkill) shows as 0, as on the game's board. */
export function tileInfo(tile: Tile): VNode[] {
  return [
    h(
      "span",
      { class: "rp-stats", "aria-hidden": "true" },
      h("span", { class: "rp-atk" }, String(tile.atk)),
      h("span", { class: "rp-hp" }, String(Math.max(tile.health, 0))),
    ),
    tile.marks.hurt ? h("span", { class: "rp-hurt", "aria-hidden": "true" }, `−${tile.marks.hurt}`) : null,
    h("span", { class: "visually-hidden" }, tileWords(tile)),
  ].filter((n): n is VNode => n !== null);
}

export function tileView(tile: Tile): VNode {
  return h(
    "li",
    { class: tileClass(tile), "data-ref": tile.ref },
    h("span", { class: "rp-art" }, picture("card", tile.cardId, tile.cardId ?? g("minion.unknownCard"), 56, true)),
    h("span", { class: "rp-info" }, ...tileInfo(tile)),
    giftList(tile),
  );
}

export function emptyBoard(): VNode {
  return h("li", { class: "rp-empty" }, g("boards.empty"));
}

// ---------- A side: hero and board ----------

export function heroFace(hero: HeroRef): VNode[] {
  return [picture("hero", hero.id, heroLabel(hero), 40, true), h("span", { class: "rp-hero-name" }, heroLabel(hero))];
}

export function heroHit(damage: number): VNode[] {
  if (damage <= 0) return [];
  return [h("span", { "aria-hidden": "true" }, `−${damage}`), h("span", { class: "visually-hidden" }, g("replay.heroHit", { n: damage }))];
}

function sideView(combat: ReplayCombat, frame: Frame, side: Side): VNode {
  const hero = heroAt(combat, frame, side);
  const tiles = frame[side];
  return h(
    "div",
    { class: `rp-side is-${side}`, "data-side": side },
    h(
      "div",
      { class: "rp-hero" },
      h("span", { class: "rp-hero-face", "data-hero": hero.id ?? "" }, ...heroFace(hero)),
      h("span", { class: "rp-hit" }, ...heroHit(frame.hero[side])),
    ),
    h(
      "ul",
      { class: "rp-board", role: "list", "aria-label": g(side === "own" ? "replay.boardOwn" : "replay.boardTheirs") },
      tiles.length > 0 ? tiles.map(tileView) : emptyBoard(),
    ),
  );
}

// ---------- One combat's player ----------

function button(act: string, className: string, text: string, label: string): VNode {
  return h("button", { type: "button", class: cls("rp-btn", className), "data-act": act, "aria-label": label, title: label }, text);
}

export function playText(playing: boolean): { text: string; label: string } {
  return playing ? { text: `❚❚ ${g("replay.pause")}`, label: g("replay.pause") } : { text: `▶ ${g("replay.play")}`, label: g("replay.play") };
}

function controls(): VNode {
  const play = playText(false);
  return h(
    "div",
    { class: "rp-controls", role: "group", "aria-label": g("replay.controls") },
    button("restart", "adv", "⏮", g("replay.restart")),
    button("back", "rp-stepper", "‹", g("replay.back")),
    button("play", "rp-play", play.text, play.label),
    button("forward", "rp-stepper", "›", g("replay.forward")),
    h(
      "label",
      { class: "rp-speed adv" },
      g("replay.speed"),
      " ",
      h(
        "select",
        { "data-act": "speed" },
        SPEEDS.map((s) => h("option", { value: s, selected: s === DEFAULT_SPEED }, g("replay.speedX", { n: s }))),
      ),
    ),
  );
}

function resultChip(combat: ReplayCombat): VNode {
  const result = combat.result ?? "unknown";
  return h(
    "span",
    { class: "rp-result" },
    h("span", { class: `res-${result}` }, t(`result.${result}`)),
    combat.inferred ? " " : null,
    combat.inferred ? sourceMark("inferred") : null,
  );
}

export function counterText(step: number, last: number): string {
  return g("replay.stepOf", { n: step, total: last });
}

function stepList(combat: ReplayCombat, pb: Playback, step: number): VNode {
  const namer = namerFor(combat);
  const replay = combat.replay;
  const items = pb.frames.map((f, i) =>
    h(
      "li",
      {},
      h(
        "button",
        { type: "button", class: "rp-step", "data-step": i, "aria-current": i === step ? "step" : undefined },
        replay ? stepWords(f.event, replay, namer) : "",
      ),
    ),
  );
  return h(
    "details",
    { class: "fold fold-compact adv rp-steps" },
    h("summary", {}, h("span", {}, g("replay.steps", { n: pb.frames.length - 1 }))),
    h("div", { class: "fold-body" }, h("ol", { class: "rp-list", start: 0 }, items)),
  );
}

/** The player of one combat, drawn at `step`. */
export function playerView(combat: ReplayCombat, pb: Playback, step: number): VNode {
  const frame = pb.frames[step];
  const last = pb.frames.length - 1;
  const who = combat.opponents.map(heroLabel).join(" + ") || g("hero.unknown");
  const words = combat.replay ? stepWords(frame.event, combat.replay, namerFor(combat)) : "";
  return h(
    "div",
    { class: "rp-player", "data-round": combat.round },
    h(
      "div",
      { class: "rp-head" },
      h("h3", { class: "rp-title" }, g("replay.combatOf", { turn: combat.round, who })),
      resultChip(combat),
      h("span", { class: "rp-counter" }, counterText(step, last)),
    ),
    h("div", { class: "rp-stage" }, sideView(combat, frame, "opponent"), h("div", { class: "rp-line", "aria-hidden": "true" }), sideView(combat, frame, "own")),
    h("p", { class: "rp-caption", "aria-live": "polite", "aria-atomic": "true" }, words),
    controls(),
    h("p", { class: "data-note rp-reduced" }, g("replay.reduced")),
    combat.replay?.complete === false ? h("p", { class: "data-note" }, sourceMark("unknown"), " ", g("replay.cut")) : null,
    pb.skipped > 0
      ? h("p", { class: "data-note" }, sourceMark("unknown"), " ", pb.skipped === 1 ? g("replay.skipped.one") : g("replay.skipped.other", { count: pb.skipped }))
      : null,
    stepList(combat, pb, step),
  );
}

// ---------- The section ----------

function chips(combats: ReplayCombat[], selected: number): VNode {
  return h(
    "div",
    { class: "rp-pick", role: "group", "aria-label": g("replay.pick") },
    combats.map((c) => {
      const result = c.result ?? "unknown";
      return h(
        "button",
        {
          type: "button",
          class: `rp-chip is-${result}`,
          "data-round": c.round,
          "aria-pressed": String(c.round === selected),
          "aria-label": g("replay.chipLabel", { turn: c.round, result: t(`result.${result}`) }),
        },
        g("replay.chip", { turn: c.round }),
      );
    }),
  );
}

function notAvailable(combats: ReplayCombat[]): VNode | null {
  if (combats.length === 0) return null;
  const turns = combats.map((c) => c.round).join(", ");
  const text = combats.length === 1 ? g("replay.notAvailable.one", { turns }) : g("replay.notAvailable.other", { turns });
  return h("p", { class: "data-note rp-missing" }, sourceMark("unknown"), " ", text);
}

/** The combat a page opens on: the last one with a replay, as in the app. */
export function firstCombat(combats: ReplayCombat[]): ReplayCombat | null {
  return combats.filter((c) => c.replay !== null).at(-1) ?? null;
}

/** The section, or null when no round of the game has a replay. */
export function replaySection(view: GameView): VNode | null {
  if (view.replays.length === 0) return null;
  const ok = view.replays.filter((c) => c.replay !== null);
  const first = firstCombat(view.replays);
  return h(
    "section",
    { class: "data-section rp", id: SECTION_ID, "aria-labelledby": `${SECTION_ID}-title` },
    h("h2", { id: `${SECTION_ID}-title` }, g("replay.title")),
    h("p", { class: "data-note adv" }, g("replay.about")),
    ok.length > 1 && first ? chips(ok, first.round) : null,
    first ? h("div", { class: "rp-holder" }, playerView(first, framesOf(first), 0)) : null,
    notAvailable(view.replays.filter((c) => c.replay === null)),
  );
}

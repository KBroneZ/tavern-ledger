// Combat replays on the game viewer (T-536, D-099) as element trees. Simple
// shows the two boards, play and pause, and the result; Advanced (`.adv`)
// adds the start, step back and forward, the speed and every step in words.
// With reduced motion Play plays without movement, and step back and forward
// show in Simple too (replay.css). Theirs on top, yours below, as in the game. The page's
// script (replayPlayer.ts) moves through the steps; these trees are what it
// draws, so the tests see the same markup.
import { type DrawnMinion, drawnAttrs } from "../art/drawn.ts";
import { t } from "../data/strings.ts";
import { exactStat, shortStat } from "../stat.ts";
import { cls, h, type VNode } from "../view/node.ts";
import { picture, sourceMark } from "../view/pieces.ts";
import { giftList } from "./gifts.ts";
import { minionTip, tipHost } from "./minionTip.ts";
import { cardLabel } from "./names.ts";
import { KEYWORDS, type Keyword } from "./keywords.ts";
import { type Frame, frameWords, type Namer, type Playback, playback, SPEEDS, type Tile } from "./replay.ts";
import type { Side } from "./replayRead.ts";
import type { GameView, HeroRef, ReplayCombat } from "./shape.ts";
import { g } from "./strings.ts";
import { heroLabel, heroTip, keywordWords } from "./words.ts";

const SECTION_ID = "gv-replays";
/** The speed a page starts at. */
export const DEFAULT_SPEED = 1;

/** How a combat's words name its minions and cards (by name from the card data, else the id) and heroes. */
export function namerFor(combat: ReplayCombat): Namer {
  return {
    minion: (ref) => cardLabel(combat.replay?.minions[ref]?.cardId ?? null, g("minion.unknownCard")),
    card: (id) => cardLabel(id, id),
    hero: (id) => heroLabel(id === null ? null : (combat.heroNames.get(id) ?? { id, name: null })),
  };
}

export function framesOf(combat: ReplayCombat): Playback {
  return combat.replay ? playback(combat.replay, combat.heroes) : { frames: [], skipped: 0, grouped: false };
}

export function heroAt(combat: ReplayCombat, frame: Frame, side: Side): HeroRef {
  const id = frame.heroes[side];
  return id === null ? { id: null, name: null } : (combat.heroNames.get(id) ?? { id, name: null });
}

// ---------- A minion ----------

/** The keywords a token wears as a mark, as the game shows them on the board. */
export const MARKED: readonly Keyword[] = ["taunt", "reborn", "venomous", "deathrattle", "windfury", "stealth"];

export function tileClass(tile: Tile): string {
  const m = tile.marks;
  const marks = Object.fromEntries(MARKED.map((k) => [`has-${k}`, tile.keywords.includes(k)]));
  return cls("rp-minion", {
    "is-golden": tile.golden,
    "has-shield": tile.shield,
    ...marks,
    "is-gaining": !!m.gained?.length,
    "is-losing": !!m.lost?.length,
    ...Object.fromEntries(MARKED.map((k) => [`gains-${k}`, !!m.gained?.includes(k)])),
    ...Object.fromEntries(MARKED.map((k) => [`loses-${k}`, !!m.lost?.includes(k)])),
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

/** What a screen reader hears for a minion: "Lantern Moth, 3/3, Golden, Taunt, Divine Shield". Its Dark Gifts follow (giftList). */
export function tileWords(tile: Tile): string {
  // Divine Shield as it is at this step: it can pop, or be given.
  const keywords = KEYWORDS.filter((k) => (k === "divine_shield" ? tile.shield : tile.keywords.includes(k)));
  return [
    cardLabel(tile.cardId, g("minion.unknownCard")),
    g("minion.stats", { attack: exactStat(tile.atk), health: exactStat(Math.max(tile.health, 0)) }),
    tile.golden ? g("minion.golden") : "",
    keywordWords(keywords),
  ].filter(Boolean).join(", ");
}

/** The id of a tile's tooltip: one combat is on the page at a time, so its minion's index is enough. */
const tipId = (tile: Tile): string => `rp-tip-${tile.ref}`;

/** The parts of a tile that change from step to step. Damage past 0 (overkill) shows as 0, as on the game's board. */
export function tileInfo(tile: Tile): VNode[] {
  return [
    h(
      "span",
      { class: "rp-stats", "aria-hidden": "true" },
      h("span", { class: "rp-atk" }, shortStat(tile.atk)),
      h("span", { class: "rp-hp" }, shortStat(Math.max(tile.health, 0))),
    ),
    tile.marks.hurt ? h("span", { class: "rp-hurt", "aria-hidden": "true" }, h("b", {}, `−${shortStat(tile.marks.hurt)}`)) : null,
    // A buff or a heal: a green burst with what it gave, as the game's.
    !tile.marks.hurt && tile.marks.gain ? h("span", { class: "rp-gain", "aria-hidden": "true" }, h("b", {}, `+${shortStat(tile.marks.gain.atk)}/+${shortStat(tile.marks.gain.health)}`)) : null,
    h("span", { class: "visually-hidden" }, tileWords(tile)),
    minionTip(tipId(tile), { cardId: tile.cardId, atk: tile.atk, health: Math.max(tile.health, 0), buffs: tile.buffs }, "start"),
  ].filter((n): n is VNode => n !== null);
}

/**
 * Each keyword's mark, drawn by the stylesheet only while the token has it
 * (its has-* class), so a mark appears and goes with the log's own steps.
 * Drawn as the game draws them, in our own SVG files (D-136).
 */
function keywordMarks(): VNode {
  return h("span", { class: "rp-keys", "aria-hidden": "true" }, MARKED.map((k) => h("i", { class: `rp-k rp-k-${k}` })));
}

/**
 * What the shared drawing needs for a token (T-372): Divine Shield is the one
 * keyword the replay tracks (it pops), the rest as they stand, and a health
 * below 0 (overkill) is drawn as 0, as on the game's board. A minion with no
 * card id has nothing to draw.
 */
export function tileDrawn(tile: Tile): DrawnMinion | null {
  if (!tile.cardId) return null;
  return {
    cardId: tile.cardId,
    atk: tile.atk,
    health: Math.max(tile.health, 0),
    golden: tile.golden,
    keywords: [...tile.keywords.filter((k) => k !== "divine_shield"), ...(tile.shield ? ["divine_shield"] : [])],
  };
}

export function tileView(tile: Tile): VNode {
  return h(
    "li",
    { class: tileClass(tile), "data-ref": tile.ref, ...drawnAttrs(tileDrawn(tile)), ...tipHost(tipId(tile)) },
    keywordMarks(),
    // The slot is where the game's drawing goes once its textures are loaded (art/runtime.ts); empty, it takes no room.
    h(
      "span",
      { class: "rp-art" },
      picture("card", tile.cardId, cardLabel(tile.cardId, g("minion.unknownCard")), 56, true),
      h("span", { class: "tlm-slot" }),
    ),
    h("span", { class: "rp-info" }, ...tileInfo(tile)),
    giftList(tile),
  );
}

export function emptyBoard(): VNode {
  return h("li", { class: "rp-empty" }, g("boards.empty"));
}

// ---------- A side: hero and board ----------

export function heroFace(hero: HeroRef): VNode[] {
  return [picture("hero", hero.id, heroLabel(hero), 40, true), h("span", { class: "rp-hero-name", title: heroTip(hero) }, heroLabel(hero))];
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
  return step === 0 ? g("replay.beforeCombat", { total: last }) : g("replay.stepOf", { n: step, total: last });
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
        replay ? frameWords(f, replay, namer) : "",
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
  const words = combat.replay ? frameWords(frame, combat.replay, namerFor(combat)) : "";
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
    h("p", { class: "rp-caption" }, words),
    // Read on a step or a pause (replayPlayer.ts), never while playing.
    h("p", { class: "visually-hidden rp-live", "aria-live": "polite", "aria-atomic": "true" }),
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

// Combat replay motion (T-560, D-132): how long each step stays at 1×, close
// to the game's own playback, and the geometry of an attack and of a board
// closing a gap or making room. The page plays these with the Web Animations
// API (transform and opacity only); these are the plain numbers behind it.
import { test } from "node:test";
import assert from "node:assert/strict";
import { BEAT_MS, beats, stepDelay } from "../../src/lib/game/replay.ts";
import { ATTACK_MS, type Box, flipDelta, IMPACT_MS, layoutBox, lungeFrames } from "../../src/lib/game/replayMotion.ts";
import type { ReplayEvent } from "../../src/lib/game/replayRead.ts";

const attack: ReplayEvent = { kind: "attack", attacker: 0, defender: 2 };
const damage = (minion: number): ReplayEvent => ({ kind: "damage", minion, amount: 2, health: 1 });
const death = (minion: number): ReplayEvent => ({ kind: "death", minion });

test("pace: one attack lasts about as long as in the game", () => {
  // Measured on the user's 73 combats: about 1.2 s from one attack to the
  // next with no death, 1.3 s with one, 2.1 s with two or more.
  const exchange = (events: ReplayEvent[]) => events.reduce((sum, e) => sum + stepDelay(e, 1), 0);
  const plain = exchange([attack, damage(2), damage(0)]);
  const oneDeath = exchange([attack, damage(2), damage(0), death(2)]);
  const twoDeaths = exchange([attack, damage(2), damage(0), death(2), death(0)]);
  assert.ok(plain >= 1000 && plain <= 1300, `no death: ${plain} ms`);
  assert.ok(oneDeath >= 1200 && oneDeath <= 1600, `one death: ${oneDeath} ms`);
  assert.ok(twoDeaths >= 1500 && twoDeaths <= 2200, `two deaths: ${twoDeaths} ms`);
});

test("pace: the speed divides every step, and the start shows long enough to read the boards", () => {
  assert.equal(stepDelay(null, 1), BEAT_MS.start);
  assert.ok(BEAT_MS.start >= 800);
  assert.equal(stepDelay(attack, 2), BEAT_MS.attack / 2);
  assert.equal(stepDelay(attack, 0.5), BEAT_MS.attack * 2);
  assert.ok(BEAT_MS.hero_damage > BEAT_MS.attack, "the end of the combat holds");
  // A speed the page does not offer is 1×.
  assert.equal(stepDelay(attack, 0), BEAT_MS.attack);
  assert.equal(stepDelay(attack, Number.NaN), BEAT_MS.attack);
});

test("pace: every kind of step has a beat, the attack's lands at the impact", () => {
  for (const kind of ["attack", "damage", "shield", "stats", "death", "summon", "trigger", "hero_damage", "board_in", "unknown"] as const) {
    assert.ok(Number.isFinite(BEAT_MS[kind]) && BEAT_MS[kind] >= 0, kind);
  }
  // The next step (the damage) draws as the attacker hits; it flies back after.
  assert.equal(IMPACT_MS, BEAT_MS.attack);
  assert.ok(ATTACK_MS > IMPACT_MS && ATTACK_MS - IMPACT_MS <= 400);
});

test("attack: the attacker winds up, hits short of the target's centre and comes back", () => {
  const from = { x: 0, y: 100, width: 60, height: 72 };
  const to = { x: 140, y: 0, width: 60, height: 72 };
  const frames = lungeFrames(from, to);
  assert.equal(frames[0].transform, "translate(0px, 0px)");
  assert.equal(frames.at(-1)?.transform, "translate(0px, 0px)");
  const hit = frames.find((f) => f.offset === IMPACT_MS / ATTACK_MS);
  assert.ok(hit, "a frame at the impact");
  const [dx, dy] = /translate\((-?[\d.]+)px, (-?[\d.]+)px\)/.exec(String(hit.transform))!.slice(1).map(Number);
  assert.ok(dx > 0 && dx < 140 && dy < 0 && dy > -100, `towards and short of the target: ${dx}, ${dy}`);
  // The wind-up pulls away from the target.
  const windUp = frames[1];
  const [wx, wy] = /translate\((-?[\d.]+)px, (-?[\d.]+)px\)/.exec(String(windUp.transform))!.slice(1).map(Number);
  assert.ok(wx <= 0 && wy >= 0, `away from the target: ${wx}, ${wy}`);
  // Only transform: compositor-only motion.
  for (const f of frames) assert.deepEqual(Object.keys(f).filter((k) => k !== "offset" && k !== "easing"), ["transform"]);
});

test("board: a minion that moved slides from where it was; one that did not, does not", () => {
  assert.deepEqual(flipDelta({ x: 100, y: 10, width: 60, height: 72 }, { x: 70, y: 10, width: 60, height: 72 }), { dx: 30, dy: 0 });
  assert.equal(flipDelta({ x: 70.2, y: 10, width: 60, height: 72 }, { x: 70, y: 10, width: 60, height: 72 }), null);
});

test("pace: what the log puts between an attack and its damage plays at the impact, not after it", () => {
  const stats: ReplayEvent = { kind: "stats", minion: 0, atk: 9, health: 2 };
  const shield: ReplayEvent = { kind: "shield", minion: 2, on: false };
  const events = [null, attack, stats, shield, damage(2), damage(0), stats];
  const b = beats(events);
  assert.equal(b.length, events.length);
  assert.equal(b[0], BEAT_MS.start);
  assert.equal(b[1], BEAT_MS.attack);
  assert.ok(b[2] <= 80 && b[3] <= 80, `the buff and the shield ride the hit: ${b[2]}, ${b[3]}`);
  assert.equal(b[4], BEAT_MS.damage);
  assert.equal(b[6], BEAT_MS.stats, "after the damage, a buff has its own beat");
  // Without an attack before it, a step keeps its beat.
  assert.deepEqual(beats([null, stats]), [BEAT_MS.start, BEAT_MS.stats]);
});

test("attack: the attacker lands on its target's edge at any distance, a long diagonal included (T-575)", () => {
  const end = (from: Box, to: Box) => {
    const f = lungeFrames(from, to).find((k) => k.offset === IMPACT_MS / ATTACK_MS) as Keyframe;
    const [dx, dy] = /translate\((-?[\d.]+)px, (-?[\d.]+)px\)/.exec(String(f.transform))!.slice(1).map(Number);
    return { x: from.x + from.width / 2 + dx, y: from.y + from.height / 2 + dy };
  };
  const tile = (x: number, y: number, w = 100): Box => ({ x, y, width: w, height: w * 1.2 });
  for (const w of [35, 43, 100]) {
    const row = (y: number) => Array.from({ length: 7 }, (_, i) => tile(i * (w + 14), y, w));
    const theirs = row(0);
    const ours = row(w * 1.2 + 60);
    // From our left-most to their right-most: the longest diagonal.
    for (const [a, b] of [[ours[0], theirs[6]], [ours[6], theirs[0]], [ours[3], theirs[2]], [ours[1], theirs[1]]]) {
      const p = end(a, b);
      const centre = (t: Box) => ({ x: t.x + t.width / 2, y: t.y + t.height / 2 });
      const nearest = [...theirs, ...ours.filter((t) => t !== a)].reduce((best, t) =>
        Math.hypot(centre(t).x - p.x, centre(t).y - p.y) < Math.hypot(centre(best).x - p.x, centre(best).y - p.y) ? t : best);
      assert.equal(nearest, b, `width ${w}: lands on its target, not between two`);
    }
  }
});

test("attack: a board still sliding into place is aimed at where it sits (layoutBox)", () => {
  assert.deepEqual(layoutBox({ x: 130, y: 20, width: 100, height: 120 }, { x: 30, y: 0 }), { x: 100, y: 20, width: 100, height: 120 });
});

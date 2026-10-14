// The combat replay section's markup (T-536): what Simple and Advanced show,
// the words a screen reader hears, and "not available" for a replay the site
// cannot read.
import { test } from "node:test";
import assert from "node:assert/strict";
import { setLang } from "../../src/lib/i18n.ts";
import { readRecord } from "../../src/lib/game/report.ts";
import { framesOf, playerView, replaySection, tileWords } from "../../src/lib/game/replayView.ts";
import { shapeGame } from "../../src/lib/game/shape.ts";
import type { VNode } from "../../src/lib/view/node.ts";
import { duosReport, INDEX, record, SESSION, soloReport } from "./gameFixtures.ts";

type Json = Record<string, unknown>;

function view(report: Json = soloReport()) {
  const result = readRecord(record(report), SESSION, INDEX);
  assert.ok(result.ok);
  return shapeGame(result.record);
}

function* walk(node: VNode | string): Generator<VNode> {
  if (typeof node === "string") return;
  yield node;
  for (const child of node.children) yield* walk(child);
}

const attr = (n: VNode, name: string) => n.attrs.find(([k]) => k === name)?.[1];
const classes = (n: VNode) => attr(n, "class")?.split(" ") ?? [];
const text = (n: VNode | string): string => (typeof n === "string" ? n : n.children.map(text).join(""));
const all = (n: VNode) => [...walk(n)];
const byClass = (n: VNode, c: string) => all(n).filter((x) => classes(x).includes(c));
const act = (n: VNode, name: string) => all(n).find((x) => attr(x, "data-act") === name) as VNode;

test("replay section: a chip per combat with a replay, the last one open at its start", () => {
  const section = replaySection(view()) as VNode;
  assert.equal(attr(section, "id"), "gv-replays");
  assert.ok(!classes(section).includes("adv"), "Simple shows it");
  const chips = byClass(section, "rp-chip");
  assert.deepEqual(chips.map(text), ["Turn 1", "Turn 2"]);
  assert.deepEqual(chips.map((c) => attr(c, "aria-pressed")), ["false", "true"]);
  assert.equal(attr(chips[0], "aria-label"), "Turn 1: Won");
  const player = byClass(section, "rp-player")[0];
  assert.equal(attr(player, "data-round"), "2");
  assert.match(text(byClass(player, "rp-title")[0]), /^Turn 2 against Tavish Stormpike$/);
  assert.equal(text(byClass(player, "rp-counter")[0]), "Step 0 of 24");
  const caption = byClass(player, "rp-caption")[0];
  assert.equal(attr(caption, "aria-live"), undefined, "the visible words are not a live region");
  assert.equal(text(caption), "Both boards as the combat starts.");
  const live = byClass(player, "rp-live")[0];
  assert.deepEqual([attr(live, "aria-live"), attr(live, "aria-atomic"), classes(live).includes("visually-hidden")], ["polite", "true", true]);
  // Theirs on top, yours below, as in the game.
  assert.deepEqual(byClass(player, "rp-side").map((s) => attr(s, "data-side")), ["opponent", "own"]);
  assert.deepEqual(byClass(player, "rp-board").map((b) => attr(b, "aria-label")), ["Their board", "Your board"]);
});

test("replay section: Simple has play and the result; Advanced adds the start, steps, speed and the step list", () => {
  const player = byClass(replaySection(view()) as VNode, "rp-player")[0];
  assert.ok(!classes(act(player, "play")).includes("adv"));
  assert.equal(attr(act(player, "play"), "aria-label"), "Play");
  assert.ok(classes(act(player, "restart")).includes("adv"));
  // Back and forward: Advanced, or Simple with reduced motion (replay.css).
  assert.ok(classes(act(player, "back")).includes("rp-stepper"));
  assert.ok(classes(act(player, "forward")).includes("rp-stepper"));
  const speed = all(player).find((x) => classes(x).includes("rp-speed")) as VNode;
  assert.ok(classes(speed).includes("adv"));
  assert.deepEqual(all(speed).filter((x) => x.tag === "option").map(text), ["0.5×", "1×", "2×", "4×"]);
  const list = byClass(player, "rp-steps")[0];
  assert.ok(classes(list).includes("adv"));
  const steps = byClass(list, "rp-step");
  assert.equal(steps.length, 25);
  assert.equal(attr(steps[0], "aria-current"), "step");
  assert.equal(text(steps[3]), "Their SYN_M_05 attacks your SYN_M_02");
  assert.match(text(byClass(player, "rp-result")[0]), /^Lost inferred$/, "an inferred result is marked");
  assert.equal(byClass(player, "rp-reduced").length, 1, "the reduced-motion hint is there for the CSS to show");
  for (const n of all(player)) for (const [name] of n.attrs) assert.doesNotMatch(name, /^on/i);
});

test("replay section: each minion's picture, stats, marks and words; Dark Gifts as the viewer draws them", () => {
  const v = view();
  const combat = v.replays[1];
  const pb = framesOf(combat);
  const start = playerView(combat, pb, 0);
  const tiles = byClass(start, "rp-minion");
  assert.equal(tiles.length, 4);
  const golden = tiles[3];
  assert.ok(classes(golden).includes("is-golden"));
  assert.ok(classes(golden).includes("has-shield"));
  assert.equal(attr(golden, "data-ref"), "1");
  assert.equal(text(byClass(golden, "rp-stats")[0]), "33");
  assert.equal(text(byClass(golden, "visually-hidden")[0]), "SYN_M_02, 3/3, Golden, Taunt, Divine Shield");
  const gift = byClass(golden, "gv-gift")[0];
  assert.match(attr(gift, "title") ?? "", /^Dark Gift: SYN_GIFT_1/);
  assert.ok(byClass(tiles[1], "gv-gift")[0] && classes(byClass(tiles[1], "gv-gift")[0]).includes("is-unknown"), "a gift the log does not name");
  assert.ok(all(start).some((x) => x.tag === "img"), "card pictures");
  // Step 4: the shield is gone; step 5: hurt and marked; overkill draws 0.
  const popped = byClass(playerView(combat, pb, 4), "rp-minion")[3];
  assert.ok(classes(popped).includes("is-popped"));
  assert.ok(!classes(popped).includes("has-shield"));
  assert.equal(tileWords(pb.frames[4].own[1]), "SYN_M_02, 3/3, Golden, Taunt");
  const lethal = playerView(combat, pb, 7);
  const hurt = byClass(lethal, "rp-minion").find((x) => attr(x, "data-ref") === "3") as VNode;
  assert.equal(text(byClass(hurt, "rp-hurt")[0]), "−3");
  assert.equal(text(byClass(hurt, "rp-hp")[0]), "0", "overkill shows as 0");
  // The end: your hero's damage, in words too.
  const end = playerView(combat, pb, 24);
  const own = byClass(end, "rp-side").find((s) => attr(s, "data-side") === "own") as VNode;
  assert.match(text(byClass(own, "rp-hit")[0]), /^−5Hero damage: 5$/);
  assert.equal(text(byClass(own, "rp-empty")[0]), "Empty board");
});

test("replay section: malformed replays are not available, in English and Spanish", () => {
  const report = soloReport();
  const rounds = report.rounds as Json[];
  rounds[1].replay = { minions: [], start: 0, events: [{ kind: "attack", attacker: 0, defender: null }], complete: true };
  rounds[2].replay = "broken";
  const section = replaySection(view(report)) as VNode;
  assert.equal(byClass(section, "rp-chip").length, 0, "one playable combat: no picker");
  assert.equal(attr(byClass(section, "rp-player")[0], "data-round"), "1");
  assert.match(text(byClass(section, "rp-missing")[0]), /Replays not available for turns 2, 3: this site can't read them\.$/);
  setLang("es");
  try {
    const es = replaySection(view(report)) as VNode;
    assert.match(text(byClass(es, "rp-missing")[0]), /Repeticiones no disponibles para los turnos 2, 3: la web no puede leerlas\.$/);
    assert.equal(text(byClass(es, "rp-counter")[0]), "Paso 0 de 5");
  } finally {
    setLang("en");
  }
  // Only malformed ones: the note and no player.
  rounds[0].replay = 7;
  const none = replaySection(view(report)) as VNode;
  assert.equal(byClass(none, "rp-player").length, 0);
  assert.match(text(byClass(none, "rp-missing")[0]), /turns 1, 2, 3/);
});

test("replay section: none when no round has a replay (older games, Duos copies the game did not show)", () => {
  assert.equal(replaySection(view(duosReport())), null);
  const report = soloReport();
  for (const r of report.rounds as Json[]) delete r.replay;
  assert.equal(replaySection(view(report)), null);
});

test("replay section: a cut replay and left-out steps say so", () => {
  const report = soloReport();
  const replay = (report.rounds as Json[])[1].replay as Json & { events: Json[] };
  replay.complete = false;
  // A kind this site does not know, and a step on a minion that died earlier.
  replay.events.push({ kind: "teleport" }, { kind: "stats", minion: 0, atk: 1, health: 1 });
  const player = byClass(replaySection(view(report)) as VNode, "rp-player")[0];
  const notes = byClass(player, "data-note").map(text);
  assert.ok(notes.some((n) => n.endsWith("This replay stops before the combat ends: the log was cut short, or the combat was longer than the app keeps.")));
  assert.ok(notes.some((n) => n.endsWith("2 steps could not be played and are left out.")), notes.join(" | "));
});

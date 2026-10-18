// Combat replays from parser revision 13 (T-575): steps grouped by the log's
// beats, each damage's source in words, and keywords given or used
// mid-combat. Every replay here is synthetic.
import { test } from "node:test";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import assert from "node:assert/strict";
import { setLang } from "../../src/lib/i18n.ts";
import { BEAT_MS, frameBeats, frameWords, groupBeat, playback, steps, type Frame, type Namer } from "../../src/lib/game/replay.ts";
import { readReplay, type Replay } from "../../src/lib/game/replayRead.ts";
import { tileClass } from "../../src/lib/game/replayView.ts";

type Json = Record<string, unknown>;

const m = (side: string, card_id: string, position: number, keywords: string[] = []): Json => ({
  side,
  card_id,
  atk: 2,
  health: 3,
  position,
  golden: false,
  keywords,
});

/**
 * A start-of-combat buff to the whole board (one beat), an attack whose
 * Rally gives Venomous (its own beat), the hit that uses it up, two deaths
 * at once, and damage from a hero power and from nowhere the log names.
 */
function rev13(): Json {
  return {
    minions: [
      m("own", "OWN_RALLY", 1),
      m("own", "OWN_MATE", 2),
      m("opponent", "FOE_A", 1, ["taunt"]),
      m("opponent", "FOE_B", 2),
    ],
    start: 4,
    events: [
      { kind: "trigger", trigger: "start_of_combat", side: "own", card_id: "OWN_MATE", source: 1 },
      { kind: "stats", minion: 0, atk: 4, health: 5 },
      { kind: "stats", minion: 1, atk: 4, health: 5 },
      { kind: "attack", attacker: 0, defender: 2 },
      { kind: "trigger", trigger: "rally", side: "own", card_id: "OWN_RALLY", source: 0 },
      { kind: "keyword", minion: 0, keyword: "venomous", on: true },
      { kind: "keyword", minion: 1, keyword: "venomous", on: true },
      { kind: "damage", minion: 2, amount: 4, health: -1, source: { how: "attack", minion: 0 } },
      { kind: "damage", minion: 0, amount: 2, health: 3, source: { how: "attack", minion: 2 } },
      { kind: "keyword", minion: 0, keyword: "venomous", on: false },
      { kind: "death", minion: 2 },
      { kind: "damage", minion: 3, amount: 1, health: 2, source: { how: "hero_power", card_id: "FOE_POWER", side: "opponent" } },
      { kind: "damage", minion: 3, amount: 1, health: 1, source: null },
    ],
    beats: [3, 1, 3, 3, 1, 1, 1],
    complete: true,
  };
}

function read(v: unknown): Replay {
  const r = readReplay(v);
  assert.ok(r && r.ok, "the replay is read");
  return r.replay;
}

const namer: Namer = { minion: (ref) => ["Rally Murloc", "Mate", "Foe A", "Foe B"][ref] ?? "?", card: (id) => (id === "FOE_POWER" ? "Their Power" : id), hero: (id) => id ?? "?" };
const NO_HEROES = { own: null, opponent: null };
const cards = (f: Frame, side: "own" | "opponent") => f[side].map((t) => t.cardId);

test("revision 13: a beat plays at once; an attack is a step of its own", () => {
  const r = read(rev13());
  assert.deepEqual(
    steps(r).map((s) => s.map((e) => e.kind)),
    [["trigger", "stats", "stats"], ["attack"], ["trigger", "keyword", "keyword"], ["damage", "damage", "keyword"], ["death"], ["damage"], ["damage"]],
  );
  const f = playback(r, NO_HEROES).frames;
  assert.equal(f.length, 8);
  // The whole board's buff shows on the same step.
  assert.deepEqual([f[1].own[0].marks.buffed, f[1].own[1].marks.buffed], [true, true]);
  assert.equal(f[1].own[1].marks.triggered, true);
  // The Rally gives Venomous to both at once; the hit uses the attacker's up.
  assert.deepEqual(f[3].own.map((t) => t.marks.gained), [["venomous"], ["venomous"]]);
  assert.deepEqual(f[3].own.map((t) => t.keywords), [["venomous"], ["venomous"]]);
  assert.deepEqual(f[4].own[0].marks.lost, ["venomous"]);
  assert.deepEqual(f[4].own[0].keywords, []);
  assert.deepEqual(f[4].own[1].keywords, ["venomous"], "the other keeps it");
  assert.deepEqual([f[4].opponent[0].marks.hurt, f[4].own[0].marks.hurt], [4, 2], "both hits land together");
  assert.deepEqual(cards(f[6], "opponent"), ["FOE_B"], "the dead one left");
});

test("an older record (no beats) plays one event per step, as before", () => {
  const old = rev13();
  delete old.beats;
  const r = read(old);
  assert.equal(r.beats, null);
  const pb = playback(r, NO_HEROES);
  assert.equal(pb.grouped, false);
  assert.equal(pb.frames.length, r.events.length + 1);
  assert.ok(pb.frames.every((f) => f.events.length <= 1));
});

test("a beat lasts each kind of step once, damage for up to two minions; one event keeps its own beat (D-132)", () => {
  const r = read(rev13());
  const pb = playback(r, NO_HEROES);
  const ms = frameBeats(pb);
  assert.equal(ms[0], BEAT_MS.start);
  assert.equal(ms[1], BEAT_MS.trigger + BEAT_MS.stats, "the board's buff: one trigger and one stats beat, not one per minion");
  assert.equal(ms[2], BEAT_MS.attack);
  assert.equal(ms[3], BEAT_MS.trigger + BEAT_MS.keyword);
  assert.equal(ms[4], 2 * BEAT_MS.damage + BEAT_MS.keyword, "the hit on both, as two damages did before");
  assert.equal(ms[5], BEAT_MS.death);
  assert.equal(groupBeat([{ kind: "death", minion: 0 }, { kind: "death", minion: 1 }, { kind: "death", minion: 2 }]), BEAT_MS.death);
  // Before: the trigger and seven buffs took 480 + 7 × 220 ms.
  const seven = Array.from({ length: 7 }, (_, i) => ({ kind: "stats" as const, minion: i, atk: 1, health: 1 }));
  assert.equal(groupBeat([{ kind: "trigger", trigger: "start_of_combat", side: "own", cardId: "X", source: null }, ...seven]), 700);
});

test("each damage says where it came from, in English and Spanish", () => {
  const r = read(rev13());
  const pb = playback(r, NO_HEROES);
  const words = (lang: "en" | "es") => {
    setLang(lang);
    const out = pb.frames.map((f) => frameWords(f, r, namer));
    setLang("en");
    return out;
  };
  const en = words("en");
  assert.equal(en[3], "Rally: your Rally Murloc. Your Rally Murloc gets Venomous. Your Mate gets Venomous");
  assert.equal(en[4], "Their Foe A takes 4 damage from your Rally Murloc (attack). Your Rally Murloc takes 2 damage from their Foe A (attack; 3 health left). Your Rally Murloc loses Venomous");
  assert.equal(en[6], "Their Foe B takes 1 damage from their Their Power (hero power; 2 health left)");
  assert.equal(en[7], "Their Foe B takes 1 damage from an unknown source (1 health left)");
  const es = words("es");
  assert.match(es[4], /^Foe A rival recibe 4 de daño de tu Rally Murloc \(ataque\)\. Tu Rally Murloc recibe 2 de daño de Foe A rival \(ataque; vida restante: 3\)\. Tu Rally Murloc pierde Ponzoña$/);
  assert.equal(es[7], "Foe B rival recibe 1 de daño de origen desconocido (vida restante: 1)");
  assert.equal(es[3], "Alentar: tu Rally Murloc. Tu Rally Murloc obtiene Ponzoña. Tu Mate obtiene Ponzoña");
});

test("a damage from an older record (no source) keeps its old words", () => {
  const old = rev13();
  for (const e of old.events as Json[]) if (e.kind === "damage") delete e.source;
  delete old.beats;
  const r = read(old);
  const f = playback(r, NO_HEROES).frames.find((x) => x.event?.kind === "damage") as Frame;
  assert.equal(frameWords(f, r, namer), "Their Foe A takes 4 damage");
});

test("a token wears its keywords as marks, and a mark appearing or going is marked on its step", () => {
  const f = playback(read(rev13()), NO_HEROES).frames;
  assert.match(tileClass(f[0].opponent[0]), /\bhas-taunt\b/);
  assert.match(tileClass(f[3].own[0]), /\bhas-venomous\b.*\bis-gaining\b/);
  assert.doesNotMatch(tileClass(f[4].own[0]), /\bhas-venomous\b/);
  assert.match(tileClass(f[4].own[0]), /\bis-losing\b/);
});

test("revision 13 fields are read strictly", () => {
  const bad = (change: (r: Json) => void) => {
    const r = rev13();
    change(r);
    const read = readReplay(r);
    return read !== null && !read.ok;
  };
  assert.ok(bad((r) => (r.beats = [3, 1])), "beats that do not add up to the events");
  assert.ok(bad((r) => (r.beats = [0, 13])), "an empty beat");
  assert.ok(bad((r) => ((r.events as Json[])[7].source = { how: 7, minion: 0 })), "a cause that is not a word");
  assert.ok(bad((r) => ((r.events as Json[])[7].source = { how: "attack", minion: 99 })), "a minion not in the replay");
  assert.ok(bad((r) => ((r.events as Json[])[11].source = { how: "trinket", card_id: "X" })), "a card with no side");
  assert.ok(bad((r) => ((r.events as Json[])[5].on = "yes")), "a keyword step that is neither on nor off");
  // A keyword a newer app writes is a step this site leaves out, not a broken replay.
  const newer = rev13();
  (newer.events as Json[])[5].keyword = "magnetic";
  const r = read(newer);
  assert.deepEqual(r.events[5], { kind: "unknown" });
  assert.equal(playback(r, NO_HEROES).skipped, 1);
});

test("before the first attack, start-of-combat buffs from the whole table play as one step", () => {
  const sot = (side: string, source: number) => ({ kind: "trigger", trigger: "start_of_combat", side, card_id: "SOT", source });
  const r = read({
    minions: [m("own", "A", 1), m("own", "B", 2), m("opponent", "C", 1), m("opponent", "D", 2)],
    start: 4,
    events: [
      sot("own", 0),
      { kind: "stats", minion: 0, atk: 3, health: 4 },
      sot("opponent", 2),
      { kind: "stats", minion: 2, atk: 3, health: 4 },
      { kind: "stats", minion: 3, atk: 3, health: 4 },
      // A start-of-combat hit is a step of its own.
      sot("opponent", 3),
      { kind: "damage", minion: 1, amount: 1, health: 2, source: { how: "start_of_combat", minion: 3 } },
      { kind: "attack", attacker: 0, defender: 2 },
      // After the first attack, each effect keeps its own step.
      sot("own", 1),
      { kind: "stats", minion: 1, atk: 5, health: 5 },
      sot("own", 0),
      { kind: "stats", minion: 0, atk: 5, health: 5 },
    ],
    beats: [2, 3, 2, 1, 2, 2],
    complete: true,
  });
  assert.deepEqual(steps(r).map((s) => s.length), [5, 2, 1, 2, 2]);
  const f = playback(r, NO_HEROES).frames;
  assert.deepEqual([...f[1].own, ...f[1].opponent].map((t) => !!t.marks.buffed), [true, false, true, true]);
});

test("a cause a newer app names is an unknown source, not a broken replay", () => {
  const r = rev13();
  (r.events as Json[])[7].source = { how: "magic", minion: 0 };
  const read = readReplay(r);
  assert.ok(read && read.ok);
  assert.deepEqual(read.replay.events[7], { kind: "damage", amount: 4, health: -1, minion: 2, source: null });
});

test("a buff or a heal marks what it gave; a loss marks nothing", () => {
  const f = playback(read(rev13()), NO_HEROES).frames;
  // The opening buff: 2/3 to 4/5 on both.
  assert.deepEqual(f[1].own.map((t) => t.marks.gain), [{ atk: 2, health: 2 }, { atk: 2, health: 2 }]);
  const r = read({ ...rev13(), events: [{ kind: "stats", minion: 0, atk: 1, health: 9 }], beats: [1] });
  assert.equal(playback(r, NO_HEROES).frames[1].own[0].marks.gain, undefined, "Attack went down: no green burst");
});

test("every token has the same box: what it carries sits out of the layout (T-575)", () => {
  const css = readFileSync(join(import.meta.dirname, "..", "..", "src", "components", "game", "replay.css"), "utf8").replace(/\r\n/g, "\n");
  const rule = (selector: string) => {
    const at = css.indexOf(`\n${selector} {`);
    assert.ok(at >= 0, selector);
    return css.slice(at, css.indexOf("}", at));
  };
  for (const selector of [".rp-minion .gv-gifts", ".rp-keys", ".rp-info", ".rp-k"]) assert.match(rule(selector), /position: absolute/, selector);
  assert.match(css, /\.rp-hurt,\n\.rp-gain \{\n  position: absolute;/);
  assert.match(rule(".rp-minion"), /width: var\(--rp-tile\)/);
});

// Sin'dorei Straight Shot's Rally takes Taunt (and Reborn) from what it
// attacks: from the owner's game of 2026-10-11, turn 11 (synthetic copy).
function untaunt(): Json {
  return {
    minions: [m("own", "BG25_016", 1), m("opponent", "BG26_149", 1, ["taunt", "divine_shield"]), m("opponent", "BG26_152", 2, ["taunt", "divine_shield"])],
    start: 3,
    events: [
      { kind: "attack", attacker: 0, defender: 1 },
      { kind: "trigger", trigger: "rally", side: "own", card_id: "BG25_016", source: 0 },
      { kind: "keyword", minion: 1, keyword: "taunt", on: false },
      { kind: "shield", minion: 1, on: false },
    ],
    beats: [1, 2, 1],
    complete: true,
  };
}

test("a Taunt taken mid-combat drops its shield on that step (T-581)", () => {
  const f = playback(read(untaunt()), NO_HEROES).frames;
  assert.match(tileClass(f[1].opponent[0]), /\bhas-taunt\b/, "before the Rally");
  const after = tileClass(f[2].opponent[0]);
  assert.doesNotMatch(after, /\bhas-taunt\b/);
  assert.match(after, /\bloses-taunt\b/);
  assert.match(tileClass(f[2].opponent[1]), /\bhas-taunt\b/, "the other keeps it");
  assert.doesNotMatch(tileClass(f[3].opponent[0]), /\b(has|loses)-taunt\b/, "gone for good");
});

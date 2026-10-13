// Combat replays on the game viewer (T-536): the replay reader (untrusted
// input, D-079), the step model ported from the app (crates/desktop/ui/
// replay.js) and each step in words.
import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { setLang } from "../../src/lib/i18n.ts";
import { control, keyAction, playback, stepWords, type Frame, type Tile } from "../../src/lib/game/replay.ts";
import { MAX_REPLAY_EVENTS, MAX_REPLAY_MINIONS, readReplay, type Replay } from "../../src/lib/game/replayRead.ts";
import { readRecord } from "../../src/lib/game/report.ts";
import { shapeGame } from "../../src/lib/game/shape.ts";
import { INDEX, longReplay, record, SESSION, shortReplay, soloReport } from "./gameFixtures.ts";

type Json = Record<string, unknown>;

function good(value: unknown): Replay {
  const read = readReplay(value);
  assert.ok(read && read.ok, "the replay is read");
  return read.replay;
}

/** A copy of the long replay with `change` applied to it. */
function edited(change: (r: Json & { minions: Json[]; events: Json[] }) => void): unknown {
  const r = structuredClone(longReplay()) as Json & { minions: Json[]; events: Json[] };
  change(r);
  return r;
}

const cards = (board: Tile[]) => board.map((t) => t.cardId);
const NO_HEROES = { own: null, opponent: null };

test("replay reader: a good replay is read as typed data", () => {
  const r = good(longReplay());
  assert.equal(r.start, 4);
  assert.equal(r.complete, true);
  assert.equal(r.minions.length, 5);
  assert.deepEqual(r.minions[1], {
    side: "own",
    cardId: "SYN_M_02",
    atk: 3,
    health: 3,
    position: 2,
    golden: true,
    keywords: ["taunt", "divine_shield"],
    gifts: ["SYN_GIFT_1"],
    giftUnknown: false,
  });
  assert.equal(r.minions[3].cardId, null, "a minion the log does not name");
  assert.equal(r.minions[3].giftUnknown, true);
  assert.equal(r.events.length, 24);
  assert.deepEqual(r.events[0], { kind: "trigger", trigger: "start_of_combat", side: "opponent", cardId: "SYN_TRINKET_1", source: null });
  assert.deepEqual(r.events[2], { kind: "attack", attacker: 2, defender: 1 });
});

test("replay reader: no replay is null, never 'not available'", () => {
  assert.equal(readReplay(undefined), null);
  assert.equal(readReplay(null), null);
});

test("replay reader: keys it does not know are ignored, unknown keywords dropped", () => {
  const r = good(edited((x) => {
    x.extra = 1;
    x.minions[0].shiny = true;
    x.minions[0].keywords = ["deathrattle", "magnetic", "deathrattle"];
    x.events[0].note = "x";
  }));
  assert.deepEqual(r.minions[0].keywords, ["deathrattle"]);
  assert.equal(r.events[0].kind, "trigger");
});

test("replay reader: a kind of step this site does not know is kept as unknown (a newer app)", () => {
  const r = good(edited((x) => {
    x.events.splice(1, 0, { kind: "magnetize", minion: 0 }, { kind: "trigger", trigger: "spellcraft", side: "own", card_id: "SYN_X", source: null });
  }));
  assert.deepEqual(r.events[1], { kind: "unknown" });
  assert.deepEqual(r.events[2], { kind: "unknown" });
  const p = playback(r, NO_HEROES);
  assert.equal(p.skipped, 2, "left out and counted, as in the app");
  assert.equal(p.frames.length, r.events.length - 2 + 1);
});

test("replay reader: anything malformed makes the whole replay not available", () => {
  const cases: [string, unknown][] = [
    ["not an object", "replay"],
    ["a list", []],
    ["minions not a list", edited((x) => (x.minions = {} as never))],
    ["events not a list", edited((x) => (x.events = "x" as never))],
    ["no start", edited((x) => delete x.start)],
    ["start past the minions", edited((x) => (x.start = 6))],
    ["start negative", edited((x) => (x.start = -1))],
    ["complete missing", edited((x) => delete x.complete)],
    ["complete not a boolean", edited((x) => (x.complete = "yes"))],
    ["minion not an object", edited((x) => (x.minions[0] = 7 as never))],
    ["minion side", edited((x) => (x.minions[0].side = "teammate"))],
    ["minion card id", edited((x) => (x.minions[0].card_id = "<img src=x>"))],
    ["minion attack not whole", edited((x) => (x.minions[0].atk = 1.5))],
    ["minion health missing", edited((x) => delete x.minions[0].health)],
    ["minion health huge", edited((x) => (x.minions[0].health = 2 ** 40))],
    ["minion slot", edited((x) => (x.minions[0].position = 65))],
    ["minion golden", edited((x) => (x.minions[0].golden = 1))],
    ["minion keywords", edited((x) => (x.minions[0].keywords = "taunt"))],
    ["minion keyword not text", edited((x) => (x.minions[0].keywords = [3]))],
    ["minion gifts", edited((x) => (x.minions[0].gifts = ["A", "B", "C", "D", "E"]))],
    ["minion gift id", edited((x) => (x.minions[0].gifts = ["not an id"]))],
    ["minion gift unknown", edited((x) => (x.minions[0].gift_unknown = "yes"))],
    ["event not an object", edited((x) => (x.events[0] = null as never))],
    ["event kind missing", edited((x) => delete x.events[0].kind)],
    ["event kind not text", edited((x) => (x.events[0].kind = 4))],
    ["attacker out of range", edited((x) => (x.events[2].attacker = 5))],
    ["attacker not whole", edited((x) => (x.events[2].attacker = 0.5))],
    ["defender missing", edited((x) => delete x.events[2].defender)],
    ["a minion named before its summon", edited((x) => (x.events[2].defender = 4))],
    ["a summon of a starting minion", edited((x) => (x.events[9].minion = 0))],
    ["a summon twice", edited((x) => x.events.push({ kind: "summon", minion: 4 }))],
    ["damage of 0", edited((x) => (x.events[4].amount = 0))],
    ["damage left health missing", edited((x) => delete x.events[4].health)],
    ["shield not a boolean", edited((x) => (x.events[3].on = "off"))],
    ["stats not whole", edited((x) => (x.events[1].atk = "6"))],
    ["trigger with no card", edited((x) => (x.events[0].card_id = null))],
    ["trigger side", edited((x) => (x.events[0].side = "both"))],
    ["trigger source not on any board", edited((x) => (x.events[8].source = 99))],
    ["hero damage of 0", edited((x) => (x.events[23].amount = 0))],
    ["hero damage past any health", edited((x) => (x.events[23].amount = 100_001))],
    ["board in, bad hero", edited((x) => x.events.push({ kind: "board_in", side: "own", hero: "a b" }))],
    ["board in, bad side", edited((x) => x.events.push({ kind: "board_in", side: "middle", hero: null }))],
  ];
  for (const [name, value] of cases) assert.deepEqual(readReplay(value), { ok: false }, name);
});

test("replay reader: an oversized replay is not available, and only the bound is looked at", () => {
  const minions = Array.from({ length: MAX_REPLAY_MINIONS + 1 }, (_, i) => ({ side: "own", card_id: "SYN_M_01", atk: 1, health: 1, position: (i % 7) + 1, golden: false, keywords: [] }));
  assert.deepEqual(readReplay({ minions, start: 0, events: [], complete: true }), { ok: false }, "too many minions");
  const events = Array.from({ length: MAX_REPLAY_EVENTS + 1 }, () => ({ kind: "attack", attacker: 0, defender: null }));
  assert.deepEqual(readReplay({ ...shortReplay(), events }), { ok: false }, "too many steps");
  // At the bound it is read in full.
  const most = good({ ...shortReplay(), events: events.slice(0, MAX_REPLAY_EVENTS) });
  assert.equal(most.events.length, MAX_REPLAY_EVENTS);
  // A huge list is refused on its length, before any step is read.
  const huge = new Proxy([] as unknown[], {
    get(target, key) {
      if (key === "length") return 10_000_000;
      if (typeof key === "string" && /^\d+$/.test(key)) throw new Error("read a step of an oversized list");
      return Reflect.get(target, key);
    },
  });
  assert.deepEqual(readReplay({ ...shortReplay(), events: huge }), { ok: false });
});

test("record reader: a malformed replay is one unreadable value; the rest of the game is read", () => {
  const report = soloReport();
  const rounds = report.rounds as Json[];
  rounds[1].replay = edited((x) => (x.events[2].attacker = 99));
  const result = readRecord(record(report), SESSION, INDEX);
  assert.ok(result.ok);
  assert.equal(result.record.unreadable, 1);
  const [r1, r2, r3] = result.record.report.rounds;
  assert.equal(r1.replay?.ok, true);
  assert.deepEqual(r2.replay, { ok: false });
  assert.equal(r3.replay, null, "a round without a replay");
  assert.equal(r2.entries.length, 2, "the round itself is still read");
});

test("step model: the start, then one board state per step (the app's rule)", () => {
  const p = playback(good(longReplay()), { own: "BG22_HERO_002", opponent: "BG22_HERO_000" });
  const f = p.frames;
  assert.equal(p.skipped, 0);
  assert.equal(f.length, 25);
  assert.equal(f[0].event, null);
  assert.deepEqual(cards(f[0].own), ["SYN_M_01", "SYN_M_02"]);
  assert.deepEqual(cards(f[0].opponent), ["SYN_M_05", null]);
  assert.equal(f[0].own[1].shield, true);
  assert.equal(f[0].own[1].golden, true);
  assert.deepEqual(f[0].own[1].gifts, ["SYN_GIFT_1"], "Dark Gifts as the viewer draws them");
  assert.equal(f[0].opponent[1].giftUnknown, true);
  assert.deepEqual(f[0].heroes, { own: "BG22_HERO_002", opponent: "BG22_HERO_000" });
  // A trigger from a trinket marks no minion; a buff does.
  assert.deepEqual(f[1].opponent.map((t) => t.marks), [{}, {}]);
  assert.deepEqual([f[2].opponent[0].atk, f[2].opponent[0].health, f[2].opponent[0].marks.buffed], [6, 9, true]);
  assert.equal(f[3].opponent[0].marks.attacker, true);
  assert.equal(f[3].own[1].marks.defender, true);
  assert.deepEqual([f[4].own[1].shield, f[4].own[1].marks.popped], [false, true]);
  assert.deepEqual([f[5].opponent[0].health, f[5].opponent[0].marks.hurt], [6, 3]);
  // A death shows on its step, then the minion leaves the board.
  assert.equal(f[8].opponent[1].marks.dying, true);
  assert.equal(f[9].opponent.length, 1);
  assert.equal(f[9].opponent[0].marks.triggered, undefined, "the dead source is gone: nothing to mark");
  // A summon goes to its slot.
  assert.deepEqual(cards(f[10].opponent), ["SYN_M_05", "SYN_M_09"]);
  assert.equal(f[10].opponent[1].marks.summoned, true);
  assert.deepEqual(f[24].hero, { own: 5, opponent: 0 });
  assert.equal(f[23].hero.own, 0, "earlier frames are not changed");
  assert.equal(f[0].own[1].health, 3, "earlier frames are not changed");
});

test("step model: a step naming a minion no longer on the board is left out and counted", () => {
  const r = good(edited((x) => {
    x.events = [
      { kind: "death", minion: 2 },
      { kind: "attack", attacker: 2, defender: 0 }, // dead already
      { kind: "damage", minion: 2, amount: 1, health: 1 },
      { kind: "summon", minion: 4 },
      { kind: "board_in", side: "opponent", hero: "SYN_HERO_D" },
    ];
  }));
  const p = playback(r, NO_HEROES);
  assert.equal(p.skipped, 2);
  assert.equal(p.frames.length, 4);
  assert.equal(p.frames[3].heroes.opponent, "SYN_HERO_D", "a Duos board brings its hero");
  assert.equal(p.frames[3].heroes.own, null);
});

test("step model: overkill shows as the log's health, the page draws 0", () => {
  const f = playback(good(shortReplay()), NO_HEROES).frames;
  assert.equal(f[2].opponent[0].health, -1);
  assert.equal(f[2].opponent[0].marks.hurt, 2);
});

function wordsOf(lang: "en" | "es"): string[] {
  setLang(lang);
  const r = good(longReplay());
  const namer = { minion: (ref: number) => r.minions[ref].cardId ?? "?", card: (id: string) => id, hero: (id: string | null) => `Hero ${id ?? "?"}` };
  const all = [null, ...r.events].map((e) => stepWords(e, r, namer));
  setLang("en");
  return all;
}

test("each step in words, in English", () => {
  const words = wordsOf("en");
  assert.deepEqual(words.slice(0, 11), [
    "Both boards as the combat starts.",
    "Start of combat: their SYN_TRINKET_1",
    "Their SYN_M_05 is now 6/9",
    "Their SYN_M_05 attacks your SYN_M_02",
    "Your SYN_M_02 loses Divine Shield",
    "Their SYN_M_05 takes 3 damage (6 health left)",
    "Your SYN_M_02 attacks their ?",
    "Their ? takes 3 damage",
    "Their ? dies",
    "Deathrattle: their ?",
    "Their SYN_M_09 is summoned",
  ]);
  assert.equal(words.at(-1), "Your hero takes 5 damage");
});

test("each step in words, in Spanish", () => {
  const words = wordsOf("es");
  assert.deepEqual(words.slice(0, 11), [
    "Los dos tableros al empezar el combate.",
    "Inicio del combate: SYN_TRINKET_1 rival",
    "SYN_M_05 rival pasa a 6/9",
    "SYN_M_05 rival ataca a tu SYN_M_02",
    "Tu SYN_M_02 pierde el Escudo divino",
    "SYN_M_05 rival recibe 3 de daño (vida restante: 6)",
    "Tu SYN_M_02 ataca a ? rival",
    "? rival recibe 3 de daño",
    "? rival muere",
    "Último aliento: ? rival",
    "Se invoca SYN_M_09 rival",
  ]);
  assert.equal(words.at(-1), "Tu héroe recibe 5 de daño");
});

test("the other steps in words", () => {
  setLang("en");
  const r = good(longReplay());
  const namer = { minion: (ref: number) => r.minions[ref].cardId ?? "?", card: (id: string) => id, hero: (id: string | null) => (id ? `Hero ${id}` : "Unknown hero") };
  const more: [unknown, string][] = [
    [{ kind: "attack", attacker: 2, defender: null }, "Their SYN_M_05 attacks"],
    [{ kind: "shield", minion: 1, on: true }, "Your SYN_M_02 gets Divine Shield"],
    [{ kind: "trigger", trigger: "avenge", side: "own", cardId: "SYN_M_01", source: 0 }, "Avenge: your SYN_M_01"],
    [{ kind: "trigger", trigger: "rally", side: "own", cardId: "SYN_M_01", source: 0 }, "Rally: your SYN_M_01"],
    [{ kind: "hero_damage", side: "opponent", amount: 7 }, "Their hero takes 7 damage"],
    [{ kind: "board_in", side: "own", hero: "MATE" }, "Your team's next board comes in: Hero MATE"],
    [{ kind: "board_in", side: "opponent", hero: null }, "Their team's next board comes in: Unknown hero"],
    [{ kind: "unknown" }, "A step this site cannot read"],
  ];
  for (const [e, text] of more) assert.equal(stepWords(e as never, r, namer), text);
});

test("game view: each round with a replay, its result and opponent; malformed ones not available", () => {
  const report = soloReport();
  (report.rounds as Json[])[2].replay = { minions: "broken" };
  const result = readRecord(record(report), SESSION, INDEX);
  assert.ok(result.ok);
  const view = shapeGame(result.record);
  assert.deepEqual(view.replays.map((c) => [c.round, c.result, c.inferred, c.replay !== null]), [
    [1, "win", false, true],
    [2, "loss", true, true],
    [3, "unknown", false, false],
  ]);
  assert.equal(view.replays[1].opponents[0].name, "Tavish Stormpike");
  assert.equal(view.replays[1].heroes.own, "BG22_HERO_002");
  assert.equal(view.replays[1].heroes.opponent, "BG22_HERO_000");
  // A round without a replay shows nothing new.
  delete (report.rounds as Json[])[2].replay;
  const again = readRecord(record(report), SESSION, INDEX);
  assert.ok(again.ok);
  assert.deepEqual(shapeGame(again.record).replays.map((c) => c.round), [1, 2]);
});

test("every replay of the user's real games is read and plays to the end", () => {
  const dir = join(import.meta.dirname, "..", "..", "..", "crates", "bg-parser", "tests", "data", "real");
  let played = 0;
  for (const file of ["b253216_solo.json", "b253216_duos.json", "b253216_duos_gifts.json"]) {
    const games = JSON.parse(readFileSync(join(dir, file), "utf8")) as Json[];
    for (const game of games) {
      for (const round of game.rounds as Json[]) {
        if (round.replay === undefined || round.replay === null) continue;
        const r = good(round.replay);
        const p = playback(r, NO_HEROES);
        const where = `${file} turn ${String(round.number)}`;
        assert.equal(p.skipped, 0, where);
        assert.equal(p.frames.length, r.events.length + 1, where);
        const last = p.frames.at(-1) as Frame;
        const alive = (side: "own" | "opponent") => last[side].filter((t) => !t.marks.dying).length;
        if (round.result === "won") assert.equal(alive("opponent"), 0, where);
        if (round.result === "lost") assert.equal(alive("own"), 0, where);
        played += 1;
      }
    }
  }
  assert.ok(played >= 17, `played ${played}`);
});

test("step model: a summon goes to its slot, the left end when the log has none (the app's rule)", () => {
  const r = good(edited((x) => {
    x.minions[4].position = 0;
    x.events = [
      { kind: "death", minion: 2 },
      { kind: "summon", minion: 4 },
    ];
  }));
  const f = playback(r, NO_HEROES).frames;
  // The dead minion leaves before the summon lands: their board is [null card], then the token on the left.
  assert.deepEqual(cards(f[1].opponent), ["SYN_M_05", null]);
  assert.deepEqual(cards(f[2].opponent), ["SYN_M_09", null]);
  const far = good(edited((x) => {
    x.minions[4].position = 9;
    x.events = [{ kind: "summon", minion: 4 }];
  }));
  assert.deepEqual(cards(playback(far, NO_HEROES).frames[1].opponent), ["SYN_M_05", null, "SYN_M_09"], "past the end: the right end");
});

test("step model: a Duos board coming in with no hero named is an unknown hero", () => {
  const r = good(edited((x) => (x.events = [{ kind: "board_in", side: "own", hero: null }])));
  const f = playback(r, { own: "BG22_HERO_002", opponent: "BG22_HERO_000" }).frames;
  assert.deepEqual(f[1].heroes, { own: null, opponent: "BG22_HERO_000" });
});

test("playback control: play, pause, steps, the ends and reduced motion", () => {
  const at = (step: number, playing = false) => ({ step, playing });
  assert.deepEqual(control(at(0), "play", 5, false), at(0, true));
  assert.deepEqual(control(at(2, true), "play", 5, false), at(2), "play again pauses");
  assert.deepEqual(control(at(5), "play", 5, false), at(0, true), "at the end, play starts over");
  assert.deepEqual(control(at(2, true), "tick", 5, false), at(3, true));
  assert.deepEqual(control(at(4, true), "tick", 5, false), at(5), "the last step stops playback");
  assert.deepEqual(control(at(4), "tick", 5, false), at(4), "a late tick after a pause does nothing");
  assert.deepEqual(control(at(0), "back", 5, false), at(0));
  assert.deepEqual(control(at(5), "forward", 5, false), at(5));
  assert.deepEqual(control(at(3, true), "forward", 5, false), at(4), "a step pauses");
  assert.deepEqual(control(at(3), "restart", 5, false), at(0));
  assert.deepEqual(control(at(3), "end", 5, false), at(5));
  assert.deepEqual(control(at(3), { goto: 99 }, 5, false), at(5));
  assert.deepEqual(control(at(3), { goto: -1 }, 5, false), at(0));
  // Reduced motion: never plays, steps still work.
  assert.deepEqual(control(at(0), "play", 5, true), at(0));
  assert.deepEqual(control(at(0, true), "tick", 5, true), at(1));
  assert.deepEqual(control(at(1), "forward", 5, true), at(2));
  // A replay with only its start: nothing to play.
  assert.deepEqual(control(at(0), "play", 0, false), at(0));
});

test("playback keys: arrows, Home and End; never with a modifier (Alt+Left is the browser's Back)", () => {
  const key = (k: string, mods: Partial<Record<"altKey" | "ctrlKey" | "metaKey" | "shiftKey", boolean>> = {}) =>
    keyAction({ key: k, altKey: false, ctrlKey: false, metaKey: false, shiftKey: false, ...mods });
  assert.deepEqual(["ArrowLeft", "ArrowRight", "Home", "End", "Enter", " ", "constructor"].map((k) => key(k)), ["back", "forward", "restart", "end", null, null, null]);
  for (const mod of ["altKey", "ctrlKey", "metaKey", "shiftKey"] as const) assert.equal(key("ArrowLeft", { [mod]: true }), null, mod);
});

test("the reader's bounds are upload-game's and the parser's", () => {
  const server = readFileSync(join(import.meta.dirname, "..", "..", "..", "supabase", "functions", "upload-game", "replay.ts"), "utf8");
  const parser = readFileSync(join(import.meta.dirname, "..", "..", "..", "crates", "bg-parser", "src", "replay.rs"), "utf8");
  const bound = (src: string, name: string) => Number(new RegExp(String.raw`${name}(?::\s*usize)?\s*=\s*([\d_]+)`).exec(src)?.[1].replaceAll("_", ""));
  for (const src of [server, parser]) {
    assert.equal(bound(src, "MAX_REPLAY_MINIONS"), MAX_REPLAY_MINIONS);
    assert.equal(bound(src, "MAX_REPLAY_EVENTS"), MAX_REPLAY_EVENTS);
  }
});

// Records from apps before parser revision 11 start some Duos combats with
// the shop board still on a side, then bring the real board in next to it;
// before revision 12 a board coming in has no slots (0). Drawn safely:
// a board coming in replaces the side's board, left to right.
function oldDuosRecord(): Replay {
  return good({
    minions: [
      { side: "own", card_id: "SYN_SHOP_1", atk: 1, health: 1, position: 1, golden: false, keywords: [] },
      { side: "own", card_id: "SYN_SHOP_2", atk: 1, health: 1, position: 2, golden: false, keywords: [] },
      { side: "opponent", card_id: "SYN_FOE", atk: 1, health: 1, position: 1, golden: false, keywords: [] },
      { side: "own", card_id: "SYN_IN_1", atk: 2, health: 2, position: 0, golden: false, keywords: [] },
      { side: "own", card_id: "SYN_IN_2", atk: 2, health: 2, position: 0, golden: false, keywords: [] },
      { side: "own", card_id: "SYN_IN_3", atk: 2, health: 2, position: 0, golden: false, keywords: [] },
    ],
    start: 3,
    events: [
      { kind: "board_in", side: "own", hero: "SYN_HERO_MATE" },
      { kind: "summon", minion: 3 },
      { kind: "summon", minion: 4 },
      { kind: "stats", minion: 4, atk: 3, health: 3 },
      { kind: "summon", minion: 5 },
      { kind: "attack", attacker: 3, defender: 2 },
    ],
    complete: true,
  });
}

test("step model: a board coming in replaces what is still drawn on its side", () => {
  const f = playback(oldDuosRecord(), NO_HEROES).frames;
  assert.deepEqual(cards(f[1].own), [], "the shop board left over by an older app leaves");
  assert.deepEqual(cards(f[1].opponent), ["SYN_FOE"], "the other side stays");
  assert.deepEqual(cards(f.at(-1)?.own ?? []), ["SYN_IN_1", "SYN_IN_2", "SYN_IN_3"]);
});

test("step model: a board coming in with no slots goes in left to right", () => {
  const f = playback(oldDuosRecord(), NO_HEROES).frames;
  assert.deepEqual(cards(f[3].own), ["SYN_IN_1", "SYN_IN_2"]);
  assert.equal(f.at(-1)?.own[0].marks.attacker, true, "the left-most attacks first, as in the log");
});

test("step model: a lone summon with no slot still goes to the left", () => {
  const r = good(edited((x) => {
    x.minions[4].position = 0;
    x.events = [{ kind: "summon", minion: 4 }];
  }));
  assert.deepEqual(cards(playback(r, NO_HEROES).frames[1].opponent), ["SYN_M_09", "SYN_M_05", null]);
});

"use strict";
const test = require("node:test");
const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const TLReplay = require("../ui/replay.js");
const I18n = require("../ui/i18n.js");
require("../ui/strings/common.js");
require("../ui/strings/details.js");
require("../ui/strings/replay.js");

const m = (side, card_id, atk, health, position, extra = {}) => ({ side, card_id, atk, health, position, golden: false, keywords: [], ...extra });

// Two of yours against two of theirs; a shield popped, a death, a summon in
// the middle of your board, a buff, then their hero takes 6.
function replay(over = {}) {
  return {
    minions: [
      m("own", "OWN_A", 2, 3, 1, { keywords: ["divine_shield"] }),
      m("own", "OWN_B", 1, 1, 2, { keywords: ["deathrattle"] }),
      m("opponent", "FOE_A", 3, 2, 1),
      m("opponent", "FOE_B", 1, 4, 2, { golden: true }),
      m("own", "TOKEN", 1, 1, 2),
    ],
    start: 4,
    events: [
      { kind: "trigger", trigger: "start_of_combat", side: "own", card_id: "OWN_B", source: 1 },
      { kind: "attack", attacker: 0, defender: 2 },
      { kind: "damage", minion: 2, amount: 2, health: 0 },
      { kind: "shield", minion: 0, on: false },
      { kind: "death", minion: 2 },
      { kind: "summon", minion: 4 },
      { kind: "stats", minion: 4, atk: 3, health: 2 },
      { kind: "hero_damage", side: "opponent", amount: 6 },
    ],
    complete: true,
    ...over,
  };
}

const cards = (board) => board.map((t) => t.id);

test("which combats have a replay: shown, not shown, not recorded", () => {
  const report = { rounds: [{ number: 2, replay: replay() }, { number: 1, replay: null }, { number: 3, replay: replay() }] };
  const found = TLReplay.combats(report);
  assert.equal(found.recorded, true);
  assert.deepEqual(found.shown.map((c) => c.round), [2, 3]);
  assert.deepEqual(found.notShown, [1]);
  // Saved before revision 8: no key at all, so nothing is "not shown".
  const old = TLReplay.combats({ rounds: [{ number: 1 }, { number: 2 }] });
  assert.deepEqual([old.recorded, old.shown, old.notShown], [false, [], []]);
  assert.equal(TLReplay.combats(null).recorded, false);
  assert.equal(TLReplay.combats({ rounds: [{ number: 1, replay: { minions: "x" } }] }).shown.length, 0);
});

test("frames: the start, then one board state per step", () => {
  const f = TLReplay.frames(replay());
  assert.equal(f.length, 9);
  assert.equal(f.skipped, 0);
  assert.equal(f[0].event, null);
  assert.deepEqual(cards(f[0].own), ["OWN_A", "OWN_B"]);
  assert.deepEqual(cards(f[0].opponent), ["FOE_A", "FOE_B"]);
  assert.equal(f[0].own[0].shield, true);
  assert.equal(f[0].opponent[1].golden, true);
  assert.equal(f[1].own[1].triggered, true, "the trigger's minion is marked");
  assert.equal(f[2].own[0].attacker, true);
  assert.equal(f[2].opponent[0].defender, true);
  assert.equal(f[3].opponent[0].hurt, 2);
  assert.equal(f[3].opponent[0].health, 0);
  assert.equal(f[4].own[0].shield, false);
  assert.equal(f[4].own[0].popped, true);
  // A death shows on its step, then the minion leaves the board.
  assert.equal(f[5].opponent[0].dying, true);
  assert.deepEqual(cards(f[6].opponent), ["FOE_B"]);
  // A summon goes to its slot: the second of your board.
  assert.deepEqual(cards(f[6].own), ["OWN_A", "TOKEN", "OWN_B"]);
  assert.equal(f[6].own[1].summoned, true);
  assert.deepEqual([f[7].own[1].atk, f[7].own[1].health, f[7].own[1].buffed], [3, 2, true]);
  assert.deepEqual(f[8].hero, { own: 0, opponent: 6 });
  assert.equal(f[0].hero.opponent, 0, "earlier frames are not changed");
});

test("frames: a step naming a minion not on the board is left out and counted", () => {
  const r = replay();
  r.events = [
    { kind: "death", minion: 2 },
    { kind: "attack", attacker: 2, defender: 0 }, // dead already
    { kind: "damage", minion: 99, amount: 1, health: 1 },
    { kind: "summon", minion: 0 }, // already on the board
    { kind: "dance" },
    null,
    { kind: "board_in", side: "opponent", hero: "HERO_NEXT" },
  ];
  const f = TLReplay.frames(r);
  assert.equal(f.skipped, 5);
  assert.equal(f.length, 3);
  assert.equal(f[2].heroes.opponent, "HERO_NEXT", "a Duos board brings its hero");
  assert.equal(TLReplay.frames(null).length, 1);
});

test("each step in words, in English and Spanish", () => {
  const r = replay();
  const name = (ref) => I18n.t(r.minions[ref].side === "own" ? "replay.your" : "replay.their", { name: r.minions[ref].card_id });
  const hero = (id) => `Hero ${id}`;
  const words = (lang) => {
    I18n.setLang(lang);
    return [null, ...r.events].map((e) => TLReplay.stepWords(e, name, hero));
  };
  assert.deepEqual(words("en"), [
    "Both boards as the combat starts.",
    "Start of combat: your OWN_B",
    "Your OWN_A attacks their FOE_A",
    "Their FOE_A takes 2 damage",
    "Your OWN_A loses Divine Shield",
    "Their FOE_A dies",
    "Your TOKEN is summoned",
    "Your TOKEN is now 3/2",
    "Their hero takes 6 damage",
  ]);
  assert.deepEqual(words("es"), [
    "Los dos tableros al empezar el combate.",
    "Inicio del combate: tu OWN_B",
    "Tu OWN_A ataca a FOE_A rival",
    "FOE_A rival recibe 2 de daño",
    "Tu OWN_A pierde el Escudo divino",
    "FOE_A rival muere",
    "Se invoca tu TOKEN",
    "Tu TOKEN pasa a 3/2",
    "El héroe rival recibe 6 de daño",
  ]);
  I18n.setLang("en");
  const more = [
    [{ kind: "damage", minion: 3, amount: 1, health: 3 }, "Their FOE_B takes 1 damage (3 health left)"],
    [{ kind: "attack", attacker: 3, defender: null }, "Their FOE_B attacks"],
    [{ kind: "trigger", trigger: "deathrattle", side: "own", card_id: "OWN_B", source: 1 }, "Deathrattle: your OWN_B"],
    [{ kind: "trigger", trigger: "start_of_combat", side: "opponent", card_id: "TRINKET", source: null }, "Start of combat: their Hero TRINKET"],
    [{ kind: "hero_damage", side: "own", amount: 15 }, "Your hero takes 15 damage"],
    [{ kind: "board_in", side: "own", hero: "MATE" }, "Your team's next board comes in: Hero MATE"],
    [{ kind: "shield", minion: 0, on: true }, "Your OWN_A gets Divine Shield"],
  ];
  more.forEach(([e, text]) => assert.equal(TLReplay.stepWords(e, name, hero), text));
});

test("the heroes and the result of a combat, as the recap has them", () => {
  const report = { rounds: [{ number: 4, entries: [{ side: "own", hero: "H_OWN" }, { side: "opponent", hero: "H_FOE" }, { side: "opponent", hero: "H_FOE2" }] }] };
  assert.deepEqual(TLReplay.heroesOf(report, 4), { own: "H_OWN", opponent: "H_FOE" });
  assert.deepEqual(TLReplay.heroesOf(report, 5), { own: null, opponent: null });
  const recap = {
    record: [{ heroes: [{ id: "H_FOE", name: "Foe" }], combats: [{ round: 4, outcome: "won", source: "log" }], boards: [] }],
    unattributed: [],
  };
  const heroText = (h) => h.name || h.id;
  assert.deepEqual(TLReplay.outcomeOf(recap, 4, heroText), { outcome: "won", source: "log", who: "Foe" });
  assert.deepEqual(TLReplay.outcomeOf(recap, 9, heroText), { outcome: "unknown", source: "unknown", who: null });
});

test("every replay of the user's real games plays to the end, a beat at a time", () => {
  const dir = path.join(__dirname, "..", "..", "bg-parser", "tests", "data", "real");
  let played = 0;
  let together = 0;
  for (const file of ["b253216_solo.json", "b253216_duos.json", "b253216_duos_rally.json"]) {
    const [game] = JSON.parse(fs.readFileSync(path.join(dir, file), "utf8"));
    for (const c of TLReplay.combats(game).shown) {
      const f = TLReplay.frames(c.replay);
      assert.equal(f.skipped, 0, `${file} turn ${c.round}`);
      assert.equal(f.length, TLReplay.steps(c.replay).length + 1);
      assert.equal(f.slice(1).reduce((n, x) => n + x.events.length, 0), c.replay.events.length);
      together += f.filter((x) => x.events.length > 1).length;
      const round = game.rounds.find((r) => r.number === c.round);
      const last = f[f.length - 1];
      const alive = (side) => last[side].filter((t) => !t.dying).length;
      if (round.result === "won") assert.equal(alive("opponent"), 0, `${file} turn ${c.round}`);
      if (round.result === "lost") assert.equal(alive("own"), 0, `${file} turn ${c.round}`);
      played += 1;
    }
  }
  assert.ok(played > 17, `${played}`);
  assert.ok(together > 100, `${together} steps play several events at once`);
});

test("frames keep each minion's Dark Gifts through the combat (T-528)", () => {
  const r = replay();
  r.minions[0].gifts = ["BG36_MidGameEffect_000t13"];
  r.minions[2].gift_unknown = true;
  const f = TLReplay.frames(r);
  f.forEach((frame) => {
    const own = frame.own.find((x) => x.id === "OWN_A");
    if (own) assert.deepEqual(own.gifts, ["BG36_MidGameEffect_000t13"]);
  });
  assert.equal(f[0].opponent[0].gift_unknown, true);
  assert.deepEqual(f[0].own[1].gifts, [], "no gifts: an empty list");
  assert.equal(f[0].own[1].gift_unknown, false);
});

// T-560, D-132: the replay plays at the game's pace, as the website's does.
test("pace: one attack lasts about as long as in the game, and the site's beats are the app's", () => {
  const sum = (events) => TLReplay.beats([null, ...events]).slice(1).reduce((a, b) => a + b, 0);
  const attack = { kind: "attack", attacker: 0, defender: 2 };
  const hurt = (minion) => ({ kind: "damage", minion, amount: 2, health: 1 });
  const plain = sum([attack, hurt(2), hurt(0)]);
  const oneDeath = sum([attack, hurt(2), hurt(0), { kind: "death", minion: 2 }]);
  assert.ok(plain >= 1000 && plain <= 1300, `no death: ${plain}`);
  assert.ok(oneDeath >= 1200 && oneDeath <= 1600, `one death: ${oneDeath}`);
  // A buff the log puts between the attack and its damage rides the hit.
  const b = TLReplay.beats([null, attack, { kind: "stats", minion: 0, atk: 9, health: 2 }, hurt(2)]);
  assert.ok(b[2] <= 80);
  // The same numbers as web/src/lib/game/replay.ts BEAT_MS.
  const site = fs.readFileSync(path.join(__dirname, "..", "..", "..", "web", "src", "lib", "game", "replay.ts"), "utf8");
  const table = /BEAT_MS[^{]*\{([^}]*)\}/.exec(site)[1];
  const siteBeats = Object.fromEntries([...table.matchAll(/(\w+):\s*(\d+)/g)].map((x) => [x[1], Number(x[2])]));
  assert.deepEqual(TLReplay.BEAT_MS, siteBeats);
});

test("motion: the attacker winds up, hits short of its target and comes back; a moved minion slides", () => {
  const frames = TLReplay.lungeFrames({ x: 0, y: 100, width: 60, height: 72 }, { x: 140, y: 0, width: 60, height: 72 });
  assert.equal(frames[0].transform, "translate(0px, 0px)");
  assert.equal(frames.at(-1).transform, "translate(0px, 0px)");
  const hit = frames.find((f) => f.offset === TLReplay.IMPACT_MS / TLReplay.ATTACK_MS);
  const [dx, dy] = /translate\((-?[\d.]+)px, (-?[\d.]+)px\)/.exec(hit.transform).slice(1).map(Number);
  assert.ok(dx > 0 && dx < 140 && dy < 0 && dy > -100);
  for (const f of frames) assert.deepEqual(Object.keys(f).filter((k) => k !== "offset" && k !== "easing"), ["transform"]);
  assert.deepEqual(TLReplay.flipDelta({ x: 100, y: 0, width: 1, height: 1 }, { x: 70, y: 0, width: 1, height: 1 }), { dx: 30, dy: 0 });
  assert.equal(TLReplay.flipDelta({ x: 70.4, y: 0, width: 1, height: 1 }, { x: 70, y: 0, width: 1, height: 1 }), null);
});

test("motion: the app moves with the website's numbers", () => {
  const web = (file) => fs.readFileSync(path.join(__dirname, "..", "..", "..", "web", "src", "lib", "game", file), "utf8");
  const site = web("replay.ts") + web("replayMotion.ts");
  const app = fs.readFileSync(path.join(__dirname, "..", "ui", "replay.js"), "utf8");
  const value = (src, name) => new RegExp(String.raw`const ${name}(?::[^=]+)? = ([^;]+);`).exec(src)?.[1];
  for (const name of ["AT_IMPACT_MS", "ATTACK_MS", "HIT_MS", "SLIDE_MS", "STOP", "WIND_UP_PX", "EASE_OUT", "EASE_IN_OUT"]) {
    assert.ok(value(site, name), `${name} on the website`);
    assert.equal(value(app, name), value(site, name), name);
  }
});

// Records from parser revision 10 or older start some Duos combats with the
// shop board still on a side, then bring the real board in next to it;
// before revision 12 a board coming in has no slots (0).
test("a board coming in replaces what is still drawn on its side, left to right", () => {
  const f = TLReplay.frames({
    minions: [
      m("own", "SHOP_1", 1, 1, 1),
      m("own", "SHOP_2", 1, 1, 2),
      m("opponent", "FOE", 1, 1, 1),
      m("own", "IN_1", 2, 2, 0),
      m("own", "IN_2", 2, 2, 0),
      m("own", "IN_3", 2, 2, 0),
    ],
    start: 3,
    events: [
      { kind: "board_in", side: "own", hero: "MATE" },
      { kind: "summon", minion: 3 },
      { kind: "summon", minion: 4 },
      { kind: "stats", minion: 4, atk: 3, health: 3 },
      { kind: "summon", minion: 5 },
      { kind: "attack", attacker: 3, defender: 2 },
    ],
    complete: true,
  });
  const cards = (board) => board.map((x) => x.id);
  assert.deepEqual(cards(f[1].own), [], "the left-over shop board leaves");
  assert.deepEqual(cards(f[1].opponent), ["FOE"]);
  assert.deepEqual(cards(f[f.length - 1].own), ["IN_1", "IN_2", "IN_3"]);
  assert.equal(f[f.length - 1].own[0].attacker, true);
});

test("both boards coming in at once each fill from the left; after the first attack a summon with no slot goes left", () => {
  const f = TLReplay.frames({
    minions: [
      m("own", "OLD_OWN", 1, 1, 1),
      m("opponent", "OLD_FOE", 1, 1, 1),
      m("own", "IN_OWN_1", 1, 1, 0),
      m("opponent", "IN_FOE_1", 1, 1, 0),
      m("own", "IN_OWN_2", 1, 1, 0),
      m("opponent", "IN_FOE_2", 1, 1, 0),
      m("own", "TOKEN", 1, 1, 0),
    ],
    start: 2,
    events: [
      { kind: "board_in", side: "own", hero: null },
      { kind: "board_in", side: "opponent", hero: null },
      { kind: "summon", minion: 2 },
      { kind: "summon", minion: 3 },
      { kind: "summon", minion: 4 },
      { kind: "summon", minion: 5 },
      { kind: "stats", minion: 0, atk: 9, health: 9 },
      { kind: "attack", attacker: 2, defender: 3 },
      { kind: "summon", minion: 6 },
    ],
    complete: true,
  });
  const cards = (board) => board.map((x) => x.id);
  const last = f[f.length - 1];
  assert.deepEqual(cards(last.opponent), ["IN_FOE_1", "IN_FOE_2"]);
  assert.deepEqual(cards(last.own), ["TOKEN", "IN_OWN_1", "IN_OWN_2"]);
  assert.equal(f.skipped, 1, "a step naming a minion of a board that left is not played");
});

// Parser revision 13 (T-575): beats, damage sources and keyword marks, the
// same rules as the website (web/test/unit/gameReplayBeats.test.ts).
function rev13() {
  return {
    minions: [m("own", "OWN_RALLY", 2, 3, 1), m("own", "OWN_MATE", 2, 3, 2), m("opponent", "FOE_A", 2, 3, 1, { keywords: ["taunt"] }), m("opponent", "FOE_B", 2, 3, 2)],
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

test("revision 13: a beat plays at once, keywords come and go, and a beat lasts each kind once", () => {
  const f = TLReplay.frames(rev13());
  assert.equal(f.length, 8);
  assert.deepEqual([f[1].own[0].buffed, f[1].own[1].buffed], [true, true]);
  assert.deepEqual(f[3].own.map((x) => x.gained), [["venomous"], ["venomous"]]);
  assert.deepEqual(f[4].own[0].lost, ["venomous"]);
  assert.deepEqual(f[4].own[0].keywords, []);
  assert.deepEqual([f[4].opponent[0].hurt, f[4].own[0].hurt], [4, 2]);
  const ms = TLReplay.frameBeats(f);
  const B = TLReplay.BEAT_MS;
  assert.deepEqual(ms.slice(0, 6), [B.start, B.trigger + B.stats, B.attack, B.trigger + B.keyword, 2 * B.damage + B.keyword, B.death]);
  const old = { ...rev13(), beats: undefined };
  const g = TLReplay.frames(old);
  assert.equal(g.length, old.events.length + 1, "an older record: one event per step");
});

test("revision 13: each damage says where it came from", () => {
  const r = rev13();
  const names = ["Rally Murloc", "Mate", "Foe A", "Foe B"];
  const name = (ref) => (r.minions[ref].side === "own" ? `your ${names[ref]}` : `their ${names[ref]}`);
  const hero = (id) => (id === "FOE_POWER" ? "Their Power" : id);
  const f = TLReplay.frames(r);
  assert.equal(TLReplay.frameWords(f[3], name, hero), "Rally: your Rally Murloc. Your Rally Murloc gets Venomous. Your Mate gets Venomous");
  assert.equal(TLReplay.frameWords(f[4], name, hero), "Their Foe A takes 4 damage from your Rally Murloc (attack). Your Rally Murloc takes 2 damage from their Foe A (attack; 3 health left). Your Rally Murloc loses Venomous");
  assert.equal(TLReplay.frameWords(f[6], name, hero), "Their Foe B takes 1 damage from their Their Power (hero power; 2 health left)");
  assert.equal(TLReplay.frameWords(f[7], name, hero), "Their Foe B takes 1 damage from an unknown source (1 health left)");
});

test("revision 13: the website and the app group the same steps", () => {
  const site = fs.readFileSync(path.join(__dirname, "..", "..", "..", "web", "src", "lib", "game", "replay.ts"), "utf8");
  // Both merge the opening's start-of-combat buffs and keep an attack alone.
  assert.match(site, /onlyBuffs\(last\) && onlyBuffs\(run\)/);
  const sot = (side, source) => ({ kind: "trigger", trigger: "start_of_combat", side, card_id: "SOT", source });
  const r = {
    minions: [m("own", "A", 1, 1, 1), m("own", "B", 1, 1, 2), m("opponent", "C", 1, 1, 1), m("opponent", "D", 1, 1, 2)],
    start: 4,
    events: [
      sot("own", 0),
      { kind: "stats", minion: 0, atk: 3, health: 4 },
      sot("opponent", 2),
      { kind: "stats", minion: 2, atk: 3, health: 4 },
      { kind: "stats", minion: 3, atk: 3, health: 4 },
      sot("opponent", 3),
      { kind: "damage", minion: 1, amount: 1, health: 2, source: { how: "start_of_combat", minion: 3 } },
      { kind: "attack", attacker: 0, defender: 2 },
      sot("own", 1),
      { kind: "stats", minion: 1, atk: 5, health: 5 },
      sot("own", 0),
      { kind: "stats", minion: 0, atk: 5, health: 5 },
    ],
    beats: [2, 3, 2, 1, 2, 2],
    complete: true,
  };
  assert.deepEqual(TLReplay.steps(r).map((s) => s.length), [5, 2, 1, 2, 2], "the same as the website's test");
});

test("revision 13: a buff marks what it gave, and every token has the same box", () => {
  const f = TLReplay.frames(rev13());
  assert.deepEqual(f[1].own.map((x) => x.gain), [{ atk: 2, health: 2 }, { atk: 2, health: 2 }]);
  const css = fs.readFileSync(path.join(__dirname, "..", "ui", "replay.css"), "utf8");
  const rule = (selector) => {
    const at = css.indexOf(`\n${selector} {`);
    assert.ok(at >= 0, selector);
    return css.slice(at, css.indexOf("}", at));
  };
  for (const selector of [".replay-gifts", ".replay-keys", ".replay-stats", ".replay-hurt, .replay-gain"]) assert.match(rule(selector), /position: absolute/, selector);
  assert.match(rule(".replay-minion"), /width: var\(--tile\)/);
});

test("a Taunt taken mid-combat drops its shield on that step (T-581)", () => {
  // Sin'dorei Straight Shot's Rally, from the owner's game of 2026-10-11 (synthetic copy).
  const f = TLReplay.frames({
    minions: [m("own", "BG25_016", 1, 1, 1), m("opponent", "BG26_149", 2, 3, 1, { keywords: ["taunt", "divine_shield"] }), m("opponent", "BG26_152", 2, 3, 2, { keywords: ["taunt", "divine_shield"] })],
    start: 3,
    events: [
      { kind: "attack", attacker: 0, defender: 1 },
      { kind: "trigger", trigger: "rally", side: "own", card_id: "BG25_016", source: 0 },
      { kind: "keyword", minion: 1, keyword: "taunt", on: false },
      { kind: "shield", minion: 1, on: false },
    ],
    beats: [1, 2, 1],
    complete: true,
  });
  assert.ok(f[1].opponent[0].keywords.includes("taunt"), "before the Rally");
  assert.ok(!f[2].opponent[0].keywords.includes("taunt"));
  assert.deepEqual(f[2].opponent[0].lost, ["taunt"]);
  assert.ok(f[2].opponent[1].keywords.includes("taunt"), "the other keeps it");
  assert.ok(!f[3].opponent[0].keywords.includes("taunt"));
});

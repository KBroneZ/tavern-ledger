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

test("every replay of the user's two real games plays to the end", () => {
  const dir = path.join(__dirname, "..", "..", "bg-parser", "tests", "data", "real");
  let played = 0;
  for (const file of ["b253216_solo.json", "b253216_duos.json"]) {
    const [game] = JSON.parse(fs.readFileSync(path.join(dir, file), "utf8"));
    for (const c of TLReplay.combats(game).shown) {
      const f = TLReplay.frames(c.replay);
      assert.equal(f.skipped, 0, `${file} turn ${c.round}`);
      assert.equal(f.length, c.replay.events.length + 1);
      const round = game.rounds.find((r) => r.number === c.round);
      const last = f[f.length - 1];
      const alive = (side) => last[side].filter((t) => !t.dying).length;
      if (round.result === "won") assert.equal(alive("opponent"), 0, `${file} turn ${c.round}`);
      if (round.result === "lost") assert.equal(alive("own"), 0, `${file} turn ${c.round}`);
      played += 1;
    }
  }
  assert.equal(played, 17);
});

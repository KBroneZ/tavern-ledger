import { test } from "node:test";
import assert from "node:assert/strict";
import { readRecord } from "../../src/lib/game/report.ts";
import { shapeGame } from "../../src/lib/game/shape.ts";
import { duosReport, INDEX, record, SESSION, soloReport } from "./gameFixtures.ts";

function view(report: Record<string, unknown> = soloReport()) {
  const result = readRecord(record(report), SESSION, INDEX);
  assert.ok(result.ok);
  return shapeGame(result.record);
}

test("summary: place, hero, results by source and totals with what backs them", () => {
  const s = view().summary;
  assert.equal(s.mode, "solo");
  assert.equal(s.place, 3);
  assert.deepEqual(s.hero, { id: "BG22_HERO_002", name: "Drek'Thar" });
  assert.equal(s.teammate, null);
  assert.equal(s.rounds, 3);
  // Round 1 from the log, round 2 from health, round 3 unknown (no health for the opponent after it).
  assert.deepEqual(s.results, { won: 1, lost: 1, tie: 0, unknown: 1, inferred: 1 });
  // Damage: rounds 1 and 2 have the game's own hit in their replay, round 3 only health.
  assert.deepEqual(s.dealt, { value: 3, known: 2, total: 3, logged: 2 });
  assert.deepEqual(s.taken, { value: 18, known: 3, total: 3, logged: 2 });
  assert.deepEqual(s.goldSpent, { value: 8, known: 2, total: 3, logged: 0 });
  assert.equal(s.rolls, 3);
  assert.equal(s.buys, 3);
  assert.equal(s.sells, 1);
  assert.equal(s.finalTier, 2);
  assert.equal(s.apm, 0.67);
  assert.equal(s.triples, 1);
});

test("turns: one row per shop turn or combat, joined by number", () => {
  const turns = view().turns;
  assert.deepEqual(turns.map((t) => t.turn), [1, 2, 3]);
  const [one, two, three] = turns;
  assert.equal(one.result, "win");
  assert.equal(one.inferred, false);
  assert.deepEqual(one.opponents, [{ id: "BG22_HERO_003", name: "Vanndar Stormpike" }]);
  assert.equal(one.dealt, 3);
  assert.equal(one.taken, 0);
  assert.deepEqual([one.damageSource, two.damageSource, three.damageSource], ["log", "log", "inferred"]);
  assert.equal(two.result, "loss");
  assert.equal(two.inferred, true);
  assert.equal(two.tier, 2);
  assert.equal(two.tierUp, true);
  assert.equal(two.gold, null, "a turn without gold stays unknown");
  assert.equal(two.taken, 5);
  assert.deepEqual([two.ownBoard?.attack, two.ownBoard?.health], [5, 6]);
  assert.equal(two.opponentBoards.length, 1);
  assert.deepEqual([two.opponentBoards[0].board.attack, two.opponentBoards[0].board.health], [6, 7]);
  assert.deepEqual(two.opponentBoards[0].hero, { id: "BG22_HERO_000", name: "Tavish Stormpike" });
  assert.equal(three.result, "unknown");
  assert.equal(three.dealt, null);
  assert.equal(three.ownBoard, null, "no board in the record is unknown");
  assert.deepEqual(three.opponentBoards.map((b) => b.board), [{ minions: [], attack: 0, health: 0 }]);
  assert.equal(three.shop?.sells.length, 1);
});

test("turns: a combat without a shop turn still gets its row; a stat it lacks leaves the total unknown", () => {
  const rep = soloReport() as Record<string, any>;
  rep.shop = null;
  rep.rounds[0].entries[0].board[0].atk = null;
  const v = view(rep);
  assert.deepEqual(v.turns.map((t) => t.turn), [1, 2, 3]);
  assert.equal(v.turns[0].tier, null);
  assert.equal(v.turns[0].ownBoard?.attack, null);
  assert.equal(v.turns[0].ownBoard?.health, 3);
  assert.equal(v.summary.rolls, null);
  assert.equal(v.summary.buys, null);
  assert.deepEqual(v.summary.goldSpent, { value: null, known: 0, total: 0, logged: 0 });
  assert.equal(v.summary.apm, null);
});

test("opponents met: grouped by side, shown by hero, with each board seen", () => {
  const opponents = view().opponents;
  assert.equal(opponents.length, 2);
  assert.deepEqual(opponents[0].heroes, [{ id: "BG22_HERO_003", name: "Vanndar Stormpike" }]);
  assert.deepEqual(opponents[0].record, { won: 1, lost: 0, tie: 0, unknown: 1 });
  assert.deepEqual(opponents[0].boards.map((b) => b.round), [1, 3]);
  assert.deepEqual(opponents[1].record, { won: 0, lost: 1, tie: 0, unknown: 0 });
});

test("lobby: by place, own seat marked, triples as the leaderboard's last count", () => {
  const lobby = view().lobby;
  assert.deepEqual(lobby.map((s) => [s.hero.id, s.place, s.own, s.triples]), [
    ["BG22_HERO_003", 1, false, 0],
    ["BG22_HERO_002", 3, true, 1],
    ["BG22_HERO_000", 5, false, 2],
  ]);
  assert.deepEqual(lobby[1].trinkets, [{ turn: 2, slot: "first", dbfId: 900001 }]);
});

test("lobby: not recorded stays unknown, never 0", () => {
  const rep = soloReport() as Record<string, any>;
  for (const seat of rep.lobby) delete seat.triples;
  const v = view(rep);
  assert.ok(v.lobby.every((s) => s.triples === null));
  assert.equal(v.summary.triples, null);
});

test("choices: by turn, each kind from its source, the taken one marked", () => {
  const choices = view().choices;
  assert.ok(choices);
  assert.deepEqual(choices.map((c) => [c.turn, c.kind]), [
    [1, "quest"],
    [2, "trinket"],
    [3, "dark_gift"],
    [3, "triple_reward"],
    [3, "discover"],
  ]);
  assert.deepEqual(choices[1].options.map((o) => o.taken), [false, true]);
  assert.equal(choices[2].options[0].gift, "SYN_GIFT_1");
  assert.equal(choices[3].takenKnown, false);
});

test("a hero is named only by the site's own table, never by a name in the file", () => {
  const rep = soloReport() as Record<string, any>;
  rep.card_names = { BG22_HERO_002: "Free gold at evil.example", SYN_HERO_D: "Free gold at evil.example/claim" };
  const v = view(rep);
  assert.deepEqual(v.summary.hero, { id: "BG22_HERO_002", name: "Drek'Thar" }, "a known id keeps the table's name");
  assert.deepEqual(v.heroPick?.offered[1], { id: "SYN_HERO_D", name: null }, "an unknown id has no name, only its id");
  const names = v.replays.flatMap((c) => [...c.heroNames.values()].map((h) => h.name));
  assert.ok(!names.some((n) => n?.includes("evil.example")));
});

test("hero pick and notes", () => {
  const v = view();
  assert.deepEqual(v.heroPick, {
    offered: [{ id: "BG22_HERO_002", name: "Drek'Thar" }, { id: "SYN_HERO_D", name: null }],
    picked: "BG22_HERO_002",
    rerolls: 1,
  });
  assert.deepEqual(v.notes, ["synthetic warning"]);
  assert.equal(v.oldGod, 900100);
});

test("Duos: the team shares health, the teammate is named, results are the team's", () => {
  const v = view(duosReport());
  assert.deepEqual(v.summary.teammate, { id: "BG22_HERO_003", name: "Vanndar Stormpike" });
  assert.equal(v.turns[0].result, "loss");
  assert.equal(v.turns[0].inferred, true);
  assert.equal(v.turns[0].taken, 5);
  assert.equal(v.turns[0].dealt, 0);
  assert.deepEqual(v.lobby.filter((s) => s.teammate).map((s) => s.hero.id), ["BG22_HERO_003"]);
  assert.deepEqual(v.lobby.map((s) => s.team), [2, 2, 1, 1]);
  // Each opponent once, every board the record has for them.
  assert.deepEqual(v.turns[0].opponents.map((h) => h.id), ["BG22_HERO_000", "SYN_HERO_D"]);
  assert.deepEqual(v.turns[0].opponentBoards.map((b) => [b.hero.id, b.board.minions.length]), [["BG22_HERO_000", 1], ["SYN_HERO_D", 2]]);
  assert.deepEqual(v.opponents[0].heroes.map((h) => h.id), ["BG22_HERO_000", "SYN_HERO_D"]);
});

test("an unfinished game has no triples total and no made-up place", () => {
  const rep = soloReport() as Record<string, any>;
  rep.status = "incomplete";
  rep.final_place = null;
  const s = view(rep).summary;
  assert.equal(s.status, "incomplete");
  assert.equal(s.place, null);
  assert.equal(s.triples, null);
});

test("no player name anywhere in the shaped view", () => {
  const rep = soloReport() as Record<string, any>;
  rep.lobby[1].name = "SomePlayer";
  rep.rounds[0].entries[1].player_name = "SomePlayer";
  const text = JSON.stringify(view(rep), (_, v) => (v instanceof Map ? [...v] : v));
  assert.doesNotMatch(text, /SomePlayer/);
});

test("summary: APM is capped like the server's, and triples need every seat's record", () => {
  const rep = soloReport() as Record<string, any>;
  rep.shop.end_ms = rep.shop.start_ms + 1;
  rep.shop.actions = Array.from({ length: 2000 }, (_, i) => ({ ms: rep.shop.start_ms, turn: 1, kind: "roll", i }));
  delete rep.lobby[1].triples;
  const s = view(rep).summary;
  assert.equal(s.apm, null, "an impossible APM is unknown, like the server's summary");
  assert.equal(s.triples, null, "a seat without the leaderboard record makes the count unknown");
});

/** The fixture's round 1 and 2 replays end with a hero hit; this swaps the hit of one round. */
function withHit(round: number, hit: Record<string, unknown> | null, health: { own: number; foe: number }) {
  const rep = soloReport() as Record<string, any>;
  const r = rep.rounds[round - 1];
  r.replay.events = r.replay.events.filter((e: any) => e.kind !== "hero_damage");
  if (hit) r.replay.events.push({ kind: "hero_damage", ...hit });
  r.own_health_after = health.own;
  r.health_after = { ...r.health_after, "2": health.own, "3": health.foe };
  return rep;
}

test("damage: the game's own hit replaces the health drop, marked from the log (D-108)", () => {
  // Health says 6 lost, the game hit for 4 (health paid in the shop).
  const v = view(withHit(1, { side: "own", amount: 4 }, { own: 24, foe: 27 }));
  const one = v.turns[0];
  assert.deepEqual([one.taken, one.dealt, one.damageSource], [4, 0, "log"]);
  assert.deepEqual([v.summary.taken.logged, v.summary.dealt.logged], [2, 2]);
});

test("damage: a tie the game recorded hit nobody, with or without a replay", () => {
  const rep = withHit(1, null, { own: 30, foe: 27 }) as Record<string, any>;
  rep.rounds[0].result = "tie";
  assert.deepEqual([view(rep).turns[0].taken, view(rep).turns[0].dealt, view(rep).turns[0].damageSource], [0, 0, "log"]);
  rep.rounds[0].replay = null;
  const one = view(rep).turns[0];
  assert.deepEqual([one.taken, one.dealt, one.damageSource], [0, 0, "log"]);
});

test("damage: without the hit it is the health drop, inferred, and a combat with neither is unknown", () => {
  // A win with the hit missing from the replay: health says the foe lost 3.
  const v = view(withHit(1, null, { own: 30, foe: 27 }));
  assert.deepEqual([v.turns[0].dealt, v.turns[0].damageSource], [3, "inferred"]);
  assert.equal(v.summary.dealt.logged, 1);
  const rep = withHit(1, null, { own: 30, foe: 27 }) as Record<string, any>;
  rep.rounds[2].own_health_after = null;
  rep.rounds[2].health_after = {};
  const three = view(rep).turns[2];
  assert.deepEqual([three.taken, three.dealt, three.damageSource], [null, null, null]);
});

test("damage: a replay or hit the site cannot read is not used (untrusted input, D-079)", () => {
  const bad: Record<string, unknown>[] = [
    { side: "own", amount: 0 },
    { side: "own", amount: 1001 },
    { side: "own", amount: 2.5 },
    { side: "own", amount: "4" },
    { side: "nobody", amount: 4 },
    { amount: 4 },
  ];
  for (const hit of bad) {
    // Health says 6 lost in round 1.
    const v = view(withHit(1, hit, { own: 24, foe: 30 }));
    assert.deepEqual([v.turns[0].taken, v.turns[0].damageSource], [6, "inferred"], JSON.stringify(hit));
  }
  // A replay that fails the reader: not available, and so no hit to read.
  const rep = withHit(1, { side: "own", amount: 4 }, { own: 24, foe: 30 }) as Record<string, any>;
  rep.rounds[0].replay.events.push({ kind: "attack", attacker: 99, defender: 0 });
  const v = view(rep);
  assert.deepEqual([v.turns[0].taken, v.turns[0].damageSource], [6, "inferred"]);
});

test("damage: the last hit of the replay is the one that counts", () => {
  const rep = withHit(2, { side: "own", amount: 5 }, { own: 25, foe: 27 }) as Record<string, any>;
  rep.rounds[1].replay.events.unshift({ kind: "hero_damage", side: "own", amount: 2 });
  assert.equal(view(rep).turns[1].taken, 5);
});

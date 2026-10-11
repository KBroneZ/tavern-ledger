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
  assert.deepEqual(s.hero, { id: "SYN_HERO_A", name: "Lantern Keeper" });
  assert.equal(s.teammate, null);
  assert.equal(s.rounds, 3);
  // Round 1 from the log, round 2 from health, round 3 unknown (no health for the opponent after it).
  assert.deepEqual(s.results, { won: 1, lost: 1, tie: 0, unknown: 1, inferred: 1 });
  assert.deepEqual(s.dealt, { value: 3, known: 2, total: 3 });
  assert.deepEqual(s.taken, { value: 18, known: 3, total: 3 });
  assert.deepEqual(s.goldSpent, { value: 8, known: 2, total: 3 });
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
  assert.deepEqual(one.opponents, [{ id: "SYN_HERO_B", name: "Brass Captain" }]);
  assert.equal(one.dealt, 3);
  assert.equal(one.taken, 0);
  assert.equal(two.result, "loss");
  assert.equal(two.inferred, true);
  assert.equal(two.tier, 2);
  assert.equal(two.tierUp, true);
  assert.equal(two.gold, null, "a turn without gold stays unknown");
  assert.equal(two.taken, 5);
  assert.deepEqual([two.ownBoard?.attack, two.ownBoard?.health], [5, 6]);
  assert.deepEqual([two.opponentBoard?.attack, two.opponentBoard?.health], [6, 7]);
  assert.equal(three.result, "unknown");
  assert.equal(three.dealt, null);
  assert.equal(three.ownBoard, null, "no board in the record is unknown");
  assert.deepEqual(three.opponentBoard, { minions: [], attack: 0, health: 0 });
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
  assert.equal(v.summary.shopTurns, 0);
  assert.equal(v.summary.rolls, null);
  assert.equal(v.summary.buys, null);
  assert.deepEqual(v.summary.goldSpent, { value: null, known: 0, total: 0 });
  assert.equal(v.summary.apm, null);
});

test("opponents met: grouped by side, shown by hero, with each board seen", () => {
  const opponents = view().opponents;
  assert.equal(opponents.length, 2);
  assert.deepEqual(opponents[0].heroes, [{ id: "SYN_HERO_B", name: "Brass Captain" }]);
  assert.deepEqual(opponents[0].record, { won: 1, lost: 0, tie: 0, unknown: 1 });
  assert.deepEqual(opponents[0].boards.map((b) => b.round), [1, 3]);
  assert.deepEqual(opponents[1].record, { won: 0, lost: 1, tie: 0, unknown: 0 });
});

test("lobby: by place, own seat marked, triples as the leaderboard's last count", () => {
  const lobby = view().lobby;
  assert.deepEqual(lobby.map((s) => [s.hero.id, s.place, s.own, s.triples]), [
    ["SYN_HERO_B", 1, false, 0],
    ["SYN_HERO_A", 3, true, 1],
    ["SYN_HERO_C", 5, false, 2],
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

test("hero pick and notes", () => {
  const v = view();
  assert.deepEqual(v.heroPick, {
    offered: [{ id: "SYN_HERO_A", name: "Lantern Keeper" }, { id: "SYN_HERO_D", name: null }],
    picked: "SYN_HERO_A",
    rerolls: 1,
  });
  assert.deepEqual(v.notes, ["synthetic warning"]);
  assert.equal(v.oldGod, 900100);
});

test("Duos: the team shares health, the teammate is named, results are the team's", () => {
  const v = view(duosReport());
  assert.deepEqual(v.summary.teammate, { id: "SYN_HERO_B", name: "Brass Captain" });
  assert.equal(v.turns[0].result, "loss");
  assert.equal(v.turns[0].inferred, true);
  assert.equal(v.turns[0].taken, 5);
  assert.equal(v.turns[0].dealt, 0);
  assert.deepEqual(v.lobby.filter((s) => s.teammate).map((s) => s.hero.id), ["SYN_HERO_B"]);
  assert.deepEqual(v.lobby.map((s) => s.team), [2, 2, 1, 1]);
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

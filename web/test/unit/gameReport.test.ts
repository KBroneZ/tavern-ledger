import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { KNOWN_REVISION, readRecord } from "../../src/lib/game/report.ts";
import { duosReport, INDEX, record, SESSION, soloReport } from "./gameFixtures.ts";

const read = (value: unknown) => {
  const result = readRecord(value, SESSION, INDEX);
  assert.ok(result.ok, "the record is read");
  return result.record;
};

test("report reader: a whole synthetic Solo game, nothing dropped", () => {
  const rec = read(record());
  assert.equal(rec.unreadable, 0);
  assert.equal(rec.newer, false);
  assert.equal(rec.parserVersion, "0.1.0");
  const r = rec.report;
  assert.equal(r.mode, "solo");
  assert.equal(r.status, "ok");
  assert.equal(r.hero, "SYN_HERO_A");
  assert.equal(r.heroNames.get("SYN_HERO_A"), "Lantern Keeper");
  assert.equal(r.finalPlace, 3);
  assert.deepEqual(r.rounds.map((x) => x.number), [1, 2, 3]);
  assert.equal(r.rounds[0].result, "won");
  assert.equal(r.rounds[1].result, null);
  assert.deepEqual(r.rounds[1].entries[0].board?.[1], {
    cardId: "SYN_M_02",
    atk: 3,
    health: 3,
    golden: true,
    keywords: ["taunt", "divine_shield"],
  });
  assert.equal(r.rounds[2].entries[0].board, null, "no board is unknown");
  assert.deepEqual(r.rounds[2].entries[1].board, [], "an empty board stays empty");
  assert.equal(r.startHealth.get(2), 30);
  assert.equal(r.shop?.turns.length, 3);
  assert.equal(r.shop?.turns[1].gold, null);
  assert.equal(r.shop?.actions, 2);
  assert.deepEqual(r.heroSelect, { offered: ["SYN_HERO_A", "SYN_HERO_D"], rerolls: 1, picked: "SYN_HERO_A" });
  assert.equal(r.choices?.length, 5);
  assert.deepEqual(r.lobby[2].triples, [{ turn: 2, count: 1 }, { turn: 3, count: 2 }]);
  assert.deepEqual(r.lobby[0].trinkets, [{ turn: 2, slot: "first", dbfId: 900001 }]);
  assert.equal(r.oldGod, 900100);
  assert.deepEqual(r.warnings, ["synthetic warning"]);
});

test("report reader: a Duos game", () => {
  const r = read(record(duosReport())).report;
  assert.equal(r.mode, "duos");
  assert.equal(r.teammateHero, "SYN_HERO_B");
  assert.equal(r.shop, null);
  assert.deepEqual(r.choices, []);
});

test("report reader: a record that is not this game's is refused whole", () => {
  for (const value of [
    null,
    "text",
    [],
    42,
    { session: SESSION, index: INDEX },
    { session: SESSION, index: INDEX, report: [] },
    record(soloReport(), { session: "Hearthstone_2026_10_09_20_15_04" }),
    record(soloReport(), { index: INDEX + 1 }),
    record({ ...soloReport(), index: INDEX + 1 }),
  ]) {
    assert.deepEqual(readRecord(value, SESSION, INDEX), { ok: false }, JSON.stringify(value)?.slice(0, 80));
  }
});

test("report reader: unknown keys are ignored, not counted", () => {
  const rep = { ...soloReport(), future_field: { a: 1 } };
  const rec = read(record(rep, { future: true }));
  assert.equal(rec.unreadable, 0);
  assert.ok(!("future_field" in rec.report));
});

test("report reader: bad values become unknown and are counted, never a crash", () => {
  const rep = soloReport() as Record<string, any>;
  rep.final_place = 9; // out of range for Solo
  rep.hero = "<script>";
  rep.build = "250000";
  rep.rounds[0].result = "victory";
  rep.rounds[0].entries[0].board[0].atk = 1.5;
  rep.rounds[0].entries[0].board[0].keywords = ["taunt", "flying", 3];
  rep.rounds[1].health_after = { "2": -5, x: 3, "3": 27 };
  rep.shop.turns[0].gold = -1;
  rep.lobby.push({ hero: "SYN_HERO_E" }); // no player id
  rep.discovers[0].picked = [7];
  rep.warnings = ["fine", "<b>bold</b>"];
  const rec = read(record(rep));
  const r = rec.report;
  assert.equal(r.finalPlace, null);
  assert.equal(r.hero, null);
  assert.equal(r.build, null);
  assert.equal(r.rounds[0].result, null);
  assert.equal(r.rounds[0].entries[0].board?.[0].atk, null);
  assert.deepEqual(r.rounds[0].entries[0].board?.[0].keywords, ["taunt"]);
  assert.deepEqual([...r.rounds[1].healthAfter], [[3, 27]]);
  assert.equal(r.shop?.turns[0].gold, null);
  assert.equal(r.lobby.length, 3);
  assert.deepEqual(r.choices?.[0].picked, []);
  assert.deepEqual(r.warnings, ["fine"]);
  assert.equal(rec.unreadable, 13);
});

test("report reader: wrong types at any level never throw", () => {
  const shapes: unknown[] = [null, 1, "x", true, [], {}, [null], [1], ["x"], [{}], { a: 1 }];
  const levels = ["lobby", "rounds", "shop", "hero_select", "discovers", "card_names", "start_health", "warnings", "status", "game_type"];
  for (const key of levels) {
    for (const shape of shapes) {
      const rep = { ...soloReport(), [key]: shape };
      assert.doesNotThrow(() => readRecord(record(rep), SESSION, INDEX), `${key}=${JSON.stringify(shape)}`);
    }
  }
  const deep = soloReport() as Record<string, any>;
  deep.rounds = [{ number: 1, entries: [{ side: "own", board: [null, 1, "x", [], { keywords: "taunt" }] }] }];
  deep.shop = { turns: [{ turn: 1, offers: [null, { roll: "0" }], buys: [1, null] }], tier_ups: [{}], actions: "many" };
  assert.doesNotThrow(() => read(record(deep)));
});

test("report reader: lists past the server's bounds are cut and counted", () => {
  const rep = soloReport() as Record<string, any>;
  rep.lobby = Array.from({ length: 20 }, (_, i) => ({ player_id: i + 1 }));
  const rec = read(record(rep));
  assert.equal(rec.report.lobby.length, 16);
  assert.ok(rec.unreadable >= 4);
});

test("report reader: no BattleTag or markup gets through as a hero name", () => {
  const rep = soloReport() as Record<string, any>;
  rep.card_names = {
    SYN_HERO_A: "Player#1234",
    SYN_HERO_B: "Player＃12345",
    SYN_HERO_C: "<img src=x>",
    SYN_HERO_D: " padded ",
    SYN_HERO_E: "Fine Name",
    "bad id": "Name",
  };
  const r = read(record(rep)).report;
  assert.deepEqual([...r.heroNames], [["SYN_HERO_E", "Fine Name"]]);
});

test("report reader: a newer parser revision is marked, an older one is not", () => {
  assert.equal(read(record(soloReport(), { parser: { version: "9.0.0", revision: KNOWN_REVISION + 1 } })).newer, true);
  assert.equal(read(record(soloReport(), { parser: { version: "0.1.0", revision: 1 } })).newer, false);
  const none = read(record(soloReport(), { parser: undefined }));
  assert.equal(none.newer, false);
  assert.equal(none.revision, null);
});

test("report reader: older records say not recorded (null), never empty", () => {
  const rep = soloReport() as Record<string, any>;
  delete rep.discovers;
  for (const seat of rep.lobby) {
    delete seat.triples;
    delete seat.trinkets;
  }
  for (const t of rep.shop.turns) delete t.passes;
  const r = read(record(rep)).report;
  assert.equal(r.choices, null);
  assert.equal(r.lobby[0].triples, null);
  assert.equal(r.lobby[0].trinkets, null);
  assert.equal(r.shop?.turns[0].passes, null);
});

test("report reader: every synthetic parser report in the repo reads without a dropped value", () => {
  const dir = join(import.meta.dirname, "..", "..", "..", "crates", "bg-parser", "tests", "data", "rust");
  for (const file of ["round_two_games.json", "data_round_games.json", "solo_shop_games.json"]) {
    const games = JSON.parse(readFileSync(join(dir, file), "utf8")) as Record<string, unknown>[];
    for (const game of games) {
      const index = game.index as number;
      const result = readRecord({ session: SESSION, index, saved_at: 1, report: game }, SESSION, index);
      assert.ok(result.ok, file);
      assert.equal(result.record.unreadable, 0, `${file} game ${index}`);
    }
  }
});

import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { KNOWN_REVISION, readRecord } from "../../src/lib/game/report.ts";
import { duosReport, INDEX, record, SESSION, soloReport } from "./gameFixtures.ts";

const read = (value: unknown) => {
  const result = readRecord(value);
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
  assert.equal(r.hero, "BG22_HERO_002");
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
    gifts: [],
    giftUnknown: false,
    buffs: null,
  });
  assert.equal(r.rounds[2].entries[0].board, null, "no board is unknown");
  assert.deepEqual(r.rounds[2].entries[1].board, [], "an empty board stays empty");
  assert.equal(r.startHealth.get(2), 30);
  assert.equal(r.shop?.turns.length, 3);
  assert.equal(r.shop?.turns[1].gold, null);
  assert.equal(r.shop?.actions, 2);
  assert.deepEqual(r.heroSelect, { offered: ["BG22_HERO_002", "SYN_HERO_D"], rerolls: 1, picked: "BG22_HERO_002" });
  assert.equal(r.choices?.length, 5);
  assert.deepEqual(r.lobby[2].triples, [{ turn: 2, count: 1 }, { turn: 3, count: 2 }]);
  assert.deepEqual(r.lobby[0].trinkets, [{ turn: 2, slot: "first", dbfId: 900001 }]);
  assert.equal(r.oldGod, 900100);
  assert.deepEqual(r.warnings, ["synthetic warning"]);
});

test("report reader: the game's build is read when it is a build number, else unknown", () => {
  assert.equal(read(record({ ...soloReport(), build: 254_477 })).report.build, 254_477);
  assert.equal(read(record(soloReport())).report.build, 250_000, "the fixture's own build");
  const { build: _, ...noBuild } = soloReport();
  assert.equal(read(record(noBuild)).report.build, null, "a file without one says nothing");
  const odd = read(record({ ...soloReport(), build: "254477" }));
  assert.equal(odd.report.build, null);
  assert.equal(odd.unreadable, 1, "a value that is not a build is counted, never shown");
});

test("report reader: a Duos game", () => {
  const r = read(record(duosReport())).report;
  assert.equal(r.mode, "duos");
  assert.equal(r.teammateHero, "BG22_HERO_003");
  assert.equal(r.shop, null);
  assert.deepEqual(r.choices, []);
});

test("report reader: a record that does not agree with itself is refused whole", () => {
  for (const value of [
    null,
    "text",
    [],
    42,
    { session: SESSION, index: INDEX },
    { session: SESSION, index: INDEX, report: [] },
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
  assert.equal(r.rounds[0].result, null);
  assert.equal(r.rounds[0].entries[0].board?.[0].atk, null);
  assert.deepEqual(r.rounds[0].entries[0].board?.[0].keywords, ["taunt"]);
  assert.deepEqual([...r.rounds[1].healthAfter], [[3, 27]]);
  assert.equal(r.shop?.turns[0].gold, null);
  assert.equal(r.lobby.length, 3);
  assert.deepEqual(r.choices?.[0].picked, []);
  assert.deepEqual(r.warnings, ["fine"]);
  assert.equal(rec.unreadable, 12);
});

test("report reader: wrong types at any level never throw", () => {
  const shapes: unknown[] = [null, 1, "x", true, [], {}, [null], [1], ["x"], [{}], { a: 1 }];
  const levels = ["lobby", "rounds", "shop", "hero_select", "discovers", "card_names", "start_health", "warnings", "status", "game_type"];
  for (const key of levels) {
    for (const shape of shapes) {
      const rep = { ...soloReport(), [key]: shape };
      assert.doesNotThrow(() => readRecord(record(rep)), `${key}=${JSON.stringify(shape)}`);
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
  for (const file of ["round_two_games.json", "data_round_games.json", "solo_shop_games.json", "game_values_games.json", "kit_games.json"]) {
    const games = JSON.parse(readFileSync(join(dir, file), "utf8")) as Record<string, unknown>[];
    for (const game of games) {
      const index = game.index as number;
      const result = readRecord({ session: SESSION, index, saved_at: 1, report: game });
      assert.ok(result.ok, file);
      assert.equal(result.record.unreadable, 0, `${file} game ${index}`);
    }
  }
});

test("report reader: real games with combat replays (revision 8) read without a dropped value", () => {
  // The replay is for a later page (T-536); until then the reader skips it.
  const dir = join(import.meta.dirname, "..", "..", "..", "crates", "bg-parser", "tests", "data", "real");
  for (const file of ["b253216_solo.json", "b253216_duos.json"]) {
    const games = JSON.parse(readFileSync(join(dir, file), "utf8")) as Record<string, any>[];
    for (const game of games) {
      assert.ok(game.rounds.some((r: Record<string, unknown>) => r.replay), `${file} has replays`);
      const index = game.index as number;
      const parser = { version: "0.1.0", revision: KNOWN_REVISION };
      const result = readRecord({ session: SESSION, index, saved_at: 1, parser, report: game });
      assert.ok(result.ok, file);
      assert.equal(result.record.unreadable, 0, `${file} game ${index}`);
      assert.equal(result.record.newer, false);
    }
  }
});

test("report reader: at most 3000 shop offers in a game, as the server checks", () => {
  const rep = soloReport() as Record<string, any>;
  const offers = Array.from({ length: 400 }, () => ({ card_id: "SYN_M_01", roll: 0, frozen: false }));
  rep.shop.turns = Array.from({ length: 10 }, (_, i) => ({ ...rep.shop.turns[0], turn: i + 1, offers }));
  const rec = read(record(rep));
  const kept = rec.report.shop!.turns.reduce((n, t) => n + t.offers.length, 0);
  assert.equal(kept, 3000);
  assert.equal(rec.unreadable, 1000);
});

test("report reader: Dark Gifts on board minions (revision 9), bad ones counted", () => {
  const report = soloReport() as Record<string, any>;
  const board = report.rounds[1].entries[0].board;
  board[0].gifts = ["BG36_MidGameEffect_000t9", "BG36_MidGameEffect_000t9", "<b>"];
  board[1].gift_unknown = true;
  const rec = read(record(report));
  const [first, second] = rec.report.rounds[1].entries[0].board ?? [];
  assert.deepEqual(first.gifts, ["BG36_MidGameEffect_000t9"], "once each; markup dropped");
  assert.equal(rec.unreadable, 1, "the bad id is counted");
  assert.equal(second.giftUnknown, true);
  assert.deepEqual(second.gifts, []);
  board[1].gift_unknown = "yes";
  const bad = read(record(report));
  assert.equal(bad.report.rounds[1].entries[0].board?.[1].giftUnknown, false);
  assert.equal(bad.unreadable, 2, "a gift_unknown that is not true or false is counted too");
});

test("report reader: buffs on board minions (revision 14): unknown before it, bad ones counted", () => {
  const report = soloReport() as Record<string, any>;
  const board = report.rounds[1].entries[0].board;
  board[0].buffs = [
    { card_id: "BG20_GEM", count: 2 },
    { card_id: "BG20_GEM", count: 1 },
    { card_id: "<b>", count: 1 },
    { card_id: "BG31_816", count: 0 },
    { card_id: "BG28_309", count: 3 },
  ];
  const old = read(record(report));
  assert.equal(old.report.rounds[1].entries[0].board?.[0].buffs, null, "a record before revision 14 did not read buffs");
  assert.equal(old.unreadable, 0, "and its buffs key is not read at all");
  const rec = read(record(report, { parser: { version: "0.1.0", revision: 14 } }));
  const [first, second] = rec.report.rounds[1].entries[0].board ?? [];
  assert.deepEqual(first.buffs, [{ cardId: "BG20_GEM", count: 2 }, { cardId: "BG28_309", count: 3 }], "a card once; markup and zero dropped");
  assert.equal(rec.unreadable, 2, "the bad id and the zero are counted");
  assert.deepEqual(second.buffs, [], "revision 14 without the key: recorded, none");
  board[1].buffs = "BG20_GEM";
  assert.deepEqual(read(record(report, { parser: { version: "0.1.0", revision: 14 } })).report.rounds[1].entries[0].board?.[1].buffs, []);
});

test("report reader: the real game with Dark Gifts reads without a dropped value", () => {
  const dir = join(import.meta.dirname, "..", "..", "..", "crates", "bg-parser", "tests", "data", "real");
  const [game] = JSON.parse(readFileSync(join(dir, "b253216_duos_gifts.json"), "utf8")) as Record<string, any>[];
  const parser = { version: "0.1.0", revision: KNOWN_REVISION };
  const result = readRecord({ session: SESSION, index: game.index, saved_at: 1, parser, report: game });
  assert.ok(result.ok);
  assert.equal(result.record.unreadable, 0);
  const buffed = result.record.report.rounds.flatMap((r) => r.entries.flatMap((e) => e.board ?? [])).filter((m) => (m.buffs ?? []).length > 0);
  assert.ok(buffed.length > 0, "the fixture's enchantments are read as buffs");
  const gifted = result.record.report.rounds.flatMap((r) => r.entries.flatMap((e) => e.board ?? [])).filter((m) => m.gifts.length > 0);
  assert.equal(gifted.length, 23);
});

test("report reader: each board's game-long values (revision 10), bad ones counted", () => {
  const report = soloReport() as Record<string, any>;
  const entries = report.rounds[1].entries;
  entries[0].values = { blood_gem_atk: 3, undead_atk: 7, spell_atk: 0, beetle_atk: "2", knights_dead: 100_000, tasty_lobster: 5 };
  entries[1].values = [3, 2];
  const rec = read(record(report));
  const [own, opponent] = rec.report.rounds[1].entries;
  assert.deepEqual(own.values, { blood_gem_atk: 3, undead_atk: 7 }, "past the server's bounds dropped; unknown names ignored");
  assert.equal(opponent.values, null);
  assert.equal(rec.unreadable, 4, "zero, a string, too large and a list are counted");
  assert.equal(read(record()).report.rounds[1].entries[0].values, null, "none saved: the printed numbers");
});

test("report reader: the record keeps the day the game was played and never its time", () => {
  const result = readRecord(record(soloReport()));
  assert.ok(result.ok);
  assert.equal(result.record.playedOn, "2026-10-09");
  assert.doesNotMatch(JSON.stringify(result.record), /20_15_03|20:15/);
  const noDate = readRecord(record(soloReport(), { session: "nope" }));
  assert.ok(noDate.ok);
  assert.equal(noDate.record.playedOn, null);
});

test("report reader: each board's hero powers and trinkets (revision 16), not recorded before it", () => {
  const dir = join(import.meta.dirname, "..", "..", "..", "crates", "bg-parser", "tests", "data", "rust");
  const games = JSON.parse(readFileSync(join(dir, "kit_games.json"), "utf8")) as Record<string, any>[];
  const parser = { version: "0.1.0", revision: KNOWN_REVISION };
  const result = readRecord({ session: SESSION, index: games[0].index, saved_at: 1, parser, report: games[0] });
  assert.ok(result.ok);
  assert.equal(result.record.unreadable, 0);
  const [own, opponent] = result.record.report.rounds[0].entries;
  const power = own.kit?.powers[0];
  assert.equal(power?.cardId, "SYN_POWER");
  assert.deepEqual(power?.counters, [5, 0, 0, 0, 0]);
  assert.equal(power?.held?.cardId, "SYN_HELD");
  assert.deepEqual(power?.held?.gifts, ["BG36_MidGameEffect_000t9"]);
  assert.deepEqual(own.kit?.trinkets.map((t) => [t.cardId, t.slot, t.target, t.held]), [
    ["SYN_LESSER", 1, null, null],
    ["BG30_Trinket_2nd", 2, null, null],
  ]);
  assert.deepEqual(opponent.kit?.powers.map((p) => [p.cardId, p.held]), [["SYN_FOE_POWER", null]], "the opponent's, never a held minion");
  assert.equal(read(record()).report.rounds[1].entries[0].kit, null, "an older record: not recorded");
});

test("report reader: a hero power or trinket that cannot be read is counted, the kit not recorded", () => {
  const report = soloReport() as Record<string, any>;
  const entries = report.rounds[1].entries;
  entries[0].kit = { powers: [{ card_id: "X", slot: 9 }], trinkets: [] };
  entries[1].kit = { powers: [], trinkets: [{ card_id: "Y", slot: 1, counters: [1, 2] }] };
  const rec = read(record(report));
  const [own, opponent] = rec.report.rounds[1].entries;
  assert.equal(own.kit, null);
  assert.equal(opponent.kit, null);
  assert.equal(rec.unreadable, 3, "the slot, then each card that could not be read");
});

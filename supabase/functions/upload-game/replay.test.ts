// Run: deno test supabase/functions/upload-game/
// Each round's combat replay (T-215, D-099, parser revision 8).
import { type Json, realDuos, realRally, realRecord, realSolo, SESSION } from "./testdata.ts";
import { validateRecord, VALIDATOR_VERSION } from "./validate.ts";

function assertEquals(actual: unknown, expected: unknown, msg = ""): void {
  const a = JSON.stringify(actual);
  const e = JSON.stringify(expected);
  if (a !== e) throw new Error(`${msg}\n  actual:   ${a}\n  expected: ${e}`);
}

function refused(rec: Json, field: string, msg = "") {
  const v = validateRecord(rec, SESSION, 1);
  assertEquals(v.ok ? "accepted" : [v.code, v.field], ["invalid_report", field], msg);
}

/** The first round with a replay that has a summon, and its index. */
function withSummon(report: Json): [Json, number] {
  const i = report.rounds.findIndex((r: Json) =>
    r.replay?.events.some((e: Json) => e.kind === "summon")
  );
  if (i < 0) throw new Error("setup: no replay with a summon");
  return [report.rounds[i], i];
}

Deno.test("real games with combat replays (revision 8) are accepted", () => {
  for (const report of [realSolo[0], realDuos[0]]) {
    const replays = report.rounds.filter((r: Json) => r.replay !== null);
    assertEquals(replays.length > 0, true, "the fixture has replays");
    const v = validateRecord(realRecord(report), SESSION, 1);
    if (!v.ok) throw new Error(`refused: ${v.code} ${v.field}`);
  }
  const kinds = new Set(
    [realSolo[0], realDuos[0]].flatMap((g: Json) =>
      g.rounds.flatMap((r: Json) => (r.replay?.events ?? []).map((e: Json) => e.kind))
    ),
  );
  for (const kind of ["attack", "damage", "shield", "death", "summon", "trigger", "hero_damage", "board_in"]) {
    assertEquals(kinds.has(kind), true, `the fixtures have a ${kind} event`);
  }
  assertEquals(VALIDATOR_VERSION >= 7, true, "a version that accepts replays");
});

Deno.test("a combat not shown (null) and records without replays are accepted", () => {
  const rec = realRecord(realSolo[0]);
  rec.report.rounds[0].replay = null;
  assertEquals(validateRecord(rec, SESSION, 1).ok, true, "not shown");
  const old = realRecord(realSolo[0]);
  old.parser.revision = 7;
  old.report.rounds.forEach((r: Json) => delete r.replay);
  assertEquals(validateRecord(old, SESSION, 1).ok, true, "revision 7");
});

Deno.test("a replay keeps to its shapes, bounds and references", () => {
  const [round, n] = withSummon(realSolo[0]);
  const at = `report.rounds[${n}].replay`;
  const summon = round.replay.events.findIndex((e: Json) => e.kind === "summon");
  const born = round.replay.events[summon].minion;
  const cases: Array<[(r: Json) => void, string]> = [
    [(r) => (r.replay = []), at],
    [(r) => delete r.replay.complete, `${at}.complete`],
    [(r) => (r.replay.extra = 1), `${at}.extra`],
    [(r) => (r.replay.start = r.replay.minions.length + 1), `${at}.start`],
    [(r) => (r.replay.minions = new Array(201).fill(r.replay.minions[0])), `${at}.minions`],
    [(r) => (r.replay.minions[0].side = "both"), `${at}.minions[0].side`],
    [(r) => (r.replay.minions[0].card_id = "<b>"), `${at}.minions[0].card_id`],
    [(r) => (r.replay.minions[0].name = "x"), `${at}.minions[0].name`],
    [(r) => (r.replay.minions[0].keywords = ["flying"]), `${at}.minions[0].keywords`],
    [(r) => (r.replay.minions[0].position = 65), `${at}.minions[0].position`],
    [(r) => (r.replay.events = new Array(2001).fill(r.replay.events[0])), `${at}.events`],
    [(r) => (r.replay.events[0] = { kind: "dance" }), `${at}.events[0].kind`],
    [(r) => (r.replay.events[0] = { kind: "constructor" }), `${at}.events[0].kind`],
    [(r) => (r.replay.events[0] = { kind: "toString" }), `${at}.events[0].kind`],
    [(r) => (r.replay.events[0] = { kind: "__proto__" }), `${at}.events[0].kind`],
    [(r) => (r.replay.events[0] = { kind: "death" }), `${at}.events[0].minion`],
    [(r) => (r.replay.events[0] = { kind: "death", minion: 999 }), `${at}.events[0].minion`],
    [(r) => (r.replay.events[0] = { kind: "death", minion: 0, who: 1 }), `${at}.events[0].who`],
    [
      (r) => (r.replay.events[0] = { kind: "damage", minion: 0, amount: 0, health: 1 }),
      `${at}.events[0].amount`,
    ],
    [
      (r) => (r.replay.events[0] = { kind: "attack", attacker: born, defender: null }),
      `${at}.events[0].attacker`,
    ],
    [(r) => (r.replay.events[summon].minion = 0), `${at}.events[${summon}].minion`],
    [
      (r) => r.replay.events.push({ kind: "summon", minion: born }),
      `${at}.events[${round.replay.events.length}].minion`,
    ],
    [
      (r) => (r.replay.events[0] = {
        kind: "trigger",
        trigger: "battlecry",
        side: "own",
        card_id: "BG_X",
        source: null,
      }),
      `${at}.events[0].trigger`,
    ],
    [
      (r) => (r.replay.events[0] = {
        kind: "trigger",
        trigger: "deathrattle",
        side: "own",
        card_id: null,
        source: null,
      }),
      `${at}.events[0].card_id`,
    ],
    [
      (r) => (r.replay.events[0] = { kind: "hero_damage", side: "own", amount: 0 }),
      `${at}.events[0].amount`,
    ],
    [
      (r) => (r.replay.events[0] = { kind: "board_in", side: "opponent", hero: "a b" }),
      `${at}.events[0].hero`,
    ],
    [(r) => (r.replay.events[0] = { kind: "shield", minion: 0, on: 1 }), `${at}.events[0].on`],
  ];
  for (const [forge, field] of cases) {
    const rec = realRecord(realSolo[0]);
    forge(rec.report.rounds[n]);
    refused(rec, field);
  }
});

Deno.test("a BattleTag in a replay is refused as a player name", () => {
  const rec = realRecord(realSolo[0]);
  const [round] = withSummon(rec.report);
  round.replay.minions[0].card_id = "Someone#1234";
  const v = validateRecord(rec, SESSION, 1);
  assertEquals(v.ok ? "accepted" : v.code, "player_name");
});

Deno.test("revision 13 replays (beats, damage sources, keyword events) are accepted", () => {
  const report = realRally[0];
  const events = report.rounds.flatMap((r: Json) => r.replay?.events ?? []);
  assertEquals(events.some((e: Json) => e.kind === "keyword" && e.keyword === "venomous"), true, "a Venomous given");
  assertEquals(events.some((e: Json) => e.kind === "damage" && e.source?.how === "attack"), true, "an attack's damage");
  assertEquals(report.rounds.every((r: Json) => !r.replay || Array.isArray(r.replay.beats)), true, "beats");
  const v = validateRecord(realRecord(report), SESSION, 1);
  if (!v.ok) throw new Error(`refused: ${v.code} ${v.field}`);
  assertEquals(VALIDATOR_VERSION >= 10, true, "a version that accepts them");
});

Deno.test("a replay without the revision 13 fields (older apps) is still accepted", () => {
  const rec = realRecord(realRally[0]);
  for (const r of rec.report.rounds) {
    if (!r.replay) continue;
    delete r.replay.beats;
    r.replay.events = r.replay.events.filter((e: Json) => e.kind !== "keyword");
    for (const e of r.replay.events) if (e.kind === "damage") delete e.source;
  }
  const v = validateRecord(rec, SESSION, 1);
  if (!v.ok) throw new Error(`refused: ${v.code} ${v.field}`);
});

Deno.test("revision 13 fields keep to their shapes and references", () => {
  const n = realRally[0].rounds.findIndex((r: Json) => r.replay?.events.some((e: Json) => e.kind === "damage"));
  const round = realRally[0].rounds[n];
  const at = `report.rounds[${n}].replay`;
  const d = round.replay.events.findIndex((e: Json) => e.kind === "damage");
  const ev = `${at}.events[${d}]`;
  const cases: Array<[(r: Json) => void, string]> = [
    [(r) => (r.replay.beats = [r.replay.events.length + 1]), `${at}.beats`],
    [(r) => (r.replay.beats = [0, r.replay.events.length]), `${at}.beats[0]`],
    [(r) => (r.replay.beats = "all"), `${at}.beats`],
    [(r) => (r.replay.events[d].source = { how: "magic" }), `${ev}.source.how`],
    [(r) => (r.replay.events[d].source = { how: "attack", minion: 999 }), `${ev}.source.minion`],
    [(r) => (r.replay.events[d].source = { how: "attack", minion: 0, card_id: "BG_X" }), `${ev}.source`],
    [(r) => (r.replay.events[d].source = { how: "trinket", card_id: "BG_X" }), `${ev}.source.side`],
    [(r) => (r.replay.events[d].source = { how: "trinket", card_id: "a b", side: "own" }), `${ev}.source.card_id`],
    [(r) => (r.replay.events[d].source = { how: "attack", minion: 0, name: "x" }), `${ev}.source.name`],
    [(r) => (r.replay.events[0] = { kind: "keyword", minion: 0, keyword: "divine_shield", on: true }), `${at}.events[0].keyword`],
    [(r) => (r.replay.events[0] = { kind: "keyword", minion: 0, keyword: "flying", on: true }), `${at}.events[0].keyword`],
    [(r) => (r.replay.events[0] = { kind: "keyword", minion: 0, keyword: "taunt", on: 1 }), `${at}.events[0].on`],
    [(r) => (r.replay.events[0] = { kind: "keyword", minion: 999, keyword: "taunt", on: true }), `${at}.events[0].minion`],
  ];
  for (const [forge, field] of cases) {
    const rec = realRecord(realRally[0]);
    forge(rec.report.rounds[n]);
    refused(rec, field);
  }
});

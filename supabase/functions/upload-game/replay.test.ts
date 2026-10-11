// Run: deno test supabase/functions/upload-game/
// Each round's combat replay (T-215, D-099, parser revision 8).
import { type Json, realDuos, realRecord, realSolo, SESSION } from "./testdata.ts";
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

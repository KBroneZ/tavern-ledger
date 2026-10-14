// Run: deno test supabase/functions/upload-game/
// Buffs on board and replay minions (parser revision 14, T-619, D-142): the
// card that gave them and how many, as objects {card_id, count}.
import { type Json, realGifts, realRecord, SESSION } from "./testdata.ts";
// The parser's report of a synthetic game with buffs on both sides' minions,
// kept current by crates/bg-parser/tests/combat_log.rs.
import buffGames from "../../../crates/bg-parser/tests/data/rust/game_buffs_games.json" with {
  type: "json",
};
import contract from "./report_keys.json" with { type: "json" };
import { validateRecord, VALIDATOR_VERSION } from "./validate.ts";

function assertEquals(actual: unknown, expected: unknown, msg = ""): void {
  const a = JSON.stringify(actual);
  const e = JSON.stringify(expected);
  if (a !== e) throw new Error(`${msg}\n  actual:   ${a}\n  expected: ${e}`);
}

function refused(rec: Json, field: string, msg = "") {
  const v = validateRecord(rec, SESSION, 1);
  assertEquals(
    v.ok ? "accepted" : [v.code, v.field],
    ["invalid_report", field],
    msg,
  );
}

/** [round, entry, minion] of the first board minion with buffs. */
function firstBuffed(report: Json): [number, number, number] {
  for (const [r, round] of report.rounds.entries()) {
    for (const [e, entry] of round.entries.entries()) {
      const m = (entry.board ?? []).findIndex((x: Json) => x.buffs);
      if (m >= 0) return [r, e, m];
    }
  }
  throw new Error("setup: no minion with buffs");
}

Deno.test("the parser's buffs have the shape the validator accepts (revision 14)", () => {
  // The synthetic game's lobby is partial, so only its minions are checked
  // here (the next test sends a whole real game).
  const [r, e, m] = firstBuffed(buffGames[0]);
  const report: Json = buffGames[0];
  assertEquals(report.rounds[r].entries[e].board[m].buffs, [
    { card_id: "BG20_GEM", count: 2 },
  ]);
  assertEquals(VALIDATOR_VERSION >= 11, true, "a version that accepts buffs");
});

Deno.test("the buff keys the validator checks are the ones the contract lists", () => {
  // checks.ts names them itself, so a key added to the contract alone would be refused.
  for (const level of ["report.rounds[].entries[].board[].buffs[]", "report.rounds[].replay.minions[].buffs[]"]) {
    assertEquals((contract.keys as Record<string, string[]>)[level], ["card_id", "count"], level);
  }
});

Deno.test("real games with buffs (blessed fixtures) are accepted", () => {
  const v = validateRecord(realRecord(realGifts[0]), SESSION, 1);
  if (!v.ok) throw new Error(`refused: ${v.code} ${v.field}`);
  const [r, e, m] = firstBuffed(realGifts[0]);
  const report: Json = realGifts[0];
  assertEquals(Array.isArray(report.rounds[r].entries[e].board[m].buffs), true);
});

Deno.test("buffs keep to their shape", () => {
  const [r, e, m] = firstBuffed(realGifts[0]);
  const at = `report.rounds[${r}].entries[${e}].board[${m}]`;
  const one = { card_id: "BG20_GEM", count: 2 };
  const cases: Array<[(x: Json) => void, string]> = [
    [(x) => (x.buffs = []), `${at}.buffs`],
    [(x) => (x.buffs = "BG20_GEM"), `${at}.buffs`],
    [(x) => (x.buffs = ["BG20_GEM"]), `${at}.buffs[0]`],
    [(x) => (x.buffs = [{ ...one, name: "Blood Gem" }]), `${at}.buffs[0].name`],
    [(x) => (x.buffs = [{ count: 2 }]), `${at}.buffs[0].card_id`],
    [(x) => (x.buffs = [{ card_id: "BG20_GEM" }]), `${at}.buffs[0].count`],
    [
      (x) => (x.buffs = [{ card_id: "<b>", count: 2 }]),
      `${at}.buffs[0].card_id`,
    ],
    [
      (x) => (x.buffs = [{ card_id: null, count: 2 }]),
      `${at}.buffs[0].card_id`,
    ],
    [(x) => (x.buffs = [{ ...one, count: 0 }]), `${at}.buffs[0].count`],
    [(x) => (x.buffs = [{ ...one, count: 1000 }]), `${at}.buffs[0].count`],
    [(x) => (x.buffs = [{ ...one, count: 1.5 }]), `${at}.buffs[0].count`],
    [(x) => (x.buffs = [one, one]), `${at}.buffs`],
    [
      (
        x,
      ) => (x.buffs = new Array(13).fill(0).map((_, i) => ({
        card_id: `BG_${i}`,
        count: 1,
      }))),
      `${at}.buffs`,
    ],
  ];
  for (const [change, field] of cases) {
    const rec = realRecord(realGifts[0]);
    change(rec.report.rounds[r].entries[e].board[m]);
    refused(rec, field, field);
  }
});

Deno.test("replay minions' buffs keep to their shape", () => {
  const report = structuredClone(realGifts[0]);
  const r = report.rounds.findIndex((x: Json) =>
    (x.replay?.minions ?? []).length > 0
  );
  const at = `report.rounds[${r}].replay.minions[0]`;
  const ok = realRecord(report);
  ok.report.rounds[r].replay.minions[0].buffs = [{
    card_id: "BG31_816",
    count: 3,
  }];
  const v = validateRecord(ok, SESSION, 1);
  if (!v.ok) throw new Error(`refused: ${v.code} ${v.field}`);
  const bad = realRecord(report);
  bad.report.rounds[r].replay.minions[0].buffs = [{
    card_id: "BG31_816",
    count: 0,
  }];
  refused(bad, `${at}.buffs[0].count`);
  const noBuffs = realRecord(report);
  delete noBuffs.report.rounds[r].replay.minions[0].buffs;
  assertEquals(
    validateRecord(noBuffs, SESSION, 1).ok,
    true,
    "a replay minion without buffs",
  );
});

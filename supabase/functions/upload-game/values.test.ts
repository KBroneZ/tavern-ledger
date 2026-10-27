// Run: deno test supabase/functions/upload-game/
// Each board's game-long values (parser revision 10, T-610, D-122): Blood
// Gem and Tavern spell upgrades, Eternal Knights that died, the Undead
// Attack and the Beetles' buffs, each from 1 to 99 999, zero left out.
import { type Json, realDuos, realRecord, SESSION } from "./testdata.ts";
// The parser's report of a synthetic game with values on both sides, kept
// current by crates/bg-parser/tests/combat_log.rs.
import valueGames from "../../../crates/bg-parser/tests/data/rust/game_values_games.json" with {
  type: "json",
};
import { validateRecord, VALIDATOR_VERSION } from "./validate.ts";

function assertEquals(actual: unknown, expected: unknown, msg = ""): void {
  const a = JSON.stringify(actual);
  const e = JSON.stringify(expected);
  if (a !== e) throw new Error(`${msg}\n  actual:   ${a}\n  expected: ${e}`);
}

const ALL = {
  blood_gem_atk: 3,
  blood_gem_health: 2,
  spell_atk: 1,
  spell_health: 4,
  knights_dead: 2,
  undead_atk: 7,
  beetle_atk: 12,
  beetle_health: 99_999,
};

/** [round, entry] of the first entry with a board, on `side`. */
function withBoard(report: Json, side: string): [number, number] {
  for (const [r, round] of report.rounds.entries()) {
    const e = round.entries.findIndex((x: Json) => x.side === side && Array.isArray(x.board));
    if (e >= 0) return [r, e];
  }
  throw new Error(`setup: no ${side} board`);
}

function withValues(values: Json, side = "own"): { rec: Json; at: string } {
  const rec = realRecord(realDuos[0]);
  const [r, e] = withBoard(rec.report, side);
  rec.report.rounds[r].entries[e].values = values;
  return { rec, at: `report.rounds[${r}].entries[${e}].values` };
}

function refused(values: Json, field: (at: string) => string, msg: string) {
  const { rec, at } = withValues(values);
  const v = validateRecord(rec, SESSION, 1);
  assertEquals(v.ok ? "accepted" : [v.code, v.field], ["invalid_report", field(at)], msg);
}

Deno.test("each board's game-long values are accepted (validator 9)", () => {
  assertEquals(VALIDATOR_VERSION >= 9, true);
  for (const side of ["own", "opponent"]) {
    const { rec } = withValues(ALL, side);
    const v = validateRecord(rec, SESSION, 1);
    if (!v.ok) throw new Error(`${side}: refused ${v.code} ${v.field}`);
  }
  const { rec } = withValues({ undead_atk: 1 });
  assertEquals(validateRecord(rec, SESSION, 1).ok, true, "one value alone");
  assertEquals(validateRecord(realRecord(realDuos[0]), SESSION, 1).ok, true, "no values (older or none read)");
});

Deno.test("the values the parser writes are accepted", () => {
  const written = valueGames[0].rounds[0].entries.map((e: Json) => e.values);
  assertEquals(written.length, 2, "both sides");
  for (const values of written) {
    if (!values) throw new Error("setup: a side without values");
    const { rec } = withValues(values);
    const v = validateRecord(rec, SESSION, 1);
    if (!v.ok) throw new Error(`refused: ${v.code} ${v.field}`);
  }
});

Deno.test("game-long values keep to their names and bounds", () => {
  const cases: Array<[Json, (at: string) => string, string]> = [
    [{}, (at) => at, "an empty object: the parser leaves the key out"],
    [null, (at) => at, "null"],
    [[3, 2], (at) => at, "a list"],
    ["3", (at) => at, "a string"],
    [{ blood_gem_atk: 0 }, (at) => `${at}.blood_gem_atk`, "zero is left out, never saved"],
    [{ spell_atk: -1 }, (at) => `${at}.spell_atk`, "negative"],
    [{ knights_dead: 100_000 }, (at) => `${at}.knights_dead`, "past the bound"],
    [{ beetle_atk: 1.5 }, (at) => `${at}.beetle_atk`, "not whole"],
    [{ undead_atk: "7" }, (at) => `${at}.undead_atk`, "a string number"],
    [{ undead_atk: true }, (at) => `${at}.undead_atk`, "a boolean"],
    [{ tasty_lobster: 3 }, (at) => `${at}.tasty_lobster`, "a name the parser does not write"],
    [{ ...ALL, name: "hello" }, (at) => `${at}.name`, "free text"],
  ];
  for (const [values, field, msg] of cases) refused(values, field, msg);
});

Deno.test("game-long values come only with a board", () => {
  const rec = realRecord(realDuos[0]);
  const [r, e] = withBoard(rec.report, "own");
  const entry = rec.report.rounds[r].entries[e];
  entry.board = null;
  entry.values = { blood_gem_atk: 1 };
  const v = validateRecord(rec, SESSION, 1);
  assertEquals(
    v.ok ? "accepted" : [v.code, v.field],
    ["invalid_report", `report.rounds[${r}].entries[${e}].values`],
  );
});

Deno.test("a board's Deity is accepted, its id up to 9 999 999 (validator 13)", () => {
  assertEquals(VALIDATOR_VERSION >= 13, true, "a version that accepts the Deity");
  const deity = { deity_dbf: 130_610, deity_atk: 77, deity_health: 64, deity_needed: 4, deity_progress: 1 };
  const { rec } = withValues(deity);
  const v = validateRecord(rec, SESSION, 1);
  if (!v.ok) throw new Error(`refused ${v.code} ${v.field}`);
  refused({ deity_dbf: 10_000_000 }, (at) => `${at}.deity_dbf`, "an id past the bound");
  refused({ deity_atk: 100_000 }, (at) => `${at}.deity_atk`, "the Deity's stats keep the game-long bound");
  refused({ deity_needed: 0 }, (at) => `${at}.deity_needed`, "zero is left out, never saved");
});

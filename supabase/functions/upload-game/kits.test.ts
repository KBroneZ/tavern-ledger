// Run: deno test supabase/functions/upload-game/
// Each board's hero powers and trinkets and replay minions' two numbers
// (T-628, D-157, parser revision 16).
import { type Json, realRecord, SESSION } from "./testdata.ts";
import { validateRecord, VALIDATOR_VERSION } from "./validate.ts";
// The Rust parser's reports of the synthetic kit games (a held minion with a
// Dark Gift, a named minion, numbers), kept current by
// crates/bg-parser/tests/start_of_combat.rs.
import kitGames from "../../../crates/bg-parser/tests/data/rust/kit_games.json" with {
  type: "json",
};

function assertEquals(actual: unknown, expected: unknown, msg = ""): void {
  const a = JSON.stringify(actual);
  const e = JSON.stringify(expected);
  if (a !== e) throw new Error(`${msg}\n  actual:   ${a}\n  expected: ${e}`);
}

function refused(rec: Json, field: string, msg = "") {
  const v = validateRecord(rec, SESSION, 1);
  assertEquals(v.ok ? "accepted" : [v.code, v.field], ["invalid_report", field], msg);
}

const ENTRY = "report.rounds[0].entries[0]";

Deno.test("the kits the parser writes are accepted", () => {
  const report = kitGames[0] as Json;
  const kit = report.rounds[0].entries[0].kit;
  assertEquals(kit.powers[0].held.gifts.length, 1, "a held minion with a gift");
  assertEquals(typeof report.rounds[0].replay.minions.find((m: Json) => m.counters), "object");
  for (const game of kitGames as Json[]) {
    const v = validateRecord(realRecord(game), SESSION, 1);
    if (!v.ok) throw new Error(`refused: ${v.code} ${v.field}`);
  }
  assertEquals(VALIDATOR_VERSION >= 12, true, "a version that accepts kits");
});

Deno.test("a kit keeps to its shape and bounds", () => {
  const cases: Array<[(k: Json) => void, string]> = [
    [(k) => delete k.trinkets, `${ENTRY}.kit.trinkets`],
    [(k) => (k.extra = 1), `${ENTRY}.kit.extra`],
    [(k) => (k.powers = new Array(5).fill(k.powers[0])), `${ENTRY}.kit.powers`],
    [(k) => (k.powers[0].card_id = null), `${ENTRY}.kit.powers[0].card_id`],
    [(k) => (k.powers[0].card_id = "<b>"), `${ENTRY}.kit.powers[0].card_id`],
    [(k) => (k.powers[0].slot = 4), `${ENTRY}.kit.powers[0].slot`],
    [(k) => (k.powers[0].counters = [1, 2]), `${ENTRY}.kit.powers[0].counters`],
    [(k) => (k.powers[0].counters = [0, 0, 0, 0, 0]), `${ENTRY}.kit.powers[0].counters`],
    [(k) => (k.powers[0].counters = [-1, 0, 0, 0, 0]), `${ENTRY}.kit.powers[0].counters[0]`],
    [(k) => (k.powers[0].counters = [1.5, 0, 0, 0, 0]), `${ENTRY}.kit.powers[0].counters[0]`],
    [(k) => (k.powers[0].target = 0), `${ENTRY}.kit.powers[0].target`],
    [(k) => (k.powers[0].target = 8), `${ENTRY}.kit.powers[0].target`],
    [(k) => (k.powers[0].held.position = 1), `${ENTRY}.kit.powers[0].held.position`],
    [(k) => (k.powers[0].held.buffs = []), `${ENTRY}.kit.powers[0].held.buffs`],
    [(k) => delete k.powers[0].held.keywords, `${ENTRY}.kit.powers[0].held.keywords`],
    [(k) => (k.powers[0].held.golden = "no"), `${ENTRY}.kit.powers[0].held.golden`],
    [(k) => (k.powers[0].held.gifts = []), `${ENTRY}.kit.powers[0].held.gifts`],
    [(k) => (k.trinkets[0].target = 1), `${ENTRY}.kit.trinkets[0].target`],
    [(k) => (k.trinkets[0].held = k.powers[0].held), `${ENTRY}.kit.trinkets[0].held`],
  ];
  for (const [change, field] of cases) {
    const rec = realRecord(kitGames[0]);
    change(rec.report.rounds[0].entries[0].kit);
    refused(rec, field, field);
  }
});

Deno.test("the opponent's kit is accepted, never with a held minion", () => {
  const game = kitGames[0] as Json;
  const opp = game.rounds[0].entries.findIndex((e: Json) => e.side === "opponent");
  assertEquals(game.rounds[0].entries[opp].kit.powers.length > 0, true, "the parser saves it");
  const rec = realRecord(game);
  rec.report.rounds[0].entries[opp].kit.powers[0].held = structuredClone(
    game.rounds[0].entries[0].kit.powers[0].held,
  );
  refused(rec, `report.rounds[0].entries[${opp}].kit.powers[0].held`);
});

Deno.test("a kit comes only with a board", () => {
  const rec = realRecord(kitGames[0]);
  rec.report.rounds[0].entries[0].board = null;
  refused(rec, `${ENTRY}.kit`);
  const without = realRecord(kitGames[0]);
  delete without.report.rounds[0].entries[0].kit;
  assertEquals(validateRecord(without, SESSION, 1).ok, true, "a board without a kit (older parser)");
});

Deno.test("replay minions' numbers keep to their shape", () => {
  const report = kitGames[0] as Json;
  const i = report.rounds[0].replay.minions.findIndex((m: Json) => m.counters);
  const at = `report.rounds[0].replay.minions[${i}].counters`;
  const cases: Array<[unknown, string]> = [
    [[0, 0], at],
    [[1], at],
    [[1, 2, 3], at],
    [[-1, 0], `${at}[0]`],
    [["1", 0], `${at}[0]`],
  ];
  for (const [value, field] of cases) {
    const rec = realRecord(report);
    rec.report.rounds[0].replay.minions[i].counters = value;
    refused(rec, field, JSON.stringify(value));
  }
});

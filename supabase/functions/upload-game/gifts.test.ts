// Run: deno test supabase/functions/upload-game/
// Dark Gifts on board and replay minions (T-528, D-107, parser revision 9).
import { type Json, realGifts, realRecord, SESSION } from "./testdata.ts";
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

/** [round, entry, minion] of the first gifted board minion. */
function firstGifted(report: Json): [number, number, number] {
  for (const [r, round] of report.rounds.entries()) {
    for (const [e, entry] of round.entries.entries()) {
      const m = (entry.board ?? []).findIndex((x: Json) => x.gifts);
      if (m >= 0) return [r, e, m];
    }
  }
  throw new Error("setup: no gifted minion");
}

function gifted(report: Json): Json[] {
  return report.rounds.flatMap((r: Json) =>
    r.entries.flatMap((e: Json) => e.board ?? []).filter((m: Json) => m.gifts || m.gift_unknown)
  );
}

Deno.test("a real game with Dark Gifts (revision 9) is accepted", () => {
  const report = realGifts[0];
  const minions = gifted(report);
  assertEquals(minions.some((m: Json) => m.gifts?.length === 2), true, "a minion with two gifts");
  assertEquals(minions.some((m: Json) => m.gift_unknown === true), true, "an unknown gift");
  const replayGifts = report.rounds.some((r: Json) =>
    (r.replay?.minions ?? []).some((m: Json) => m.gifts)
  );
  assertEquals(replayGifts, true, "gifts on replay minions");
  const v = validateRecord(realRecord(report), SESSION, 1);
  if (!v.ok) throw new Error(`refused: ${v.code} ${v.field}`);
  assertEquals(VALIDATOR_VERSION >= 8, true, "a version that accepts gifts");
});

Deno.test("gifts keep to their shape", () => {
  const [r, e, m] = firstGifted(realGifts[0]);
  const at = `report.rounds[${r}].entries[${e}].board[${m}]`;
  const cases: Array<[(x: Json) => void, string]> = [
    [(x) => (x.gifts = []), `${at}.gifts`],
    [(x) => (x.gifts = "BG36_MidGameEffect_000t9"), `${at}.gifts`],
    [(x) => (x.gifts = ["<b>"]), `${at}.gifts`],
    [(x) => (x.gifts = [7]), `${at}.gifts`],
    [(x) => (x.gifts = ["BG36_MidGameEffect_000t9", "BG36_MidGameEffect_000t9"]), `${at}.gifts`],
    [(x) => (x.gifts = new Array(5).fill(0).map((_, i) => `G${i}`)), `${at}.gifts`],
    [(x) => (x.gift_unknown = false), `${at}.gift_unknown`],
    [(x) => (x.gift_unknown = "yes"), `${at}.gift_unknown`],
  ];
  for (const [change, field] of cases) {
    const rec = realRecord(realGifts[0]);
    change(rec.report.rounds[r].entries[e].board[m]);
    refused(rec, field, field);
  }
});

Deno.test("replay minions' gifts keep to their shape", () => {
  const report = realGifts[0];
  const r = report.rounds.findIndex((x: Json) => (x.replay?.minions ?? []).some((m: Json) => m.gifts));
  const i = report.rounds[r].replay.minions.findIndex((m: Json) => m.gifts);
  const at = `report.rounds[${r}].replay.minions[${i}]`;
  const rec = realRecord(report);
  rec.report.rounds[r].replay.minions[i].gifts = ["a b"];
  refused(rec, `${at}.gifts`);
  const ok = realRecord(report);
  delete ok.report.rounds[r].replay.minions[i].gifts;
  assertEquals(validateRecord(ok, SESSION, 1).ok, true, "a replay minion without gifts");
});

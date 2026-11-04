// "Today and this week" (T-361): the viewer's day and week as calendar days,
// and the summary of the games in them. Made-up games; no player's data.
import { test } from "node:test";
import assert from "node:assert/strict";
import { addDays, localDay, periodBounds, REVISIT, summarize, summarizePeriod } from "../../src/lib/data/period.ts";
import { gameRow, type GameRow } from "../../src/lib/data/rows.ts";
import { gameRaw } from "../helpers/rows.ts";

let counter = 0;
/** A game of `mode` played on `day` with `place` (null: not finished). */
function game(day: string | null, place: number | null, mode: "solo" | "duos" = "solo", status = "ok"): GameRow {
  counter += 1;
  return gameRow(
    gameRaw({
      public_id: `00000000-0000-4000-8000-${String(counter).padStart(12, "0")}`,
      game_index: counter,
      played_on: day,
      counted_place: place,
      mode,
      status: place === null && status === "ok" ? "incomplete" : status,
    }),
  );
}

// Monday 2026-10-19 to Sunday 2026-10-25; "today" is a Wednesday.
const WED = "2026-10-21";

test("the week of a Wednesday runs from its Monday to the Sunday", () => {
  assert.deepEqual(periodBounds(WED), { today: WED, weekStart: "2026-10-19", weekEnd: "2026-10-25" });
});

test("Monday is the first day of its week and Sunday the last", () => {
  assert.deepEqual(periodBounds("2026-10-19"), { today: "2026-10-19", weekStart: "2026-10-19", weekEnd: "2026-10-25" });
  assert.deepEqual(periodBounds("2026-10-25"), { today: "2026-10-25", weekStart: "2026-10-19", weekEnd: "2026-10-25" });
  assert.equal(periodBounds("2026-10-26").weekStart, "2026-10-26", "the next midnight starts another week");
});

test("a week can start in the month or the year before", () => {
  assert.deepEqual(periodBounds("2026-10-01"), { today: "2026-10-01", weekStart: "2026-09-28", weekEnd: "2026-10-04" });
  assert.deepEqual(periodBounds("2027-01-01"), { today: "2027-01-01", weekStart: "2026-12-28", weekEnd: "2027-01-03" });
  assert.equal(addDays("2028-02-28", 1), "2028-02-29", "a leap day");
  assert.equal(addDays("2026-10-25", 1), "2026-10-26");
});

test("a day that is not a calendar day is refused", () => {
  assert.throws(() => periodBounds("2026-10"), RangeError);
});

test("the day is the viewer's own: local midnight and the last second before it", () => {
  assert.equal(localDay(new Date(2026, 9, 25, 23, 59, 59)), "2026-10-25");
  assert.equal(localDay(new Date(2026, 9, 26, 0, 0, 0)), "2026-10-26");
});

test("a named moment is a different day in different time zones", () => {
  const zone = process.env.TZ;
  const now = new Date("2026-10-24T11:30:00Z");
  try {
    process.env.TZ = "Pacific/Auckland";
    assert.equal(localDay(now), "2026-10-25", "00:30 on Sunday 25 October, summer time");
    process.env.TZ = "America/Los_Angeles";
    assert.equal(localDay(now), "2026-10-24", "04:30 on Saturday");
    // 2026-10-25T11:30Z: Monday 26th 00:30 in Auckland (a new week), still Sunday 25th 04:30 in Los Angeles.
    const sunday = new Date("2026-10-25T11:30:00Z");
    process.env.TZ = "Pacific/Auckland";
    assert.equal(periodBounds(localDay(sunday)).weekStart, "2026-10-26");
    process.env.TZ = "America/Los_Angeles";
    assert.equal(periodBounds(localDay(sunday)).weekStart, "2026-10-19");
  } finally {
    if (zone === undefined) delete process.env.TZ;
    else process.env.TZ = zone;
  }
});

test("a mode with no game in the stretch has no row, and no games at all has none", () => {
  assert.equal(summarize([], "solo", "2026-10-19", "2026-10-25"), null);
  const out = summarizePeriod([game("2026-10-12", 2)], periodBounds(WED));
  assert.deepEqual([out.today, out.week], [[], []]);
});

test("a game with no day is in neither today nor the week", () => {
  const out = summarizePeriod([game(null, 1), game(WED, 3)], periodBounds(WED));
  assert.equal(out.week[0].games, 1);
});

test("games, average place and record per mode, today and this week", () => {
  const rows = [
    game(WED, 1), // newest first, as the API gives them
    game(WED, 5),
    game("2026-10-19", 3),
    game("2026-10-20", 2, "duos"),
    game("2026-10-18", 8), // the Sunday before: last week's
    game("2026-10-26", 8), // the Monday after: next week's
  ];
  const out = summarizePeriod(rows, periodBounds(WED));
  const solo = out.week.find((p) => p.mode === "solo");
  assert.deepEqual([solo?.games, solo?.placed, solo?.avgPlace, solo?.topHalf, solo?.topHalfGames], [3, 3, 3, 4, 2]);
  const duos = out.week.find((p) => p.mode === "duos");
  assert.deepEqual([duos?.games, duos?.avgPlace, duos?.topHalf, duos?.topHalfGames], [1, 2, 2, 1]);
  assert.deepEqual(out.today.map((p) => [p.mode, p.games, p.avgPlace]), [["solo", 2, 3]], "Duos was played on Tuesday only");
});

test("one mode only gives one row", () => {
  const out = summarizePeriod([game(WED, 4, "duos")], periodBounds(WED));
  assert.deepEqual(out.week.map((p) => p.mode), ["duos"]);
  assert.equal(out.week[0].topHalfGames, 0, "4th of 4 is the bottom half");
});

test("games with no place count as games, not in the average or the record", () => {
  const row = summarize([game(WED, null), game(WED, 2), game(WED, 6)], "solo", WED, WED);
  assert.deepEqual([row?.games, row?.placed, row?.avgPlace, row?.topHalfGames], [3, 2, 4, 1]);
  const none = summarize([game(WED, null)], "solo", WED, WED);
  assert.deepEqual([none?.games, none?.placed, none?.avgPlace, none?.topHalfGames, none?.revisit], [1, 0, null, 0, []]);
});

test("revisit: the best place, a close loss and a big swing, each a different game", () => {
  // Oldest to newest: 4, 8, 5, 1 (listed newest first).
  const rows = [game(WED, 1), game(WED, 5), game(WED, 8), game(WED, 4)];
  const r = summarize(rows, "solo", WED, WED)?.revisit ?? [];
  assert.deepEqual(r.map((x) => [x.reason, x.place]), [["best", 1], ["close", 5], ["swing", 8]]);
  assert.equal(r[0].game, rows[0]);
  assert.ok(r.length <= REVISIT);
});

test("revisit: a game that fits two reasons shows once, and a close loss in Duos is third", () => {
  assert.deepEqual(summarize([game(WED, 5)], "solo", WED, WED)?.revisit.map((x) => x.reason), ["best"]);
  const duos = summarize([game(WED, 4, "duos"), game(WED, 3, "duos"), game(WED, 1, "duos")], "duos", WED, WED)?.revisit ?? [];
  assert.deepEqual(duos.map((x) => [x.reason, x.place]), [["best", 1], ["close", 3]]);
});

test("revisit: ties go to the newest game, and no swing when every place is near the average", () => {
  // Newest first; oldest to newest: 3, 7, 3, 7, 5. Average 5: the 3s and 7s are two away.
  const rows = [game(WED, 5), game(WED, 7), game(WED, 3), game(WED, 7), game(WED, 3)];
  const r = summarize(rows, "solo", WED, WED)?.revisit ?? [];
  const at = (reason: string) => rows.indexOf(r.find((x) => x.reason === reason)?.game as GameRow);
  assert.equal(at("best"), 2, "the newest of the two 3s");
  assert.equal(at("close"), 0);
  assert.equal(at("swing"), 1, "the newest of the 7s");
  const calm = summarize([game(WED, 3), game(WED, 4), game(WED, 4), game(WED, 3)], "solo", WED, WED)?.revisit ?? [];
  assert.deepEqual(calm.map((x) => x.reason), ["best"]);
});

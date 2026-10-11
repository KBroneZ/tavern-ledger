// The public leaderboards page (T-506): row checks, the boards' values and
// labels, the period in the address, the reads with a stubbed server, and
// the page body (tables, empty boards, explanations, profile links).
import { afterEach, test } from "node:test";
import assert from "node:assert/strict";
import { makeClient } from "../../src/lib/api.ts";
import { DataError } from "../../src/lib/data/rows.ts";
import { loadLeaderboards, type Leaderboards } from "../../src/lib/leaderboards/api.ts";
import {
  BOARD_KEYS,
  BOARDS,
  boardRow,
  boardTitle,
  DEFAULT_PERIOD,
  FEW_GAMES,
  FEW_HERO_GAMES,
  fewGames,
  formatValue,
  groupBoards,
  parsePeriod,
  refreshedText,
  valueLabel,
  withPeriod,
  type BoardDef,
  type BoardKey,
} from "../../src/lib/leaderboards/boards.ts";
import { LB_STRINGS, lt } from "../../src/lib/leaderboards/strings.ts";
import { aboutFold, boardSection, modePanels, rowCells, updatedLine } from "../../src/lib/leaderboards/view.ts";
import { elements, textOf, toHtml } from "../helpers/html.ts";
import { json, memoryStorage, stubFetch } from "../helpers/fetchStub.ts";

const CONFIG = { url: "http://127.0.0.1:54321", anonKey: "sb_publishable_test" };
const A = "0f8fad5b-d9cb-469f-a165-70867728950e";
const B = "1b4e28ba-2fa1-11d2-883f-0016d3cca427";
const realFetch = globalThis.fetch;

afterEach(() => {
  globalThis.fetch = realFetch;
});

const def = (key: BoardKey): BoardDef => BOARDS.find((d) => d.key === key) as BoardDef;

function raw(over: Record<string, unknown> = {}): Record<string, unknown> {
  return { board: "avg_place", standing: 1, user_id: A, display_name: "Keeper", hero: null, value: 3.5, games: 25, ...over };
}

// ------------------------------------------------------------------ rows

test("a row is checked and named", () => {
  assert.deepEqual(boardRow(raw(), "solo"), { board: "avg_place", standing: 1, userId: A, name: "Keeper", hero: null, value: 3.5, games: 25 });
  assert.equal(boardRow(raw({ board: "hero_avg_place", hero: "BG20_HERO_202" }), "solo").hero, "BG20_HERO_202");
});

test("anything unexpected in a row throws instead of showing", () => {
  const bad: Record<string, unknown>[] = [
    { board: "rating" },
    { board: "hero_avg_place" },
    { hero: "BG20_HERO_202" },
    { hero: "<script>" , board: "hero_avg_place" },
    { user_id: "not-a-uuid" },
    { display_name: "Name#1234" },
    { display_name: "" },
    { display_name: null },
    { display_name: "x".repeat(33) },
    { value: "3.5" },
    { value: null },
    { value: -1 },
    { value: Number.NaN },
    { standing: 0 },
    { standing: 1.5 },
    { games: 0 },
  ];
  for (const over of bad) assert.throws(() => boardRow(raw(over), "solo"), DataError, JSON.stringify(over));
  assert.throws(() => boardRow("row", "solo"), DataError);
});

test("a value outside its board's range throws", () => {
  for (const [over, m] of [
    [{ value: 0.5 }, "solo"],
    [{ value: 8.5 }, "solo"],
    [{ value: 4.5 }, "duos"],
    [{ board: "hero_avg_place", hero: "BG20_HERO_202", value: 9 }, "solo"],
    [{ board: "win_rate", value: 100.1 }, "solo"],
    [{ board: "top_half_rate", value: 250 }, "duos"],
    [{ board: "most_golden", value: 8 }, "solo"],
  ] as const) {
    assert.throws(() => boardRow(raw(over), m), DataError, JSON.stringify(over));
  }
  assert.equal(boardRow(raw({ value: 8 }), "solo").value, 8);
  assert.equal(boardRow(raw({ value: 4 }), "duos").value, 4);
  assert.equal(boardRow(raw({ board: "win_rate", value: 100 }), "solo").value, 100);
  assert.equal(boardRow(raw({ board: "games", value: 1200 }), "solo").value, 1200);
});

test("rows are grouped per board in the server's order, every board present", () => {
  const rows = [boardRow(raw(), "solo"), boardRow(raw({ board: "games", value: 30 }), "solo"), boardRow(raw({ user_id: B, standing: 2, value: 4 }), "solo")];
  const boards = groupBoards(rows);
  assert.deepEqual([...boards.keys()], [...BOARD_KEYS]);
  assert.deepEqual(boards.get("avg_place")?.map((r) => r.userId), [A, B]);
  assert.equal(boards.get("apm")?.length, 0);
});

// ------------------------------------------------------------------ values and labels

test("values read as the server rounded them", () => {
  assert.equal(formatValue(def("avg_place"), 4.4), "4.40");
  assert.equal(formatValue(def("top_half_rate"), 52), "52.0%");
  assert.equal(formatValue(def("win_rate"), 12.5), "12.5%");
  assert.equal(formatValue(def("games"), 125), "125");
  assert.equal(formatValue(def("apm"), 12.5), "12.50");
  assert.equal(formatValue(def("biggest_board"), 125000), "125K");
});

test("the top half reads top 4 in Solo and top 2 in Duos", () => {
  assert.equal(boardTitle(def("top_half_rate"), "solo"), "Top 4 rate");
  assert.equal(boardTitle(def("top_half_rate"), "duos"), "Top 2 rate");
  assert.equal(valueLabel(def("top_half_rate"), "duos"), "Top 2");
  assert.equal(boardTitle(def("top_half_streak"), "duos"), "Longest top 2 streak");
  assert.equal(boardTitle(def("avg_place"), "duos"), "Average place");
});

test("six main boards in Simple, six more in Advanced, each with its strings", () => {
  assert.equal(BOARDS.filter((d) => d.simple).length, 6);
  assert.equal(BOARDS.filter((d) => !d.simple).length, 6);
  assert.deepEqual(BOARDS.map((d) => d.key), [...BOARD_KEYS]);
  for (const d of BOARDS) {
    for (const m of ["solo", "duos"] as const) {
      assert.ok(boardTitle(d, m).length > 0);
      assert.ok(valueLabel(d, m).length > 0);
    }
    assert.ok(`board.${d.key}.how` in LB_STRINGS, d.key);
  }
});

test("no string promises a rating or names other players", () => {
  for (const text of Object.values(LB_STRINGS)) {
    assert.doesNotMatch(text, /\bmmr\b|battletag(?!s)/i, text);
  }
  assert.throws(() => lt("board.how.count"), /needs/);
});

// ------------------------------------------------------------------ the period in the address

test("the period comes from the address, 30 days by default", () => {
  assert.equal(DEFAULT_PERIOD, "30d");
  assert.equal(parsePeriod(""), "30d");
  assert.equal(parsePeriod("?period=7d"), "7d");
  assert.equal(parsePeriod("?period=all&mode=duos"), "all");
  assert.equal(parsePeriod("?period=365d"), "30d");
  assert.equal(withPeriod("https://tavernledger.net/leaderboards/?mode=duos", "7d"), "https://tavernledger.net/leaderboards/?mode=duos&period=7d");
  assert.equal(withPeriod("https://tavernledger.net/leaderboards/?period=7d", "30d"), "https://tavernledger.net/leaderboards/");
});

test("the rebuild time reads in UTC; nothing else is accepted", () => {
  assert.equal(refreshedText("2026-10-11T14:20:31.5+00:00"), "11 Oct 2026, 14:20 UTC");
  assert.equal(refreshedText(null), null);
  assert.equal(refreshedText("soon"), null);
  assert.equal(updatedLine("2026-10-11T14:20:00Z"), "Updated 11 Oct 2026, 14:20 UTC.");
  assert.equal(updatedLine(null), "Not built yet.");
});

// ------------------------------------------------------------------ reads

test("reads every mode and period once, each answer in its own place, plus the rebuild time", async () => {
  const seen = stubFetch({
    "/rest/v1/rpc/leaderboard": async (_url, init) => {
      const args = JSON.parse(String(init.body)) as { p_mode: string; p_period: string };
      // One row per call, named after the mode and period it was asked for.
      return json([raw({ display_name: `${args.p_mode} ${args.p_period}`, value: 2 })]);
    },
    "/rest/v1/rpc/leaderboard_refreshed_at": () => json("2026-10-11T14:20:00+00:00"),
  });
  const data = await loadLeaderboards(makeClient(CONFIG, memoryStorage()));
  assert.equal(seen.filter((s) => s.startsWith("POST /rest/v1/rpc/leaderboard")).length, 7);
  assert.equal(data.refreshedAt, "2026-10-11T14:20:00+00:00");
  for (const m of ["solo", "duos"] as const) {
    for (const p of ["7d", "30d", "all"] as const) {
      assert.equal(data.boards[m][p].get("avg_place")?.[0].name, `${m} ${p}`);
      assert.equal(data.boards[m][p].get("games")?.length, 0);
    }
  }
});

test("a failed read, a bad row, too many rows or a bad time throws", async () => {
  const cases: Record<string, () => Response>[] = [
    { board: () => json({ message: "boom" }, 500), time: () => json(null) },
    { board: () => json([raw({ user_id: "x" })]), time: () => json(null) },
    { board: () => json(Array.from({ length: 51 }, (_, i) => raw({ standing: i + 1 }))), time: () => json(null) },
    { board: () => json([]), time: () => json(42) },
    { board: () => json([]), time: () => json("soon") },
    { board: () => json([]), time: () => json({ message: "boom" }, 500) },
  ];
  for (const c of cases) {
    stubFetch({ "/rest/v1/rpc/leaderboard": c.board, "/rest/v1/rpc/leaderboard_refreshed_at": c.time });
    await assert.rejects(loadLeaderboards(makeClient(CONFIG, memoryStorage())), DataError);
  }
  stubFetch({ "/rest/v1/rpc/leaderboard": () => json([]), "/rest/v1/rpc/leaderboard_refreshed_at": () => json(null) });
  assert.equal((await loadLeaderboards(makeClient(CONFIG, memoryStorage()))).refreshedAt, null);
});

// ------------------------------------------------------------------ the page body

test("a row: standing, the name linked to the public profile, value, games", () => {
  const cells = rowCells(def("avg_place"), boardRow(raw(), "solo"));
  assert.equal(cells.length, 4);
  assert.equal(cells[0], 1);
  assert.equal(toHtml((cells[1] as { node: never }).node), `<a href="/profile/?id=${A}">Keeper</a>`);
  assert.equal(cells[2], "3.50");
  assert.equal(cells[3], 25);
  const hero = rowCells(def("hero_avg_place"), boardRow(raw({ board: "hero_avg_place", hero: "BG20_HERO_202" }), "solo"));
  assert.equal(hero.length, 5);
  assert.match(textOf((hero[2] as { node: never }).node), /^\s*Master Nguyen$/);
});

test("a name is text, never markup", () => {
  const html = toHtml(boardSection(def("games"), [boardRow(raw({ board: "games", display_name: "a_b.c-d" }), "solo")], "solo", "7d"));
  assert.match(html, />a_b\.c-d</);
  assert.doesNotMatch(html, /<script/);
});

test("a board: titled section, table with a caption, top 10 then Show all, and how it is counted", () => {
  const rows = Array.from({ length: 12 }, (_, i) => boardRow(raw({ standing: i + 1, value: 2 + i / 10 }), "solo"));
  const section = boardSection(def("avg_place"), rows, "duos", "30d");
  const html = toHtml(section);
  assert.match(html, /<section class="lb-board" aria-labelledby="lb-duos-30d-avg_place"><h3 id="lb-duos-30d-avg_place">Average place<\/h3>/);
  assert.match(html, /<caption>Average place, Duos, the last 30 days<\/caption>/);
  assert.equal(elements(section).filter((e) => e.tag === "tr" && e.attrs.some(([n, v]) => n === "class" && v === "more")).length, 2);
  assert.match(html, /Show all 12/);
  const text = textOf(section);
  assert.match(text, /How this is counted/);
  assert.match(text, /listed from their first one/);
  assert.match(text, /fewer than 20 games is marked "few games"/);
  assert.match(text, /Self-reported/);
  assert.match(text, /Equal values share a place/);
});

test("an empty board says so plainly, with no minimum", () => {
  for (const key of ["apm", "hero_avg_place"] as const) {
    const text = textOf(boardSection(def(key), [], "solo", "7d"));
    assert.match(text, /No public players yet/);
    assert.match(text, /No player with a public profile has a finished game for this board in the last 7 days/);
    assert.doesNotMatch(text, /20|10|at least|minimum/);
  }
});

test("the hero board's line is games with the hero", () => {
  const row = boardRow(raw({ board: "hero_avg_place", hero: "BG20_HERO_202" }), "solo");
  const listed = textOf(boardSection(def("hero_avg_place"), [row], "solo", "30d"));
  assert.match(listed, /first game with a hero; a value from fewer than 10 games with the hero is marked "few games"/);
  assert.doesNotMatch(listed, /at least \d+ of them/);
});

// ------------------------------------------------------------------ few games (D-103)

test("the few-games line is D-082's old minimum: 20 games, 10 with the hero", () => {
  assert.equal(FEW_GAMES, 20);
  assert.equal(FEW_HERO_GAMES, 10);
  assert.equal(fewGames(boardRow(raw({ games: 1 }), "solo")), true);
  assert.equal(fewGames(boardRow(raw({ games: 19 }), "solo")), true);
  assert.equal(fewGames(boardRow(raw({ games: 20 }), "solo")), false);
  const hero = (games: number) => boardRow(raw({ board: "hero_avg_place", hero: "BG20_HERO_202", games }), "solo");
  assert.equal(fewGames(hero(9)), true);
  assert.equal(fewGames(hero(10)), false);
  assert.equal(fewGames(boardRow(raw({ board: "games", value: 1, games: 1 }), "solo")), false, "the games board is the count itself");
});

test("a row under the line is shown, with its games and the few-games mark; one over it has no mark", () => {
  const one = rowCells(def("win_rate"), boardRow(raw({ board: "win_rate", value: 100, games: 1 }), "solo"));
  const value = toHtml((one[2] as { node: never }).node);
  assert.match(value, /100\.0%/);
  assert.match(value, /<span class="mark-src mark-few" title="Rests on fewer than 20 games: it can change a lot with the next one\.">few games<\/span>/);
  assert.equal(one[3], 1);
  const many = rowCells(def("win_rate"), boardRow(raw({ board: "win_rate", value: 50, games: 20 }), "solo"));
  assert.equal(many[2], "50.0%");
  const hero = rowCells(def("hero_avg_place"), boardRow(raw({ board: "hero_avg_place", hero: "BG20_HERO_202", games: 9 }), "solo"));
  assert.match(toHtml((hero[3] as { node: never }).node), /fewer than 10 games with this hero/);
});

test("rows keep the server's order: by value, ties by more games, few-game rows not moved or hidden", () => {
  const rows = [
    boardRow(raw({ standing: 1, value: 1, games: 25, display_name: "Many" }), "solo"),
    boardRow(raw({ standing: 1, value: 1, games: 1, user_id: B, display_name: "One" }), "solo"),
  ];
  const text = textOf(boardSection(def("avg_place"), rows, "solo", "7d"));
  assert.ok(text.indexOf("Many") < text.indexOf("One"));
  assert.equal(text.match(/few games/g)?.length, 2, "the row's mark and the explanation");
});

test("both modes, three periods each; Advanced boards marked for the switch", () => {
  const empty = () => groupBoards([]);
  const data: Leaderboards = {
    refreshedAt: null,
    boards: {
      solo: { "7d": groupBoards([boardRow(raw(), "solo")]), "30d": empty(), all: empty() },
      duos: { "7d": empty(), "30d": empty(), all: empty() },
    },
  };
  const panels = modePanels(data);
  assert.deepEqual(panels.map((p) => p.attrs), [[["data-mode-panel", "solo"]], [["data-mode-panel", "duos"]]]);
  const all = panels.flatMap(elements);
  assert.equal(all.filter((e) => e.attrs.some(([n]) => n === "data-period-panel")).length, 6);
  assert.equal(all.filter((e) => e.attrs.some(([n, v]) => n === "class" && v === "lb-board")).length, 72);
  assert.equal(all.filter((e) => e.attrs.some(([n, v]) => n === "class" && v === "lb-group adv")).length, 6);
  const ids = all.flatMap((e) => e.attrs.filter(([n]) => n === "id").map(([, v]) => v));
  assert.equal(new Set(ids).size, ids.length, "ids are unique across panels");
});

test("the about fold says who is listed, how to opt out and links the privacy policy", () => {
  const fold = aboutFold();
  const text = textOf(fold);
  assert.match(text, /Public profile/);
  assert.match(text, /as soon as they save/);
  assert.match(text, /every 10 minutes/);
  assert.match(text, /every public player with a finished game/);
  assert.match(text, /fewer than 20 games \(10 with the hero on the hero board\) is marked "few games"/);
  assert.match(text, /no ratings, no BattleTags/);
  assert.match(toHtml(fold), /href="\/privacy\/#public-profiles-and-leaderboards"/);
});

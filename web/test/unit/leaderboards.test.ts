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
  assert.deepEqual(boardRow(raw()), { board: "avg_place", standing: 1, userId: A, name: "Keeper", hero: null, value: 3.5, games: 25 });
  assert.equal(boardRow(raw({ board: "hero_avg_place", hero: "BG20_HERO_202" })).hero, "BG20_HERO_202");
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
  for (const over of bad) assert.throws(() => boardRow(raw(over)), DataError, JSON.stringify(over));
  assert.throws(() => boardRow("row"), DataError);
});

test("rows are grouped per board in the server's order, every board present", () => {
  const rows = [boardRow(raw()), boardRow(raw({ board: "games", value: 30 })), boardRow(raw({ user_id: B, standing: 2, value: 4 }))];
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
  assert.throws(() => lt("board.how.min"), /needs/);
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

test("reads every mode and period once, plus the rebuild time", async () => {
  const seen = stubFetch({
    "/rest/v1/rpc/leaderboard": async (_url, init) => {
      const args = JSON.parse(String(init.body)) as { p_mode: string; p_period: string };
      return json(args.p_mode === "solo" && args.p_period === "7d" ? [raw(), raw({ board: "games", value: 25 })] : []);
    },
    "/rest/v1/rpc/leaderboard_refreshed_at": () => json("2026-10-11T14:20:00+00:00"),
  });
  const data = await loadLeaderboards(makeClient(CONFIG, memoryStorage()));
  assert.equal(seen.filter((s) => s.startsWith("POST /rest/v1/rpc/leaderboard")).length, 7);
  assert.equal(data.refreshedAt, "2026-10-11T14:20:00+00:00");
  assert.equal(data.boards.solo["7d"].get("avg_place")?.[0].name, "Keeper");
  assert.equal(data.boards.solo["7d"].get("games")?.[0].value, 25);
  assert.equal(data.boards.duos.all.get("avg_place")?.length, 0);
});

test("a failed read, a bad row, too many rows or a bad time throws", async () => {
  const cases: Record<string, () => Response>[] = [
    { board: () => json({ message: "boom" }, 500), time: () => json(null) },
    { board: () => json([raw({ user_id: "x" })]), time: () => json(null) },
    { board: () => json(Array.from({ length: 51 }, (_, i) => raw({ standing: i + 1 }))), time: () => json(null) },
    { board: () => json([]), time: () => json(42) },
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
  const cells = rowCells(def("avg_place"), boardRow(raw()));
  assert.equal(cells.length, 4);
  assert.equal(cells[0], 1);
  assert.equal(toHtml((cells[1] as { node: never }).node), `<a href="/profile/?id=${A}">Keeper</a>`);
  assert.equal(cells[2], "3.50");
  assert.equal(cells[3], 25);
  const hero = rowCells(def("hero_avg_place"), boardRow(raw({ board: "hero_avg_place", hero: "BG20_HERO_202" })));
  assert.equal(hero.length, 5);
  assert.match(textOf((hero[2] as { node: never }).node), /BG20_HERO_202/);
});

test("a name is text, never markup", () => {
  const html = toHtml(boardSection(def("games"), [boardRow(raw({ board: "games", display_name: "a_b.c-d" }))], "solo", "7d"));
  assert.match(html, />a_b\.c-d</);
  assert.doesNotMatch(html, /<script/);
});

test("a board: titled section, table with a caption, top 10 then Show all, and how it is counted", () => {
  const rows = Array.from({ length: 12 }, (_, i) => boardRow(raw({ standing: i + 1, value: 2 + i / 10 })));
  const section = boardSection(def("avg_place"), rows, "duos", "30d");
  const html = toHtml(section);
  assert.match(html, /<section class="lb-board" aria-labelledby="lb-duos-30d-avg_place"><h3 id="lb-duos-30d-avg_place">Average place<\/h3>/);
  assert.match(html, /<caption>Average place, Duos, the last 30 days<\/caption>/);
  assert.equal(elements(section).filter((e) => e.tag === "tr" && e.attrs.some(([n, v]) => n === "class" && v === "more")).length, 2);
  assert.match(html, /Show all 12/);
  const text = textOf(section);
  assert.match(text, /How this is counted/);
  assert.match(text, /at least 20/);
  assert.match(text, /Self-reported/);
  assert.match(text, /Equal values share a place/);
});

test("an empty board says why", () => {
  const text = textOf(boardSection(def("apm"), [], "solo", "7d"));
  assert.match(text, /Nobody here yet/);
  assert.match(text, /20 finished games in the last 7 days/);
});

test("both modes, three periods each; Advanced boards marked for the switch", () => {
  const empty = () => groupBoards([]);
  const data: Leaderboards = {
    refreshedAt: null,
    boards: {
      solo: { "7d": groupBoards([boardRow(raw())]), "30d": empty(), all: empty() },
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
  assert.match(text, /at once/);
  assert.match(text, /every 10 minutes/);
  assert.match(text, /20 finished games/);
  assert.match(text, /no ratings, no BattleTags/);
  assert.match(toHtml(fold), /href="\/privacy\/#public-profiles-and-leaderboards"/);
});

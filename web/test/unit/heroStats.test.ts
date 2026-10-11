// The heroes page's data shaping (T-505): rows read as untrusted input, the
// two sorts, the "few games" marks, pick rates with both counts, and links
// to the games list that keep the hero, the mode and the profile.
import { test } from "node:test";
import assert from "node:assert/strict";
import { DataError } from "../../src/lib/data/rows.ts";
import {
  heroGamesHref,
  heroStat,
  heroStatCells,
  hasHeroes,
  offeredOnly,
  parseSort,
  pickCell,
  sortHeroes,
  statsPagePath,
  withSort,
  type HeroStat,
} from "../../src/lib/heroes/heroes.ts";
import { toHtml } from "../helpers/html.ts";
import { heroRaw, type RawRow } from "../helpers/rows.ts";

const ID = "0f8fad5b-d9cb-469f-a165-70867728950e";

function raw(over: RawRow = {}): RawRow {
  return heroRaw({ worst_place: 7, offered_games: 4, picked_games: 3, ...over });
}

const stat = (over: RawRow = {}): HeroStat => heroStat(raw(over));

test("a hero row keeps the worst place, offered and picked games", () => {
  const r = stat();
  assert.equal(r.worstPlace, 7);
  assert.equal(r.offeredGames, 4);
  assert.equal(r.pickedGames, 3);
  assert.equal(r.bestPlace, 1);
});

test("counts that cannot hold together refuse the answer", () => {
  assert.throws(() => stat({ picked_games: 5, offered_games: 4 }), DataError);
  assert.throws(() => stat({ games: 2, placed_games: 2, top_half: 1, wins: 1, picked_games: 3, offered_games: 9 }), DataError);
  assert.throws(() => stat({ placed_games: 6 }), DataError);
  assert.throws(() => stat({ top_half: 6 }), DataError);
  assert.throws(() => stat({ wins: 4, top_half: 3 }), DataError);
  assert.throws(() => stat({ offered_games: -1 }), DataError);
  assert.throws(() => stat({ offered_games: "many" }), DataError);
});

test("a place out of range is unknown, never a number", () => {
  assert.equal(stat({ worst_place: 9 }).worstPlace, null);
  assert.equal(stat({ mode: "duos", worst_place: 5, best_place: 1, avg_place: 2 }).worstPlace, null);
});

const rows = [
  stat({ hero: "BG_A", games: 12, placed_games: 12, avg_place: 4.1, top_half: 6, wins: 1 }),
  stat({ hero: "BG_B", games: 3, placed_games: 3, avg_place: 1.3, top_half: 3, wins: 2 }),
  stat({ hero: "BG_C", games: 6, placed_games: 6, avg_place: 2.5, top_half: 4, wins: 1 }),
  stat({ hero: "BG_D", games: 6, placed_games: 6, avg_place: 2.5, top_half: 4, wins: 1 }),
  stat({ hero: "BG_E", games: 1, placed_games: 0, avg_place: null, top_half: 0, wins: 0, best_place: null, worst_place: null, picked_games: 0 }),
  stat({ hero: "BG_F", games: 0, placed_games: 0, avg_place: null, top_half: 0, wins: 0, best_place: null, worst_place: null, offered_games: 3, picked_games: 0 }),
  stat({ mode: "duos", hero: "BG_A", games: 2, placed_games: 2, avg_place: 2, top_half: 1, wins: 0, worst_place: 3, picked_games: 1 }),
];

test("by games: most games first, then the better average, then the card id", () => {
  assert.deepEqual(sortHeroes(rows, "solo", "games").map((r) => r.hero), ["BG_A", "BG_C", "BG_D", "BG_B", "BG_E"]);
});

test("by average place: enough games first, then few, then none; heroes never played are left out", () => {
  assert.deepEqual(sortHeroes(rows, "solo", "avg").map((r) => r.hero), ["BG_C", "BG_D", "BG_A", "BG_B", "BG_E"]);
  assert.deepEqual(sortHeroes(rows, "duos", "avg").map((r) => r.hero), ["BG_A"]);
});

test("sorting leaves the input as it was", () => {
  const before = rows.map((r) => r.hero);
  sortHeroes(rows, "solo", "avg");
  assert.deepEqual(rows.map((r) => r.hero), before);
});

test("heroes offered and never played are listed apart, most offered first", () => {
  assert.deepEqual(offeredOnly(rows, "solo").map((r) => r.hero), ["BG_F"]);
  assert.deepEqual(offeredOnly(rows, "duos"), []);
  assert.equal(hasHeroes(rows, "solo"), true);
  assert.equal(hasHeroes([rows[5]], "solo"), false);
});

test("the sort lives in the address; games is the default and leaves none", () => {
  assert.equal(parseSort("?sort=avg"), "avg");
  assert.equal(parseSort("?sort=bogus"), "games");
  assert.equal(parseSort(""), "games");
  assert.equal(withSort("https://x.test/player-heroes/?id=1&mode=duos#t", "avg"), "/player-heroes/?id=1&mode=duos&sort=avg#t");
  assert.equal(withSort("https://x.test/player-heroes/?sort=avg", "games"), "/player-heroes/");
});

test("pick rate: the rate with both counts; marked on few choices; unknown with none", () => {
  const cell = (r: HeroStat) => {
    const c = pickCell(r);
    return typeof c === "object" && "node" in c ? [c.node].flat().map(toHtml).join("") : String(c);
  };
  assert.match(cell(stat({ offered_games: 10, picked_games: 4 })), /^40% · 4 of 10$/);
  assert.match(cell(stat({ offered_games: 4, picked_games: 3 })), /75% · 3 of 4.*few games/);
  assert.match(cell(stat({ offered_games: 0, picked_games: 0 })), /Unknown/);
});

test("a hero's cells: link to its games, few-games marks, wins, best and worst", () => {
  const few = heroStatCells(rows[1], null);
  assert.deepEqual(few[0], { hero: "BG_B", name: "Unknown hero", href: "/games/?mode=solo&hero=BG_B", unknownId: "BG_B" });
  assert.match(JSON.stringify(few[2]), /few games/);
  assert.match(JSON.stringify(few[3]), /few games/);
  const enough = heroStatCells(rows[0], ID);
  assert.equal(enough[2], "4.10");
  assert.equal(enough[3], "50%");
  assert.equal(enough[4], "1");
  assert.deepEqual(enough[5], { place: 1, mode: "solo" });
  assert.deepEqual(enough[6], { place: 7, mode: "solo" });
  assert.match(String((enough[0] as { href: string }).href), new RegExp(`^/games/\\?id=${ID}&mode=solo&hero=BG_A$`));
  // No finished game: no average, no rate, no wins (unknown, never 0), and the games say so.
  const none = heroStatCells(rows[4], null);
  assert.equal(none[1], "1 (0 placed)");
  for (const i of [2, 3, 4]) assert.match(JSON.stringify(none[i]), /Unknown/, String(i));
});

test("no link for a hero with no card id", () => {
  assert.equal(heroGamesHref(null, "solo", null), undefined);
});

test("the stats pages' addresses keep the profile and the mode", () => {
  assert.equal(statsPagePath("heroes", "solo", null), "/player-heroes/");
  assert.equal(statsPagePath("curious", "duos", null), "/curious/?mode=duos");
  assert.equal(statsPagePath("heroes", "duos", ID), `/player-heroes/?id=${ID}&mode=duos`);
});

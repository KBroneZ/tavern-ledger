// The dashboard's and the games list's bodies (T-503): key numbers in
// Simple, the rest behind Advanced, a panel per mode, the states, and links
// that keep the profile.
import { test } from "node:test";
import assert from "node:assert/strict";
import { DEFAULT_FILTERS } from "../../src/lib/data/filters.ts";
import { gameRow, heroRow, overview, recordRow, type RecordRow } from "../../src/lib/data/rows.ts";
import type { Dashboard } from "../../src/lib/dataApi.ts";
import { dashboardPanels, isEmpty, modePanel } from "../../src/lib/view/dashboardView.ts";
import { listBody } from "../../src/lib/view/gamesView.ts";
import type { VNode } from "../../src/lib/view/node.ts";
import { elements, textOf, toHtml } from "../helpers/html.ts";
import { gameRaw, heroRaw, overviewRow, recordRaw } from "../helpers/rows.ts";

const ID = "0f8fad5b-d9cb-469f-a165-70867728950e";
const opts = { gamesHref: (m: string) => `/games/?mode=${m}`, owner: true, userId: ID };

function dashboard(withDuos = true): Dashboard {
  return {
    modes: {
      solo: { overview: overview(overviewRow()), places: [3, 1, null, 5], total: 31, lastGames: [gameRow(gameRaw())] },
      duos: withDuos
        ? { overview: overview(overviewRow({ mode: "duos", avg_place: 2 })), places: [2, 1], total: 31, lastGames: [] }
        : { overview: null, places: [], total: 0, lastGames: [] },
    },
    heroes: [heroRow(heroRaw())],
    records: [recordRow(recordRaw())].filter((r): r is RecordRow => r !== null),
  };
}

const byClass = (node: VNode, name: string) => elements(node).filter((e) => e.attrs.some(([k, v]) => k === "class" && v.split(" ").includes(name)));
const attr = (node: VNode, name: string) => node.attrs.find(([k]) => k === name)?.[1];

test("one panel per mode, marked for the mode switch", () => {
  const panels = dashboardPanels(dashboard(), opts);
  assert.deepEqual(panels.map((p) => attr(p, "data-mode-panel")), ["solo", "duos"]);
});

test("Simple shows five key numbers; damage, the economy and the rest are Advanced only (T-559)", () => {
  const solo = modePanel("solo", dashboard(), opts);
  const [simple, advanced] = byClass(solo, "tiles");
  assert.equal(byClass(simple, "tile").length, 5);
  assert.match(textOf(simple), /Combats won/);
  assert.equal(attr(advanced, "class"), "tiles adv");
  assert.match(textOf(advanced), /Gold spent per game/);
  assert.equal(byClass(solo, "simple-only").length, 1);
});

test("the panel has the charts, last games, heroes and records, and links to the games list", () => {
  const html = toHtml(modePanel("solo", dashboard(), opts));
  for (const part of ["chart-places", "chart-trend", "Last game (Solo)", "Heroes by games played (Solo)", "Best values and when they were set (Solo)", 'href="/games/?mode=solo"']) {
    assert.ok(html.includes(part), part);
  }
});

test("ids are unique across both panels", () => {
  const ids = dashboardPanels(dashboard(), opts).flatMap(elements).map((e) => attr(e, "id")).filter(Boolean);
  assert.equal(new Set(ids).size, ids.length, ids.join(", "));
});

test("a mode with no games says so; a player with none at all is empty", () => {
  const dash = dashboard(false);
  assert.match(textOf(modePanel("duos", dash, opts)), /No Duos games yet/);
  assert.equal(isEmpty(dash), false);
  assert.equal(isEmpty({ ...dash, modes: { ...dash.modes, solo: { ...dash.modes.solo, overview: null } } }), true);
});

test("no records and no heroes say so instead of an empty table", () => {
  const html = toHtml(modePanel("solo", { ...dashboard(), records: [], heroes: [] }, opts));
  assert.match(html, /No records yet/);
  assert.match(html, /No heroes recorded yet/);
});

test("the charts say when they count only the latest games", () => {
  const dash = dashboard();
  const big = { ...dash, modes: { ...dash.modes, solo: { ...dash.modes.solo, total: 5000 } } };
  assert.match(textOf(modePanel("solo", big, opts)), /cover the last 1000 games/);
  assert.doesNotMatch(textOf(modePanel("solo", dash, opts)), /count the last/);
});

const listPage = (over = {}) => ({ rows: [gameRow(gameRaw()), gameRow(gameRaw({ mode: "duos", counted_place: 1 }))], total: 60, filters: { ...DEFAULT_FILTERS, page: 2 }, profileId: ID, userId: ID, invalid: [], ...over });

test("games list: count, rows with the mode when every mode is listed, and pages that keep the profile and filters", () => {
  const html = listBody(listPage()).map(toHtml).join("");
  assert.match(html, /Games 26–27 of 60/);
  assert.match(html, /<th scope="col">Mode<\/th>/);
  assert.match(html, /Page 2 of 3/);
  assert.ok(html.includes(`href="/games/?id=${ID}" rel="prev"`), html);
  assert.ok(html.includes(`href="/games/?id=${ID}&amp;page=3" rel="next"`), html);
});

test("games list: one mode leaves the mode column out; one page needs no pager", () => {
  const html = listBody(listPage({ total: 2, filters: { ...DEFAULT_FILTERS, mode: "solo" }, profileId: null })).map(toHtml).join("");
  assert.doesNotMatch(html, />Mode</);
  assert.doesNotMatch(html, /class="pager"/);
});

test("games list: no match, a page past the end and filters left out each say so", () => {
  assert.match(listBody(listPage({ rows: [], total: 0 })).map(textOf).join(" "), /No games match these filters/);
  const past = listBody(listPage({ rows: [], total: 30, filters: { ...DEFAULT_FILTERS, page: 9 } })).map(toHtml).join("");
  assert.match(past, /No games on this page/);
  assert.ok(past.includes(`href="/games/?id=${ID}"`));
  const bad = listBody(listPage({ invalid: ["hero", "from", "from"] })).map(textOf).join(" ");
  assert.match(bad, /were ignored: hero, from\./);
});

test("games list: one game on the page reads as one game", () => {
  const html = listBody(listPage({ rows: [gameRow(gameRaw())], total: 26 })).map(textOf).join(" ");
  assert.match(html, /Game 26 of 26/);
});

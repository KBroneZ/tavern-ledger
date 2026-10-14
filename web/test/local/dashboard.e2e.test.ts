// End-to-end run of the data pages' reads (T-503) against the local Supabase
// stack, with the same functions the pages use: the owner's dashboard and
// games list, a private profile hidden from everyone else, and the same
// numbers for everyone once it is public. Synthetic users and games only.
//
// Skipped unless TAVERN_SUPABASE_LOCAL=1 and `npx supabase start` is running.
// Keys are read from `npx supabase status` into memory and never printed.
//
//   $env:TAVERN_SUPABASE_LOCAL = "1"; npm run test:local

import { after, before, describe, test } from "node:test";
import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { randomBytes } from "node:crypto";
import { join } from "node:path";
import type { SupabaseClient } from "@supabase/supabase-js";
import { makeClient, saveProfile, signIn } from "../../src/lib/api.ts";
import type { SiteConfig } from "../../src/lib/config.ts";
import { loadDashboard, loadGames, loadHeroes, profileAccess } from "../../src/lib/dataApi.ts";
import { DEFAULT_FILTERS, parseFilters } from "../../src/lib/data/filters.ts";
import { memoryStorage } from "../helpers/fetchStub.ts";
import { DUMMY_CAPTCHA } from "../helpers/captcha.ts";

const ENABLED = process.env.TAVERN_SUPABASE_LOCAL === "1";
const REPO = join(import.meta.dirname, "..", "..", "..");
const SESSION = "Hearthstone_2026_10_01_20_00_00";

interface Stack {
  url: string;
  anon: string;
  service: string;
}

function stack(): Stack {
  const out = execFileSync("npx supabase status -o json", { cwd: REPO, encoding: "utf8", shell: true, timeout: 120_000 });
  const s = JSON.parse(out.slice(out.indexOf("{")));
  return { url: s.API_URL, anon: s.ANON_KEY, service: s.SERVICE_ROLE_KEY };
}

// Six Solo games (one cut off) and two Duos games, all made up.
const GAMES = [
  { i: 1, type: "GT_BATTLEGROUNDS", hero: "TB_BaconShop_HERO_01", place: 1, day: "2026-10-01", status: "ok" },
  { i: 2, type: "GT_BATTLEGROUNDS", hero: "TB_BaconShop_HERO_01", place: 3, day: "2026-10-02", status: "ok" },
  { i: 3, type: "GT_BATTLEGROUNDS", hero: "TB_BaconShop_HERO_02", place: 5, day: "2026-10-03", status: "ok" },
  { i: 4, type: "GT_BATTLEGROUNDS", hero: "TB_BaconShop_HERO_02_SKIN_A", place: 8, day: "2026-10-04", status: "ok" },
  { i: 5, type: "GT_BATTLEGROUNDS", hero: "TB_BaconShop_HERO_03", place: null, day: "2026-10-05", status: "incomplete" },
  { i: 6, type: "GT_BATTLEGROUNDS", hero: "TB_BaconShop_HERO_03", place: 2, day: "2026-10-06", status: "ok" },
  { i: 7, type: "GT_BATTLEGROUNDS_DUO", hero: "TB_BaconShop_HERO_04", place: 1, day: "2026-10-07", status: "ok" },
  { i: 8, type: "GT_BATTLEGROUNDS_DUO", hero: "TB_BaconShop_HERO_04", place: 3, day: "2026-10-08", status: "ok" },
];

describe("data pages on the local stack", { skip: !ENABLED && "set TAVERN_SUPABASE_LOCAL=1" }, () => {
  const s = ENABLED ? stack() : ({} as Stack);
  const config: SiteConfig = { url: s.url, anonKey: s.anon };
  const users: { id: string; email: string; password: string; client: SupabaseClient }[] = [];
  const service = (path: string, init: RequestInit = {}) =>
    fetch(`${s.url}${path}`, {
      ...init,
      headers: { apikey: s.service, Authorization: `Bearer ${s.service}`, "Content-Type": "application/json", ...(init.headers ?? {}) },
    });
  const owner = () => users[0];
  const anon = () => makeClient(config, memoryStorage());

  async function makeUser(): Promise<void> {
    const email = `web-dash-${randomBytes(6).toString("hex")}@example.test`;
    const password = randomBytes(12).toString("base64url");
    const res = await service("/auth/v1/admin/users", { method: "POST", body: JSON.stringify({ email, password, email_confirm: true }) });
    assert.equal(res.status, 200);
    const { id } = (await res.json()) as { id: string };
    const client = makeClient(config, memoryStorage());
    assert.equal(await signIn(client, email, password, DUMMY_CAPTCHA), null);
    users.push({ id, email, password, client });
  }

  before(async () => {
    if (!ENABLED) return;
    await makeUser();
    await makeUser();
    const games = GAMES.map((g) => ({
      user_id: owner().id,
      session: SESSION,
      game_index: g.i,
      game_type: g.type,
      status: g.status,
      hero_card_id: g.hero,
      final_place: g.place,
      build: 12345,
      played_on: g.day,
      tribes_offered: {},
      saved_at: 1_760_000_000 + g.i,
      content_sha256: "0".repeat(64),
      size_bytes: 100,
    }));
    assert.equal((await service("/rest/v1/games", { method: "POST", body: JSON.stringify(games) })).status, 201);
    // Summaries for the finished Solo games only; the rest have none (unknown).
    const stats = [1, 2, 3, 4, 6].map((i) => ({
      user_id: owner().id,
      session: SESSION,
      game_index: i,
      summary_version: 1,
      rounds: 10 + i,
      combats_won: 5 + i,
      combats_lost: 5,
      combats_tied: 0,
      combats_unknown: 0,
      combats_inferred: i === 3 ? 2 : 0,
      win_streak: 3,
      gold_spent: 100 + i,
      gold_spent_turns: 10,
      shop_turns: 10,
      rolls: 10,
      free_rolls: 1,
      buys: 20,
      sells: 5,
      apm: i === 6 ? null : 30 + i,
      actions: i === 6 ? null : 300,
      triples: 2,
    }));
    assert.equal((await service("/rest/v1/game_stats", { method: "POST", body: JSON.stringify(stats) })).status, 201);
  });

  after(async () => {
    if (!ENABLED) return;
    // Deleting a user takes their profile, games and summaries with it.
    for (const u of users) await service(`/auth/v1/admin/users/${u.id}`, { method: "DELETE" });
  });

  test("the owner's dashboard: places, rates and totals from the views", async () => {
    assert.deepEqual(await profileAccess(owner().client, owner().id, owner().id), { kind: "own-private", name: null });
    const dash = await loadDashboard(owner().client, owner().id);
    const solo = dash.modes.solo.overview;
    assert.ok(solo);
    assert.equal(solo.games, 6);
    assert.equal(solo.placedGames, 5);
    assert.equal(solo.incompleteGames, 1);
    assert.equal(solo.avgPlace, 3.8);
    assert.equal(solo.topHalf, 3);
    assert.equal(solo.wins, 1);
    assert.equal(solo.goldSpentGames, 5);
    assert.equal(solo.apmGames, 4, "a game without APM is left out, not counted as 0");
    assert.deepEqual(dash.modes.solo.places, [2, null, 8, 5, 3, 1], "newest first; the cut-off game has no place");
    assert.equal(dash.modes.solo.lastGames[0].place, 2);
    assert.equal(dash.modes.duos.overview?.games, 2);
    assert.equal(dash.modes.duos.overview?.topHalf, 1);
    assert.equal(dash.modes.duos.lastGames[0].rounds, null, "no summary: unknown");
    const skins = dash.heroes.find((h) => h.hero === "TB_BaconShop_HERO_02");
    assert.equal(skins?.games, 2, "a skin counts under its base hero");
    assert.ok(dash.records.some((r) => r.record === "win_streak" && r.value === 3));
  });

  test("a private profile shows nothing to anyone else, signed in or not", async () => {
    for (const viewer of [anon(), users[1].client]) {
      const viewerId = viewer === users[1].client ? users[1].id : null;
      assert.deepEqual(await profileAccess(viewer, owner().id, viewerId), { kind: "private" });
      const dash = await loadDashboard(viewer, owner().id);
      assert.equal(dash.modes.solo.overview, null);
      assert.equal(dash.modes.duos.overview, null);
      assert.deepEqual(dash.heroes, []);
      assert.deepEqual(dash.records, []);
      assert.deepEqual(await loadGames(viewer, owner().id, DEFAULT_FILTERS), { rows: [], total: 0 });
    }
  });

  test("once public, everyone reads the same numbers as the owner", async () => {
    assert.equal(await saveProfile(owner().client, owner().id, { displayName: "Dash Tester", isPublic: true }), null);
    assert.deepEqual(await profileAccess(anon(), owner().id, null), { kind: "public", name: "Dash Tester", own: false });
    const mine = await loadDashboard(owner().client, owner().id);
    assert.deepEqual(await loadDashboard(anon(), owner().id), mine);
    assert.deepEqual(await loadDashboard(users[1].client, owner().id), mine);
    assert.equal(await saveProfile(owner().client, owner().id, { displayName: "Dash Tester", isPublic: false }), null);
    assert.deepEqual(await profileAccess(anon(), owner().id, null), { kind: "private" });
  });

  test("the games list: filters, pages and the total", async () => {
    const c = owner().client;
    const id = owner().id;
    const all = await loadGames(c, id, DEFAULT_FILTERS);
    assert.equal(all.total, 8);
    assert.deepEqual(all.rows.map((g) => g.playedOn), GAMES.map((g) => g.day).reverse());
    const of = async (q: string) => loadGames(c, id, parseFilters(q).filters);
    assert.equal((await of("?mode=solo")).total, 6);
    assert.equal((await of("?mode=duos&place=first")).total, 1);
    assert.equal((await of("?mode=solo&place=top")).total, 3);
    assert.equal((await of("?place=bottom")).total, 3, "5th and 8th in Solo, 3rd in Duos");
    assert.equal((await of("?place=none")).total, 1);
    assert.equal((await of("?hero=TB_BaconShop_HERO_02")).total, 2);
    assert.equal((await of("?from=2026-10-03&to=2026-10-05")).total, 3);
    assert.deepEqual(await of("?page=2"), { rows: [], total: 8 }, "past the last page: no rows, the total stays");
    assert.deepEqual((await loadHeroes(c, id)).slice(0, 3).sort(), ["TB_BaconShop_HERO_01", "TB_BaconShop_HERO_02", "TB_BaconShop_HERO_03"]);
  });
});

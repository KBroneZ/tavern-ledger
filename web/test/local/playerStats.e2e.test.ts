// End-to-end run of the heroes and curious stats pages' reads (T-505)
// against the local Supabase stack, with the functions the pages use: the
// columns they ask for exist, the pick counts and records with their games
// come back as the views compute them, and a private profile shows nothing
// to anyone else. Synthetic users and games only.
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
import { loadCurious, loadHeroStats } from "../../src/lib/heroes/load.ts";
import { sortHeroes } from "../../src/lib/heroes/heroes.ts";
import { memoryStorage } from "../helpers/fetchStub.ts";

const ENABLED = process.env.TAVERN_SUPABASE_LOCAL === "1";
const REPO = join(import.meta.dirname, "..", "..", "..");
const SESSION = "Hearthstone_2026_10_01_20_00_00";
const [H1, H2, H3] = ["TB_BaconShop_HERO_11", "TB_BaconShop_HERO_12", "TB_BaconShop_HERO_13"];

function stack(): { url: string; anon: string; service: string } {
  const out = execFileSync("npx supabase status -o json", { cwd: REPO, encoding: "utf8", shell: true, timeout: 120_000 });
  const s = JSON.parse(out.slice(out.indexOf("{")));
  return { url: s.API_URL, anon: s.ANON_KEY, service: s.SERVICE_ROLE_KEY };
}

// Three Solo games, all made up: H1 twice (offered both times), H2 once; H3 offered, never played.
const GAMES = [
  { i: 1, hero: H1, place: 2, day: "2026-10-01", offered: [H1, H2, H3], dealt: 9 },
  { i: 2, hero: H1, place: 6, day: "2026-10-02", offered: [H1, H3], dealt: 14 },
  { i: 3, hero: H2, place: 1, day: "2026-10-03", offered: null, dealt: 14 },
];

describe("heroes and curious stats on the local stack", { skip: !ENABLED && "set TAVERN_SUPABASE_LOCAL=1" }, () => {
  const s = ENABLED ? stack() : { url: "", anon: "", service: "" };
  const config: SiteConfig = { url: s.url, anonKey: s.anon };
  const users: { id: string; client: SupabaseClient }[] = [];
  const service = (path: string, init: RequestInit = {}) =>
    fetch(`${s.url}${path}`, {
      ...init,
      headers: { apikey: s.service, Authorization: `Bearer ${s.service}`, "Content-Type": "application/json", ...(init.headers ?? {}) },
    });
  const owner = () => users[0];
  const anon = () => makeClient(config, memoryStorage());

  async function makeUser(): Promise<void> {
    const email = `web-stats-${randomBytes(6).toString("hex")}@example.test`;
    const password = randomBytes(12).toString("base64url");
    const res = await service("/auth/v1/admin/users", { method: "POST", body: JSON.stringify({ email, password, email_confirm: true }) });
    assert.equal(res.status, 200);
    const { id } = (await res.json()) as { id: string };
    const client = makeClient(config, memoryStorage());
    assert.equal(await signIn(client, email, password), null);
    users.push({ id, client });
  }

  before(async () => {
    if (!ENABLED) return;
    await makeUser();
    await makeUser();
    const games = GAMES.map((g) => ({
      user_id: owner().id,
      session: SESSION,
      game_index: g.i,
      game_type: "GT_BATTLEGROUNDS",
      status: "ok",
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
    const stats = GAMES.map((g) => ({
      user_id: owner().id,
      session: SESSION,
      game_index: g.i,
      summary_version: 1,
      rounds: 12,
      damage_dealt: g.dealt * 2,
      damage_dealt_combats: 4,
      damage_dealt_max: g.dealt,
      gold_spent: 100,
      gold_spent_turns: 10,
      shop_turns: 10,
      heroes_offered: g.offered,
    }));
    assert.equal((await service("/rest/v1/game_stats", { method: "POST", body: JSON.stringify(stats) })).status, 201);
  });

  after(async () => {
    if (!ENABLED) return;
    // Deleting a user takes their profile, games and summaries with it.
    for (const u of users) await service(`/auth/v1/admin/users/${u.id}`, { method: "DELETE" });
  });

  test("heroes: places, worst place and pick counts from user_heroes", async () => {
    const rows = await loadHeroStats(owner().client, owner().id);
    const h1 = rows.find((r) => r.hero === H1);
    assert.ok(h1);
    assert.deepEqual([h1.games, h1.placedGames, h1.avgPlace, h1.bestPlace, h1.worstPlace], [2, 2, 4, 2, 6]);
    assert.deepEqual([h1.offeredGames, h1.pickedGames], [2, 2]);
    const h2 = rows.find((r) => r.hero === H2);
    assert.deepEqual([h2?.offeredGames, h2?.pickedGames], [1, 0], "played in a game with no recorded choice: not a pick");
    const h3 = rows.find((r) => r.hero === H3);
    assert.deepEqual([h3?.games, h3?.offeredGames], [0, 2], "offered, never played");
    assert.deepEqual(sortHeroes(rows, "solo", "games").map((r) => r.hero), [H1, H2]);
  });

  test("curious: records with the first game that reached them, and the totals", async () => {
    const data = await loadCurious(owner().client, owner().id);
    const dealt = data.records.find((r) => r.mode === "solo" && r.record === "damage_dealt_max");
    assert.ok(dealt);
    assert.deepEqual([dealt.value, dealt.session, dealt.gameIndex, dealt.games, dealt.inferred], [14, SESSION, 2, 3, true]);
    const solo = data.overviews.find((o) => o.mode === "solo");
    assert.deepEqual([solo?.goldSpent, solo?.goldSpentGames, solo?.games], [300, 3, 3]);
  });

  test("a private profile shows nothing to anyone else; public, the same as the owner", async () => {
    for (const viewer of [anon(), users[1].client]) {
      assert.deepEqual(await loadHeroStats(viewer, owner().id), []);
      assert.deepEqual(await loadCurious(viewer, owner().id), { overviews: [], records: [] });
    }
    assert.equal(await saveProfile(owner().client, owner().id, { displayName: "Stats Tester", isPublic: true }), null);
    assert.deepEqual(await loadHeroStats(anon(), owner().id), await loadHeroStats(owner().client, owner().id));
    assert.deepEqual(await loadCurious(users[1].client, owner().id), await loadCurious(owner().client, owner().id));
    assert.equal(await saveProfile(owner().client, owner().id, { displayName: "Stats Tester", isPublic: false }), null);
    assert.deepEqual(await loadHeroStats(anon(), owner().id), []);
  });
});

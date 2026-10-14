// End-to-end run of the community hero stats (T-507) against the local
// Supabase stack, with the functions the page and the account page use: the
// switch is off until a player turns it on, a hero shows only once enough
// opted-in players have played it, a player who did not opt in never counts,
// nobody can read anyone else's choice, the export has it, and turning it off
// takes the games out at the next rebuild. Synthetic users and games only.
//
// Skipped unless TAVERN_SUPABASE_LOCAL=1 and `npx supabase start` is running.
// Keys are read from `npx supabase status` into memory and never printed.
// The stats are rebuilt by pg_cron once a day; the test rebuilds them
// at once through the database container (docker exec, local only).
//
//   $env:TAVERN_SUPABASE_LOCAL = "1"; npm run test:local

import { after, before, describe, test } from "node:test";
import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { randomBytes } from "node:crypto";
import { join } from "node:path";
import type { SupabaseClient } from "@supabase/supabase-js";
import { loadCommunityConsent, makeClient, saveCommunityConsent, saveProfile, signIn } from "../../src/lib/api.ts";
import type { SiteConfig } from "../../src/lib/config.ts";
import { loadCommunityStats } from "../../src/lib/community/api.ts";
import { memoryStorage } from "../helpers/fetchStub.ts";
import { DUMMY_CAPTCHA } from "../helpers/captcha.ts";

const ENABLED = process.env.TAVERN_SUPABASE_LOCAL === "1";
const REPO = join(import.meta.dirname, "..", "..", "..");
const SESSION = "Hearthstone_2026_10_01_20_00_00";
const DB_CONTAINER = "supabase_db_tavern-ledger";

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

function rebuild(): void {
  execFileSync("docker", ["exec", DB_CONTAINER, "psql", "-U", "postgres", "-tAc", "select private.refresh_community_heroes()"], {
    encoding: "utf8",
    timeout: 60_000,
  });
}

/** Yesterday (UTC): the last day a rebuild counts, inside both periods. */
function yesterday(): string {
  return new Date(Date.now() - 86_400_000).toISOString().slice(0, 10);
}

interface User {
  id: string;
  client: SupabaseClient;
}

describe("community hero stats on the local stack", { skip: !ENABLED && "set TAVERN_SUPABASE_LOCAL=1" }, () => {
  const s = ENABLED ? stack() : ({} as Stack);
  const config: SiteConfig = { url: s.url, anonKey: s.anon };
  const tag = randomBytes(3).toString("hex");
  // A hero id of this run only, so other data on the stack cannot change the numbers.
  const HERO = `TL_E2E_${tag}`;
  const OTHER = `TL_E2E_${tag}_B`;
  const users: User[] = [];
  const service = (path: string, init: RequestInit = {}) =>
    fetch(`${s.url}${path}`, {
      ...init,
      headers: { apikey: s.service, Authorization: `Bearer ${s.service}`, "Content-Type": "application/json", ...(init.headers ?? {}) },
    });
  const anon = () => makeClient(config, memoryStorage());

  async function makeUser(isPublic: boolean, games: number, place: number): Promise<User> {
    const email = `web-ch-${randomBytes(6).toString("hex")}@example.test`;
    const password = randomBytes(12).toString("base64url");
    const res = await service("/auth/v1/admin/users", { method: "POST", body: JSON.stringify({ email, password, email_confirm: true }) });
    assert.equal(res.status, 200);
    const { id } = (await res.json()) as { id: string };
    const client = makeClient(config, memoryStorage());
    assert.equal(await signIn(client, email, password, DUMMY_CAPTCHA), null);
    if (isPublic) assert.equal(await saveProfile(client, id, { displayName: `CH ${tag} ${users.length}`, isPublic }), null);
    const day = yesterday();
    const rows = Array.from({ length: games }, (_, k) => ({
      user_id: id,
      session: SESSION,
      game_index: k + 1,
      game_type: "GT_BATTLEGROUNDS",
      status: "ok",
      hero_card_id: HERO,
      final_place: place,
      build: 12345,
      played_on: day,
      tribes_offered: {},
      saved_at: 1_760_000_000 + k,
      content_sha256: "0".repeat(64),
      size_bytes: 100,
    }));
    if (games > 0) {
      assert.equal((await service("/rest/v1/games", { method: "POST", body: JSON.stringify(rows) })).status, 201);
      const stats = rows.map((g) => ({ user_id: id, session: SESSION, game_index: g.game_index, summary_version: 1, rounds: 10, heroes_offered: [HERO, OTHER] }));
      assert.equal((await service("/rest/v1/game_stats", { method: "POST", body: JSON.stringify(stats) })).status, 201);
    }
    const user = { id, client };
    users.push(user);
    return user;
  }

  const ourHero = async (client: SupabaseClient) => (await loadCommunityStats(client)).heroes.solo["30d"].find((r) => r.hero === HERO);

  before(async () => {
    if (!ENABLED) return;
    // Five players, two public and three private, 10 games each in 3rd place;
    // a sixth, public, with 100 wins, who never opts in.
    for (let i = 0; i < 5; i++) await makeUser(i < 2, 10, 3);
    await makeUser(true, 100, 1);
  });

  after(async () => {
    if (!ENABLED) return;
    // Deleting a user takes their profile, choice, games and summaries with it.
    for (const u of users) await service(`/auth/v1/admin/users/${u.id}`, { method: "DELETE" });
    rebuild();
  });

  test("the switch is off for every account until its owner turns it on", async () => {
    for (const u of users) assert.equal(await loadCommunityConsent(u.client, u.id), false);
    rebuild();
    assert.equal(await ourHero(anon()), undefined, "no opted-in games, nothing shown");
  });

  test("five players opt in: the hero shows with its games, players and picks; the sixth never counts", async () => {
    for (const u of users.slice(0, 5)) assert.equal(await saveCommunityConsent(u.client, true), null);
    for (const u of users.slice(0, 5)) assert.equal(await loadCommunityConsent(u.client, u.id), true);
    // Before the rebuild nothing changes on the page.
    assert.equal(await ourHero(anon()), undefined);
    rebuild();
    const row = await ourHero(anon());
    assert.ok(row);
    // Shown floored and rounded: "50+" games from "5+" players.
    assert.equal(row.games, 50);
    assert.equal(row.players, 5);
    assert.equal(row.avgPlace, 3);
    assert.equal(row.topHalfRate, 100);
    assert.equal(row.winRate, 0, "the sixth player's 100 wins are not in it");
    assert.deepEqual(row.pick, { rate: 100, offeredGames: 50, offeredPlayers: 5 });
    // A signed-in reader, the sixth player included, sees the same row.
    assert.deepEqual(await ourHero(users[5].client), row);
    // The other hero was offered 50 times but never played: not a row.
    assert.equal((await loadCommunityStats(anon())).heroes.solo["30d"].find((r) => r.hero === OTHER), undefined);
  });

  test("nobody reads anyone else's choice; anonymous visitors cannot set one", async () => {
    const [first, second] = users;
    assert.equal(await loadCommunityConsent(second.client, first.id), false, "another account sees no row");
    const visitor = anon();
    assert.match((await saveCommunityConsent(visitor, true)) ?? "", /Sign in again|Could not save/);
    const { error } = await visitor.from("community_consent").select("hero_stats");
    assert.ok(error, "anonymous visitors have no right on the table");
  });

  test("the export has the switch's state", async () => {
    const { data, error } = await users[0].client.rpc("export_my_data");
    assert.equal(error, null);
    const consent = (data as { community_consent: { hero_stats: boolean; policy_version: string } }).community_consent;
    assert.equal(consent.hero_stats, true);
    assert.equal(consent.policy_version, "2026-10-11");
    const other = await users[5].client.rpc("export_my_data");
    assert.deepEqual((other.data as { community_consent: unknown }).community_consent, { hero_stats: false, updated_at: null, policy_version: null });
  });

  test("one player turns it off: their games leave at the next rebuild, and the hero with them", async () => {
    const first = users[0];
    assert.equal(await saveCommunityConsent(first.client, false), null);
    assert.equal(await loadCommunityConsent(first.client, first.id), false);
    rebuild();
    assert.equal(await ourHero(anon()), undefined, "40 games from 4 players are not enough");
  });
});

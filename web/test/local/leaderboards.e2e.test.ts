// End-to-end run of the public leaderboards (T-506) against the local
// Supabase stack, with the functions the page and the account page use: a
// public player is listed from the first game (D-103: no minimum) and a row
// on few games is marked, a private one is never listed, a refused display
// name says so, and a profile made private leaves the boards at once.
// Synthetic users and games only.
//
// Skipped unless TAVERN_SUPABASE_LOCAL=1 and `npx supabase start` is running.
// Keys are read from `npx supabase status` into memory and never printed.
// The boards are rebuilt by pg_cron every 10 minutes; the test rebuilds them
// at once through the database container (docker exec, local only).
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
import { loadLeaderboards, type Leaderboards } from "../../src/lib/leaderboards/api.ts";
import { fewGames } from "../../src/lib/leaderboards/boards.ts";
import { memoryStorage } from "../helpers/fetchStub.ts";

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
  execFileSync("docker", ["exec", DB_CONTAINER, "psql", "-U", "postgres", "-tAc", "select private.refresh_leaderboards()"], {
    encoding: "utf8",
    timeout: 60_000,
  });
}

/** Yesterday (UTC), inside every period. */
function yesterday(): string {
  return new Date(Date.now() - 86_400_000).toISOString().slice(0, 10);
}

interface User {
  id: string;
  name: string;
  client: SupabaseClient;
}

describe("leaderboards on the local stack", { skip: !ENABLED && "set TAVERN_SUPABASE_LOCAL=1" }, () => {
  const s = ENABLED ? stack() : ({} as Stack);
  const config: SiteConfig = { url: s.url, anonKey: s.anon };
  const tag = randomBytes(3).toString("hex");
  const users: User[] = [];
  const service = (path: string, init: RequestInit = {}) =>
    fetch(`${s.url}${path}`, {
      ...init,
      headers: { apikey: s.service, Authorization: `Bearer ${s.service}`, "Content-Type": "application/json", ...(init.headers ?? {}) },
    });
  const anon = () => makeClient(config, memoryStorage());

  async function makeUser(name: string, isPublic: boolean, games: number, place: number): Promise<User> {
    const email = `web-lb-${randomBytes(6).toString("hex")}@example.test`;
    const password = randomBytes(12).toString("base64url");
    const res = await service("/auth/v1/admin/users", { method: "POST", body: JSON.stringify({ email, password, email_confirm: true }) });
    assert.equal(res.status, 200);
    const { id } = (await res.json()) as { id: string };
    const client = makeClient(config, memoryStorage());
    assert.equal(await signIn(client, email, password), null);
    assert.equal(await saveProfile(client, id, { displayName: name, isPublic }), null);
    const day = yesterday();
    const rows = Array.from({ length: games }, (_, k) => ({
      user_id: id,
      session: SESSION,
      game_index: k + 1,
      game_type: "GT_BATTLEGROUNDS",
      status: "ok",
      hero_card_id: "TB_BaconShop_HERO_01",
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
      const stats = rows.map((g) => ({ user_id: id, session: SESSION, game_index: g.game_index, summary_version: 1, rounds: 10, apm: 15, triples: 2 }));
      assert.equal((await service("/rest/v1/game_stats", { method: "POST", body: JSON.stringify(stats) })).status, 201);
    }
    const user = { id, name, client };
    users.push(user);
    return user;
  }

  const ids = (data: Leaderboards, board: "avg_place" | "apm" | "hero_avg_place" = "avg_place") =>
    (data.boards.solo["7d"].get(board) ?? []).map((r) => r.userId);

  before(async () => {
    if (!ENABLED) return;
    await makeUser(`LB Public ${tag}`, true, 20, 2);
    await makeUser(`LB Private ${tag}`, false, 25, 1);
    await makeUser(`LB Few ${tag}`, true, 19, 1);
    rebuild();
  });

  after(async () => {
    if (!ENABLED) return;
    // Deleting a user takes their profile, games, summaries and board rows with it.
    for (const u of users) await service(`/auth/v1/admin/users/${u.id}`, { method: "DELETE" });
  });

  test("anonymous: public players are listed with the games behind each value, few games marked; no private one", async () => {
    const [pub, priv, few] = users;
    const data = await loadLeaderboards(anon());
    assert.ok(data.refreshedAt, "the rebuild time is there");
    for (const board of ["avg_place", "apm", "hero_avg_place"] as const) {
      const row = (data.boards.solo["7d"].get(board) ?? []).find((r) => r.userId === pub.id);
      assert.ok(row, board);
      assert.equal(row.name, pub.name);
      assert.equal(row.games, 20);
      assert.equal(fewGames(row), false, `${board}: 20 games is not few`);
    }
    // D-103: 19 games are listed (no minimum), marked "few games" (under 20), and 19 > 10 on the hero board.
    for (const board of ["avg_place", "apm", "hero_avg_place"] as const) {
      const row = (data.boards.solo["7d"].get(board) ?? []).find((r) => r.userId === few.id);
      assert.ok(row, `${board}: a public player with 19 games is listed`);
      assert.equal(row.games, 19);
      assert.equal(fewGames(row), board !== "hero_avg_place", board);
    }
    for (const period of ["7d", "30d", "all"] as const) {
      const listed = [...data.boards.solo[period].values()].flat().map((r) => r.userId);
      assert.ok(listed.includes(pub.id), period);
      assert.ok(!listed.includes(priv.id), `${period}: a private profile is never listed`);
      assert.ok(listed.includes(few.id), `${period}: no minimum, 19 games are listed`);
    }
    assert.ok(!ids(data).includes(priv.id));
  });

  test("another signed-in user, and the private owner, read the same boards", async () => {
    const [pub, priv] = users;
    for (const reader of [priv.client, pub.client]) {
      const data = await loadLeaderboards(reader);
      assert.ok(ids(data).includes(pub.id));
      assert.ok(!ids(data).includes(priv.id));
    }
  });

  test("the account page: a listed word is refused with a clear message", async () => {
    const few = users[2];
    const message = await saveProfile(few.client, few.id, { displayName: "Blizzard Official", isPublic: true });
    assert.match(message ?? "", /not allowed/i);
    assert.equal(await saveProfile(few.client, few.id, { displayName: `LB Few ${tag}`, isPublic: true }), null);
  });

  test("a profile made private leaves every board at once, before any rebuild", async () => {
    const pub = users[0];
    assert.equal(await saveProfile(pub.client, pub.id, { displayName: pub.name, isPublic: false }), null);
    const data = await loadLeaderboards(anon());
    const listed = (["solo", "duos"] as const).flatMap((m) => (["7d", "30d", "all"] as const).flatMap((p) => [...data.boards[m][p].values()].flat()));
    assert.ok(!listed.some((r) => r.userId === pub.id));
    // Public again: back at the next rebuild, not before.
    assert.equal(await saveProfile(pub.client, pub.id, { displayName: pub.name, isPublic: true }), null);
    assert.ok(!ids(await loadLeaderboards(anon())).includes(pub.id));
    rebuild();
    assert.ok(ids(await loadLeaderboards(anon())).includes(pub.id));
  });
});

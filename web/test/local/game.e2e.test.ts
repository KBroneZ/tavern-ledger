// End-to-end check of the game viewer's read (T-504) against the local
// Supabase stack, with the same function the page uses (loadGame over the
// client's Storage download): the owner reads their game; anyone reads it
// once the profile is public; a private game, another user's private game and
// a missing game all give the same answer.
//
// Skipped unless TAVERN_SUPABASE_LOCAL=1 and `npx supabase start` is running.
// Keys are read from `npx supabase status` into memory and never printed.
// Both users are made up (example.test) and removed even if a step fails.
//
//   $env:TAVERN_SUPABASE_LOCAL = "1"; npm run test:local

import { after, before, describe, test } from "node:test";
import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { randomBytes, randomUUID } from "node:crypto";
import { gzipSync } from "node:zlib";
import { join } from "node:path";
import type { SupabaseClient, SupportedStorage } from "@supabase/supabase-js";
import { makeClient, saveProfile, signIn } from "../../src/lib/api.ts";
import type { SiteConfig } from "../../src/lib/config.ts";
import { type Download, loadGame } from "../../src/lib/game/load.ts";
import { storagePath } from "../../src/lib/game/url.ts";
import { INDEX, record, SESSION } from "../unit/gameFixtures.ts";

const ENABLED = process.env.TAVERN_SUPABASE_LOCAL === "1";
const REPO = join(import.meta.dirname, "..", "..", "..");

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

function memoryStorage(): SupportedStorage {
  const items = new Map<string, string>();
  return {
    getItem: (k) => items.get(k) ?? null,
    setItem: (k, v) => void items.set(k, v),
    removeItem: (k) => void items.delete(k),
  };
}

const downloadWith = (client: SupabaseClient): Download => (path) => client.storage.from("games").download(path);

describe("game viewer read on the local stack", { skip: !ENABLED && "set TAVERN_SUPABASE_LOCAL=1" }, () => {
  const s = ENABLED ? stack() : ({} as Stack);
  const config: SiteConfig = { url: s.url, anonKey: s.anon };
  const password = randomBytes(12).toString("base64url");
  const users: { id: string; email: string; client: SupabaseClient }[] = [];
  const service = (path: string, init: RequestInit = {}) =>
    fetch(`${s.url}${path}`, {
      ...init,
      headers: { apikey: s.service, Authorization: `Bearer ${s.service}`, ...(init.headers ?? {}) },
    });
  const ref = () => ({ userId: users[0].id, session: SESSION, index: INDEX });

  before(async () => {
    for (const name of ["owner", "other"]) {
      const email = `web-game-${name}-${randomBytes(6).toString("hex")}@example.test`;
      const res = await service("/auth/v1/admin/users", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ email, password, email_confirm: true }),
      });
      assert.equal(res.status, 200, "user created");
      const id = ((await res.json()) as { id: string }).id;
      const client = makeClient(config, memoryStorage());
      assert.equal(await signIn(client, email, password), null);
      users.push({ id, email, client });
    }
    const gz = gzipSync(JSON.stringify(record()));
    const row = {
      user_id: users[0].id,
      session: SESSION,
      game_index: INDEX,
      game_type: "GT_BATTLEGROUNDS",
      status: "ok",
      hero_card_id: "BG22_HERO_002",
      final_place: 3,
      build: 250000,
      played_on: "2026-10-09",
      tribes_offered: { BEAST: 3 },
      saved_at: 1_791_000_000,
      content_sha256: "0".repeat(64),
      size_bytes: gz.length,
    };
    const ins = await service("/rest/v1/games", { method: "POST", headers: { "Content-Type": "application/json" }, body: JSON.stringify(row) });
    assert.equal(ins.status, 201);
    const up = await service(`/storage/v1/object/games/${storagePath(ref())}`, {
      method: "POST",
      headers: { "Content-Type": "application/gzip" },
      body: gz,
    });
    assert.equal(up.status, 200);
  });

  after(async () => {
    if (users[0]) {
      await service(`/storage/v1/object/games`, {
        method: "DELETE",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ prefixes: [storagePath(ref())] }),
      });
    }
    for (const u of users) await service(`/auth/v1/admin/users/${u.id}`, { method: "DELETE" });
  });

  test("the owner reads their private game in full", async () => {
    const result = await loadGame(downloadWith(users[0].client), ref());
    assert.equal(result.kind, "ok");
    if (result.kind === "ok") {
      assert.equal(result.record.report.hero, "BG22_HERO_002");
      assert.equal(result.record.unreadable, 0);
    }
  });

  test("private: an anonymous visitor and another user get the same answer as a missing game", async () => {
    const anon = makeClient(config, memoryStorage());
    const missing = await loadGame(downloadWith(anon), { ...ref(), userId: randomUUID() });
    assert.deepEqual(missing, { kind: "unavailable" });
    assert.deepEqual(await loadGame(downloadWith(anon), ref()), missing);
    assert.deepEqual(await loadGame(downloadWith(users[1].client), ref()), missing);
    assert.deepEqual(await loadGame(downloadWith(users[0].client), { ...ref(), index: INDEX + 1 }), missing);
  });

  test("public: anyone reads the game once the owner's profile is public, and not after it is private again", async () => {
    assert.equal(await saveProfile(users[0].client, users[0].id, { displayName: "Game E2E", isPublic: true }), null);
    const anon = makeClient(config, memoryStorage());
    assert.equal((await loadGame(downloadWith(anon), ref())).kind, "ok");
    assert.equal((await loadGame(downloadWith(users[1].client), ref())).kind, "ok");
    assert.equal(await saveProfile(users[0].client, users[0].id, { displayName: "Game E2E", isPublic: false }), null);
    assert.deepEqual(await loadGame(downloadWith(anon), ref()), { kind: "unavailable" });
  });
});

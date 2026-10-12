// End-to-end run of the "improve the simulator" switch (T-547) against the
// local Supabase stack, with the functions the account page uses: off until
// the owner turns it on, nobody else can read or set it, the owner's list
// (service role) holds only the games of accounts whose switch is on now, the
// export has it, and turning it off takes the games out of the list at once.
// Synthetic users and games only.
//
// Skipped unless TAVERN_SUPABASE_LOCAL=1 and `npx supabase start` is running.
// Keys are read from `npx supabase status` into memory and never printed.
//
//   $env:TAVERN_SUPABASE_LOCAL = "1"; npm run test:local

import { after, before, describe, test } from "node:test";
import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { randomBytes } from "node:crypto";
import { gzipSync } from "node:zlib";
import { join } from "node:path";
import type { SupabaseClient } from "@supabase/supabase-js";
import { loadSimulatorConsent, makeClient, saveSimulatorConsent, signIn } from "../../src/lib/api.ts";
import type { SiteConfig } from "../../src/lib/config.ts";
import { memoryStorage } from "../helpers/fetchStub.ts";

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

interface User {
  id: string;
  client: SupabaseClient;
  path: string;
}

describe("the simulator switch on the local stack", { skip: !ENABLED && "set TAVERN_SUPABASE_LOCAL=1" }, () => {
  const s = ENABLED ? stack() : ({} as Stack);
  const config: SiteConfig = { url: s.url, anonKey: s.anon };
  const users: User[] = [];
  const service = (path: string, init: RequestInit = {}) =>
    fetch(`${s.url}${path}`, {
      ...init,
      headers: { apikey: s.service, Authorization: `Bearer ${s.service}`, "Content-Type": "application/json", ...(init.headers ?? {}) },
    });
  const anon = () => makeClient(config, memoryStorage());

  /** The owner's list: storage paths of the games that may be used now. */
  async function listed(): Promise<string[]> {
    const res = await service("/rest/v1/rpc/simulator_files", { method: "POST", body: JSON.stringify({ p_limit: 10000 }) });
    assert.equal(res.status, 200);
    return ((await res.json()) as { storage_path: string }[]).map((r) => r.storage_path);
  }

  /** A signed-in user with one game and its file. */
  async function makeUser(): Promise<User> {
    const email = `web-sc-${randomBytes(6).toString("hex")}@example.test`;
    const password = randomBytes(12).toString("base64url");
    const res = await service("/auth/v1/admin/users", { method: "POST", body: JSON.stringify({ email, password, email_confirm: true }) });
    assert.equal(res.status, 200);
    const { id } = (await res.json()) as { id: string };
    const client = makeClient(config, memoryStorage());
    assert.equal(await signIn(client, email, password), null);
    const game = {
      user_id: id,
      session: SESSION,
      game_index: 1,
      game_type: "GT_BATTLEGROUNDS_DUO",
      status: "ok",
      played_on: new Date().toISOString().slice(0, 10),
      saved_at: 1_760_000_000,
      content_sha256: "0".repeat(64),
      size_bytes: 100,
    };
    assert.equal((await service("/rest/v1/games", { method: "POST", body: JSON.stringify(game) })).status, 201);
    const path = `${id}/${SESSION}-1.json.gz`;
    const put = await service(`/storage/v1/object/games/${path}`, {
      method: "POST",
      body: gzipSync(Buffer.from("{}")),
      headers: { "Content-Type": "application/gzip" },
    });
    assert.equal(put.status, 200);
    const user = { id, client, path };
    users.push(user);
    return user;
  }

  before(async () => {
    if (!ENABLED) return;
    await makeUser();
    await makeUser();
  });

  after(async () => {
    if (!ENABLED) return;
    for (const u of users) {
      await service("/storage/v1/object/games", { method: "DELETE", body: JSON.stringify({ prefixes: [u.path] }) });
      await service(`/auth/v1/admin/users/${u.id}`, { method: "DELETE" });
    }
  });

  test("the switch is off for a new account, and its games are not listed", async () => {
    for (const u of users) assert.equal(await loadSimulatorConsent(u.client, u.id), false);
    const paths = await listed();
    for (const u of users) assert.ok(!paths.includes(u.path));
  });

  test("the owner turns it on: only that account's game is listed", async () => {
    const [first, second] = users;
    assert.equal(await saveSimulatorConsent(first.client, true), null);
    assert.equal(await loadSimulatorConsent(first.client, first.id), true);
    const paths = await listed();
    assert.ok(paths.includes(first.path));
    assert.ok(!paths.includes(second.path));
  });

  test("nobody else can read the choice or the list", async () => {
    const [first, second] = users;
    assert.equal(await loadSimulatorConsent(second.client, first.id), false, "another account sees no row");
    const own = await second.client.rpc("simulator_files", { p_limit: 10 });
    assert.ok(own.error, "a signed-in user cannot list the games");
    const visitor = anon();
    assert.match((await saveSimulatorConsent(visitor, true)) ?? "", /Sign in again|Could not save/);
    assert.ok((await visitor.from("simulator_consent").select("improve_simulator")).error);
    assert.ok((await visitor.rpc("simulator_files", { p_limit: 10 })).error);
  });

  test("the export has the switch's state", async () => {
    const { data, error } = await users[0].client.rpc("export_my_data");
    assert.equal(error, null);
    const consent = (data as { simulator_consent: { improve_simulator: boolean; policy_version: string } }).simulator_consent;
    assert.equal(consent.improve_simulator, true);
    assert.equal(consent.policy_version, "2026-10-12");
    const other = await users[1].client.rpc("export_my_data");
    assert.deepEqual((other.data as { simulator_consent: unknown }).simulator_consent, {
      improve_simulator: false,
      updated_at: null,
      policy_version: null,
    });
  });

  test("turning it off takes the game out of the list at once", async () => {
    const first = users[0];
    assert.equal(await saveSimulatorConsent(first.client, false), null);
    assert.equal(await loadSimulatorConsent(first.client, first.id), false);
    assert.ok(!(await listed()).includes(first.path));
  });
});

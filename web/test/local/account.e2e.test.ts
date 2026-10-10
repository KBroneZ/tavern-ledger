// End-to-end run of the website's account flow against the local Supabase
// stack (T-104c), with the same functions the pages use:
// sign up, confirm through the local mail catcher, password reset and the
// other email links (D-048), set the profile, public profile, export, the delete-account checks (origin, recent sign-in) and
// deletion, then a check that nothing of the user remains.
//
// Skipped unless TAVERN_SUPABASE_LOCAL=1 and `npx supabase start` is running.
// Keys are read from `npx supabase status` into memory and never printed.
// The user is made up (example.test) and removed even if a step fails.
//
//   $env:TAVERN_SUPABASE_LOCAL = "1"; npm run test:local

import { describe, test, after } from "node:test";
import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { randomBytes, randomUUID } from "node:crypto";
import { gzipSync } from "node:zlib";
import { join } from "node:path";
import type { SupabaseClient, SupportedStorage } from "@supabase/supabase-js";
import {
  deleteAccount,
  exportAll,
  finishAuthLink,
  loadOwnProfile,
  loadPublicProfile,
  makeClient,
  requestPasswordReset,
  saveProfile,
  setNewPassword,
  signIn,
  signUp,
} from "../../src/lib/api.ts";
import { CHECK_EMAIL, RESET_SENT } from "../../src/lib/auth.ts";
import { parseAuthLink } from "../../src/lib/authLink.ts";
import type { SiteConfig } from "../../src/lib/config.ts";
import { deleteOutcome } from "../../src/lib/deleteAccount.ts";

const ENABLED = process.env.TAVERN_SUPABASE_LOCAL === "1";
const REPO = join(import.meta.dirname, "..", "..", "..");
const SITE = "http://127.0.0.1:3000";
const DB_CONTAINER = "supabase_db_tavern-ledger";
const SESSION = "Hearthstone_2026_10_09_18_30_00";

interface Stack {
  url: string;
  anon: string;
  service: string;
  mail: string;
}

function stack(): Stack {
  const out = execFileSync("npx supabase status -o json", { cwd: REPO, encoding: "utf8", shell: true, timeout: 120_000 });
  const s = JSON.parse(out.slice(out.indexOf("{")));
  return { url: s.API_URL, anon: s.ANON_KEY, service: s.SERVICE_ROLE_KEY, mail: s.MAILPIT_URL ?? s.INBUCKET_URL };
}

function psql(sql: string): string {
  return execFileSync("docker", ["exec", DB_CONTAINER, "psql", "-U", "postgres", "-tAc", sql], {
    encoding: "utf8",
    timeout: 60_000,
  }).trim();
}

function memoryStorage(): SupportedStorage {
  const items = new Map<string, string>();
  return {
    getItem: (k) => items.get(k) ?? null,
    setItem: (k, v) => void items.set(k, v),
    removeItem: (k) => void items.delete(k),
  };
}

async function confirmationLink(mail: string, email: string): Promise<string> {
  for (let attempt = 0; attempt < 20; attempt++) {
    const search = await fetch(`${mail}/api/v1/search?query=${encodeURIComponent(`to:"${email}"`)}`);
    const found = (await search.json()) as { messages?: { ID: string }[] };
    const id = found.messages?.[0]?.ID;
    if (id) {
      const message = (await (await fetch(`${mail}/api/v1/message/${id}`)).json()) as { Text?: string; HTML?: string };
      const text = `${message.Text ?? ""} ${message.HTML ?? ""}`.replaceAll("&amp;", "&");
      const link = /https?:\/\/[^\s"'<>]+\/auth\/v1\/verify\?[^\s"'<>]+/.exec(text)?.[0];
      if (link) return link;
    }
    await new Promise((r) => setTimeout(r, 250));
  }
  throw new Error("no confirmation email arrived");
}

/** Where an email link's /auth/v1/verify sends the browser. */
async function landing(link: string): Promise<URL> {
  const res = await fetch(link, { redirect: "manual" });
  await res.body?.cancel();
  return new URL(res.headers.get("location") ?? "");
}

async function deleteMail(mail: string, email: string): Promise<void> {
  await fetch(`${mail}/api/v1/search?query=${encodeURIComponent(`to:"${email}"`)}`, { method: "DELETE" });
}

/** Counts of everything that could still hold the user. */
function leftovers(userId: string, email: string): Record<string, number> {
  const q = (sql: string) => Number(psql(sql));
  return {
    users: q(`select count(*) from auth.users where id = '${userId}' or email = '${email}'`),
    identities: q(`select count(*) from auth.identities where user_id = '${userId}'`),
    sessions: q(`select count(*) from auth.sessions where user_id = '${userId}'`),
    refresh_tokens: q(`select count(*) from auth.refresh_tokens where user_id = '${userId}'`),
    audit: q(
      `select count(*) from auth.audit_log_entries where payload ->> 'actor_id' = '${userId}' ` +
        `or payload -> 'traits' ->> 'user_id' = '${userId}' or payload::text like '%${email}%'`,
    ),
    profiles: q(`select count(*) from public.profiles where user_id = '${userId}'`),
    games: q(`select count(*) from public.games where user_id = '${userId}'`),
    objects: q(`select count(*) from storage.objects where name like '${userId}/%'`),
  };
}

describe("website account flow on the local stack", { skip: !ENABLED && "set TAVERN_SUPABASE_LOCAL=1" }, () => {
  const s = ENABLED ? stack() : ({} as Stack);
  const config: SiteConfig = { url: s.url, anonKey: s.anon };
  const email = `web-e2e-${randomBytes(6).toString("hex")}@example.test`;
  let password = randomBytes(12).toString("base64url");
  const client: SupabaseClient = ENABLED ? makeClient(config, memoryStorage()) : ({} as SupabaseClient);
  let userId = "";
  const fileName = () => `${userId}/${SESSION}-1.json.gz`;
  const gameJson = JSON.stringify({ synthetic: true, hero: "BG20_HERO_202", place: 2 });
  const gz = gzipSync(Buffer.from(gameJson));

  const service = (path: string, init: RequestInit = {}) =>
    fetch(`${s.url}${path}`, {
      ...init,
      headers: { apikey: s.service, Authorization: `Bearer ${s.service}`, ...(init.headers ?? {}) },
    });

  after(async () => {
    if (!ENABLED) return;
    if (userId && psql(`select count(*) from auth.users where id = '${userId}'`) !== "0") {
      await service(`/storage/v1/object/games`, { method: "DELETE", headers: { "Content-Type": "application/json" }, body: JSON.stringify({ prefixes: [fileName()] }) });
      await service(`/auth/v1/admin/users/${userId}`, { method: "DELETE" });
    }
    await deleteMail(s.mail, email);
  });

  test("sign up, confirm by email and get a session", async () => {
    const outcome = await signUp(client, email, password, `${SITE}/account/`);
    assert.deepEqual(outcome, { kind: "check-email", message: CHECK_EMAIL });
    const link = await confirmationLink(s.mail, email);
    const target = await landing(link);
    assert.equal(target.origin + target.pathname, `${SITE}/account/`, "the link goes back to the account page");
    const parsed = parseAuthLink(target.search, target.hash);
    assert.equal(parsed.kind, "code", "PKCE code in the redirect");
    const finished = await finishAuthLink(client, config, parsed);
    assert.deepEqual(finished, { recovery: false, message: "Your email is confirmed.", tone: "ok" });
    const { data } = await client.auth.getUser();
    userId = data.user?.id ?? "";
    assert.match(userId, /^[0-9a-f-]{36}$/);
  });

  test("password reset: the link opens the new password form and only the new password works", async () => {
    await deleteMail(s.mail, email);
    const outcome = await requestPasswordReset(client, email, `${SITE}/account/`);
    assert.deepEqual(outcome, { kind: "check-email", message: RESET_SENT });
    const nobody = `nobody-${randomBytes(4).toString("hex")}@example.test`;
    assert.deepEqual(await requestPasswordReset(makeClient(config, memoryStorage()), nobody, `${SITE}/account/`), outcome);

    const link = await confirmationLink(s.mail, email);
    assert.match(link, /type=recovery/);
    const target = await landing(link);
    assert.equal(target.origin + target.pathname, `${SITE}/account/`);
    const parsed = parseAuthLink(target.search, target.hash);
    assert.equal(parsed.kind, "code");
    assert.deepEqual(await finishAuthLink(client, config, parsed), { recovery: true, message: null, tone: "info" });

    const newPassword = randomBytes(12).toString("base64url");
    assert.equal(await setNewPassword(client, newPassword), null);
    const other = makeClient(config, memoryStorage());
    assert.ok(await signIn(other, email, password), "the old password is refused");
    assert.equal(await signIn(other, email, newPassword), null);
    password = newPassword;

    // The same link again: refused, and the page can say why.
    const again = await landing(link);
    assert.deepEqual(parseAuthLink(again.search, again.hash), { kind: "error", code: "otp_expired" });
  });

  test("a link made without PKCE (as from the dashboard) is explained and its session ended", async () => {
    const sessions = () => psql(`select count(*) from auth.sessions where user_id = '${userId}'`);
    const before = sessions();
    const res = await service("/auth/v1/admin/generate_link", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ type: "magiclink", email, redirect_to: `${SITE}/account/` }),
    });
    assert.equal(res.status, 200);
    const body = (await res.json()) as { action_link?: string; properties?: { action_link?: string } };
    const action = body.action_link ?? body.properties?.action_link ?? "";
    const target = await landing(action);
    const parsed = parseAuthLink(target.search, target.hash);
    assert.equal(parsed.kind, "implicit");
    assert.equal(sessions(), String(Number(before) + 1), "the link made a session");
    const outcome = await finishAuthLink(makeClient(config, memoryStorage()), config, parsed);
    assert.match(outcome.message ?? "", /for the Tavern Ledger app/);
    assert.equal(sessions(), before, "and the site ended it");
  });

  test("sign-up and sign-in answers do not reveal that the email exists", async () => {
    const other = makeClient(config, memoryStorage());
    const again = await signUp(other, email, randomBytes(12).toString("base64url"), `${SITE}/account/`);
    assert.deepEqual(again, { kind: "check-email", message: CHECK_EMAIL });
    const wrongPassword = await signIn(other, email, "wrong-password-123");
    const noAccount = await signIn(other, `nobody-${randomBytes(4).toString("hex")}@example.test`, "wrong-password-123");
    assert.ok(wrongPassword);
    assert.equal(wrongPassword, noAccount);
  });

  test("a new profile is private and has no name", async () => {
    assert.deepEqual(await loadOwnProfile(client, userId), { displayName: null, isPublic: false });
    assert.deepEqual(await loadPublicProfile(makeClient(config, memoryStorage()), userId), { kind: "not-found" });
  });

  test("a BattleTag-shaped name is refused by the database", async () => {
    const error = await saveProfile(client, userId, { displayName: "Name#1234", isPublic: false });
    assert.match(error ?? "", /letters/);
  });

  test("a public profile shows the name and the game summary, nothing else", async () => {
    // What the upload function (T-104d) will do: a row and a file, by the service role.
    const row = {
      user_id: userId,
      session: SESSION,
      game_index: 1,
      game_type: "GT_BATTLEGROUNDS_DUO",
      status: "ok",
      hero_card_id: "BG20_HERO_202",
      final_place: 2,
      build: 12345,
      played_on: "2026-10-09",
      tribes_offered: { BEAST: 3 },
      saved_at: 1_760_000_000,
      content_sha256: "0".repeat(64),
      size_bytes: gz.length,
    };
    const ins = await service("/rest/v1/games", { method: "POST", headers: { "Content-Type": "application/json" }, body: JSON.stringify(row) });
    assert.equal(ins.status, 201);
    const up = await service(`/storage/v1/object/games/${fileName()}`, {
      method: "POST",
      headers: { "Content-Type": "application/gzip" },
      body: gz,
    });
    assert.equal(up.status, 200);

    assert.equal(await saveProfile(client, userId, { displayName: "Web E2E Tester", isPublic: true }), null);
    const anon = makeClient(config, memoryStorage());
    assert.deepEqual(await loadPublicProfile(anon, userId), {
      kind: "found",
      name: "Web E2E Tester",
      games: [{ date: "2026-10-09", mode: "Duos", hero: "BG20_HERO_202", place: "2nd" }],
    });
  });

  test("a private profile and a missing one give the same answer", async () => {
    assert.equal(await saveProfile(client, userId, { displayName: "Web E2E Tester", isPublic: false }), null);
    const anon = makeClient(config, memoryStorage());
    const hidden = await loadPublicProfile(anon, userId);
    const missing = await loadPublicProfile(anon, randomUUID());
    assert.deepEqual(hidden, { kind: "not-found" });
    assert.deepEqual(missing, hidden);
  });

  test("the export holds every row and the game file's bytes", async () => {
    const data = (await exportAll(client)) as Record<string, any>;
    assert.equal(data.format, "tavern-ledger-export");
    assert.equal(data.account.id, userId);
    assert.equal(data.account.email, email);
    assert.equal(data.profile.display_name, "Web E2E Tester");
    assert.equal(data.games.length, 1);
    assert.equal(data.games[0].hero_card_id, "BG20_HERO_202");
    assert.equal(data.files.length, 1);
    assert.ok(data.identities.length >= 1);
    const actions = data.auth_events.map((e: { payload?: { action?: string } }) => e.payload?.action);
    for (const action of ["user_signedup", "login", "user_recovery_requested", "user_modified"]) {
      assert.ok(actions.includes(action), `${action} in ${actions.join(", ")}`);
    }
    assert.deepEqual(data.file_contents, [
      { name: fileName(), encoding: "gzip+base64", data: gz.toString("base64") },
    ]);
    assert.equal(JSON.stringify(data).includes(s.service), false);
  });

  // The local stack's gateway (Kong) answers every preflight itself and
  // stamps "Access-Control-Allow-Origin: *" on answers, so the function's own
  // CORS headers (unit tests) cannot be seen here. What the stack does show:
  // the site's preflight passes, and the function refuses another origin.
  test("delete-account lets the site's preflight through and refuses other origins", async () => {
    const fn = `${s.url}/functions/v1/delete-account`;
    const ok = await fetch(fn, {
      method: "OPTIONS",
      headers: {
        Origin: SITE,
        "Access-Control-Request-Method": "DELETE",
        "Access-Control-Request-Headers": "authorization, apikey",
      },
    });
    assert.ok(ok.status >= 200 && ok.status < 300, `preflight status ${ok.status}`);
    await ok.body?.cancel();

    const { data } = await client.auth.getSession();
    const res = await fetch(fn, {
      method: "DELETE",
      headers: { Origin: "https://evil.test", Authorization: `Bearer ${data.session?.access_token}`, apikey: s.anon },
    });
    assert.equal(res.status, 403);
    assert.deepEqual(await res.json(), { error: "origin not allowed" });
    assert.equal(psql(`select count(*) from auth.users where id = '${userId}'`), "1");
  });

  test("a stale sign-in must sign in again before deleting", async () => {
    psql(`update auth.users set last_sign_in_at = now() - interval '1 hour' where id = '${userId}'`);
    assert.deepEqual(await deleteAccount(client, config), { kind: "reauthenticate" });
    assert.equal(psql(`select count(*) from storage.objects where name like '${userId}/%'`), "1");
    assert.equal(await signIn(client, email, password), null);
  });

  test("deletion from the site's origin leaves nothing of the user", async () => {
    const { data } = await client.auth.getSession();
    const res = await fetch(`${s.url}/functions/v1/delete-account`, {
      method: "DELETE",
      headers: { Origin: SITE, Authorization: `Bearer ${data.session?.access_token}`, apikey: s.anon },
    });
    assert.deepEqual(deleteOutcome(res.status, await res.json()), { kind: "deleted" });
    assert.deepEqual(leftovers(userId, email), {
      users: 0,
      identities: 0,
      sessions: 0,
      refresh_tokens: 0,
      audit: 0,
      profiles: 0,
      games: 0,
      objects: 0,
    });
  });
});

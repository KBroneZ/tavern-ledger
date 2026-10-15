import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import * as config from "../../src/lib/config.ts";
import {
  contentSecurityPolicy,
  readConfig,
  NO_STORE_PAGES,
  scriptsAllowed,
  signupsOpen,
  TURNSTILE_ORIGIN,
  TURNSTILE_PAGES,
  TURNSTILE_SCRIPT,
  TURNSTILE_SCRIPT_SOURCE,
  TURNSTILE_TEST_SITE_KEY,
  turnstileSiteKey,
  WASM_WORKER,
} from "../../src/lib/config.ts";

// Made-up keys with the shape of Supabase's (a JWT whose payload names the role).
function fakeJwt(role: string): string {
  const b64 = (o: unknown) => Buffer.from(JSON.stringify(o)).toString("base64url");
  return `${b64({ alg: "HS256", typ: "JWT" })}.${b64({ iss: "supabase", role })}.c2lnbmF0dXJl`;
}

test("reads the project URL and the public key", () => {
  const config = readConfig({
    PUBLIC_SUPABASE_URL: "https://abcd.supabase.co",
    PUBLIC_SUPABASE_ANON_KEY: "sb_publishable_x",
  });
  assert.deepEqual(config, {
    url: "https://abcd.supabase.co",
    anonKey: "sb_publishable_x",
  });
});

test("accepts a legacy anon JWT and a local http URL", () => {
  const config = readConfig({
    PUBLIC_SUPABASE_URL: "http://127.0.0.1:54321",
    PUBLIC_SUPABASE_ANON_KEY: fakeJwt("anon"),
  });
  assert.equal(config.url, "http://127.0.0.1:54321");
});

test("no settings at all means accounts are closed: no backend", () => {
  assert.equal(readConfig({}), null);
  assert.equal(readConfig({ PUBLIC_SUPABASE_URL: "", PUBLIC_SUPABASE_ANON_KEY: "" }), null);
});

test("only one of the two settings stops the build", () => {
  assert.throws(
    () => readConfig({ PUBLIC_SUPABASE_URL: "https://abcd.supabase.co" }),
    /PUBLIC_SUPABASE_ANON_KEY/,
  );
  assert.throws(() => readConfig({ PUBLIC_SUPABASE_ANON_KEY: "sb_publishable_x" }), /PUBLIC_SUPABASE_URL/);
});

test("the URL must be a bare http(s) origin", () => {
  for (const url of [
    "abcd.supabase.co",
    "ftp://abcd.supabase.co",
    "https://abcd.supabase.co/rest/v1",
    "https://user:pw@abcd.supabase.co",
    "javascript:alert(1)",
  ]) {
    assert.throws(
      () => readConfig({ PUBLIC_SUPABASE_URL: url, PUBLIC_SUPABASE_ANON_KEY: "sb_publishable_x" }),
      /PUBLIC_SUPABASE_URL/,
      url,
    );
  }
});

test("plain http is only allowed for the local stack", () => {
  assert.throws(
    () =>
      readConfig({
        PUBLIC_SUPABASE_URL: "http://abcd.supabase.co",
        PUBLIC_SUPABASE_ANON_KEY: "sb_publishable_x",
      }),
    /https/,
  );
});

test("a service role or secret key is refused, so it never ships in the site", () => {
  for (const key of [fakeJwt("service_role"), "sb_secret_abc123", "not a key"]) {
    assert.throws(
      () => readConfig({ PUBLIC_SUPABASE_URL: "https://abcd.supabase.co", PUBLIC_SUPABASE_ANON_KEY: key }),
      /PUBLIC_SUPABASE_ANON_KEY/,
    );
  }
});

test("the CSP allows scripts and styles from the site only and connects to Supabase only", () => {
  const csp = contentSecurityPolicy("https://abcd.supabase.co");
  const directives = Object.fromEntries(
    csp.split(";").map((d) => d.trim().split(/\s+/)).map(([k, ...v]) => [k, v.join(" ")]),
  );
  assert.equal(directives["default-src"], "'none'");
  assert.equal(directives["script-src"], "'self'");
  assert.equal(directives["style-src"], "'self'");
  assert.equal(directives["font-src"], "'self'");
  assert.equal(directives["connect-src"], "https://abcd.supabase.co");
  assert.equal(directives["base-uri"], "'none'");
  assert.equal(directives["form-action"], "'self'");
  assert.doesNotMatch(csp, /unsafe|\*|data:/);
});

test("with accounts closed the CSP allows no script, form or connection", () => {
  const csp = contentSecurityPolicy(null);
  for (const directive of ["default-src", "script-src", "connect-src", "form-action", "base-uri"]) {
    assert.match(csp, new RegExp(`(^|; )${directive} 'none'(;|$)`), directive);
  }
  assert.match(csp, /(^|; )style-src 'self'(;|$)/);
  assert.match(csp, /(^|; )font-src 'self'(;|$)/);
});

test("pictures: the open site adds exactly HearthstoneJSON's art host (D-074) and the game textures' host (D-151) to img-src", () => {
  // The whole header, so any other change shows up here. Textures load as
  // images (WebGL reads those images, CSS masks them), so nothing but img-src
  // changes: no connect-src, no script or style source.
  assert.equal(
    contentSecurityPolicy("https://abcd.supabase.co"),
    "default-src 'none'; script-src 'self'; style-src 'self'; img-src 'self' https://art.hearthstonejson.com https://assets.tavernledger.net; " +
      "font-src 'self'; connect-src https://abcd.supabase.co; base-uri 'none'; form-action 'self'",
  );
  // Accounts closed: no script draws a game, so no picture comes from outside.
  assert.match(contentSecurityPolicy(null), /(^|; )img-src 'self'(;|$)/);
});

test("the simulator's policy adds only 'wasm-unsafe-eval' to script-src, and only when asked (D-090)", () => {
  const plain = contentSecurityPolicy("https://abcd.supabase.co");
  const wasm = contentSecurityPolicy("https://abcd.supabase.co", { wasm: true });
  assert.equal(wasm, plain.replace("script-src 'self'", "script-src 'self' 'wasm-unsafe-eval'"));
  assert.equal(wasm.match(/unsafe/g)?.length, 1);
  assert.doesNotMatch(wasm.replace(" 'wasm-unsafe-eval'", ""), /unsafe|\*|data:/);
  // Closed: no script at all, so nothing to compile either.
  assert.equal(contentSecurityPolicy(null, { wasm: true }), contentSecurityPolicy(null));
  // The page never compiles WebAssembly (its worker does): no page is named here.
  assert.equal("WASM_PAGES" in config, false);
  assert.match("simWorker-Cif63Ow6.js", WASM_WORKER);
  assert.doesNotMatch("main-Cif63Ow6.js", WASM_WORKER);
  assert.doesNotMatch("simWorker-x/../a.js", WASM_WORKER);
});

test("the pages that must not be stored are every page about the signed-in user's own data, in both languages", () => {
  const pages = ["account", "me", "games", "game", "player-heroes", "curious"];
  assert.deepEqual(NO_STORE_PAGES.toSorted(), pages.flatMap((p) => [`/${p}/`, `/es/${p}/`]).toSorted());
});

const BACKEND = { PUBLIC_SUPABASE_URL: "https://abcd.supabase.co", PUBLIC_SUPABASE_ANON_KEY: "sb_publishable_x" };

test("sign-ups are closed unless the build says exactly true", () => {
  assert.equal(signupsOpen(BACKEND), false);
  assert.equal(signupsOpen({ ...BACKEND, PUBLIC_SIGNUPS_OPEN: "" }), false);
  assert.equal(signupsOpen({ ...BACKEND, PUBLIC_SIGNUPS_OPEN: "false" }), false);
  assert.equal(signupsOpen({ ...BACKEND, PUBLIC_SIGNUPS_OPEN: "true" }), true);
  assert.equal(signupsOpen({}), false);
});

test("a sign-up setting that is not true or false stops the build", () => {
  for (const value of ["yes", "1", "TRUE", " true"]) {
    assert.throws(() => signupsOpen({ ...BACKEND, PUBLIC_SIGNUPS_OPEN: value }), /PUBLIC_SIGNUPS_OPEN/, value);
  }
});

test("open sign-ups without a backend stop the build", () => {
  assert.throws(() => signupsOpen({ PUBLIC_SIGNUPS_OPEN: "true" }), /PUBLIC_SIGNUPS_OPEN/);
});

test("scriptsAllowed: in dev or with accounts open, never in the accounts-closed build", () => {
  assert.equal(scriptsAllowed({}, false), false);
  assert.equal(scriptsAllowed({}, true), true);
  assert.equal(scriptsAllowed({ PUBLIC_SUPABASE_URL: "https://abc.supabase.co", PUBLIC_SUPABASE_ANON_KEY: "sb_publishable_x" }, false), true);
});

// Cloudflare's published test keys (always pass); never a real key in the repo.
const TEST_SITE_KEY = "1x00000000000000000000AA";
const TEST_SECRET_KEY = "1x0000000000000000000000000000000AA";

test("Turnstile: an open-accounts build needs the site key, a closed build does not", () => {
  assert.equal(turnstileSiteKey({}), null);
  assert.equal(turnstileSiteKey({ PUBLIC_TURNSTILE_SITE_KEY: TEST_SITE_KEY }), null, "closed: no form, no widget");
  assert.throws(() => turnstileSiteKey(BACKEND), /PUBLIC_TURNSTILE_SITE_KEY is not set/);
  assert.throws(() => turnstileSiteKey({ ...BACKEND, PUBLIC_TURNSTILE_SITE_KEY: "" }), /PUBLIC_TURNSTILE_SITE_KEY is not set/);
  assert.equal(turnstileSiteKey({ ...BACKEND, PUBLIC_TURNSTILE_SITE_KEY: TEST_SITE_KEY }), TEST_SITE_KEY);
  assert.equal(turnstileSiteKey({ ...BACKEND, PUBLIC_TURNSTILE_SITE_KEY: "0x4AAAAAAABkMYinukE8nzKd" }), "0x4AAAAAAABkMYinukE8nzKd");
  assert.equal(TURNSTILE_TEST_SITE_KEY, TEST_SITE_KEY);
});

test("Turnstile: a Cloudflare Pages build refuses Cloudflare's test keys", () => {
  for (const key of [TEST_SITE_KEY, "2x00000000000000000000AB", "3x00000000000000000000FF"]) {
    assert.throws(() => turnstileSiteKey({ ...BACKEND, CF_PAGES: "1", PUBLIC_TURNSTILE_SITE_KEY: key }), /test keys/, key);
  }
  const real = "0x4AAAAAAABkMYinukE8nzKd";
  assert.equal(turnstileSiteKey({ ...BACKEND, CF_PAGES: "1", PUBLIC_TURNSTILE_SITE_KEY: real }), real);
  assert.equal(turnstileSiteKey({ ...BACKEND, PUBLIC_TURNSTILE_SITE_KEY: TEST_SITE_KEY }), TEST_SITE_KEY, "local and CI");
});

test("Turnstile: a secret key or anything else is refused and never echoed", () => {
  for (const key of [TEST_SECRET_KEY, "0x4AAAAAAABkMYinukE8nzKdAAAAAAAAAAAAA", "sitekey", "0x4AAA<script>", " 1x00000000000000000000AA"]) {
    assert.throws(
      () => turnstileSiteKey({ ...BACKEND, PUBLIC_TURNSTILE_SITE_KEY: key }),
      (e: Error) => /PUBLIC_TURNSTILE_SITE_KEY must be/.test(e.message) && !e.message.includes(key.trim()),
      key,
    );
  }
});

test("Turnstile's policy adds only its own origin, to script-src and frame-src, and only when asked", () => {
  const plain = contentSecurityPolicy("https://abcd.supabase.co");
  const withCaptcha = contentSecurityPolicy("https://abcd.supabase.co", { turnstile: true });
  assert.equal(
    withCaptcha,
    plain
      .replace("script-src 'self'", `script-src 'self' ${TURNSTILE_SCRIPT_SOURCE}`)
      .replace("; base-uri", `; frame-src ${TURNSTILE_ORIGIN}; base-uri`),
  );
  assert.equal(TURNSTILE_ORIGIN, "https://challenges.cloudflare.com");
  // Scripts: Cloudflare's one file only; the frame: its origin.
  assert.equal(TURNSTILE_SCRIPT_SOURCE, "https://challenges.cloudflare.com/turnstile/v0/api.js");
  assert.doesNotMatch(withCaptcha, /unsafe|\*|data:/);
  // No frame anywhere else.
  assert.doesNotMatch(plain, /frame-src/);
  // Closed: no script, no form, so no widget either.
  assert.equal(contentSecurityPolicy(null, { turnstile: true }), contentSecurityPolicy(null));
});

test("Turnstile is on exactly the pages whose forms make Supabase send an email or check a password", () => {
  assert.deepEqual(TURNSTILE_PAGES, [
    "/signin/",
    "/es/signin/",
    "/forgot-password/",
    "/es/forgot-password/",
    "/account/",
    "/es/account/",
    "/desktop-sign-in/",
    "/es/desktop-sign-in/",
  ]);
  assert.equal(TURNSTILE_SCRIPT, "https://challenges.cloudflare.com/turnstile/v0/api.js?render=explicit");
});

test("every page's layout checks the Turnstile key, so an open-accounts build cannot ship without it", () => {
  const layout = readFileSync(join(import.meta.dirname, "..", "..", "src", "layouts", "Base.astro"), "utf8");
  assert.match(layout, /^turnstileSiteKey\(\{ \.\.\.import\.meta\.env, CF_PAGES: process\.env\.CF_PAGES \}\);$/m);
});

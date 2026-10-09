import { test } from "node:test";
import assert from "node:assert/strict";
import { contentSecurityPolicy, readConfig } from "../../src/lib/config.ts";

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
});

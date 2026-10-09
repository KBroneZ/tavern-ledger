// Checks on the built site (run after `npm run build`): every page carries
// the Content-Security-Policy, nothing is inlined that the policy would have
// to allow, and no key other than the public one is in the files. The site
// is built in one of two ways, read from the same settings as the build:
// accounts open (Supabase settings given) or closed (none given).
import { test } from "node:test";
import assert from "node:assert/strict";
import { existsSync, readdirSync, readFileSync, statSync } from "node:fs";
import { join, relative } from "node:path";
import { loadEnv } from "vite";
import { readConfig } from "../../src/lib/config.ts";

const ROOT = join(import.meta.dirname, "..", "..");
const DIST = join(ROOT, "dist");
const CONFIG = readConfig(loadEnv("production", ROOT, "PUBLIC_"));
const ACCOUNT_PAGES = ["signin/index.html", "account/index.html", "profile/index.html"];

function files(dir: string): string[] {
  return readdirSync(dir).flatMap((name) => {
    const path = join(dir, name);
    return statSync(path).isDirectory() ? files(path) : [path];
  });
}

const ALL = existsSync(DIST) ? files(DIST) : [];
const PAGES = ALL.filter((f) => f.endsWith(".html"));
const name = (f: string) => relative(DIST, f).replaceAll("\\", "/");

test("the site was built", () => {
  assert.ok(PAGES.length >= 6, "run `npm run build` first");
  for (const page of ["index.html", "signin/index.html", "account/index.html", "profile/index.html", "privacy/index.html", "terms/index.html"]) {
    assert.ok(PAGES.map(name).includes(page), page);
  }
});

test("every page has the CSP as its first meta after the charset", () => {
  for (const page of PAGES) {
    const html = readFileSync(page, "utf8");
    const metas = [...html.matchAll(/<meta [^>]*>/g)].map((m) => m[0]);
    assert.match(metas[0] ?? "", /charset/, name(page));
    assert.match(metas[1] ?? "", /http-equiv="Content-Security-Policy"/, name(page));
    const csp = /content="([^"]*)"/.exec(metas[1])?.[1] ?? "";
    assert.match(csp, /default-src 'none'/, name(page));
    assert.match(csp, /script-src 'self'(;|$)/, name(page));
    assert.match(csp, /style-src 'self'(;|$)/, name(page));
    if (CONFIG) assert.match(csp, /connect-src https?:\/\/[^\s;*]+(;|$)/, name(page));
    else assert.match(csp, /connect-src 'none'(;|$)/, name(page));
    assert.doesNotMatch(csp, /unsafe|\*/, name(page));
  }
});

test("no inline script, style or event handler in any page", () => {
  for (const page of PAGES) {
    const html = readFileSync(page, "utf8");
    for (const tag of html.matchAll(/<script\b[^>]*>/g)) {
      assert.match(tag[0], /\ssrc="\/_astro\/[^"]+\.js"/, `${name(page)}: ${tag[0]}`);
    }
    assert.doesNotMatch(html, /<script\b[^>]*>[^<\s]/, name(page));
    assert.doesNotMatch(html, /<style\b/, name(page));
    assert.doesNotMatch(html, /\sstyle="/, name(page));
    assert.doesNotMatch(html, /\son[a-z]+="/, name(page));
    for (const link of html.matchAll(/<link\b[^>]*rel="stylesheet"[^>]*>/g)) {
      assert.match(link[0], /href="\/_astro\//, name(page));
    }
  }
});

test("every page shows the fan-project notice", () => {
  for (const page of PAGES) {
    assert.match(
      readFileSync(page, "utf8"),
      /Unofficial fan project\. Not affiliated with or endorsed by Blizzard Entertainment\./,
      name(page),
    );
  }
});

test("_headers carries the same policy plus frame-ancestors and the other security headers", () => {
  const headers = readFileSync(join(DIST, "_headers"), "utf8");
  const page = readFileSync(join(DIST, "index.html"), "utf8");
  const pageCsp = /http-equiv="Content-Security-Policy" content="([^"]*)"/.exec(page)?.[1];
  assert.ok(pageCsp);
  assert.ok(headers.includes(`Content-Security-Policy: ${pageCsp}; frame-ancestors 'none'`));
  assert.match(headers, /X-Content-Type-Options: nosniff/);
  assert.match(headers, /X-Frame-Options: DENY/);
  assert.match(headers, /Referrer-Policy: no-referrer/);
  assert.match(headers, /Permissions-Policy: camera=\(\)/);
  assert.match(headers, /Strict-Transport-Security: max-age=31536000; includeSubDomains/);
});

test("accounts open: the account pages have their forms and scripts", { skip: !CONFIG }, () => {
  for (const page of ACCOUNT_PAGES) {
    const html = readFileSync(join(DIST, page), "utf8");
    assert.match(html, /<script\b/, page);
    assert.doesNotMatch(html, /Accounts are not open yet/, page);
  }
  assert.match(readFileSync(join(DIST, "signin/index.html"), "utf8"), /id="signup-form"/);
});

test("accounts closed: no form, no script and no backend anywhere", { skip: !!CONFIG }, () => {
  for (const page of ACCOUNT_PAGES) {
    const html = readFileSync(join(DIST, page), "utf8");
    assert.match(html, /Accounts are not open yet/, page);
  }
  for (const page of PAGES) {
    const html = readFileSync(page, "utf8");
    assert.doesNotMatch(html, /<script\b|<form\b|<input\b|href="\/(signin|account)\/"/, name(page));
  }
  assert.deepEqual(ALL.filter((f) => f.endsWith(".js")).map(name), []);
});

function jwtRoles(text: string): string[] {
  return [...text.matchAll(/eyJ[A-Za-z0-9_-]+\.(eyJ[A-Za-z0-9_-]+)\.[A-Za-z0-9_-]+/g)].map((m) => {
    try {
      return JSON.parse(Buffer.from(m[1], "base64url").toString("utf8")).role ?? "?";
    } catch {
      return "?";
    }
  });
}

test("only the public key is in the files", () => {
  for (const file of ALL) {
    const text = readFileSync(file, "utf8");
    assert.doesNotMatch(text, /sb_secret_[A-Za-z0-9]/, name(file));
    for (const role of jwtRoles(text)) assert.equal(role, "anon", name(file));
  }
});

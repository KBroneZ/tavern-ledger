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
import { readConfig, signupsOpen } from "../../src/lib/config.ts";

const ROOT = join(import.meta.dirname, "..", "..");
const DIST = join(ROOT, "dist");
const ENV = loadEnv("production", ROOT, "PUBLIC_");
const CONFIG = readConfig(ENV);
const SIGNUPS = signupsOpen(ENV);
const ACCOUNT_PAGES = ["signin/index.html", "account/index.html", "profile/index.html", "forgot-password/index.html"];

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
  for (const page of ["index.html", ...ACCOUNT_PAGES, "privacy/index.html", "terms/index.html"]) {
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
    assert.match(csp, CONFIG ? /script-src 'self'(;|$)/ : /script-src 'none'(;|$)/, name(page));
    assert.match(csp, CONFIG ? /form-action 'self'(;|$)/ : /form-action 'none'(;|$)/, name(page));
    assert.match(csp, /style-src 'self'(;|$)/, name(page));
    assert.match(csp, /font-src 'self'(;|$)/, name(page));
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

test("every indexable page names its canonical address on the apex domain", () => {
  for (const page of PAGES) {
    const html = readFileSync(page, "utf8");
    const canonical = /<link rel="canonical" href="([^"]*)">/.exec(html)?.[1];
    if (html.includes('<meta name="robots" content="noindex">')) {
      assert.equal(canonical, undefined, name(page));
      continue;
    }
    const path = "/" + name(page).replace(/index\.html$/, "");
    assert.equal(canonical, `https://tavernledger.net${path}`, name(page));
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
  assert.match(headers, /Cross-Origin-Opener-Policy: same-origin/);
  assert.match(headers, /\/_astro\/\*\n  Cache-Control: public, max-age=31536000, immutable/);
  assert.match(headers, /Permissions-Policy: camera=\(\)/);
  assert.match(headers, /Strict-Transport-Security: max-age=\d+; includeSubDomains/);
});

test("accounts open: the account pages have their forms and scripts", { skip: !CONFIG }, () => {
  for (const page of ACCOUNT_PAGES) {
    const html = readFileSync(join(DIST, page), "utf8");
    const sources = [...html.matchAll(/<script\b[^>]*\ssrc="\/([^"]+)"/g)].map((m) => m[1]);
    assert.ok(sources.length > 0, page);
    for (const src of sources) assert.ok(existsSync(join(DIST, src)), `${page}: ${src}`);
    assert.doesNotMatch(html, /Accounts are not open yet/, page);
  }
  const signin = readFileSync(join(DIST, "signin/index.html"), "utf8");
  assert.match(signin, /href="\/forgot-password\/"/);
  const forgot = readFileSync(join(DIST, "forgot-password/index.html"), "utf8");
  assert.match(forgot, /id="reset-form"/);
  // Sent before the script runs, a form would become a GET with the email
  // (and password) in the address: its button only works once the script does.
  for (const [page, html] of [["signin", signin], ["forgot-password", forgot]]) {
    const buttons = [...html.matchAll(/<button\b[^>]*type="submit"[^>]*>/g)].map((m) => m[0]);
    assert.ok(buttons.length > 0, page);
    for (const button of buttons) assert.match(button, /\sdisabled\b/, `${page}: ${button}`);
  }
  const account = readFileSync(join(DIST, "account/index.html"), "utf8");
  assert.match(account, /id="set-password-form"/);
  if (SIGNUPS) {
    assert.match(signin, /id="signup-form"/);
    assert.match(signin, /accept the <a href="\/terms\/">terms<\/a> and confirm you are\s+16 or older/);
  } else {
    // Sign-ups closed (the hosted default): sign-in stays, no sign-up form is rendered.
    // The real gate is the hosted project's disable_signup (deploy.md, section 10.9).
    assert.doesNotMatch(signin, /id="signup-form"|autocomplete="new-password"/);
    assert.match(signin, /Sign-up is not open yet/);
    assert.match(signin, /id="signin-form"/);
  }
});

test("accounts closed: no form, no script and no backend anywhere", { skip: !!CONFIG }, () => {
  for (const page of ACCOUNT_PAGES) {
    const html = readFileSync(join(DIST, page), "utf8");
    assert.match(html, /Accounts are not open yet/, page);
  }
  for (const page of PAGES) {
    const html = readFileSync(page, "utf8");
    assert.doesNotMatch(html, /<script\b|<form\b|<input\b|href="\/(signin|account|forgot-password)\/"/, name(page));
  }
  assert.deepEqual(ALL.filter((f) => f.endsWith(".js")).map(name), []);
  for (const file of ALL) {
    const text = readFileSync(file, "utf8");
    // No project URL (the policy links to supabase.com, which is fine) and no key of any kind.
    assert.doesNotMatch(text, /\.supabase\.co(?![a-z])|127\.0\.0\.1:54321|sb_publishable_|eyJ[A-Za-z0-9_-]+\./, name(file));
  }
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

test("fonts and styles are our own files: every url() is a local font, none is external", () => {
  const css = ALL.filter((f) => f.endsWith(".css"));
  assert.ok(css.length > 0, "no stylesheet in dist");
  const declared = css.flatMap((f) => [...readFileSync(f, "utf8").matchAll(/url\(/g)]);
  assert.ok(declared.length >= 4, "the four font files are declared");
  for (const file of css) {
    const text = readFileSync(file, "utf8");
    assert.doesNotMatch(text, /@import|https?:\/\//i, name(file));
    const urls = [...text.matchAll(/url\(\s*["']?([^"')]+)["']?\s*\)/g)].map((m) => m[1]);

    for (const url of urls) {
      assert.match(url, /^\/fonts\/[a-z0-9-]+\.woff2$/, url);
      assert.ok(existsSync(join(DIST, url)), url);
    }
  }
});

test("the OFL text ships next to the fonts", () => {
  for (const licence of ["fonts/OFL-Unbounded.txt", "fonts/OFL-DM-Mono.txt"]) {
    assert.match(readFileSync(join(DIST, licence), "utf8"), /SIL OPEN FONT LICENSE Version 1\.1/i, licence);
  }
});

test("no tag loads anything from another site", () => {
  for (const page of PAGES) {
    const html = readFileSync(page, "utf8");
    for (const tag of html.matchAll(/<(?:link|script|img|source|iframe|video|audio)\b[^>]*\s(?:src|href)="(?:https?:)?\/\/[^"]*"[^>]*>/g)) {
      if (/rel="canonical"/.test(tag[0])) continue;
      assert.fail(`${name(page)}: ${tag[0]}`);
    }
  }
});

test("the home page has the example game card, labelled as an example", () => {
  const html = readFileSync(join(DIST, "index.html"), "utf8");
  assert.match(html, /class="live"/);
  assert.match(html, /Example game/);
  assert.match(html, /made-up numbers/);
});

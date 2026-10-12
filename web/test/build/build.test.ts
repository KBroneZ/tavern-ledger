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
import { contentSecurityPolicy, readConfig, signupsOpen, WASM_PAGES, WASM_WORKER } from "../../src/lib/config.ts";

const ROOT = join(import.meta.dirname, "..", "..");
const DIST = join(ROOT, "dist");
const ENV = loadEnv("production", ROOT, "PUBLIC_");
const CONFIG = readConfig(ENV);
const SIGNUPS = signupsOpen(ENV);
const DATA_PAGES = ["me/index.html", "games/index.html", "profile/index.html", "leaderboards/index.html", "player-heroes/index.html", "curious/index.html", "heroes/index.html"];
const ACCOUNT_PAGES = ["signin/index.html", "account/index.html", "forgot-password/index.html", ...DATA_PAGES];

function files(dir: string): string[] {
  return readdirSync(dir).flatMap((name) => {
    const path = join(dir, name);
    return statSync(path).isDirectory() ? files(path) : [path];
  });
}

const ALL = existsSync(DIST) ? files(DIST) : [];
const PAGES = ALL.filter((f) => f.endsWith(".html"));
const name = (f: string) => relative(DIST, f).replaceAll("\\", "/");
/** The simulator page in each language (WASM_PAGES, as built files). */
const WASM_FILES = WASM_PAGES.map((path) => `${path.slice(1)}index.html`);
const isWasmPage = (f: string) => WASM_FILES.includes(name(f));

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
    // D-090: the simulator page alone may also compile WebAssembly.
    const wasm = CONFIG !== null && isWasmPage(page);
    assert.match(csp, wasm ? /script-src 'self' 'wasm-unsafe-eval'(;|$)/ : CONFIG ? /script-src 'self'(;|$)/ : /script-src 'none'(;|$)/, name(page));
    assert.match(csp, CONFIG ? /form-action 'self'(;|$)/ : /form-action 'none'(;|$)/, name(page));
    assert.match(csp, /style-src 'self'(;|$)/, name(page));
    assert.match(csp, /font-src 'self'(;|$)/, name(page));
    // D-074: the open site adds exactly HearthstoneJSON's art host for pictures.
    assert.match(csp, CONFIG ? /img-src 'self' https:\/\/art\.hearthstonejson\.com(;|$)/ : /img-src 'self'(;|$)/, name(page));
    if (CONFIG) assert.match(csp, /connect-src https?:\/\/[^\s;*]+(;|$)/, name(page));
    else assert.match(csp, /connect-src 'none'(;|$)/, name(page));
    assert.doesNotMatch(wasm ? csp.replace(" 'wasm-unsafe-eval'", "") : csp, /unsafe|\*/, name(page));
  }
});

test("no inline script, style or event handler in any page", () => {
  for (const page of PAGES) {
    const html = readFileSync(page, "utf8");
    for (const tag of html.matchAll(/<script\b[^>]*>/g)) {
      // Our bundles, or the language script (public/lang.js, T-508).
      assert.match(tag[0], /\ssrc="(?:\/_astro\/[^"]+\.js|\/lang\.js)"/, `${name(page)}: ${tag[0]}`);
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

test("_headers: the simulator page and its worker alone get 'wasm-unsafe-eval', replacing the site-wide policy", () => {
  const headers = readFileSync(join(DIST, "_headers"), "utf8");
  const rules = headers.split(/\n(?=\S)/).map((block) => block.trim().split("\n").map((l) => l.trim()));
  const csp = (rule: string[]) => rule.filter((l) => l.startsWith("Content-Security-Policy:"));
  const withWasm = rules.filter((r) => csp(r).some((l) => l.includes("wasm-unsafe-eval")));
  if (!CONFIG) {
    assert.deepEqual(withWasm, [], "accounts closed: no script, nothing to compile");
    assert.doesNotMatch(headers, /wasm/);
    return;
  }
  const workers = readdirSync(join(DIST, "_astro")).filter((f) => WASM_WORKER.test(f));
  assert.equal(workers.length, 1, "one simulator worker");
  assert.deepEqual(withWasm.map((r) => r[0]), [...WASM_PAGES, `/_astro/${workers[0]}`]);
  const expected = `Content-Security-Policy: ${contentSecurityPolicy(CONFIG.url, { wasm: true })}; frame-ancestors 'none'`;
  for (const rule of withWasm) {
    // Pages joins repeated headers: each rule drops the site-wide policy first.
    assert.deepEqual(rule.slice(1), ["! Content-Security-Policy", expected], rule[0]);
  }
  // The site-wide rule comes first and stays without it.
  assert.equal(rules[0][0], "/*");
  assert.doesNotMatch(csp(rules[0]).join(""), /wasm/);
  // Each page's own meta policy matches its header.
  for (const file of WASM_FILES) {
    const meta = /http-equiv="Content-Security-Policy" content="([^"]*)"/.exec(readFileSync(join(DIST, file), "utf8"))?.[1];
    assert.equal(`Content-Security-Policy: ${meta}; frame-ancestors 'none'`, expected, file);
  }
  // The worker carries the engine and loads nothing: no fetch, no import.
  const worker = readFileSync(join(DIST, "_astro", workers[0]), "utf8");
  assert.doesNotMatch(worker, /\bfetch\(|importScripts|\bimport\(/);
});

test("the simulator page: noindex, its script and frame when accounts are open, the notice when closed", () => {
  for (const file of WASM_FILES) {
    const html = readFileSync(join(DIST, file), "utf8");
    assert.match(html, /<meta name="robots" content="noindex">/, file);
    if (CONFIG) {
      for (const id of ["sim-load", "sim-table", "sim-opponent", "sim-player", "sim-result", "sim-mode"]) assert.match(html, new RegExp(`id="${id}"`), `${file}: ${id}`);
      assert.match(html, /state-loading/, "the loading blocks keep the room");
      const sources = [...html.matchAll(/<script\b[^>]*\ssrc="\/([^"]+)"/g)].map((m) => m[1]);
      assert.ok(sources.length >= 1, "the simulator's script");
      for (const src of sources) assert.ok(existsSync(join(DIST, src)), src);
    } else {
      assert.match(html, /Accounts are not open yet|Las cuentas aún no están abiertas/);
    }
  }
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
  for (const page of ACCOUNT_PAGES) {
    assert.match(readFileSync(join(DIST, "es", page), "utf8"), /Las cuentas aún no están abiertas/, `es/${page}`);
  }
  for (const page of PAGES) {
    // The language switch links a notice page to its own twin (T-508); no other link to them.
    const html = readFileSync(page, "utf8").replace(/<a\b[^>]*data-lang-switch[^>]*>/g, "");
    assert.doesNotMatch(html, /<script\b|<form\b|<input\b|href="(?:\/es)?\/(signin|account|forgot-password)\/"/, name(page));
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

test("fonts and styles are our own files: every url() is a local font or the brand mark, none is external", () => {
  const css = ALL.filter((f) => f.endsWith(".css"));
  assert.ok(css.length > 0, "no stylesheet in dist");
  const declared = css.flatMap((f) => [...readFileSync(f, "utf8").matchAll(/url\(/g)]);
  assert.ok(declared.length >= 3, "the three font files are declared");
  for (const file of css) {
    const text = readFileSync(file, "utf8");
    assert.doesNotMatch(text, /@import|https?:\/\//i, name(file));
    const urls = [...text.matchAll(/url\(\s*["']?([^"')]+)["']?\s*\)/g)].map((m) => m[1]);

    for (const url of urls) {
      // The masthead draws the logo (public/mark.svg, D-094) as a background,
      // the tavern buttons and panels their painted art (T-530, D-105), the
      // simulator its tall panel (T-526, D-112).
      assert.match(url, /^\/(?:fonts\/[a-z0-9-]+\.woff2|mark\.svg|art\/home\/(?:btn-[a-z-]+(?:@2x)?|panel-(?:lead|card|strip))\.webp|art\/sim\/panel-tall\.webp)$/, url);
      assert.ok(existsSync(join(DIST, url)), url);
    }
  }
});

test("the OFL text ships next to the fonts", () => {
  for (const licence of ["fonts/OFL-Bricolage-Grotesque.txt", "fonts/OFL-Figtree.txt"]) {
    assert.match(readFileSync(join(DIST, licence), "utf8"), /SIL OPEN FONT LICENSE Version 1\.1/i, licence);
  }
});

test("only the two D-052 families ship, within the 120 KB font budget", () => {
  const fonts = ALL.filter((f) => f.endsWith(".woff2"));
  assert.deepEqual(fonts.map(name).sort(), [
    "fonts/bricolage-grotesque-latin-wdth-normal.woff2",
    "fonts/figtree-latin-ext-wght-normal.woff2",
    "fonts/figtree-latin-wght-normal.woff2",
  ]);
  const total = fonts.reduce((sum, f) => sum + statSync(f).size, 0);
  assert.ok(total <= 120 * 1024, `${total} bytes of fonts`);
});

test("only the home page preloads a font, and it is the headline font", () => {
  for (const page of PAGES) {
    const html = readFileSync(page, "utf8");
    const preloads = [...html.matchAll(/<link\b[^>]*rel="preload"[^>]*>/g)].map((m) => m[0]);
    if (name(page) !== "index.html" && name(page) !== "es/index.html") {
      assert.deepEqual(preloads, [], name(page));
      continue;
    }
    assert.equal(preloads.length, 1);
    assert.match(preloads[0], /href="\/fonts\/bricolage-grotesque-latin-wdth-normal\.woff2"/);
    assert.match(preloads[0], /as="font"/);
    assert.match(preloads[0], /crossorigin/);
  }
});

test("no page loads anything from another origin", () => {
  for (const page of PAGES) {
    const html = readFileSync(page, "utf8");
    // Only <a> links may point elsewhere (they load nothing by themselves);
    // the canonical link names an address and loads nothing either.
    for (const tag of html.matchAll(/<(?!a\b)([a-z]+)\b[^>]*\s(?:src|href|srcset|poster|data|action|formaction)="\s*(?:[a-z][a-z0-9+.-]*:|\/\/)[^"]*"[^>]*>/gi)) {
      if (tag[1] === "link" && /rel="(?:canonical|alternate)"/.test(tag[0])) continue;
      assert.fail(`${name(page)}: ${tag[0]}`);
    }
    assert.doesNotMatch(html, /<(?:iframe|object|embed)\b/, name(page));
  }
});

test("pictures are our own files, with their size and a text alternative", () => {
  for (const page of PAGES) {
    const html = readFileSync(page, "utf8");
    for (const [img] of html.matchAll(/<img\b[^>]*>/g)) {
      // Built pages show made-up data only, so their pictures are our own
      // placeholders, plus the home page's painting (T-530) and the
      // simulator's (T-526); card art (D-074) is drawn in the browser.
      const src = /\ssrc="([^"]*)"/.exec(img)?.[1] ?? "";
      assert.match(src, /^\/(?:placeholders\/(?:hero|card)-\d\.svg|art\/home\/tavern-\d+\.webp|art\/sim\/arena-\d+\.webp)$/, `${name(page)}: ${img}`);
      assert.ok(existsSync(join(DIST, src)), src);
      // Width and height keep the room (no layout shift); alt is there, empty when decorative.
      assert.match(img, /\swidth="\d+"/, img);
      assert.match(img, /\sheight="\d+"/, img);
      assert.match(img, /\salt(?:="[^"]*")?[\s>]/, img);
    }
  }
});

test("the game viewer page: noindex, its script when accounts are open, the notice when closed", () => {
  const html = readFileSync(join(DIST, "game/index.html"), "utf8");
  assert.match(html, /<meta name="robots" content="noindex">/);
  if (CONFIG) {
    assert.match(html, /id="game-body"/);
    assert.match(html, /aria-busy="true"/);
    assert.match(html, /state-loading/, "the loading block keeps the room");
    assert.match(html, /data-switch="detail"/);
    const sources = [...html.matchAll(/<script\b[^>]*\ssrc="\/([^"]+)"/g)].map((m) => m[1]);
    assert.ok(sources.length >= 1, "the viewer's script");
    for (const src of sources) assert.ok(existsSync(join(DIST, src)), src);
  } else {
    assert.match(html, /Accounts are not open yet/);
  }
});

test("the dev-only demo game is not in the built site", () => {
  for (const file of ALL.filter((f) => /\.(js|html)$/.test(f))) {
    assert.doesNotMatch(readFileSync(file, "utf8"), /SYN_HERO|gameFixtures/, name(file));
  }
});

test("the style page has every piece, made-up data only, and stays out of search engines", () => {
  const html = readFileSync(join(DIST, "style/index.html"), "utf8");
  assert.match(html, /<meta name="robots" content="noindex">/);
  const pieces = ['class="tile', 'class="topn"', 'class="fold', 'data-switch="detail"', 'data-switch="mode"', "place-first",
    'class="pic ', "pic-missing", "state-loading", "chart-places", "chart-trend", "chart-turns"];
  for (const piece of pieces) assert.ok(html.includes(piece), piece);
  assert.match(html, /made-up/);
  // Not in the menu: nothing links to it.
  for (const page of PAGES) assert.doesNotMatch(readFileSync(page, "utf8"), /href="\/style\/"/, name(page));
});

test("the home page's pinned scene is one described picture plus five captions as text", () => {
  const html = readFileSync(join(DIST, "index.html"), "utf8");
  const section = /<section class="pin" id="scene"[\s\S]*?<\/section>/.exec(html)?.[0] ?? "";
  assert.match(section, /aria-labelledby="scene-title"/);
  const img = /<div class="stage3d" role="img" aria-label="([^"]+)"/.exec(section);
  assert.ok(img, "the scene is one image with a description");
  for (const layer of ["board", "tavern", "leaderboard", "Tavern Ledger"]) assert.match(img[1], new RegExp(layer));
  const captions = [...section.matchAll(/<li class="cap c(\d)"[^>]*>[\s\S]*?<h3>([^<]+)<\/h3>[\s\S]*?<p>([^<]+)<\/p>/g)];
  assert.deepEqual(
    captions.map((c) => c[1]),
    ["1", "2", "3", "4", "5"],
  );
  assert.match(captions[4][3], /clicks pass through/);
  assert.match(section, /<div class="pips" aria-hidden="true">/);
  // Our own shapes only: no picture of any kind in the scene.
  assert.doesNotMatch(section, /<img\b|<svg\b|url\(/);
});

test("the home page's painting: decorative, sized, in AVIF and WebP with a small file for phones", () => {
  for (const file of ["index.html", "es/index.html"]) {
    const html = readFileSync(join(DIST, file), "utf8");
    const backdrop = /<div class="backdrop" aria-hidden="true">[\s\S]*?<\/picture>/.exec(html)?.[0] ?? "";
    assert.ok(backdrop, `${file}: the backdrop is hidden from screen readers`);
    assert.match(backdrop, /<source type="image\/avif" media="\(max-width: 560px\)" srcset="\/art\/home\/tavern-800\.avif"/);
    assert.match(backdrop, /<source type="image\/avif" srcset="\/art\/home\/tavern-1080\.avif 1080w, \/art\/home\/tavern-1344\.avif 1344w"/);
    assert.match(backdrop, /<img [^>]*width="1344" height="3168" alt=""/);
    for (const src of backdrop.matchAll(/\/art\/home\/(tavern-\d+\.(?:avif|webp))/g)) {
      assert.ok(existsSync(join(DIST, "art", "home", src[1])), src[1]);
    }
    // The script that moves it ships only where scripts may run.
    assert.equal(/<script [^>]*src="\/_astro\//.test(html), CONFIG !== null, file);
  }
  // The descent also works with no script (accounts closed): the root scroll
  // timeline moves the painting by its own height less the screen's.
  const home = readFileSync(join(DIST, "index.html"), "utf8");
  const css = [...home.matchAll(/<link\b[^>]*rel="stylesheet"[^>]*href="\/([^"]+)"/g)].map((m) => readFileSync(join(DIST, m[1]), "utf8")).join("\n");
  assert.match(css, /\.backdrop-art:not\(\[data-descent=script\]\)\{[^}]*animation-name:descend[^}]*animation-timeline:scroll\(root\)/);
  assert.match(css, /@keyframes descend\{[^@]*translateY\(calc\(100lvh - 100%\)\)/);
  // A mid phone downloads one small painting, not the full one.
  assert.ok(statSync(join(DIST, "art", "home", "tavern-800.avif")).size < 120 * 1024);
});

test("the home page's buttons by role: one gold key action, wood for common ones, small stone for small ones", () => {
  const html = readFileSync(join(DIST, "index.html"), "utf8");
  const roles = [...html.matchAll(/<a class="tb tb-(key|common|small)" href="([^"]+)">\s*<span class="tb-label">([^<]+)/g)].map((m) => [m[1], m[2], m[3].trim()]);
  assert.deepEqual(roles, [
    ["common", "#scene", "Show me"],
    ["common", "#safe", "Is it safe?"],
    ["small", "#main", "Back up to the door"],
  ]);
  // Nothing to download yet (T-105): the real gold button, unavailable, links
  // nowhere, and the line it points to says so (rule 8).
  for (const [file, label, note] of [["index.html", "Download", "Not out yet"], ["es/index.html", "Descargar", "Aún no ha salido"]]) {
    const page = readFileSync(join(DIST, file), "utf8");
    const key = [...page.matchAll(/<(a|button) class="tb tb-key"[^>]*>\s*<span class="tb-label">([^<]+)/g)];
    assert.equal(key.length, 1, `${file}: one key action`);
    assert.equal(key[0][1], "button", `${file}: not a link`);
    assert.equal(key[0][2].trim(), label);
    assert.match(key[0][0], /type="button"/);
    assert.match(key[0][0], /aria-disabled="true"/);
    assert.match(key[0][0], /aria-describedby="download-note"/);
    assert.match(page, new RegExp(`<p class="key-note" id="download-note">${note}</p>`), file);
  }
  for (const art of ["btn-key", "btn-key-lit", "btn-common", "btn-small"]) {
    for (const scale of ["", "@2x"]) assert.ok(existsSync(join(DIST, "art", "home", `${art}${scale}.webp`)), `${art}${scale}`);
  }
});

test("the scene's depth: decorative, empty hero tokens filled only at view time, nothing loaded from outside", () => {
  for (const file of ["index.html", "es/index.html"]) {
    const html = readFileSync(join(DIST, file), "utf8");
    const section = /<section class="pin" id="scene"[\s\S]*?<\/section>/.exec(html)?.[0] ?? "";
    const depth = /<div class="pin-depth" aria-hidden="true">([\s\S]*?)<\/div>\s*<div class="caption-col">/.exec(section);
    assert.ok(depth, `${file}: the depth layer comes first in the stage and is hidden from screen readers`);
    assert.equal([...depth[1].matchAll(/<span class="token"><\/span>/g)].length, 6, `${file}: six empty tokens`);
    // Hero portraits come from HearthstoneJSON at view time (D-074), never in the page.
    assert.doesNotMatch(depth[1], /<img\b|hearthstonejson/);
    // One home script, and only where scripts may run.
    const scripts = [...html.matchAll(/<script\b[^>]*src="\/_astro\/[^"]+"/g)];
    assert.equal(scripts.length, CONFIG !== null ? 1 : 0, file);
  }
});

test("every number on the home page that is not real is labelled as made up", () => {
  const html = readFileSync(join(DIST, "index.html"), "utf8");
  const big = [...html.matchAll(/<p class="big">\s*(\d+)\s*<small>([^<]+)<\/small>/g)];
  assert.ok(big.length > 0);
  for (const [, , label] of big) assert.match(label, /made-up example/);
});

test("the pinned scene takes at most about 1.5 screens of scrolling", () => {
  const css = ALL.filter((f) => f.endsWith(".css"))
    .map((f) => readFileSync(f, "utf8"))
    .join("\n");
  // Every height the moving scene sets on .pin, in viewport heights.
  const heights = [...css.matchAll(/\.pin\{[^}]*?\bheight:\s*(\d+(?:\.\d+)?)vh/g)].map((m) => Number(m[1]));
  assert.ok(heights.length >= 2, "desktop and narrow heights are set");
  for (const vh of heights) {
    // The scene scrolls for (height - 100vh); keep every step readable but short.
    assert.ok(vh >= 200 && vh <= 260, `.pin is ${vh}vh`);
  }
});

test("the still picture is the default: scroll-driven motion sits behind both queries, any other behind no-preference", () => {
  // The home page's own stylesheets: the data pages' loading shimmer is not part of the scene.
  const home = readFileSync(join(DIST, "index.html"), "utf8");
  const sheets = [...home.matchAll(/<link\b[^>]*rel="stylesheet"[^>]*href="\/([^"]+)"/g)].map((m) => join(DIST, m[1]));
  assert.ok(sheets.length > 0, "the home page links its stylesheets");
  const css = sheets
    .map((f) => readFileSync(f, "utf8"))
    .join("\n");
  // Cut out each @supports (animation-timeline ...) block, braces matched.
  let rest = css;
  let at = rest.indexOf("@supports");
  while (at !== -1) {
    const open = rest.indexOf("{", at);
    if (!/animation-timeline/.test(rest.slice(at, open))) {
      at = rest.indexOf("@supports", open);
      continue;
    }
    let depth = 0;
    let end = open;
    for (; end < rest.length; end++) {
      if (rest[end] === "{") depth++;
      else if (rest[end] === "}" && --depth === 0) break;
    }
    // Everything inside sits in one nested @media that also asks for no-preference.
    const inner = rest.slice(open + 1, end).trim();
    assert.match(inner, /^@media[^{]*prefers-reduced-motion:\s*no-preference[^{]*\{/, "the moving scene also waits for no-preference");
    let level = 0;
    let closes = 0;
    for (let i = 0; i < inner.length; i++) {
      if (inner[i] === "{") level++;
      else if (inner[i] === "}" && --level === 0) closes++;
    }
    assert.equal(closes, 1, "nothing in the @supports block outside its @media");
    rest = rest.slice(0, at) + rest.slice(end + 1);
    at = rest.indexOf("@supports", at);
  }
  assert.ok(rest.length < css.length, "the moving scene is there");
  // Then each top-level @media (prefers-reduced-motion: no-preference) block:
  // time-based motion only (the sparks, T-530); nothing scroll-driven in it.
  const before = rest.length;
  at = rest.search(/@media\s*\(prefers-reduced-motion:\s*no-preference\)\s*\{/);
  while (at !== -1) {
    const open = rest.indexOf("{", at);
    let depth = 0;
    let end = open;
    for (; end < rest.length; end++) {
      if (rest[end] === "{") depth++;
      else if (rest[end] === "}" && --depth === 0) break;
    }
    assert.doesNotMatch(rest.slice(open, end), /animation-timeline|view-timeline|scroll-timeline/);
    rest = rest.slice(0, at) + rest.slice(end + 1);
    at = rest.search(/@media\s*\(prefers-reduced-motion:\s*no-preference\)\s*\{/);
  }
  assert.ok(rest.length < before, "the sparks wait for no-preference");
  assert.doesNotMatch(rest, /animation-(?:name|timeline)\s*:/);
  for (const shorthand of rest.matchAll(/animation\s*:\s*([^;}]+)/g)) assert.match(shorthand[1], /^none/);
  assert.doesNotMatch(rest, /view-timeline/);
});

// The browser pieces (T-503, D-073) must draw exactly what the Astro
// components draw: the same made-up data, rendered both ways.
test("the browser pieces draw the same markup as the Astro components on /style/", async () => {
  const html = readFileSync(join(DIST, "style/index.html"), "utf8");
  const p = await import("../../src/lib/view/pieces.ts");
  const f = await import("../../src/lib/data/format.ts");
  const sample = await import("../../src/lib/data/sample.ts");
  const { toHtml } = await import("../helpers/html.ts");
  const pieces = [
    p.statTile({ hero: true, label: "Average place, Solo", value: f.average(3.42), backing: f.backing(30, 31), trend: f.trend(-0.3, "lower"), spark: { values: sample.recent, min: 1, max: 8, invert: true } }),
    p.statTile({ label: "Combats won", value: "41", source: "inferred", backing: f.backing(31, 31) }),
    p.statTile({ label: "Actions per minute", value: f.average(null), source: "unknown", backing: f.backing(0, 31) }),
    p.topTable({ caption: "Heroes, most games first (made-up)", columns: sample.heroColumns, rows: sample.heroRows(), top: 5 }),
    p.topTable({ caption: "Last games (made-up)", columns: sample.gameColumns, rows: sample.gameRows }),
    p.placeChart(sample.soloPlaces, "solo", "places-solo"),
    p.placeChart(sample.duosPlaces, "duos", "places-duos"),
    p.placeTrend(sample.recent, 8, "trend"),
    p.stateBlock({ kind: "empty" }),
    p.stateBlock({ kind: "unknown" }),
    p.stateBlock({ kind: "loading" }),
    p.stateBlock({ kind: "error" }),
    p.placeBadge(null, "solo"),
    p.placeBadge(1, "duos", "lg"),
    p.picture("hero", "SAMPLE_HERO_01", "Lantern Keeper", 56),
    p.picture("hero", null, "Hero not recorded", 40),
    p.sourceMark("inferred"),
  ];
  // Built pages show their pictures from the placeholders, live pages from the
  // art host (D-074); the markup is otherwise the same.
  const builtSource = (markup: string) =>
    markup.replaceAll(/src="https:\/\/art\.hearthstonejson\.com\/v1\/256x\/[A-Za-z0-9_]+\.jpg" data-fallback="([^"]+)"/g, 'src="$1" data-fallback="$1"');
  for (const piece of pieces) {
    const markup = builtSource(toHtml(piece));
    assert.ok(html.includes(markup), `not on /style/: ${markup.slice(0, 160)}…`);
  }
});

test("accounts open: the data pages are a static frame with reserved room for the data", { skip: !CONFIG }, () => {
  const page = (p: string) => readFileSync(join(DIST, p), "utf8");
  for (const [file, kind] of [["me/index.html", "own"], ["profile/index.html", "public"]]) {
    const html = page(file);
    assert.match(html, new RegExp(`id="dash" data-kind="${kind}" aria-busy="true"`), file);
    assert.match(html, /class="dash-body" id="dash-body"/, file);
    assert.match(html, /class="mode-panels" id="dash-modes"/, file);
    assert.match(html, /data-switch="mode" data-controls="dash-modes"/, file);
    assert.match(html, /data-switch="detail"/, file);
    assert.match(html, /state-loading/, file);
    assert.match(html, /id="dash-status" role="status" aria-live="polite"/, file);
  }
  const games = page("games/index.html");
  assert.match(games, /<form class="list-filters" id="list-filters" method="get" action="\/games\/"/);
  for (const name of ["mode", "hero", "from", "to", "place"]) assert.match(games, new RegExp(`name="${name}"`), name);
  assert.match(games, /class="dash-body" id="list-body"/);
  // The owner's pages stay out of search engines; the menu links to the dashboard.
  for (const file of ["me/index.html", "games/index.html"]) assert.match(page(file), /<meta name="robots" content="noindex">/, file);
  assert.match(page("index.html"), /href="\/me\/"/);
});

test("accounts open: the heroes and curious stats pages are a static frame with reserved room (T-505)", { skip: !CONFIG }, () => {
  for (const [file, kind] of [["player-heroes/index.html", "heroes"], ["curious/index.html", "curious"]]) {
    const html = readFileSync(join(DIST, file), "utf8");
    assert.match(html, new RegExp(`id="pp" data-page="${kind}" aria-busy="true"`), file);
    assert.match(html, /class="dash-body" id="pp-body"/, file);
    assert.match(html, /class="mode-panels" id="pp-modes"/, file);
    assert.match(html, /data-switch="mode" data-controls="pp-modes"/, file);
    assert.match(html, /data-switch="detail"/, file);
    assert.match(html, /state-loading/, file);
    assert.match(html, /id="pp-status" role="status" aria-live="polite"/, file);
    assert.match(html, /<meta name="robots" content="noindex">/, file);
  }
});

test("accounts open: the community hero stats are a static frame with reserved room, linked from the menu (T-507)", { skip: !CONFIG }, () => {
  const page = (p: string) => readFileSync(join(DIST, p), "utf8");
  const html = page("heroes/index.html");
  assert.match(html, /id="ch" aria-busy="true"/);
  assert.match(html, /class="dash-body ch-periods" id="ch-body" data-period="30d"/);
  assert.match(html, /class="mode-panels" id="ch-modes"/);
  assert.match(html, /data-switch="mode" data-controls="ch-modes"/);
  assert.match(html, /data-switch="detail"/);
  assert.match(html, /role="group" aria-label="Period" id="ch-period-switch"[^>]*hidden/);
  for (const period of ["30d", "all"]) assert.match(html, new RegExp(`data-value="${period}"`), period);
  assert.match(html, /state-loading/);
  assert.match(html, /id="ch-status" role="status" aria-live="polite"/);
  assert.match(html, /<meta name="robots" content="noindex">/);
  assert.match(page("privacy/index.html"), /id="community-hero-stats"/);
  for (const file of ["index.html", "me/index.html", "privacy/index.html"]) {
    assert.match(page(file), /<a href="\/heroes\/"[^>]*>Hero stats<\/a>/, file);
  }
  assert.match(html, /<a href="\/heroes\/" aria-current="page">/);
  // The player's own heroes page is not the community one.
  assert.doesNotMatch(page("player-heroes/index.html"), /<a href="\/heroes\/" aria-current="page">/);
  // The account page's switch: off until the script reads the owner's choice, with the policy link.
  const account = page("account/index.html");
  assert.match(account, /id="community-form"/);
  assert.match(account, /<input type="checkbox" name="hero_stats">/);
  assert.match(account, /Include my games in community hero stats/);
  assert.match(account, /href="\/privacy\/#community-hero-stats"/);
});

test("accounts open: the leaderboards are a static frame with reserved room, linked from the menu", { skip: !CONFIG }, () => {
  const page = (p: string) => readFileSync(join(DIST, p), "utf8");
  const html = page("leaderboards/index.html");
  assert.match(html, /id="lb" aria-busy="true"/);
  assert.match(html, /class="dash-body lb-periods" id="lb-body" data-period="30d"/);
  assert.match(html, /class="mode-panels" id="lb-modes"/);
  assert.match(html, /data-switch="mode" data-controls="lb-modes"/);
  assert.match(html, /data-switch="detail"/);
  assert.match(html, /role="group" aria-label="Period" id="lb-period-switch"[^>]*hidden/);
  for (const period of ["7d", "30d", "all"]) assert.match(html, new RegExp(`data-value="${period}"`), period);
  assert.match(html, /state-loading/);
  assert.match(html, /id="lb-status" role="status" aria-live="polite"/);
  // Public, but kept out of search engines (it lists display names); every page's menu links to it.
  assert.match(html, /<meta name="robots" content="noindex">/);
  assert.match(page("privacy/index.html"), /id="public-profiles-and-leaderboards"/);
  for (const file of ["index.html", "me/index.html", "privacy/index.html"]) {
    assert.match(page(file), /<a href="\/leaderboards\/"[^>]*>Leaderboards<\/a>/, file);
  }
  assert.match(html, /<a href="\/leaderboards\/" aria-current="page">/);
});

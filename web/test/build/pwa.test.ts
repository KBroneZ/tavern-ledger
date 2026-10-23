// Checks on the built site for the installable web app (T-637): the manifest
// in both languages, the icons, what every page links, the service worker and
// its headers (accounts open), and that the closed build ships no script and
// no worker. Run after `npm run build`, like the other build tests.
import { test } from "node:test";
import assert from "node:assert/strict";
import { existsSync, readdirSync, readFileSync, statSync } from "node:fs";
import { join, relative } from "node:path";
import { Script } from "node:vm";
import { loadEnv } from "vite";
import { contentSecurityPolicy, readConfig, WASM_WORKER } from "../../src/lib/config.ts";
import { builtFile, killSwitchSource, NEVER_CACHED } from "../../src/lib/pwa/build.ts";
import { THEME_COLOR } from "../../src/lib/pwa/manifest.ts";

const ROOT = join(import.meta.dirname, "..", "..");
const DIST = join(ROOT, "dist");
const CONFIG = readConfig(loadEnv("production", ROOT, "PUBLIC_"));

function files(dir: string): string[] {
  return readdirSync(dir).flatMap((name) => {
    const path = join(dir, name);
    return statSync(path).isDirectory() ? files(path) : [path];
  });
}

const BUILT = existsSync(DIST) ? files(DIST).map((f) => relative(DIST, f).replaceAll("\\", "/")) : [];
const PAGES = BUILT.filter((f) => f.endsWith(".html"));
const read = (file: string) => readFileSync(join(DIST, file), "utf8");
const isSpanish = (page: string) => page.startsWith("es/");
const manifest = (lang: "en" | "es") => JSON.parse(read(lang === "es" ? "es/manifest.webmanifest" : "manifest.webmanifest"));

/** Width and height from a PNG's header. */
function pngSize(file: string): [number, number] {
  const bytes = readFileSync(join(DIST, file));
  assert.equal(bytes.subarray(1, 4).toString("latin1"), "PNG", file);
  return [bytes.readUInt32BE(16), bytes.readUInt32BE(20)];
}

test("both manifests name one app, the whole site and the page to open on", () => {
  for (const lang of ["en", "es"] as const) {
    const m = manifest(lang);
    assert.equal(m.id, "/");
    assert.equal(m.scope, "/");
    assert.equal(m.display, "standalone");
    assert.equal(m.lang, lang);
    assert.equal(m.start_url, lang === "es" ? "/es/" : "/");
    assert.equal(m.name, "Tavern Ledger");
    assert.ok(m.short_name.length > 0 && m.description.length > 0);
    assert.ok(PAGES.includes(builtFile(m.start_url)), "the start page is built");
  }
  assert.notEqual(manifest("en").description, manifest("es").description);
});

test("the manifest's colours are the site's page background", () => {
  const css = readFileSync(join(ROOT, "src/styles/global.css"), "utf8");
  assert.equal(/--bg:\s*(#[0-9a-f]{6});/i.exec(css)?.[1], THEME_COLOR);
  for (const lang of ["en", "es"] as const) {
    assert.equal(manifest(lang).theme_color, THEME_COLOR);
    assert.equal(manifest(lang).background_color, THEME_COLOR);
  }
});

test("the icons are our own mark at 192, 512 and a maskable 512, and exist at the sizes the manifest says", () => {
  const icons = manifest("en").icons as { src: string; sizes: string; type: string; purpose: string }[];
  assert.deepEqual(
    icons.map((i) => [i.src, i.sizes, i.purpose]),
    [
      ["/icons/icon-192.png", "192x192", "any"],
      ["/icons/icon-512.png", "512x512", "any"],
      ["/icons/icon-maskable-512.png", "512x512", "maskable"],
    ],
  );
  for (const icon of icons) {
    assert.equal(icon.type, "image/png");
    assert.deepEqual(pngSize(icon.src.slice(1)), icon.sizes.split("x").map(Number), icon.src);
  }
  assert.deepEqual(manifest("es").icons, icons);
  // The iOS icon is the one the pages already linked.
  assert.deepEqual(pngSize("apple-touch-icon.png"), [180, 180]);
});

test("every page links its own language's manifest, the bar colour and the iOS tags", () => {
  assert.ok(PAGES.length > 6, "run `npm run build` first");
  for (const page of PAGES) {
    const html = read(page);
    const want = isSpanish(page) ? "/es/manifest.webmanifest" : "/manifest.webmanifest";
    assert.ok(html.includes(`<link rel="manifest" href="${want}">`), page);
    assert.ok(html.includes(`<meta name="theme-color" content="${THEME_COLOR}">`), page);
    assert.ok(html.includes('<meta name="apple-mobile-web-app-capable" content="yes">'), page);
    assert.ok(html.includes('<meta name="mobile-web-app-capable" content="yes">'), page);
    assert.ok(html.includes('<link rel="apple-touch-icon" href="/apple-touch-icon.png">'), page);
  }
});

test("the offline page exists in both languages, is not indexed, and is not in the sitemap", () => {
  for (const [page, retry] of [
    ["offline/index.html", "Try again"],
    ["es/offline/index.html", "Volver a intentarlo"],
  ] as const) {
    const html = read(page);
    assert.match(html, /<meta name="robots" content="noindex">/, page);
    assert.equal(html.includes(`data-reload>${retry}</button>`), CONFIG !== null, `${page}: the retry button needs the page script`);
  }
  assert.doesNotMatch(read("sitemap.xml"), /offline/);
});

test("the CSP lets pages link the manifest, and register the worker only when there are scripts", () => {
  const policy = contentSecurityPolicy(CONFIG?.url ?? null);
  assert.match(policy, /(^|; )manifest-src 'self'(;|$)/);
  assert.equal(/worker-src/.test(policy), CONFIG !== null);
  for (const page of PAGES) {
    const meta = /http-equiv="Content-Security-Policy" content="([^"]*)"/.exec(read(page))?.[1] ?? "";
    assert.match(meta, /(^|; )manifest-src 'self'(;|$)/, page);
    assert.equal(/(^|; )worker-src 'self'(;|$)/.test(meta), CONFIG !== null, page);
  }
});

if (CONFIG === null) {
  test("accounts closed: no offline worker, only the script that removes an older one; no web app script, no install line", () => {
    assert.ok(!BUILT.includes("pwa.js"));
    assert.equal(read("sw.js"), killSwitchSource());
    assert.match(read("sw.js"), /unregister\(\)/);
    assert.ok(!/fetch|precache/.test(read("sw.js")), "it answers no request");
    // It is never cached, so a browser with an older worker sees it on its next visit.
    const rules = read("_headers")
      .split(/\n(?=\S)/)
      .map((block) => block.trim().split("\n").map((l) => l.trim()));
    assert.deepEqual(rules.find((r) => r[0] === "/sw.js"), ["/sw.js", "Cache-Control: no-cache"]);
    for (const page of PAGES) {
      const html = read(page);
      assert.ok(!html.includes("pwa.js") && !html.includes("data-install"), page);
    }
  });
} else {
  const sw = () => read("sw.js");
  
  const buildOf = () =>
    JSON.parse(/const BUILD = (\{[\s\S]*?\n\});/.exec(sw())![1].replace(/;$/, "")) as { version: string; precache: string[]; never: string[]; known: string[] };

  test("every page loads the web app's script after the page, and has the install line, hidden", () => {
    for (const page of PAGES) {
      const html = read(page);
      assert.equal(html.match(/<script defer src="\/pwa\.js"><\/script>/g)?.length, 1, page);
      assert.match(
        html,
        /<p class="install" data-install hidden><button class="install-button" type="button" data-install-button hidden>[^<]+<\/button><span data-install-ios hidden>[^<]+<\/span><\/p>/,
        page,
      );
    }
    assert.ok(read("es/index.html").includes("Añadir a la pantalla de inicio"));
    assert.ok(read("index.html").includes("Add to home screen"));
  });

  test("the service worker is plain script that compiles, with a version and the shell it keeps", () => {
    assert.doesNotThrow(() => new Script(sw(), { filename: "sw.js" }));
    assert.doesNotMatch(sw(), /^export /m, "the rules' exports are stripped");
    const build = buildOf();
    assert.match(build.version, /^[0-9a-f]{12}$/);
    for (const path of ["/offline/", "/es/offline/", "/lang.js", "/pwa.js", "/icons/icon-192.png"]) assert.ok(build.precache.includes(path), path);
    for (const path of build.precache) assert.ok(BUILT.includes(builtFile(path)), `${path} is built`);
  });

  test("the worker keeps hashed styles and scripts, never the simulator's engine or any page but the offline ones", () => {
    const { precache } = buildOf();
    assert.ok(precache.some((p) => p.startsWith("/_astro/") && p.endsWith(".css")));
    assert.ok(precache.some((p) => p.startsWith("/_astro/") && p.endsWith(".js")));
    assert.ok(!precache.some((p) => WASM_WORKER.test(p.replace("/_astro/", ""))), "the simulator's worker is kept when first used");
    assert.ok(!precache.some((p) => p.endsWith(".wasm")));
    assert.deepEqual(precache.filter((p) => p.endsWith("/")), ["/offline/", "/es/offline/"]);
  });

  test("the worker only keeps addresses the build made: every one is a built page or file, none is an account page, and the install's files are among them", () => {
    const { known, precache, never } = buildOf();
    for (const address of known) assert.ok(BUILT.includes(builtFile(address)), `${address} is built`);
    for (const address of never) assert.ok(!known.includes(address), `${address} is not kept`);
    for (const address of ["/404.html", "/es/404.html", "/sw.js", "/_headers"]) assert.ok(!known.includes(address), address);
    for (const path of precache) assert.ok(known.includes(path), `${path} is kept at install`);
    assert.ok(known.includes("/privacy/") && known.includes("/es/privacy/") && known.includes("/"));
  });

  test("the worker's never list is the account, dashboard and sign-in pages, each a built page", () => {
    assert.deepEqual(buildOf().never, NEVER_CACHED);
    for (const path of NEVER_CACHED) assert.ok(BUILT.includes(builtFile(path)), `${path} is a built page`);
    for (const word of ["supabase.co", "ci-placeholder", "apikey", "sb_publishable"]) assert.ok(!sw().toLowerCase().includes(word), `no ${word} in the worker`);
  });

  test("_headers: the worker is never cached and gets its own policy, which only changes what it may fetch", () => {
    const rules = read("_headers")
      .split(/\n(?=\S)/)
      .map((block) => block.trim().split("\n").map((l) => l.trim()));
    const rule = rules.find((r) => r[0] === "/sw.js");
    const policy = `${contentSecurityPolicy(CONFIG!.url, { serviceWorker: true })}; frame-ancestors 'none'`;
    assert.deepEqual(rule, ["/sw.js", "! Content-Security-Policy", `Content-Security-Policy: ${policy}`, "Cache-Control: no-cache"]);
    assert.ok(policy.includes("connect-src 'self';"));
    assert.ok(!policy.includes(CONFIG!.url), "the worker never talks to Supabase");
  });
}

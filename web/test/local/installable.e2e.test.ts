// End-to-end check of the installable web app (T-637) in a real Chromium: the
// browser's own installability test, what the worker keeps and never keeps,
// and the offline pages, on an Android Chrome emulation (phone screen, touch,
// user agent) and an iPhone Safari one for the home-screen line. The site is
// served from dist/ with the rules of dist/_headers, as Cloudflare Pages
// would; "offline" is the server stopping, since the browser's own offline
// switch does not reach a worker's requests.
//
// Skipped unless TAVERN_BROWSER names Edge or Chrome and `npm run build` has
// built the accounts-open site (PUBLIC_SUPABASE_URL and the others set).
//
//   $env:TAVERN_BROWSER = "C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe"; npm run test:local
//
// Not covered, because Chromium cannot say it: Safari's own behaviour (it has
// no install prompt; the footer line is the only help it gets) and the
// installed app's standalone window, which need a phone.

import { after, before, describe, test } from "node:test";
import assert from "node:assert/strict";
import { existsSync } from "node:fs";
import { gzipSync } from "node:zlib";
import { join } from "node:path";
import { ANDROID_CHROME, Browser, IPHONE_SAFARI, serveDist, type Site } from "../helpers/browser.ts";
import { record } from "../unit/gameFixtures.ts";

const EXECUTABLE = process.env.TAVERN_BROWSER ?? "";
const DIST = join(import.meta.dirname, "..", "..", "dist");
const RUN = EXECUTABLE !== "" && existsSync(EXECUTABLE) && existsSync(join(DIST, "pwa.js"));

describe("installable web app", { skip: RUN ? false : "set TAVERN_BROWSER and build the accounts-open site" }, () => {
  let site: Site;
  let browser: Browser;

  before(async () => {
    site = await serveDist(DIST);
    browser = await Browser.launch(EXECUTABLE);
    await browser.emulate(ANDROID_CHROME, 412, 915);
  });

  after(async () => {
    await browser?.close();
    await site?.stop();
  });

  const title = () => browser.evaluate<string>("document.title");
  const heading = () => browser.evaluate<string>("document.querySelector('main h1')?.textContent ?? ''");
  const kept = () =>
    browser.evaluate<string[]>(
      `(async () => (await Promise.all((await caches.keys()).map(async (n) => (await (await caches.open(n)).keys()).map((r) => new URL(r.url).pathname + new URL(r.url).search)))).flat())()`,
    );

  test("the browser finds the site installable, with a manifest that has no errors", async () => {
    await browser.go(`${site.origin}/`, 1500);
    const worker = await browser.evaluate<{ scope: string; script: string }>(
      "navigator.serviceWorker.ready.then((r) => ({ scope: r.scope, script: r.active.scriptURL }))",
    );
    assert.deepEqual(worker, { scope: `${site.origin}/`, script: `${site.origin}/sw.js` });
    await new Promise((done) => setTimeout(done, 1500));
    assert.deepEqual((await browser.page("Page.getAppManifest")).errors, []);
    assert.deepEqual((await browser.page("Page.getInstallabilityErrors")).installabilityErrors, []);
    assert.ok(await browser.evaluate<boolean>("!!navigator.serviceWorker.controller"), "the worker controls the page");
    assert.ok((await browser.screenshot()).length > 1000);
  });

  test("online, pages are kept; account, sign-in and game pages and any address with a query never are", async () => {
    await browser.evaluate("localStorage.setItem('tl-lang', 'en')");
    for (const path of ["/privacy/", "/es/privacy/", "/me/", "/account/", "/signin/?code=SECRETCODE", "/game/?id=1", "/games/", "/curious/", "/forgot-password/"]) {
      await browser.go(`${site.origin}${path}`);
    }
    const files = await kept();
    for (const path of ["/offline/", "/es/offline/", "/privacy/", "/es/privacy/", "/lang.js", "/pwa.js", "/icons/icon-192.png"]) {
      assert.ok(files.includes(path), `${path} is kept`);
    }
    assert.ok(files.some((f) => f.startsWith("/_astro/") && f.endsWith(".css")));
    for (const file of files) {
      assert.doesNotMatch(file, /^\/(es\/)?(me|account|signin|games|game|curious|forgot-password|desktop-sign-in|player-heroes)\//, `${file} must not be kept`);
      assert.ok(!file.includes("?"), `${file} has a query`);
      assert.doesNotMatch(file, /SECRETCODE/);
    }
    assert.ok(!files.some((f) => /simWorker/.test(f)), "the simulator's worker is kept only when used");
  });

  test("offline, a visited page shows from the copy; any other, and every account page, shows the offline page", async () => {
    await site.stop();
    try {
      await browser.go(`${site.origin}/privacy/`, 1500);
      assert.equal(await title(), "Privacy · Tavern Ledger");
      for (const path of ["/simulator/", "/me/", "/account/", "/signin/", "/game/?id=1", "/leaderboards/"]) {
        await browser.go(`${site.origin}${path}`, 1500);
        assert.equal(await heading(), "You are offline", path);
        assert.equal(await browser.evaluate("!!document.querySelector('[data-reload]')"), true, `${path}: the retry button`);
      }
      await browser.evaluate("localStorage.setItem('tl-lang', 'es')");
      await browser.go(`${site.origin}/es/simulator/`, 1500);
      assert.equal(await heading(), "Sin conexión");
    } finally {
      await site.start();
    }
  });

  test("saved games: a saved game opens offline with the game page, the offline page lists it, and an unsaved one shows the offline page", async () => {
    const user = "11111111-1111-4111-8111-111111111111";
    const game = "22222222-2222-4222-8222-222222222222";
    const other = "33333333-3333-4333-8333-333333333333";
    const report = gzipSync(JSON.stringify(record())).toString("base64");
    await browser.evaluate("localStorage.setItem('tl-lang', 'en')");
    // A page the worker controls (a game page, as in use).
    await browser.go(`${site.origin}/game/?u=${user}&id=${other}`, 1500);
    // What the game page saves (lib/pwa/saved.ts): the file and the list, written here the same way, then the message to the worker that keeps the game page.
    await browser.evaluate(`(async () => {
      const cache = await caches.open("tl-replays");
      await cache.put("/__saved__/game/${game}", new Response(Uint8Array.from(atob("${report}"), (c) => c.charCodeAt(0))));
      await cache.put("/__saved__/index", new Response(JSON.stringify([{ userId: "${user}", publicId: "${game}", savedAt: 1, played: "2026-10-24", title: "Pyramad · 3rd" }])));
      (await navigator.serviceWorker.getRegistration()).active.postMessage({ type: "keep-game-shell" });
      for (let i = 0; i < 50 && !(await cache.match("/__saved__/shell/es")); i++) await new Promise((done) => setTimeout(done, 100));
    })()`);
    const saved = await browser.evaluate<string[]>(`caches.open("tl-replays").then((c) => c.keys()).then((ks) => ks.map((r) => new URL(r.url).pathname).sort())`);
    assert.deepEqual(saved, [`/__saved__/game/${game}`, "/__saved__/index", "/__saved__/shell/en", "/__saved__/shell/es"]);
    await site.stop();
    try {
      await browser.go(`${site.origin}/game/?u=${user}&id=${game}`, 2000);
      assert.equal(await browser.evaluate("!!document.getElementById('game-body')"), true, "the game page opens from the device");
      assert.match(await browser.evaluate<string>("document.getElementById('game-body').innerText"), /You are reading the copy saved on this device/, "and draws the saved game");
      await browser.go(`${site.origin}/game/?u=${user}&id=${other}`, 1500);
      assert.equal(await heading(), "You are offline", "a game that was not saved");
      assert.deepEqual(
        await browser.evaluate(`[...document.querySelectorAll("[data-saved-list] a")].map((a) => a.textContent + " | " + a.getAttribute("href"))`),
        [`Pyramad · 3rd · 2026-10-24 | /game/?u=${user}&id=${game}`],
      );
      assert.equal(await browser.evaluate("document.querySelector('[data-saved]').hidden"), false);
    } finally {
      await site.start();
      await browser.evaluate(`caches.delete("tl-replays")`);
    }
  });

  test("the home-screen line on iPhone Safari is words only: Safari has no install prompt", async () => {
    await browser.evaluate("localStorage.setItem('tl-lang', 'en')");
    await browser.emulate(IPHONE_SAFARI, 390, 844);
    // Safari has no install event; hide Chromium's so the page shows what Safari would.
    await browser.page("Page.addScriptToEvaluateOnNewDocument", {
      source: "window.addEventListener('beforeinstallprompt', (e) => e.stopImmediatePropagation(), true)",
    });
    await browser.go(`${site.origin}/`, 1500);
    const line = await browser.evaluate<{ hidden: boolean; words: boolean; button: boolean; text: string }>(
      `(() => { const h = document.querySelector('[data-install]'); return { hidden: h.hidden, words: !h.querySelector('[data-install-ios]').hidden, button: !h.querySelector('[data-install-button]').hidden, text: h.innerText }; })()`,
    );
    assert.deepEqual(line, { hidden: false, words: true, button: false, text: "To add this site to your home screen, tap Share, then “Add to Home Screen”." });
  });
});

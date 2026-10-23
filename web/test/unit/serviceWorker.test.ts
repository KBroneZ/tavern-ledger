// The installable web app's worker (T-637): what it keeps and what it never
// keeps. The generated /sw.js runs in a VM against a stand-in Cache Storage.
import { test } from "node:test";
import assert from "node:assert/strict";
import { cacheName, classify, isNeverCached, isStorable, offlinePage, staleCaches } from "../../src/lib/pwa/rules.js";
import { NEVER_CACHED, killSwitchSource, knownAddresses, precacheList } from "../../src/lib/pwa/build.ts";
import { runInNewContext } from "node:vm";
import { NO_STORE_PAGES, TURNSTILE_PAGES } from "../../src/lib/config.ts";
import { BUILD, ORIGIN, SUPABASE, Worker, page, request } from "../helpers/serviceWorker.ts";

const SCOPE = { origin: ORIGIN, never: NEVER_CACHED, known: new Set(BUILD.known) };
const get = (path: string, mode: "navigate" | "same-origin" = "same-origin") => classify({ method: "GET", url: `${ORIGIN}${path}`, mode }, SCOPE);

test("the never list is every account, dashboard and sign-in page in both languages", () => {
  for (const path of [...NO_STORE_PAGES, ...TURNSTILE_PAGES]) assert.ok(NEVER_CACHED.includes(path), path);
  for (const name of ["signin", "forgot-password", "desktop-sign-in", "account", "me", "games", "game", "player-heroes", "curious"]) {
    assert.ok(NEVER_CACHED.includes(`/${name}/`) && NEVER_CACHED.includes(`/es/${name}/`), name);
  }
});

test("classify: our own pages and files are kept, hashed files are cache-first", () => {
  assert.equal(get("/", "navigate"), "network-first");
  assert.equal(get("/es/privacy/", "navigate"), "network-first");
  assert.equal(get("/fonts/figtree-latin-wght-normal.woff2"), "network-first");
  assert.equal(get("/lang.js"), "network-first");
  assert.equal(get("/_astro/Base.abc.css"), "immutable");
});

test("classify: account, dashboard and sign-in pages are never kept, with or without the closing slash", () => {
  for (const path of NEVER_CACHED) {
    assert.equal(get(path, "navigate"), "private-page", path);
    assert.equal(get(path.slice(0, -1), "navigate"), "private-page", path);
    assert.equal(get(path), "bypass", `${path} as a sub-resource`);
  }
});

test("classify: an address with a query is never kept (an email link's code, a game's id)", () => {
  assert.equal(get("/signin/?code=abc", "navigate"), "private-page");
  assert.equal(get("/game/?id=1", "navigate"), "private-page");
  assert.equal(get("/privacy/?x=1", "navigate"), "private-page");
  assert.equal(get("/lang.js?v=2"), "bypass");
});

test("classify: only an address the build made is kept, spelled as the build spells it", () => {
  // Another spelling of a page the host may serve, or a page that is not there: the network only, nothing kept.
  for (const path of ["/me//", "/%6De/", "/%6de/", "/ME/", "/me/index.html", "/me.html", "/account/index.html", "/privacy", "/privacy/index.html", "/privacy//", "/./privacy/", "/missing/", "/es/me/index.html"]) {
    const url = new URL(path, ORIGIN).pathname;
    const expected = BUILD.known.includes(url) ? "network-first" : "private-page";
    assert.equal(get(path, "navigate"), expected, path);
    assert.equal(get(path), BUILD.known.includes(url) ? "network-first" : "bypass", `${path} as a sub-resource`);
  }
  assert.equal(get("/privacy/", "navigate"), "network-first", "the exact address is kept");
  // A hashed file the build does not have is not trusted as immutable either.
  assert.equal(get("/_astro/Other.zzz.css"), "bypass");
});

test("classify: other origins and anything but GET are left to the browser", () => {
  for (const url of [`${SUPABASE}/rest/v1/games`, "https://art.hearthstonejson.com/v1/256x/X.jpg", "https://assets.tavernledger.net/1/a.png", "https://challenges.cloudflare.com/turnstile/v0/api.js"]) {
    assert.equal(classify({ method: "GET", url, mode: "cors" }, SCOPE), "bypass", url);
  }
  for (const method of ["POST", "PUT", "PATCH", "DELETE", "HEAD"]) {
    assert.equal(classify({ method, url: `${ORIGIN}/privacy/`, mode: "navigate" }, SCOPE), "bypass", method);
  }
  assert.equal(get("/sw.js"), "bypass", "the worker's own script");
});

test("isNeverCached ignores the closing slash, not the path", () => {
  assert.ok(isNeverCached("/me", ["/me/"]) && isNeverCached("/me/", ["/me/"]));
  assert.ok(!isNeverCached("/me-too/", ["/me/"]) && !isNeverCached("/", ["/me/"]));
});

test("isStorable: only a plain 200 from our own address, never no-store or private", () => {
  const make = (init: { status?: number; type?: string; redirected?: boolean; cc?: string }) => ({
    status: init.status ?? 200,
    type: init.type ?? "basic",
    redirected: init.redirected ?? false,
    headers: new Headers(init.cc ? { "Cache-Control": init.cc } : {}),
  });
  assert.ok(isStorable(make({})));
  assert.ok(isStorable(make({ cc: "public, max-age=0, must-revalidate" })));
  assert.ok(isStorable(make({ cc: "public, max-age=31536000, immutable" })));
  for (const bad of [{ status: 404 }, { status: 206 }, { status: 500 }, { type: "opaque" }, { type: "cors" }, { type: "opaqueredirect" }, { redirected: true }, { cc: "no-store" }, { cc: "No-Store" }, { cc: "private" }, { cc: "private, max-age=60" }, { cc: "max-age=0, no-store" }]) {
    assert.ok(!isStorable(make(bad)), JSON.stringify(bad));
  }
});

test("offlinePage follows the address's language", () => {
  assert.equal(offlinePage("/"), "/offline/");
  assert.equal(offlinePage("/download/"), "/offline/");
  assert.equal(offlinePage("/es"), "/es/offline/");
  assert.equal(offlinePage("/es/download/"), "/es/offline/");
  assert.equal(offlinePage("/escape/"), "/offline/");
});

test("staleCaches: ours from another version only", () => {
  const names = [cacheName("old"), cacheName("v1"), "tl-replays", "other-site", "tl-v1"];
  assert.deepEqual(staleCaches(names, "v1"), [cacheName("old")]);
});

test("install keeps the whole shell in this build's cache, then takes over at once", async () => {
  const worker = new Worker();
  worker.network = (req) => page(`copy of ${new URL(req.url).pathname}`);
  await worker.lifecycle("install");
  assert.deepEqual(worker.kept().toSorted(), BUILD.precache.map((p) => `${ORIGIN}${p}`).toSorted());
  assert.deepEqual([...worker.stores.keys()], [cacheName("v1")]);
  assert.ok(worker.skipped);
});

test("install fails, and does not take over, when one shell file is missing", async () => {
  const worker = new Worker();
  worker.network = (req) => (req.url.endsWith(".css") ? page("", {}, 404) : page("ok"));
  await assert.rejects(worker.lifecycle("install"));
  assert.ok(!worker.skipped);
  assert.deepEqual(worker.kept(), [], "no half-filled cache");
});

test("activate deletes the caches of older builds and nothing else, then claims the pages", async () => {
  const worker = new Worker();
  for (const name of [cacheName("v0"), cacheName("v1"), "tl-replays", "someone-else"]) worker.stores.set(name, new Map());
  await worker.lifecycle("activate");
  assert.deepEqual([...worker.stores.keys()].toSorted(), [cacheName("v1"), "someone-else", "tl-replays"].toSorted());
  assert.ok(worker.claimed);
});

test("a visited page is kept and answers offline", async () => {
  const worker = new Worker();
  worker.network = () => page("<h1>Privacy</h1>");
  const online = await worker.fetch(request(`${ORIGIN}/privacy/`, "navigate"));
  assert.equal(await online!.text(), "<h1>Privacy</h1>");
  assert.deepEqual(worker.kept(), [`${ORIGIN}/privacy/`]);
  worker.network = () => {
    throw new TypeError("offline");
  };
  const offline = await worker.fetch(request(`${ORIGIN}/privacy/`, "navigate"));
  assert.equal(await offline!.text(), "<h1>Privacy</h1>");
});

test("a page never seen, offline, shows the offline page in the address's language", async () => {
  const worker = new Worker();
  worker.network = (req) => page(`copy of ${new URL(req.url).pathname}`);
  await worker.lifecycle("install");
  worker.network = () => {
    throw new TypeError("offline");
  };
  assert.equal(await (await worker.fetch(request(`${ORIGIN}/download/`, "navigate")))!.text(), "copy of /offline/");
  assert.equal(await (await worker.fetch(request(`${ORIGIN}/es/download/`, "navigate")))!.text(), "copy of /es/offline/");
  // A sub-resource that is not kept is a plain network error, not a page.
  assert.equal((await worker.fetch(request(`${ORIGIN}/art/x.webp`)))!.type, "error");
});

test("account, dashboard and sign-in pages are fetched and never kept; offline they show the offline page", async () => {
  const worker = new Worker();
  worker.network = (req) => page(`private ${new URL(req.url).pathname}`);
  await worker.lifecycle("install");
  const before = worker.kept().length;
  for (const path of NEVER_CACHED) {
    const response = await worker.fetch(request(`${ORIGIN}${path}`, "navigate"));
    assert.equal(await response!.text(), `private ${path}`);
  }
  assert.equal(worker.kept().length, before, "nothing was added");
  worker.network = () => {
    throw new TypeError("offline");
  };
  assert.equal(await (await worker.fetch(request(`${ORIGIN}/me/`, "navigate")))!.text(), "private /offline/");
});

test("a page marked no-store is not kept even if its address is allowed", async () => {
  const worker = new Worker();
  worker.network = () => page("x", { "Content-Type": "text/html", "Cache-Control": "no-store" });
  await worker.fetch(request(`${ORIGIN}/privacy/`, "navigate"));
  assert.deepEqual(worker.kept(), []);
});

test("an address with a query is not kept, a redirect is not kept, an error is not kept", async () => {
  const worker = new Worker();
  worker.network = () => page("x");
  await worker.fetch(request(`${ORIGIN}/signin/?code=secret`, "navigate"));
  await worker.fetch(request(`${ORIGIN}/privacy/?utm=1`, "navigate"));
  const redirected = page("x");
  Object.defineProperty(redirected, "redirected", { value: true });
  worker.network = () => redirected;
  await worker.fetch(request(`${ORIGIN}/terms/`, "navigate"));
  worker.network = () => page("nope", {}, 404);
  await worker.fetch(request(`${ORIGIN}/missing/`, "navigate"));
  assert.deepEqual(worker.kept(), []);
});

test("other origins and writes are not answered at all, and nothing of theirs is kept", async () => {
  const worker = new Worker();
  worker.network = () => page('{"games":[]}', { "Content-Type": "application/json" });
  assert.equal(await worker.fetch(request(`${SUPABASE}/rest/v1/games?select=*`)), null);
  assert.equal(await worker.fetch(request(`${SUPABASE}/auth/v1/token?grant_type=password`, "same-origin", "POST")), null);
  assert.equal(await worker.fetch(request(`${ORIGIN}/api/anything`, "same-origin", "POST")), null);
  assert.equal(await worker.fetch(request(`${ORIGIN}/sw.js`)), null);
  assert.deepEqual(worker.kept(), []);
  assert.deepEqual(worker.fetched, [], "the worker never fetched for them");
});

test("a hashed file comes from the cache once it was fetched", async () => {
  const worker = new Worker();
  let calls = 0;
  worker.network = () => {
    calls += 1;
    return page("body{}", { "Content-Type": "text/css", "Cache-Control": "public, max-age=31536000, immutable" });
  };
  const file = `${ORIGIN}/_astro/game.BxMRQOT6.css`;
  await worker.fetch(request(file));
  await worker.fetch(request(file));
  assert.equal(calls, 1);
  assert.deepEqual(worker.kept(), [file]);
});

test("a slow network gives way to the kept copy of a page", async () => {
  const worker = new Worker();
  worker.network = () => page("copy");
  await worker.fetch(request(`${ORIGIN}/download/`, "navigate"));
  // The network answers after 300 ms; the test clock's wait is 20 ms.
  worker.network = () => new Promise<Response>((done) => setTimeout(() => done(page("later")), 300));
  const response = await worker.fetch(request(`${ORIGIN}/download/`, "navigate"));
  assert.equal(await response!.text(), "copy");
});

test("knownAddresses: pages by folder, files by path, never the account pages, the 404s, the worker or the host's rules", () => {
  const built = ["index.html", "es/index.html", "privacy/index.html", "me/index.html", "es/me/index.html", "signin/index.html", "404.html", "es/404.html", "_headers", "sw.js", "lang.js", "_astro/Base.abc.css", "fonts/a.woff2", "manifest.webmanifest"];
  assert.deepEqual(knownAddresses(built), ["/", "/_astro/Base.abc.css", "/es/", "/fonts/a.woff2", "/lang.js", "/manifest.webmanifest", "/privacy/"]);
});

test("everything the install keeps is an address the worker may keep", () => {
  const built = ["offline/index.html", "es/offline/index.html", "lang.js", "pwa.js", "mark.svg", "favicon.svg", "icons/icon-192.png", "fonts/bricolage-grotesque-latin-wdth-normal.woff2", "fonts/figtree-latin-wght-normal.woff2", "_astro/a.css", "_astro/b.js", "_astro/simWorker-AbCd.js"];
  const known = knownAddresses(built);
  const precache = precacheList(built);
  assert.ok(precache.includes("/_astro/a.css") && !precache.includes("/_astro/simWorker-AbCd.js"));
  for (const path of precache) assert.ok(known.includes(path), path);
});

test("the kill switch of a build with no worker removes our caches and the old worker, and nothing else", async () => {
  const handlers = new Map<string, (event: { waitUntil(p: Promise<unknown>): void }) => void>();
  const deleted: string[] = [];
  let unregistered = false;
  let skipped = false;
  runInNewContext(killSwitchSource(), {
    self: {
      addEventListener: (type: string, handler: never) => handlers.set(type, handler),
      skipWaiting: () => void (skipped = true),
      registration: { unregister: async () => void (unregistered = true) },
    },
    caches: { keys: async () => [cacheName("a"), "tl-replays", "other"], delete: async (name: string) => void deleted.push(name) },
  });
  handlers.get("install")!({ waitUntil: () => {} });
  const waits: Promise<unknown>[] = [];
  handlers.get("activate")!({ waitUntil: (p) => void waits.push(p) });
  await Promise.all(waits);
  assert.ok(skipped && unregistered);
  assert.deepEqual(deleted, [cacheName("a")]);
});

test("a slow network is waited for when there is no copy of the page to show instead", async () => {
  const worker = new Worker();
  worker.network = () => new Promise<Response>((done) => setTimeout(() => done(page("late but real")), 100));
  const response = await worker.fetch(request(`${ORIGIN}/download/`, "navigate"));
  assert.equal(await response!.text(), "late but real", "the test clock gives up after 20 ms, so a wait of 100 ms shows the page was waited for");
  assert.deepEqual(worker.kept(), [`${ORIGIN}/download/`]);
});

test("install fails, and keeps nothing, when a shell file is redirected or marked no-store", async () => {
  for (const make of [
    () => {
      const redirected = page("sign in");
      Object.defineProperty(redirected, "redirected", { value: true });
      return redirected;
    },
    () => page("private", { "Content-Type": "text/html", "Cache-Control": "no-store" }),
    () => page("private", { "Content-Type": "text/html", "Cache-Control": "private, max-age=60" }),
  ]) {
    const worker = new Worker();
    worker.network = (req) => (req.url.endsWith(".css") ? make() : page("ok"));
    await assert.rejects(worker.lifecycle("install"), /not a plain answer/);
    assert.deepEqual(worker.kept(), []);
    assert.ok(!worker.skipped);
  }
});

test("a page that answers after the wait is still kept, so the next visit has the new one", async () => {
  const worker = new Worker();
  worker.network = () => page("old");
  await worker.fetch(request(`${ORIGIN}/privacy/`, "navigate"));
  worker.network = () => new Promise<Response>((done) => setTimeout(() => done(page("new")), 100));
  // The wait is 20 ms on the test clock: the old copy shows, and the new answer replaces it once it arrives.
  assert.equal(await (await worker.fetch(request(`${ORIGIN}/privacy/`, "navigate")))!.text(), "old");
  worker.network = () => {
    throw new TypeError("offline");
  };
  assert.equal(await (await worker.fetch(request(`${ORIGIN}/privacy/`, "navigate")))!.text(), "new");
});

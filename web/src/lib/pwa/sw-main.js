// The service worker's events (T-637). The build puts, in this order, a
// header comment, `const BUILD = { version, precache, never, known }`, the
// rules (rules.js) and this file into /sw.js (lib/pwa/build.ts). What may be
// kept and what never is decided in rules.js; this file only fetches, stores
// and answers.
//
// Updates: a new build makes a new /sw.js (the version is in it), the browser
// installs it next to the old one, and it takes over at once (skipWaiting and
// claim). That is safe because pages are network-first: a page always comes
// with the files of its own build, and a tab left open on the old build asks
// the network for any file the new cache does not have.

const NETWORK_WAIT_MS = 5000;
// The site answers on more than one address (the public one, Cloudflare's, the local preview): the worker's own is the one that counts.
const SCOPE = { origin: self.location.origin, never: BUILD.never, known: new Set(BUILD.known) };

self.addEventListener("install", (event) => {
  event.waitUntil(precache().then(() => self.skipWaiting()));
});

self.addEventListener("activate", (event) => {
  event.waitUntil(dropOldCaches().then(() => self.clients.claim()));
});

// A game page asks for the game page itself to be kept with the saved games (lib/pwa/saved.ts askForShells).
self.addEventListener("message", (event) => {
  if (event.data && event.data.type === KEEP_SHELL_MESSAGE) event.waitUntil(keepGameShells().catch(() => {}));
});

self.addEventListener("fetch", (event) => {
  const kind = classify(event.request, SCOPE);
  if (kind === "bypass") return;
  if (kind === "private-page") event.respondWith(fetchOrOffline(event.request));
  else if (kind === "immutable") event.respondWith(cacheFirst(event));
  else event.respondWith(networkFirst(event));
});

/**
 * All or nothing: one file missing, redirected or marked private fails the
 * install, and the old worker keeps working. Every answer goes through the same
 * rule as a later copy (isStorable) before any of them is put in the cache.
 */
async function precache() {
  const answers = await Promise.all(
    BUILD.precache.map(async (path) => {
      const response = await fetch(new Request(new URL(path, SCOPE.origin), { cache: "reload" }));
      if (!isStorable(response)) throw new Error(`${path} is not a plain answer, so it is not kept`);
      return [path, response];
    }),
  );
  const cache = await caches.open(cacheName(BUILD.version));
  for (const [path, response] of answers) await cache.put(new URL(path, SCOPE.origin).href, response);
}

async function dropOldCaches() {
  for (const name of staleCaches(await caches.keys(), BUILD.version)) await caches.delete(name);
}

/** This build's copy of a request, if there is one (never another version's or another feature's cache). */
async function lookup(request) {
  const cache = await caches.open(cacheName(BUILD.version));
  return cache.match(request);
}

/** Puts a copy of a storable answer in this build's cache, without delaying the answer. */
function keep(event, request, response) {
  if (!isStorable(response)) return;
  const copy = response.clone();
  event.waitUntil(caches.open(cacheName(BUILD.version)).then((cache) => cache.put(request, copy)));
}

/** The network, and the offline page when there is none; nothing is stored. */
async function fetchOrOffline(request) {
  try {
    return await fetch(request);
  } catch {
    return (await savedGamePage(request)) ?? offlineResponse(request);
  }
}

/**
 * The game page shell, for the address of a game the player saved on this
 * device (rules.js savedGame). Only when both the shell and the game's file
 * are saved, and only read: the worker never writes to that cache. `caches.match`
 * with a cache name does not create the cache when nobody saved anything.
 */
async function savedGamePage(request) {
  const url = new URL(request.url);
  const saved = savedGame(url.pathname, url.search);
  if (!saved) return null;
  const [shell, game] = await Promise.all([saved.shell, saved.game].map((key) => caches.match(key, { cacheName: SAVED_CACHE })));
  return shell && game ? shell : null;
}

/**
 * Fetches the game page in both languages and puts it with the saved games.
 * Only for a player who has saved a game (the cache exists), and only a plain
 * 200 from our own address. The pages carry `no-store`, which is why isStorable
 * is not the test here: the page is the same static shell for everyone and holds
 * no account data, and it is useless offline unless it is kept.
 */
async function keepGameShells() {
  if (!(await caches.has(SAVED_CACHE))) return;
  const langs = Object.keys(GAME_PAGES);
  const answers = await Promise.all(langs.map((lang) => fetch(new Request(new URL(GAME_PAGES[lang], SCOPE.origin), { cache: "reload", credentials: "omit" }))));
  if (!answers.every((r) => r.status === 200 && r.type === "basic" && !r.redirected)) return;
  const cache = await caches.open(SAVED_CACHE);
  for (const [i, lang] of langs.entries()) await cache.put(savedShellKey(lang), answers[i]);
}

async function offlineResponse(request) {
  const page = await lookup(offlinePage(new URL(request.url).pathname));
  return page ?? Response.error();
}

async function cacheFirst(event) {
  const request = event.request;
  const cached = await lookup(request);
  if (cached) return cached;
  const response = await fetch(request);
  keep(event, request, response);
  return response;
}

/** Waits for an answer, but not for ever: on a bad connection the copy of the page shows instead. */
function withinWait(answer) {
  return new Promise((resolve, reject) => {
    const timer = setTimeout(() => reject(new Error("the network is too slow")), NETWORK_WAIT_MS);
    answer.then(
      (response) => {
        clearTimeout(timer);
        resolve(response);
      },
      (error) => {
        clearTimeout(timer);
        reject(error);
      },
    );
  });
}

/** Puts a copy of a storable answer in this build's cache; the copy is made before anything reads the answer. */
async function keepNow(request, response) {
  if (!isStorable(response)) return;
  const copy = response.clone();
  const cache = await caches.open(cacheName(BUILD.version));
  await cache.put(request, copy);
}

async function networkFirst(event) {
  const request = event.request;
  // The fetch starts at once and its answer is kept whenever it arrives, even after a slow page gave way to its
  // copy: the next visit then has the new page. The event is asked to wait for it before the first await.
  const fetched = fetch(request);
  event.waitUntil(fetched.then((response) => keepNow(request, response)).catch(() => {}));
  // A page gives up on a slow network only when it has a copy to show instead; without one it waits for the network.
  const copy = request.mode === "navigate" ? await lookup(request) : undefined;
  try {
    return await (copy ? withinWait(fetched) : fetched);
  } catch {
    const cached = copy ?? (await lookup(request));
    if (cached) return cached;
    return request.mode === "navigate" ? offlineResponse(request) : Response.error();
  }
}

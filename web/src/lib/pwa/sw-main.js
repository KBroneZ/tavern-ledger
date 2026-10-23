// The service worker's events (T-637). The build puts, in this order, a
// header comment, `const BUILD = { version, precache, never }`, the
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
const SCOPE = { origin: self.location.origin, never: BUILD.never };

self.addEventListener("install", (event) => {
  event.waitUntil(precache().then(() => self.skipWaiting()));
});

self.addEventListener("activate", (event) => {
  event.waitUntil(dropOldCaches().then(() => self.clients.claim()));
});

self.addEventListener("fetch", (event) => {
  const kind = classify(event.request, SCOPE);
  if (kind === "bypass") return;
  if (kind === "private-page") event.respondWith(fetchOrOffline(event.request));
  else if (kind === "immutable") event.respondWith(cacheFirst(event));
  else event.respondWith(networkFirst(event));
});

/** All or nothing: one file missing fails the install, and the old worker keeps working. */
async function precache() {
  const cache = await caches.open(cacheName(BUILD.version));
  await cache.addAll(BUILD.precache.map((path) => new Request(new URL(path, SCOPE.origin), { cache: "reload" })));
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
    return offlineResponse(request);
  }
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

/** Waits for the network, but for a page not for ever: on a bad connection the copy shows instead. */
function networkWithTimeout(request) {
  if (request.mode !== "navigate") return fetch(request);
  return new Promise((resolve, reject) => {
    const timer = setTimeout(() => reject(new Error("the network is too slow")), NETWORK_WAIT_MS);
    fetch(request).then(
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

async function networkFirst(event) {
  const request = event.request;
  try {
    const response = await networkWithTimeout(request);
    keep(event, request, response);
    return response;
  } catch {
    const cached = await lookup(request);
    if (cached) return cached;
    return request.mode === "navigate" ? offlineResponse(request) : Response.error();
  }
}

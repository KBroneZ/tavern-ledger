// What the service worker may keep on the phone, and what never (T-637). Plain
// JavaScript with no imports, because the build pastes this file into
// /sw.js (lib/pwa/build.ts, without the `export` keywords) while the unit tests
// import it as it is. The rules only look at the request and the response;
// the worker (sw-main.js) does the fetching and the storing.
//
// The line it draws: the site's own pages and files, as the build names them
// and visited without a query in the address, may be kept to read again offline. Nothing else is ever
// stored (but for the player's own saved games and the game page shells, at the
// end of this file, which the player turns on): no other origin (Supabase, the card art host, the anti-robot
// check), no page about one account, no sign-in page, no address with a query
// (an email link's code, a game's id), no response that says `no-store`.

/** Every cache this worker makes starts with this and the build's version; old versions are deleted on activate. */
export const CACHE_PREFIX = "tl-v-";

/** The folder of files whose names carry a content hash: they never change, so the cache answers first. */
export const IMMUTABLE_PREFIX = "/_astro/";

/** The page shown when a page is not on the device and there is no connection, one per language; the install keeps both (build.ts). */
export const OFFLINE_PAGES = { en: "/offline/", es: "/es/offline/" };

/** @param {string} version */
export const cacheName = (version) => `${CACHE_PREFIX}${version}`;

/**
 * The caches an activating worker deletes: ours, from another version.
 * @param {string[]} names
 * @param {string} version
 */
export const staleCaches = (names, version) => names.filter((name) => name.startsWith(CACHE_PREFIX) && name !== cacheName(version));

/** "/me/" and "/me" are the same page; the host redirects one to the other. */
const bare = (path) => (path.length > 1 && path.endsWith("/") ? path.slice(0, -1) : path);

/**
 * Whether this path is a page that must never be kept: `never` is the build's
 * list of account, dashboard and sign-in pages in both languages (config.ts).
 * @param {string} pathname
 * @param {readonly string[]} never
 */
export const isNeverCached = (pathname, never) => never.some((path) => bare(path) === bare(pathname));

/**
 * What the worker does with a request:
 *  - "bypass": it does not answer; the browser goes to the network as if there were no worker.
 *    Everything that is not a plain GET of our own files with no query.
 *  - "private-page": a page about one account or a sign-in page, any page with a query, and any
 *    page address the build does not list (another spelling of a page, such as /me/index.html or
 *    /%6De/, or a page that does not exist). The network answers and nothing is stored; offline,
 *    the offline page shows instead.
 *  - "immutable": a hashed file; the cache answers first.
 *  - "network-first": the rest of our files; the network answers, a copy is kept, and offline the
 *    copy answers.
 * @param {{ method: string, url: string, mode: string }} request
 * @param {{ origin: string, never: readonly string[], known: ReadonlySet<string> }} build
 * @returns {"bypass" | "private-page" | "immutable" | "network-first"}
 */
export function classify(request, build) {
  if (request.method !== "GET") return "bypass";
  const url = new URL(request.url);
  if (url.origin !== build.origin) return "bypass";
  // The worker's own script is fetched by the browser to check for updates; it must never be served from a copy.
  if (url.pathname === "/sw.js") return "bypass";
  const navigation = request.mode === "navigate";
  if (isNeverCached(url.pathname, build.never) || url.search !== "") return navigation ? "private-page" : "bypass";
  // Only an address the build made is ever kept, spelled exactly as the build spells it: a host may serve
  // one page under several spellings, and a copy of one the never list does not name must not slip in.
  if (!build.known.has(url.pathname)) return navigation ? "private-page" : "bypass";
  if (url.pathname.startsWith(IMMUTABLE_PREFIX)) return "immutable";
  return "network-first";
}

/**
 * Whether a response may be put in the cache. A copy is made only of a plain
 * answer to our own address: not an error, not a redirect (a redirect can be
 * a sign-in), not a partial answer, and nothing the site marked `no-store` or
 * `private`.
 * @param {{ status: number, type: string, redirected: boolean, headers: { get(name: string): string | null } }} response
 */
export function isStorable(response) {
  if (response.status !== 200 || response.type !== "basic" || response.redirected) return false;
  const control = (response.headers.get("Cache-Control") ?? "").toLowerCase();
  if (/(^|[\s,])(no-store|private)([\s,=]|$)/.test(control)) return false;
  return true;
}

/**
 * The offline page in the language of the address that failed: Spanish
 * addresses start with /es/ (lang.js sends everyone to their own language).
 * @param {string} pathname
 */
export const offlinePage = (pathname) => (pathname === "/es" || pathname.startsWith("/es/") ? OFFLINE_PAGES.es : OFFLINE_PAGES.en);

// Saved games (T-637, second part). A player who turns on "Keep my last games
// on this device" has the game page store, in a cache of its own, the report
// file of each of their own games they open and a copy of the game page
// (a static shell, the same for everyone). The page writes the files and the
// list; the worker only reads them, to open a saved game's address when there is
// no connection, and writes one thing itself, the game page shells, when a page
// asks it to (sw-main.js keepGameShells), because the page may not fetch its own site.

/** The cache of saved games. Its name does not start with CACHE_PREFIX, so a new build never deletes it. */
export const SAVED_CACHE = "tl-replays";

/** Where, inside that cache, the page puts its entries. These addresses are only keys: nothing is ever fetched from them. */
export const SAVED_PREFIX = "/__saved__/";
export const SAVED_INDEX_KEY = `${SAVED_PREFIX}index`;
/** @param {string} publicId */
export const savedGameKey = (publicId) => `${SAVED_PREFIX}game/${publicId}`;
/** @param {"en" | "es"} lang */
export const savedShellKey = (lang) => `${SAVED_PREFIX}shell/${lang}`;

/** The message a game page sends the worker after saving a game: "fetch the game page again and keep it with the saved games". */
export const KEEP_SHELL_MESSAGE = "keep-game-shell";

/** The game pages, one per language: the only pages a saved game may open offline. */
export const GAME_PAGES = { en: "/game/", es: "/es/game/" };

const UUID = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/;

/**
 * What a game page's address needs to open offline: the cache keys of the page
 * shell and of the game's file, and the player and game it names; or null when
 * the address is not a game page with a valid player and game id.
 * @param {string} pathname
 * @param {string} search
 * @returns {{ shell: string, game: string, userId: string, publicId: string } | null}
 */
export function savedGame(pathname, search) {
  const lang = Object.keys(GAME_PAGES).find((l) => bare(GAME_PAGES[l]) === bare(pathname));
  if (!lang) return null;
  const query = new URLSearchParams(search);
  const userId = (query.get("u") ?? "").toLowerCase();
  const publicId = (query.get("id") ?? "").toLowerCase();
  if (!UUID.test(userId) || !UUID.test(publicId)) return null;
  return { shell: savedShellKey(/** @type {"en" | "es"} */ (lang)), game: savedGameKey(publicId), userId, publicId };
}

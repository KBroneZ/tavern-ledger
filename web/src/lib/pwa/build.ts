// Build-time side of the installable web app (T-637): which files the worker
// keeps ready, the version that names its caches, and /sw.js itself. Used by
// astro.config.mjs after the pages are built; the unit tests run the same
// functions on a small made-up folder.
import { createHash } from "node:crypto";
import { readdirSync, readFileSync, statSync, writeFileSync } from "node:fs";
import { NO_STORE_PAGES, TURNSTILE_PAGES, WASM_WORKER } from "../config.ts";
import { CACHE_PREFIX, SAVED_CACHE } from "./rules.js";

/** What the install keeps ready, besides the hashed files of /_astro/ (see precacheList). */
export const SHELL_FILES: readonly string[] = [
  "/offline/",
  "/es/offline/",
  "/lang.js",
  "/pwa.js",
  "/mark.svg",
  "/favicon.svg",
  "/icons/icon-192.png",
  "/fonts/bricolage-grotesque-latin-wdth-normal.woff2",
  "/fonts/figtree-latin-wght-normal.woff2",
];

/**
 * Pages the worker never keeps a copy of, in both languages: the account and
 * dashboard pages (NO_STORE_PAGES) and every page with a form that signs in
 * or sends an email (TURNSTILE_PAGES), which includes the account page.
 */
export const NEVER_CACHED: readonly string[] = [...new Set([...NO_STORE_PAGES, ...TURNSTILE_PAGES])].toSorted();

/**
 * The files kept at install. All the stylesheets and scripts of the build
 * (hashed, so a file is never half old) except the simulator's engine and
 * worker, which are large and only the simulator page uses: they are kept
 * when first used. `built` is every file of the build as a path from its
 * root, with forward slashes.
 */
export function precacheList(built: readonly string[]): string[] {
  const hashed = built
    .filter((path) => path.startsWith("_astro/") && /\.(css|js)$/.test(path) && !WASM_WORKER.test(path.slice("_astro/".length)))
    .map((path) => `/${path}`);
  return [...SHELL_FILES, ...hashed.toSorted()];
}

/**
 * Every address the worker may ever keep a copy of, as the browser asks for
 * it: a page is its folder ("/privacy/"), a file is its path. Not the account
 * and sign-in pages (NEVER_CACHED), the 404 pages, the worker itself or the
 * host's header rules. `built` is as in precacheList.
 */
export function knownAddresses(built: readonly string[]): string[] {
  const addresses = built
    .filter((path) => !["_headers", "sw.js", "404.html", "es/404.html"].includes(path))
    .map((path) => (path === "index.html" ? "/" : path.endsWith("/index.html") ? `/${path.slice(0, -"index.html".length)}` : `/${path}`));
  return addresses.filter((address) => !NEVER_CACHED.includes(address)).toSorted();
}

/** Every file (not folder) of the built site `dir`, as a path from its root with forward slashes. */
export function builtFiles(dir: URL): string[] {
  return readdirSync(dir, { recursive: true })
    .map((path) => String(path).replaceAll("\\", "/"))
    .filter((path) => statSync(new URL(path, dir)).isFile());
}

/** Where a kept address is in the build: a page is its folder's index.html. */
export const builtFile = (path: string): string => (path.endsWith("/") ? `${path.slice(1)}index.html` : path.slice(1));

/** Twelve hex digits that change whenever a kept file, the rules or the worker's code does. */
export function buildVersion(precache: readonly string[], read: (path: string) => string | Buffer, code: readonly string[]): string {
  const hash = createHash("sha256");
  for (const path of precache) hash.update(path).update("\0").update(read(path)).update("\0");
  for (const part of code) hash.update(part).update("\0");
  return hash.digest("hex").slice(0, 12);
}

export interface WorkerBuild {
  version: string;
  precache: readonly string[];
  never: readonly string[];
  known: readonly string[];
}

const here = new URL(".", import.meta.url);

/** The worker's two source files, as written. */
export function workerParts(): { rules: string; main: string } {
  return {
    rules: readFileSync(new URL("rules.js", here), "utf8"),
    main: readFileSync(new URL("sw-main.js", here), "utf8"),
  };
}

/** The text of /sw.js: the build's facts, the rules without their `export`s, then the events. */
export function serviceWorkerSource(build: WorkerBuild, parts = workerParts()): string {
  return [
    "// Tavern Ledger service worker (T-637). Written by the build from lib/pwa/; do not edit.",
    '"use strict";',
    `const BUILD = ${JSON.stringify(build, null, 2)};`,
    parts.rules.replace(/^export /gm, ""),
    parts.main,
  ].join("\n");
}

/**
 * The /sw.js of a build with no offline worker (accounts closed). A browser
 * that still has a worker from an earlier build asks for /sw.js on its next
 * visit, gets this, and the worker removes every copy it kept and itself, so
 * closing accounts again never leaves old pages behind on a phone.
 */
export function killSwitchSource(): string {
  return [
    "// Tavern Ledger (T-637): this build has no offline worker. A worker left by an earlier build removes its copies and itself.",
    '"use strict";',
    "self.addEventListener(\"install\", () => self.skipWaiting());",
    "self.addEventListener(\"activate\", (event) => {",
    "  event.waitUntil(",
    "    (async () => {",
    `      for (const name of await caches.keys()) if (name.startsWith(${JSON.stringify(CACHE_PREFIX)}) || name === ${JSON.stringify(SAVED_CACHE)}) await caches.delete(name);`,
    "      await self.registration.unregister();",
    "    })(),",
    "  );",
    "});",
    "",
  ].join("\n");
}

/**
 * Writes /sw.js into the built site `dir` (a file URL ending in "/"), after
 * checking that everything it will keep is there: a missing file would fail the
 * install in every browser. `built` lists the build's files from its root.
 */
export function writeServiceWorker(dir: URL, built: readonly string[]): { version: string; precache: string[] } {
  const precache = precacheList(built);
  const missing = precache.filter((path) => !built.includes(builtFile(path)));
  if (missing.length > 0) throw new Error(`the service worker would keep files the build does not have: ${missing.join(", ")}`);
  const parts = workerParts();
  const known = knownAddresses(built);
  // The never and known lists are in the version too: a page added to either must not keep an old copy in the same cache.
  const code = [parts.rules, parts.main, JSON.stringify(NEVER_CACHED), JSON.stringify(known)];
  const version = buildVersion(precache, (path) => readFileSync(new URL(builtFile(path), dir)), code);
  writeFileSync(new URL("sw.js", dir), serviceWorkerSource({ version, precache, never: NEVER_CACHED, known }, parts));
  return { version, precache };
}

// Build-time side of the installable web app (T-637): which files the worker
// keeps ready, the version that names its caches, and /sw.js itself. Used by
// astro.config.mjs after the pages are built; the unit tests run the same
// functions on a small made-up folder.
import { createHash } from "node:crypto";
import { readFileSync, writeFileSync } from "node:fs";
import { NO_STORE_PAGES, TURNSTILE_PAGES, WASM_WORKER } from "../config.ts";

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
 * Writes /sw.js into the built site `dir` (a file URL ending in "/"), after
 * checking that everything it will keep is there: a missing file would fail the
 * install in every browser. `built` lists the build's files from its root.
 */
export function writeServiceWorker(dir: URL, built: readonly string[]): { version: string; precache: string[] } {
  const precache = precacheList(built);
  const missing = precache.filter((path) => !built.includes(builtFile(path)));
  if (missing.length > 0) throw new Error(`the service worker would keep files the build does not have: ${missing.join(", ")}`);
  const parts = workerParts();
  const version = buildVersion(precache, (path) => readFileSync(new URL(builtFile(path), dir)), [parts.rules, parts.main]);
  writeFileSync(new URL("sw.js", dir), serviceWorkerSource({ version, precache, never: NEVER_CACHED }, parts));
  return { version, precache };
}

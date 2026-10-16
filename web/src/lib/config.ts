// Build-time settings of the site. Only the project URL and the public (anon
// or publishable) key may reach the browser; anything else stops the build.
// With neither setting the site builds with accounts closed: no backend, no
// forms, no scripts (until the hosted Supabase project exists).
import { TEXTURE_ORIGIN } from "./art/textureSet.ts";
import { ART_ORIGIN } from "./data/pictures.ts";

/** The one source the simulator's worker adds to script-src (D-090). */
export const WASM_SOURCE = "'wasm-unsafe-eval'";
/**
 * The simulator's worker script under /_astro/ (Vite names it after its
 * source, simWorker.ts). It alone gets WASM_SOURCE: the worker compiles the
 * engine, no page does.
 */
export const WASM_WORKER = /^simWorker-[A-Za-z0-9_-]+\.js$/;
/** Pages about one account: the browser and the CDN must not keep a copy (Cache-Control: no-store). */
export const NO_STORE_PAGES: readonly string[] = [
  "/account/", "/es/account/",
  "/me/", "/es/me/",
  "/games/", "/es/games/",
  "/game/", "/es/game/",
  "/player-heroes/", "/es/player-heroes/",
  "/curious/", "/es/curious/",
];

/** Cloudflare Turnstile (T-562): its script and its frame come from here, and from nowhere else. */
export const TURNSTILE_ORIGIN = "https://challenges.cloudflare.com";
/**
 * The widget's script, exactly as Cloudflare publishes it (never proxied or
 * copied). script-src allows this one file, not the whole origin; its frame
 * comes from the origin (frame-src).
 */
export const TURNSTILE_SCRIPT_SOURCE = `${TURNSTILE_ORIGIN}/turnstile/v0/api.js`;
export const TURNSTILE_SCRIPT = `${TURNSTILE_SCRIPT_SOURCE}?render=explicit`;
/** Cloudflare's published test site key that always passes: local development and CI. */
export const TURNSTILE_TEST_SITE_KEY = "1x00000000000000000000AA";
/**
 * The pages with a form that makes Supabase Auth send an email or check a
 * password (sign-in, sign-up, password reset, the password asked again before
 * deleting an account, the desktop app's sign-in link). They alone load
 * Turnstile, so they alone get its origin in their policy.
 */
export const TURNSTILE_PAGES: readonly string[] = [
  "/signin/",
  "/es/signin/",
  "/forgot-password/",
  "/es/forgot-password/",
  "/account/",
  "/es/account/",
  "/desktop-sign-in/",
  "/es/desktop-sign-in/",
];

export interface SiteConfig {
  url: string;
  anonKey: string;
}

export type ConfigSource = Record<string, string | undefined>;

const LOCAL_HOSTS = new Set(["127.0.0.1", "localhost"]);

function checkUrl(raw: string | undefined): string {
  if (!raw) throw new Error("PUBLIC_SUPABASE_URL is not set (see web/.env.example)");
  let url: URL;
  try {
    url = new URL(raw);
  } catch {
    throw new Error("PUBLIC_SUPABASE_URL is not a URL");
  }
  if (url.protocol !== "https:" && url.protocol !== "http:") {
    throw new Error("PUBLIC_SUPABASE_URL must be an http(s) URL");
  }
  if (url.origin !== raw) {
    throw new Error("PUBLIC_SUPABASE_URL must be the bare project origin, e.g. https://<ref>.supabase.co");
  }
  if (url.protocol === "http:" && !LOCAL_HOSTS.has(url.hostname)) {
    throw new Error("PUBLIC_SUPABASE_URL must use https outside the local stack");
  }
  return raw;
}

/** Role named in a JWT's payload, or null if the key is not a JWT. */
function jwtRole(key: string): string | null {
  const parts = key.split(".");
  if (parts.length !== 3) return null;
  try {
    const b64 = parts[1].replaceAll("-", "+").replaceAll("_", "/");
    const payload = JSON.parse(atob(b64.padEnd(b64.length + ((4 - (b64.length % 4)) % 4), "=")));
    return typeof payload?.role === "string" ? payload.role : null;
  } catch {
    return null;
  }
}

function checkKey(raw: string | undefined): string {
  if (!raw) throw new Error("PUBLIC_SUPABASE_ANON_KEY is not set (see web/.env.example)");
  if (/^sb_publishable_[A-Za-z0-9_-]+$/.test(raw)) return raw;
  if (jwtRole(raw) === "anon") return raw;
  // Never say which kind of key it was or echo it: it may be a secret.
  throw new Error("PUBLIC_SUPABASE_ANON_KEY must be the project's anon or publishable key");
}

/** The site's settings, or null when accounts are closed (neither is set). */
export function readConfig(env: ConfigSource): SiteConfig | null {
  if (!env.PUBLIC_SUPABASE_URL && !env.PUBLIC_SUPABASE_ANON_KEY) return null;
  return {
    url: checkUrl(env.PUBLIC_SUPABASE_URL),
    anonKey: checkKey(env.PUBLIC_SUPABASE_ANON_KEY),
  };
}

// A site key is "0x" (a test key "1x" to "3x") and about 22 letters; a secret
// key has the same start and is longer, so it never fits.
const TURNSTILE_SITE_KEY = /^[0-3]x[A-Za-z0-9_-]{18,28}$/;
// Cloudflare's published test keys start "1x" to "3x"; a real one "0x".
const TURNSTILE_TEST_KEY = /^[1-3]x/;

/**
 * The Turnstile widget's site key (public by design), or null when accounts
 * are closed (no form, so no widget). With accounts open it must be set:
 * Supabase Auth refuses sign-ins and emails without a token once its CAPTCHA
 * protection is on, so a build without the key would ship broken forms.
 */
export function turnstileSiteKey(env: ConfigSource): string | null {
  if (readConfig(env) === null) return null;
  const raw = env.PUBLIC_TURNSTILE_SITE_KEY;
  if (!raw) throw new Error("PUBLIC_TURNSTILE_SITE_KEY is not set: the sign-in forms need it (see web/.env.example)");
  // Never echo it: a secret key put here by mistake must not reach the build log.
  if (!TURNSTILE_SITE_KEY.test(raw)) {
    throw new Error("PUBLIC_TURNSTILE_SITE_KEY must be the Turnstile widget's site key, never its secret key");
  }
  // Cloudflare Pages sets CF_PAGES=1: the deployed site never ships a test key.
  if (env.CF_PAGES === "1" && TURNSTILE_TEST_KEY.test(raw)) {
    throw new Error("PUBLIC_TURNSTILE_SITE_KEY is one of Cloudflare's test keys: the deployed site needs the real widget's key");
  }
  return raw;
}

/**
 * Whether the site offers sign-up: only when the build says exactly "true"
 * (PUBLIC_SIGNUPS_OPEN) and has a backend. Closed by default, matching the
 * hosted project, where sign-ups stay off until the owner opens them.
 */
export function signupsOpen(env: ConfigSource): boolean {
  const raw = env.PUBLIC_SIGNUPS_OPEN ?? "";
  if (raw !== "" && raw !== "true" && raw !== "false") {
    throw new Error('PUBLIC_SIGNUPS_OPEN must be "true", "false" or unset');
  }
  if (raw !== "true") return false;
  if (readConfig(env) === null) {
    throw new Error("PUBLIC_SIGNUPS_OPEN needs PUBLIC_SUPABASE_URL and PUBLIC_SUPABASE_ANON_KEY");
  }
  return true;
}

/**
 * Content-Security-Policy for every page: scripts and styles come only from
 * the site's own files (the build inlines none), and the only connection is
 * to the Supabase project. Pictures may also come from HearthstoneJSON's art
 * host (D-074), which the game viewer draws in the browser, and the game's
 * minion textures from their own host (D-151): both only as images (the
 * textures are read by WebGL and masks from those images, never fetched). With accounts
 * closed there is no script, form, connection or outside picture at all. frame-ancestors
 * only works as a header: the build writes it to dist/_headers (astro.config.mjs).
 *
 * `wasm` is for the simulator's worker script only (T-606, D-089, D-090):
 * script-src also gets 'wasm-unsafe-eval', which lets a script compile
 * WebAssembly and nothing else (no eval, no inline script). Every other
 * directive stays the same.
 *
 * `turnstile` is for TURNSTILE_PAGES only (T-562): script-src and frame-src
 * get Cloudflare's challenge origin, for the widget's script and its frame.
 */
export function contentSecurityPolicy(
  supabaseOrigin: string | null,
  options: { wasm?: boolean; turnstile?: boolean } = {},
): string {
  // Accounts closed: no page has a script or a form, so allow none at all.
  const open = supabaseOrigin !== null;
  const captcha = open && options.turnstile === true;
  const scripts = open
    ? ["'self'", options.wasm ? WASM_SOURCE : "", captcha ? TURNSTILE_SCRIPT_SOURCE : ""].filter(Boolean).join(" ")
    : "'none'";
  return [
    "default-src 'none'",
    `script-src ${scripts}`,
    "style-src 'self'",
    `img-src ${open ? `'self' ${ART_ORIGIN} ${TEXTURE_ORIGIN}` : "'self'"}`,
    "font-src 'self'",
    `connect-src ${supabaseOrigin ?? "'none'"}`,
    ...(captcha ? [`frame-src ${TURNSTILE_ORIGIN}`] : []),
    "base-uri 'none'",
    `form-action ${open ? "'self'" : "'none'"}`,
  ].join("; ");
}

/**
 * Whether pages may carry a script: in dev, or when accounts are open. The
 * accounts-closed build allows none (script-src 'none'), so it ships none.
 */
export function scriptsAllowed(env: ConfigSource, dev: boolean): boolean {
  return dev || readConfig(env) !== null;
}

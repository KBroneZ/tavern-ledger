// Build-time settings of the site. Only the project URL and the public (anon
// or publishable) key may reach the browser; anything else stops the build.
// With neither setting the site builds with accounts closed: no backend, no
// forms, no scripts (until the hosted Supabase project exists).

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
 * to the Supabase project. With accounts closed there is no script, form or
 * connection at all. frame-ancestors
 * only works as a header: the build writes it to dist/_headers (astro.config.mjs).
 */
export function contentSecurityPolicy(supabaseOrigin: string | null): string {
  // Accounts closed: no page has a script or a form, so allow none at all.
  const open = supabaseOrigin !== null;
  return [
    "default-src 'none'",
    `script-src ${open ? "'self'" : "'none'"}`,
    "style-src 'self'",
    "img-src 'self'",
    `connect-src ${supabaseOrigin ?? "'none'"}`,
    "base-uri 'none'",
    `form-action ${open ? "'self'" : "'none'"}`,
  ].join("; ");
}

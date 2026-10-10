// Which service key the functions use (T-104g, D-042; fallback removed in
// T-104j, D-053).
//
// Supabase gives every Edge Function the new secret keys in
// SUPABASE_SECRET_KEYS, a JSON object keyed by key name ("default" is the one
// every project has) (https://supabase.com/docs/guides/functions/secrets).
// Only that key is used. The legacy service_role JWT
// (SUPABASE_SERVICE_ROLE_KEY) is never read, so legacy keys can be turned off
// in the dashboard. A missing or broken value is an error: the function
// answers 500 to every request and logs why, and the desktop app keeps its
// games to send again later. Never logs a key, only where it came from.

export type GetEnv = (name: string) => string | undefined;

export type ServiceKey =
  | { ok: true; key: string }
  | { ok: false; problem: string };

export function serviceKeyFromEnv(get: GetEnv): ServiceKey {
  const raw = get("SUPABASE_SECRET_KEYS");
  if (!raw) return { ok: false, problem: "SUPABASE_SECRET_KEYS is not set" };
  let keys: unknown;
  try {
    keys = JSON.parse(raw);
  } catch (_e) {
    return { ok: false, problem: "SUPABASE_SECRET_KEYS is not JSON" };
  }
  const key = keys !== null && typeof keys === "object" && !Array.isArray(keys)
    ? (keys as Record<string, unknown>).default
    : undefined;
  if (typeof key === "string" && key.startsWith("sb_secret_")) return { ok: true, key };
  return { ok: false, problem: "SUPABASE_SECRET_KEYS has no default key" };
}

/** One log line: which key the function runs with, never the key itself. */
export function describeServiceKey(name: string, service: ServiceKey): string {
  if (!service.ok) return `${name}: no service key: ${service.problem}; every request gets a 500`;
  return `${name}: service key from SUPABASE_SECRET_KEYS (new secret key)`;
}

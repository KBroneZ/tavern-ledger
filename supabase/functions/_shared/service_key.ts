// Which service key the functions use (T-104g, D-042).
//
// Supabase gives every Edge Function the new secret keys in
// SUPABASE_SECRET_KEYS, a JSON object keyed by key name ("default" is the one
// every project has), and the legacy service_role JWT in
// SUPABASE_SERVICE_ROLE_KEY while legacy keys stay enabled
// (https://supabase.com/docs/guides/functions/secrets). The new key is used
// when it is there; the legacy one is a fallback only until the new key is
// seen in production (the log line below says which), and then goes.
// Never logs a key, only where it came from.

export type GetEnv = (name: string) => string | undefined;

export type ServiceKey =
  | { ok: true; key: string; source: "secret_key" }
  | { ok: true; key: string; source: "legacy"; problem: string | null }
  | { ok: false; problem: string };

/** The default key in SUPABASE_SECRET_KEYS, or why there is none. */
function keyFromSecretKeys(raw: string): { key: string } | { problem: string } {
  let keys: unknown;
  try {
    keys = JSON.parse(raw);
  } catch (_e) {
    return { problem: "SUPABASE_SECRET_KEYS is not JSON" };
  }
  const key = keys !== null && typeof keys === "object" && !Array.isArray(keys)
    ? (keys as Record<string, unknown>).default
    : undefined;
  if (typeof key === "string" && key.startsWith("sb_secret_")) return { key };
  return { problem: "SUPABASE_SECRET_KEYS has no default key" };
}

export function serviceKeyFromEnv(get: GetEnv): ServiceKey {
  const raw = get("SUPABASE_SECRET_KEYS");
  let problem: string | null = null;
  if (raw) {
    const found = keyFromSecretKeys(raw);
    if ("key" in found) return { ok: true, key: found.key, source: "secret_key" };
    problem = found.problem;
  }
  const legacy = get("SUPABASE_SERVICE_ROLE_KEY");
  if (legacy) return { ok: true, key: legacy, source: "legacy", problem };
  return {
    ok: false,
    problem: problem
      ? `${problem}, and SUPABASE_SERVICE_ROLE_KEY is not set`
      : "neither SUPABASE_SECRET_KEYS nor SUPABASE_SERVICE_ROLE_KEY is set",
  };
}

/** One log line: which key the function runs with, never the key itself. */
export function describeServiceKey(name: string, service: ServiceKey): string {
  if (!service.ok) return `${name}: no service key: ${service.problem}`;
  if (service.source === "secret_key") {
    return `${name}: service key from SUPABASE_SECRET_KEYS (new secret key)`;
  }
  const why = service.problem ? `; ${service.problem}` : "";
  return `${name}: service key from SUPABASE_SERVICE_ROLE_KEY (legacy fallback${why})`;
}

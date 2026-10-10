import { describeServiceKey, serviceKeyFromEnv } from "../_shared/service_key.ts";
import { handle, parseOrigins } from "./handler.ts";

const url = Deno.env.get("SUPABASE_URL");
const service = serviceKeyFromEnv((name) => Deno.env.get(name));
const serviceNote = describeServiceKey("delete-account", service);
// Said once per instance, so the logs show which key production uses (D-042,
// D-053). Without the key every request below gets a 500.
if (service.ok) console.info(serviceNote);
else console.error(serviceNote);
// The website's origins, comma-separated. Unset: the public site, which is
// not a secret and the same for every deploy of this project (the local
// stack sets its own in supabase/config.toml).
const SITE_ORIGINS = Deno.env.get("SITE_ORIGINS") || "https://tavernledger.net";
const allowedOrigins = parseOrigins(SITE_ORIGINS);
const configuredOrigins = SITE_ORIGINS.split(",").filter((o) => o.trim());
if (configuredOrigins.length !== allowedOrigins.length) {
  // A typo (e.g. a trailing slash) would silently refuse every browser.
  console.error(
    `delete-account: ${
      configuredOrigins.length - allowedOrigins.length
    } SITE_ORIGINS entries ignored (not exact origins)`,
  );
}

Deno.serve((req) => {
  if (!url || !service.ok) {
    console.error(url ? serviceNote : "delete-account: SUPABASE_URL missing");
    return new Response(JSON.stringify({ error: "server not configured" }), {
      status: 500,
      headers: { "Content-Type": "application/json" },
    });
  }
  return handle(req, { url, serviceKey: service.key, allowedOrigins });
});

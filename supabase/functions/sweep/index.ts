import { describeServiceKey, serviceKeyFromEnv } from "../_shared/service_key.ts";
import { handle } from "./handler.ts";

const url = Deno.env.get("SUPABASE_URL");
const service = serviceKeyFromEnv((name) => Deno.env.get(name));
const serviceNote = describeServiceKey("sweep", service);
// Said once per instance, so the logs show which key production uses (D-042,
// D-053). Without the key every request below gets a 500.
if (service.ok) console.info(serviceNote);
else console.error(serviceNote);

Deno.serve((req) => {
  if (!url || !service.ok) {
    console.error(url ? serviceNote : "sweep: SUPABASE_URL missing");
    return new Response(JSON.stringify({ error: "server not configured" }), {
      status: 500,
      headers: { "Content-Type": "application/json" },
    });
  }
  return handle(req, { url, serviceKey: service.key });
});

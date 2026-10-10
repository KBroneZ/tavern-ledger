import { describeServiceKey, serviceKeyFromEnv } from "../_shared/service_key.ts";
import { handle } from "./handler.ts";

const url = Deno.env.get("SUPABASE_URL");
const service = serviceKeyFromEnv((name) => Deno.env.get(name));
const serviceNote = describeServiceKey("sweep", service);
// Said once per instance, so the logs show which key production uses (D-042).
if (service.ok && service.source === "secret_key") console.info(serviceNote);
else console.warn(serviceNote);

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

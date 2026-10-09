import { handle } from "./handler.ts";

const url = Deno.env.get("SUPABASE_URL");
const serviceKey = Deno.env.get("SUPABASE_SERVICE_ROLE_KEY");

Deno.serve((req) => {
  if (!url || !serviceKey) {
    console.error("sweep: SUPABASE_URL or SUPABASE_SERVICE_ROLE_KEY missing");
    return new Response(JSON.stringify({ error: "server not configured" }), {
      status: 500,
      headers: { "Content-Type": "application/json" },
    });
  }
  return handle(req, { url, serviceKey });
});

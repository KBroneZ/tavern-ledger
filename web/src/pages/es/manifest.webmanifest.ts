// /es/manifest.webmanifest (T-637): the installable web app's manifest, in Spanish.
import type { APIRoute } from "astro";
import { webManifest } from "../../lib/pwa/manifest.ts";

export const GET: APIRoute = () =>
  new Response(JSON.stringify(webManifest("es"), null, 2), { headers: { "Content-Type": "application/manifest+json; charset=utf-8" } });

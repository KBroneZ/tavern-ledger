// /manifest.webmanifest (T-637): the installable web app's manifest, in English.
import type { APIRoute } from "astro";
import { webManifest } from "../lib/pwa/manifest.ts";

export const GET: APIRoute = () =>
  new Response(JSON.stringify(webManifest("en"), null, 2), { headers: { "Content-Type": "application/manifest+json; charset=utf-8" } });

// /sitemap.xml (T-578): the indexable pages, and the minion pages when this
// build has them (lib/site/sitemap.ts).
import type { APIRoute } from "astro";
import { hasMinionPages, poolFor } from "../lib/minions/load.ts";
import { tribePages, tribePath } from "../lib/minions/view.ts";
import { INDEXED_PAGES, sitemapXml } from "../lib/site/sitemap.ts";

export const GET: APIRoute = ({ site }) => {
  if (!site) throw new Error("the sitemap needs the site's address (astro.config.mjs site)");
  const minions = hasMinionPages() ? ["/minions/", ...tribePages(poolFor("en")).map((p) => tribePath(p.slug))] : [];
  return new Response(sitemapXml(site, [...INDEXED_PAGES, ...minions]), { headers: { "Content-Type": "application/xml; charset=utf-8" } });
};

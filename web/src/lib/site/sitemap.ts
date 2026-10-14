// The sitemap (/sitemap.xml, T-578): every page search engines may index,
// each in both languages with its alternates, as the pages' own
// <link rel="alternate"> say (layouts/Base.astro). A build test checks that
// it lists exactly the built pages that are not noindex.
import { LANGS, pathIn } from "../i18n.ts";

/** The indexable pages besides the minion pages, as English paths. */
export const INDEXED_PAGES: readonly string[] = ["/", "/download/", "/privacy/", "/terms/", "/accuracy/"];

const escape = (text: string): string => text.replaceAll("&", "&amp;").replaceAll("<", "&lt;").replaceAll(">", "&gt;").replaceAll('"', "&quot;");

/** The sitemap of `paths` (English paths) on `site`, one entry per page and language. */
export function sitemapXml(site: URL, paths: readonly string[]): string {
  const at = (path: string) => escape(new URL(path, site).href);
  const entries = paths.flatMap((path) => {
    const links = [
      ...LANGS.map((l) => `    <xhtml:link rel="alternate" hreflang="${l}" href="${at(pathIn(path, l))}"/>`),
      `    <xhtml:link rel="alternate" hreflang="x-default" href="${at(pathIn(path, "en"))}"/>`,
    ];
    return LANGS.map((l) => ["  <url>", `    <loc>${at(pathIn(path, l))}</loc>`, ...links, "  </url>"].join("\n"));
  });
  return [
    '<?xml version="1.0" encoding="UTF-8"?>',
    '<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9" xmlns:xhtml="http://www.w3.org/1999/xhtml">',
    ...entries,
    "</urlset>",
    "",
  ].join("\n");
}

// "What's new" on the home page (T-607, D-089: a short line on the site
// instead of an email). Newest first, one dated line each, no claims beyond
// what is built (rule 8). The text and link label are site strings.
import type { SiteKey } from "./strings.ts";

export interface NewsItem {
  /** YYYY-MM-DD, the day it went live. */
  date: string;
  text: SiteKey;
  link: SiteKey;
  href: string;
}

export const NEWS: readonly NewsItem[] = [
  // The website simulator (T-606, session 076): add this line once it is
  // merged and live; its strings are already in both languages.
  // { date: "2026-10-11", text: "news.simulator", link: "news.simulator.link", href: "/simulator/" },
  { date: "2026-10-11", text: "news.accuracy", link: "news.accuracy.link", href: "/accuracy/" },
  { date: "2026-10-11", text: "news.heroes", link: "news.heroes.link", href: "/heroes/" },
];

// The web app manifest (T-637), one per language: /manifest.webmanifest and
// /es/manifest.webmanifest. Both name the same app (`id`) and the whole site as
// its scope, so the language switch stays inside the installed app; they differ
// in the language and the page the app opens on. Colours are the site's own
// (`--bg` in styles/global.css; a test keeps them equal). Icons are drawn from
// our own mark only (public/icons/).
import { href, setLang, type Lang } from "../i18n.ts";
import { st } from "../site/strings.ts";

/** The page background (`--bg`): the colour behind the app while it opens and around the browser's own bars. */
export const THEME_COLOR = "#120d0b";

/** The manifest of a language (a file keeps its address in every language, so pathIn does not add /es/). */
export const manifestPath = (language: Lang): string => (language === "es" ? "/es/manifest.webmanifest" : "/manifest.webmanifest");

export interface WebManifest {
  id: string;
  name: string;
  short_name: string;
  description: string;
  lang: Lang;
  dir: "ltr";
  start_url: string;
  scope: string;
  display: "standalone";
  theme_color: string;
  background_color: string;
  categories: string[];
  icons: { src: string; sizes: string; type: string; purpose: "any" | "maskable" }[];
}

export function webManifest(language: Lang): WebManifest {
  setLang(language);
  return {
    id: "/",
    name: "Tavern Ledger",
    short_name: "Tavern Ledger",
    description: st("pwa.description"),
    lang: language,
    dir: "ltr",
    start_url: href("/"),
    scope: "/",
    display: "standalone",
    theme_color: THEME_COLOR,
    background_color: THEME_COLOR,
    categories: ["games", "utilities"],
    icons: [
      { src: "/icons/icon-192.png", sizes: "192x192", type: "image/png", purpose: "any" },
      { src: "/icons/icon-512.png", sizes: "512x512", type: "image/png", purpose: "any" },
      { src: "/icons/icon-maskable-512.png", sizes: "512x512", type: "image/png", purpose: "maskable" },
    ],
  };
}

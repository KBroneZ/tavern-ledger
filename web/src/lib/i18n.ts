// The site's languages (T-508, D-086): English is the source, Spanish (Spain)
// is complete. Like the app (D-065), an own string table and no dependency:
// each part of the site has its English table (strings.ts) and its Spanish
// one (strings.es.ts), and a test fails when a key is missing, empty or has
// other placeholders in one language.
//
// One page is in one language. English pages live at /…, Spanish ones at
// /es/…, built from the same components. The language is set once per page:
// at build time by the page (Astro renders one page at a time,
// build.concurrency 1), in the browser by lib/i18n/boot.ts from <html lang>.
// Everything that draws text reads it when it draws, never at import.

export const LANGS = ["en", "es"] as const;
export type Lang = (typeof LANGS)[number];

/** The locale for numbers and dates. English keeps the site's own "en"; Spanish is Spain's. */
const LOCALES: Record<Lang, string> = { en: "en", es: "es-ES" };

let current: Lang = "en";

export const isLang = (value: unknown): value is Lang => (LANGS as readonly unknown[]).includes(value);

/** Sets the page's language; anything that is not one of ours is English. */
export function setLang(value: unknown): Lang {
  current = isLang(value) ? value : "en";
  return current;
}

export const lang = (): Lang => current;
export const locale = (): string => LOCALES[current];

export type Vars = Record<string, string | number>;

const PLACEHOLDER = /\{([a-zA-Z]+)\}/g;

/** The names of a text's placeholders, sorted (for the parity test). */
export const placeholders = (text: string): string[] => [...new Set([...text.matchAll(PLACEHOLDER)].map((m) => m[1]))].sort();

export interface Table<K extends string> {
  name: string;
  en: Readonly<Record<K, string>>;
  es: Readonly<Record<K, string>>;
}

/** Every table, for the parity test. */
export const TABLES: Table<string>[] = [];

/**
 * A lookup over one table: the text of `key` in the page's language with its
 * placeholders filled. A missing key or placeholder throws rather than print
 * "{total}"; the parity test makes sure Spanish lacks none.
 */
export function makeT<K extends string>(table: Table<K>): (key: K, vars?: Vars) => string {
  TABLES.push(table as Table<string>);
  return (key, vars = {}) => {
    const text: string | undefined = table[current][key];
    if (text === undefined) throw new Error(`no string ${table.name}:${key} in ${current}`);
    return text.replaceAll(PLACEHOLDER, (_, name: string) => {
      if (!(name in vars)) throw new Error(`string ${key} needs {${name}}`);
      return String(vars[name]);
    });
  };
}

/** "one" or "other" by the language's plural rules (Spanish "many" counts as other). */
export function pluralForm(count: number): "one" | "other" {
  return new Intl.PluralRules(locale()).select(count) === "one" ? "one" : "other";
}

const numberFormats = new Map<string, Intl.NumberFormat>();
const dateFormats = new Map<string, Intl.DateTimeFormat>();

/** A cached Intl.NumberFormat for the page's language. */
export function numberFormat(options: Intl.NumberFormatOptions): Intl.NumberFormat {
  const key = `${current}|${JSON.stringify(options)}`;
  let format = numberFormats.get(key);
  if (!format) {
    format = new Intl.NumberFormat(locale(), options);
    numberFormats.set(key, format);
  }
  return format;
}

/**
 * A cached Intl.DateTimeFormat for the page's language. `english` is the
 * locale English uses for this format, so the English pages keep the dates
 * they had ("en-GB" puts the day first).
 */
export function dateFormat(options: Intl.DateTimeFormatOptions, english = "en"): Intl.DateTimeFormat {
  const key = `${current}|${english}|${JSON.stringify(options)}`;
  let format = dateFormats.get(key);
  if (!format) {
    format = new Intl.DateTimeFormat(current === "en" ? english : locale(), options);
    dateFormats.set(key, format);
  }
  return format;
}

/** A list joined the language's way: "a, b and c" / "a, b y c". */
export const listFormat = (items: readonly string[]): string =>
  new Intl.ListFormat(locale(), { style: "long", type: "conjunction" }).format(items);

// ---------- Addresses ----------

const ES_PREFIX = "/es";

/** The path without its language prefix: "/es/games/" → "/games/". */
export function basePath(path: string): string {
  if (path === ES_PREFIX) return "/";
  return path.startsWith(`${ES_PREFIX}/`) ? path.slice(ES_PREFIX.length) : path;
}

/** The language a path is in. */
export const pathLang = (path: string): Lang => (path === ES_PREFIX || path.startsWith(`${ES_PREFIX}/`) ? "es" : "en");

/**
 * A site path in a language: "/games/?id=1" → "/es/games/?id=1". Only our
 * own absolute paths change; anything else (another site, "#top", a file
 * such as /_astro/a.js or /fonts/x.woff2) is returned as it is.
 */
export function pathIn(path: string, target: Lang = current): string {
  // A file (it has an extension) is the same for every language.
  const file = /\.[a-z0-9]+$/i.test(path.split(/[?#]/)[0]);
  if (!path.startsWith("/") || path.startsWith("//") || file) {
    return path;
  }
  const base = basePath(path);
  return target === "en" ? base : `${ES_PREFIX}${base}`;
}

/** Shorthand for our own links on the page being drawn: "/games/" in its language. */
export const href = (path: string): string => pathIn(path, current);

/** getStaticPaths for a page built in every language: "/x/" and "/es/x/". */
export const langPaths = () =>
  LANGS.map((l) => ({ params: { lang: l === "en" ? undefined : l }, props: { lang: l } }));

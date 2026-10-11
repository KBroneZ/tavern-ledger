// The site in English and Spanish (T-508, D-086), checked on the built
// files in every build mode: each page has its Spanish twin under /es/, the
// right <html lang>, alternates for search engines, a switch to the other
// language, links that stay in the page's language, and no English left in
// the Spanish pages' text.
import { test } from "node:test";
import assert from "node:assert/strict";
import { existsSync, readdirSync, readFileSync, statSync } from "node:fs";
import { join, relative } from "node:path";
import { loadEnv } from "vite";
import { readConfig } from "../../src/lib/config.ts";

const ROOT = join(import.meta.dirname, "..", "..");
const DIST = join(ROOT, "dist");
const CONFIG = readConfig(loadEnv("production", ROOT, "PUBLIC_"));
const SITE = "https://tavernledger.net";

function files(dir: string): string[] {
  return readdirSync(dir).flatMap((n) => {
    const path = join(dir, n);
    return statSync(path).isDirectory() ? files(path) : [path];
  });
}

const PAGES = (existsSync(DIST) ? files(DIST) : []).filter((f) => f.endsWith(".html")).map((f) => relative(DIST, f).replaceAll("\\", "/"));
const read = (page: string) => readFileSync(join(DIST, page), "utf8");
const ENGLISH = PAGES.filter((p) => !p.startsWith("es/"));
const SPANISH = PAGES.filter((p) => p.startsWith("es/"));
// Built in English only: the style page (made-up data for the design system).
const ENGLISH_ONLY = ["style/index.html"];
const urlPath = (page: string) => "/" + page.replace(/index\.html$/, "");

test("every page has its Spanish twin under /es/, and nothing else is there", () => {
  assert.ok(ENGLISH.length >= 15, "run `npm run build` first");
  const twins = ENGLISH.filter((p) => !ENGLISH_ONLY.includes(p)).map((p) => `es/${p}`);
  assert.deepEqual([...SPANISH].sort(), twins.sort());
  assert.ok(SPANISH.includes("es/404.html"), "Cloudflare Pages serves es/404.html under /es/");
});

test("<html lang> is the page's language", () => {
  for (const page of ENGLISH) assert.match(read(page), /<html lang="en">/, page);
  for (const page of SPANISH) assert.match(read(page), /<html lang="es">/, page);
});

test("indexable pages name both languages and x-default; noindex pages name none", () => {
  for (const page of PAGES) {
    const html = read(page);
    const alternates = [...html.matchAll(/<link rel="alternate" hreflang="([^"]+)" href="([^"]+)">/g)].map((m) => [m[1], m[2]]);
    if (html.includes('<meta name="robots" content="noindex">')) {
      assert.deepEqual(alternates, [], page);
      continue;
    }
    const en = urlPath(page.replace(/^es\//, ""));
    assert.deepEqual(alternates, [
      ["en", `${SITE}${en}`],
      ["es", `${SITE}/es${en}`],
      ["x-default", `${SITE}${en}`],
    ], page);
  }
});

test("each translated page has a switch to the same page in the other language", () => {
  for (const page of PAGES) {
    const html = read(page);
    const switches = [...html.matchAll(/<a\b[^>]*data-lang-switch="([^"]+)"[^>]*>/g)];
    if (ENGLISH_ONLY.includes(page) || page.endsWith("404.html")) {
      assert.equal(switches.length, 0, page);
      continue;
    }
    assert.equal(switches.length, 1, page);
    const tag = switches[0][0];
    const spanish = page.startsWith("es/");
    const target = spanish ? urlPath(page.slice(3)) : `/es${urlPath(page)}`;
    assert.equal(switches[0][1], spanish ? "en" : "es", page);
    assert.match(tag, new RegExp(`href="${target}"`), page);
    assert.match(tag, new RegExp(`hreflang="${spanish ? "en" : "es"}" lang="${spanish ? "en" : "es"}"`), page);
  }
});

test("links stay in the page's language (the switch apart)", () => {
  for (const page of PAGES) {
    const html = read(page).replace(/<a\b[^>]*data-lang-switch[^>]*>/g, "");
    const links = [...html.matchAll(/<(?:a|form)\b[^>]*\s(?:href|action)="(\/[^"]*)"/g)].map((m) => m[1]);
    for (const link of links) {
      if (page.startsWith("es/")) assert.match(link, /^\/es\//, `${page}: ${link}`);
      else assert.doesNotMatch(link, /^\/es\//, `${page}: ${link}`);
    }
  }
});

test("the language script: on every translated page when accounts are open, nowhere when closed", () => {
  for (const page of PAGES) {
    const has = read(page).includes('<script src="/lang.js"></script>');
    const translated = !ENGLISH_ONLY.includes(page) && !page.endsWith("404.html");
    assert.equal(has, Boolean(CONFIG) && translated, page);
  }
  assert.equal(existsSync(join(DIST, "lang.js")), Boolean(CONFIG));
});

// A safety net, not proof: the key-parity test (test/unit/i18n.test.ts) is
// what keeps every string translated. Words that only English text has; card ids, the brand and the English
// fan notice (marked lang="en") are taken out first.
const ENGLISH_WORDS = /\b(?:the|and|your|you|with|games?|not|this|from|sign in|account|place|heroes?|loading|show)\b/i;

function visibleText(html: string): string {
  return html
    .replace(/<head>[\s\S]*?<\/head>/, "")
    .replace(/<(script|style)\b[\s\S]*?<\/\1>/g, "")
    .replace(/<code>[\s\S]*?<\/code>/g, " ")
    .replace(/<([a-z]+)\b[^>]*\slang="en"[^>]*>[\s\S]*?<\/\1>/g, "")
    .replace(/<[^>]+\s(?:aria-label|title|alt|placeholder)="([^"]*)"[^>]*>/g, " $1 ")
    .replace(/<[^>]+>/g, " ")
    .replace(/Tavern Ledger|Hearthstone(?:JSON)?|Blizzard Entertainment|BattleTag|Cloudflare|Supabase|GitHub|HearthstoneJSON/g, " ")
    .replace(/&[a-z#0-9]+;/g, " ");
}

test("the Spanish pages' text has no English left", () => {
  for (const page of SPANISH) {
    const text = visibleText(read(page));
    const hit = ENGLISH_WORDS.exec(text);
    assert.equal(hit, null, `${page}: …${hit ? text.slice(Math.max(0, hit.index - 60), hit.index + 60).replace(/\s+/g, " ") : ""}…`);
  }
});

test("the Spanish legal pages are marked as translations and dated as the English", () => {
  const months = ["January", "February", "March", "April", "May", "June", "July", "August", "September", "October", "November", "December"];
  const meses = ["enero", "febrero", "marzo", "abril", "mayo", "junio", "julio", "agosto", "septiembre", "octubre", "noviembre", "diciembre"];
  for (const page of ["privacy/index.html", "terms/index.html"]) {
    const english = /Last updated: (\d+) ([A-Z][a-z]+) (\d{4})/.exec(read(page));
    assert.ok(english, page);
    const spanish = read(`es/${page}`);
    const [, day, month, year] = english;
    assert.ok(spanish.includes(`Última actualización: ${day} de ${meses[months.indexOf(month)]} de ${year}`), page);
    assert.match(spanish, /Traducción del original en inglés[\s\S]*?prevalece la inglesa/, page);
    assert.ok(spanish.includes(`href="https://tavernledger.net/${page.replace("index.html", "")}"`), `${page}: links to the English it translates`);
  }
});

test("every page has the social card for link previews, its alt text in the page's language (T-523)", () => {
  assert.ok(existsSync(join(DIST, "social-card.png")), "public/social-card.png is copied into the build");
  const alt = { en: "Tavern Ledger logo, a tankard drawn from the letters TL, over the line Hearthstone Battlegrounds tracker", es: "Logotipo de Tavern Ledger, una jarra dibujada con las letras TL, sobre la frase Hearthstone Battlegrounds tracker" };
  for (const page of PAGES) {
    const html = read(page);
    const meta = (key: string) => [...html.matchAll(/<meta (?:property|name)="([^"]+)" content="([^"]*)">/g)].filter((m) => m[1] === key).map((m) => m[2]);
    assert.deepEqual(meta("og:image"), [`${SITE}/social-card.png`], page);
    assert.deepEqual(meta("og:image:width"), ["1200"], page);
    assert.deepEqual(meta("og:image:height"), ["630"], page);
    assert.deepEqual(meta("og:image:alt"), [page.startsWith("es/") ? alt.es : alt.en], page);
    assert.deepEqual(meta("twitter:card"), ["summary_large_image"], page);
    assert.deepEqual(meta("twitter:image"), [`${SITE}/social-card.png`], page);
    assert.deepEqual(meta("og:site_name"), ["Tavern Ledger"], page);
    assert.equal(meta("og:title").length, 1, page);
    assert.equal(meta("og:description").length, 1, page);
  }
});

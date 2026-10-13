// The built minion pages (T-578, D-134) and the sitemap, in every build
// mode. Accounts open (card data from the synthetic files in CI): the index
// and one page per type in both languages, each indexable with its own
// title, description, canonical, alternates, Open Graph, breadcrumb and
// structured data; the minions in the pool only, a minion of two types on
// both pages, one of every type on all of them, golden stats. Accounts
// closed: no card data, so no minion page and no link to one. Either way the
// sitemap lists exactly the indexable pages.
import { test } from "node:test";
import assert from "node:assert/strict";
import { existsSync, readdirSync, readFileSync, statSync } from "node:fs";
import { join, relative } from "node:path";
import { loadEnv } from "vite";
import { readConfig } from "../../src/lib/config.ts";

const ROOT = join(import.meta.dirname, "..", "..");
const DIST = join(ROOT, "dist");
const OPEN = readConfig(loadEnv("production", ROOT, "PUBLIC_")) !== null;
const SITE = "https://tavernledger.net";
/** The fixture's types with a minion of their own (test/fixtures/sim-cards). */
const SLUGS = ["elemental", "mech", "murloc", "pirate", "quilboar", "undead", "no-tribe"];
const read = (page: string) => readFileSync(join(DIST, page), "utf8");
const page = (lang: string, slug: string | null) => `${lang === "es" ? "es/" : ""}minions/${slug ? `${slug}/` : ""}index.html`;
const entries = (html: string) => [...html.matchAll(/<li class="mn-entry" id="m-([A-Za-z0-9_]+)">/g)].map((m) => m[1]);

function files(dir: string): string[] {
  return readdirSync(dir).flatMap((n) => {
    const path = join(dir, n);
    return statSync(path).isDirectory() ? files(path) : [path];
  });
}

/** The page's structured data, parsed (it must be valid JSON). */
function jsonLd(html: string): { "@context": string; "@graph": Record<string, unknown>[] } {
  const blocks = [...html.matchAll(/<script type="application\/ld\+json">([^<]*)<\/script>/g)];
  assert.equal(blocks.length, 1, "one structured data block");
  return JSON.parse(blocks[0][1]);
}

test("accounts open: the index and a page per type, in English and Spanish", { skip: !OPEN }, () => {
  for (const lang of ["en", "es"]) {
    assert.ok(existsSync(join(DIST, page(lang, null))), `${lang} index: run \`npm run build\` first`);
    for (const slug of SLUGS) assert.ok(existsSync(join(DIST, page(lang, slug))), page(lang, slug));
    assert.ok(!existsSync(join(DIST, page(lang, "beast"))), "a type with only an every-type minion has no page");
  }
});

test("accounts open: each minion page has its own head for search engines and link previews", { skip: !OPEN }, () => {
  const titles = new Set<string>();
  for (const lang of ["en", "es"]) {
    for (const slug of [null, ...SLUGS]) {
      const file = page(lang, slug);
      const html = read(file);
      const path = `/${file.replace(/index\.html$/, "")}`;
      const base = path.replace(/^\/es\//, "/");
      assert.doesNotMatch(html, /<meta name="robots" content="noindex">/, file);
      assert.match(html, new RegExp(`<link rel="canonical" href="${SITE}${path}">`), file);
      assert.match(html, new RegExp(`<link rel="alternate" hreflang="en" href="${SITE}${base}">`), file);
      assert.match(html, new RegExp(`<link rel="alternate" hreflang="es" href="${SITE}/es${base}">`), file);
      assert.match(html, new RegExp(`<link rel="alternate" hreflang="x-default" href="${SITE}${base}">`), file);
      assert.match(html, new RegExp(`<meta property="og:url" content="${SITE}${path}">`), file);
      const title = /<title>([^<]+)<\/title>/.exec(html)?.[1] ?? "";
      assert.ok(title.length > 20 && !titles.has(title), `${file}: its own title`);
      titles.add(title);
      assert.match(html, /<meta property="og:title" content="[^"]+">/, file);
      assert.match(html, /<meta name="description" content="[^"]{60,}">/, file);
      assert.match(html, /<html lang="(en|es)">/, file);
      assert.match(html, /Unofficial fan project\. Not affiliated with or endorsed by Blizzard Entertainment\./, file);
      assert.match(html, /<nav class="mn-crumbs" aria-label="[^"]+">/, file);
      assert.match(html, /© Blizzard Entertainment, (?:from|desde) fixture\./, `${file}: the art and data credit`);
    }
  }
});

test("accounts open: the structured data is valid JSON: a breadcrumb and the page's list", { skip: !OPEN }, () => {
  for (const lang of ["en", "es"]) {
    const index = jsonLd(read(page(lang, null)));
    assert.equal(index["@context"], "https://schema.org");
    const tiles = index["@graph"].find((g) => g["@type"] === "ItemList") as { itemListElement: { url: string }[] };
    assert.deepEqual(
      tiles.itemListElement.map((i) => i.url),
      SLUGS.map((s) => `${SITE}${lang === "es" ? "/es" : ""}/minions/${s}/`),
    );
    for (const slug of SLUGS) {
      const html = read(page(lang, slug));
      const graph = jsonLd(html)["@graph"];
      const crumbs = graph.find((g) => g["@type"] === "BreadcrumbList") as { itemListElement: { position: number; item: string }[] };
      assert.deepEqual(crumbs.itemListElement.map((i) => i.position), [1, 2, 3]);
      assert.equal(crumbs.itemListElement[2].item, `${SITE}${lang === "es" ? "/es" : ""}/minions/${slug}/`);
      const list = graph.find((g) => g["@type"] === "ItemList") as { numberOfItems: number; itemListElement: { url: string }[] };
      const ids = entries(html);
      assert.equal(list.numberOfItems, ids.length, `${slug}: one item per minion on the page`);
      assert.deepEqual(
        list.itemListElement.map((i) => i.url.split("#m-")[1]),
        ids,
        `${slug}: the list follows the page`,
      );
    }
  }
});

test("accounts open: pool minions only, grouped by tier; two types on both pages, every type on all", { skip: !OPEN }, () => {
  const murloc = read(page("en", "murloc"));
  const pirate = read(page("en", "pirate"));
  assert.ok(entries(murloc).includes("BG_SYN_MURPIR") && entries(pirate).includes("BG_SYN_MURPIR"));
  for (const slug of SLUGS.filter((s) => s !== "no-tribe")) {
    assert.ok(entries(read(page("en", slug))).includes("BG_SYN_AMALGAM"), slug);
  }
  assert.deepEqual(entries(read(page("en", "no-tribe"))), ["BG_NOT_A_CARD"]);
  for (const slug of SLUGS) {
    const html = read(page("en", slug));
    assert.doesNotMatch(html, /BG_SYN_OLDMUR|BG_SYN_MURPIR_G|BG_ICC_026t/, `${slug}: not in the pool`);
  }
  // Tier 3 before tier 4, each a section with its heading.
  assert.ok(murloc.indexOf('id="tier-3"') < murloc.indexOf('id="tier-4"'));
  assert.ok(murloc.indexOf('id="tier-3"') < murloc.indexOf("BG_SYN_MURPIR") && murloc.indexOf("BG_SYN_MURPIR") < murloc.indexOf('id="tier-4"'));
  assert.match(murloc, /<a href="\/minions\/pirate\/">Pirate<\/a>/, "the other type, linked");
  assert.match(murloc, /Every type/);
  assert.match(murloc, /Duos only/);
});

test("accounts open: stats, golden stats and card text, in the page's language", { skip: !OPEN }, () => {
  const en = read(page("en", "murloc"));
  assert.match(en, /<h3>Synthetic Deckhand Fin<\/h3>/);
  assert.match(en, /3 Attack, 4 Health/);
  assert.match(en, /Golden: 6 Attack, 8 Health/);
  assert.match(en, /<b>Battlecry:<\/b> Give a friendly Murloc \+1\/\+1\. <i>\(Synthetic\.\)<\/i>/);
  assert.doesNotMatch(en, /\[x\]/, "the card data's layout mark is dropped");
  assert.match(en, /<b>Taunt<\/b><br>This is every type\./, "a keyword alone on its line keeps it");
  const es = read(page("es", "murloc"));
  assert.match(es, /<h1>Múrlocs<\/h1>/);
  assert.match(es, /<h3>Aleta grumete sintética<\/h3>/);
  assert.match(es, /Dorado: 6 de ataque, 8 de salud/);
  assert.match(es, /<b>Grito de batalla:<\/b>/);
  assert.match(es, /<a href="\/es\/minions\/pirate\/">Pirata<\/a>/);
});

test("accounts open: pictures keep their room, load lazily and fall back to our own placeholder", { skip: !OPEN }, () => {
  for (const slug of SLUGS) {
    const html = read(page("en", slug));
    const imgs = [...html.matchAll(/<img\b[^>]*>/g)].map((m) => m[0]).filter((i) => i.includes("hearthstonejson"));
    assert.ok(imgs.length >= entries(html).length, slug);
    for (const img of imgs) assert.match(img, /loading="lazy"/, img);
    // The fallback needs the page's one small script.
    assert.equal([...html.matchAll(/<script type="module" src="\/_astro\/[^"]+\.js"><\/script>/g)].length, 1, slug);
  }
});

test("accounts open: the footer on every page and the hero stats page link the index", { skip: !OPEN }, () => {
  for (const file of ["index.html", "download/index.html", "privacy/index.html"]) {
    assert.match(read(file), /<a href="\/minions\/">Minions by type<\/a>/, file);
  }
  assert.match(read("es/index.html"), /<a href="\/es\/minions\/">Esbirros por tipo<\/a>/);
  assert.match(read("heroes/index.html"), /<a href="\/minions\/">Every minion in the tavern&#39;s pool, by type<\/a>/);
});

test("accounts closed: no minion page and no link to one", { skip: OPEN }, () => {
  assert.ok(!existsSync(join(DIST, "minions")) && !existsSync(join(DIST, "es", "minions")));
  for (const file of files(DIST).filter((f) => f.endsWith(".html"))) {
    assert.doesNotMatch(readFileSync(file, "utf8"), /href="(?:\/es)?\/minions\//, relative(DIST, file));
  }
});

test("the sitemap lists exactly the indexable pages, each with its alternates; robots.txt names it", () => {
  const xml = read("sitemap.xml");
  assert.match(xml, /^<\?xml version="1\.0" encoding="UTF-8"\?>\n<urlset xmlns="http:\/\/www\.sitemaps\.org\/schemas\/sitemap\/0\.9" xmlns:xhtml="http:\/\/www\.w3\.org\/1999\/xhtml">/);
  const locs = [...xml.matchAll(/<loc>([^<]+)<\/loc>/g)].map((m) => m[1]).sort();
  const indexable = files(DIST)
    .filter((f) => f.endsWith(".html"))
    .filter((f) => !readFileSync(f, "utf8").includes('<meta name="robots" content="noindex">'))
    .map((f) => `${SITE}/${relative(DIST, f).replaceAll("\\", "/").replace(/index\.html$/, "")}`)
    .sort();
  assert.deepEqual(locs, indexable);
  assert.equal(locs.some((l) => l.includes("/minions/")), OPEN);
  for (const url of xml.split("<url>").slice(1)) {
    // Each page names its English and Spanish twins, English as the default, like the page's own head.
    const loc = /<loc>([^<]+)<\/loc>/.exec(url)?.[1] ?? "";
    const en = loc.replace(`${SITE}/es/`, `${SITE}/`);
    const es = en.replace(`${SITE}/`, `${SITE}/es/`);
    const links = [...url.matchAll(/<xhtml:link rel="alternate" hreflang="([a-z-]+)" href="([^"]+)"\/>/g)].map((m) => [m[1], m[2]]);
    assert.deepEqual(links, [["en", en], ["es", es], ["x-default", en]], loc);
    const html = readFileSync(join(DIST, loc.slice(SITE.length + 1), "index.html"), "utf8");
    for (const [lang, href] of links) assert.ok(html.includes(`<link rel="alternate" hreflang="${lang}" href="${href}">`), `${loc}: ${lang}`);
  }
  assert.match(read("robots.txt"), /^Sitemap: https:\/\/tavernledger\.net\/sitemap\.xml$/m);
});

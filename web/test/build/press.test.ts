// The press kit page (T-704), checked on the built files in every build mode:
// both languages, indexable, the release's own facts, the logo files it
// offers exist, the screenshots are links to the repository (never hosted
// here: they carry card art), the fixed fan notice, the contact address, and
// a footer link from every page.
import { test } from "node:test";
import assert from "node:assert/strict";
import { existsSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { RELEASE } from "../../src/lib/site/release.ts";

const ROOT = join(import.meta.dirname, "..", "..");
const DIST = join(ROOT, "dist");
const PAGES = [
  { file: "press/index.html", lang: "en", title: "Press kit", notice: "Unofficial fan project. Not affiliated with or endorsed by Blizzard Entertainment." },
  { file: "es/press/index.html", lang: "es", title: "Kit de prensa", notice: "Proyecto no oficial de fans. Sin afiliación con Blizzard Entertainment ni respaldo de esta." },
] as const;
const read = (file: string) => readFileSync(join(DIST, file), "utf8");

test("the press page exists in both languages, indexable, with its canonical address", () => {
  for (const page of PAGES) {
    assert.ok(existsSync(join(DIST, page.file)), "run `npm run build` first");
    const html = read(page.file);
    assert.match(html, new RegExp(`<html lang="${page.lang}">`));
    assert.doesNotMatch(html, /<meta name="robots" content="noindex">/);
    assert.match(html, new RegExp(`<title>${page.title} · Tavern Ledger</title>`));
    assert.match(html, new RegExp(`<link rel="canonical" href="https://tavernledger\\.net/${page.file.replace(/index\.html$/, "")}">`));
  }
});

test("the facts are the release's own, and nothing is counted that the project does not have", () => {
  for (const page of PAGES) {
    const html = read(page.file);
    assert.ok(html.includes(`${RELEASE.version} (beta)`), page.file);
    assert.match(html, page.lang === "en" ? /24 Oct 2026/ : /24 oct 2026/, page.file);
    assert.match(html, /AGPL-3\.0/);
    const main = /<main\b[\s\S]*<\/main>/.exec(html)?.[0] ?? "";
    assert.doesNotMatch(main, /\b(?:downloads|descargas|users|usuarios|reviews|reseñas)\b/i, `${page.file}: no made-up figure`);
  }
});

test("it says what the app reads, with the account risk, and links the terms section", () => {
  for (const page of PAGES) {
    const html = read(page.file);
    assert.ok(html.includes(page.lang === "en" ? "the account at risk is the player&#39;s" : "la cuenta en riesgo es la del jugador"), page.file);
    assert.ok(html.includes(page.lang === "en" ? 'href="/terms/#9-the-game"' : 'href="/es/terms/#9-el-juego"'), page.file);
  }
});

test("the logo files exist; the screenshots are links to the repository, not hosted here", () => {
  for (const page of PAGES) {
    const html = read(page.file);
    const files = [...html.matchAll(/<a href="(\/[^"]+\.(?:svg|png))" download>/g)].map((m) => m[1]);
    assert.deepEqual(files, ["/press/lockup.svg", "/press/mark.svg", "/press/app-tile.svg", "/icons/icon-512.png", "/social-card.png"], page.file);
    for (const file of files) assert.ok(existsSync(join(DIST, file)), file);
    const prefix = "https://github.com/KBroneZ/tavern-ledger/blob/main/";
    const shots = [...html.matchAll(/<a href="(https:\/\/github\.com\/KBroneZ\/tavern-ledger\/blob\/main\/docs\/screenshots\/[a-z-]+\.webp)">/g)].map((m) => m[1]);
    assert.equal(shots.length, 5, page.file);
    for (const url of shots) assert.ok(existsSync(join(ROOT, "..", url.slice(prefix.length))), url);
    assert.doesNotMatch(html, /<img\b[^>]*screenshots/, `${page.file}: no screenshot hosted on the site`);
  }
});

test("the fan notice is quoted as fixed text, and the contact address is there", () => {
  for (const page of PAGES) {
    const html = read(page.file);
    assert.ok(html.includes(`<blockquote class="press-notice"><p>${page.notice}</p></blockquote>`), page.file);
    assert.ok(html.includes('<a href="mailto:contact@tavernledger.net">contact@tavernledger.net</a>'), page.file);
  }
});

test("every page's footer links the press page in its language", () => {
  for (const [file, link] of [
    ["index.html", '<a href="/press/">Press kit</a>'],
    ["download/index.html", '<a href="/press/">Press kit</a>'],
    ["es/index.html", '<a href="/es/press/">Kit de prensa</a>'],
  ]) {
    assert.ok(read(file).includes(link), file);
  }
});

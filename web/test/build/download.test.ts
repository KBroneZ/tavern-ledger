// The download page (T-558), checked on the built files in every build mode:
// both languages, the one gold button pointing at the release's installer,
// the release facts from lib/site/release.ts and nothing else, the four steps
// and the folded extras, and the Download link in the menu.
import { test } from "node:test";
import assert from "node:assert/strict";
import { existsSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { RELEASE, installerUrl, releasesUrl } from "../../src/lib/site/release.ts";

const DIST = join(import.meta.dirname, "..", "..", "dist");
const PAGES = [
  { file: "download/index.html", lang: "en", download: "Download", all: "All releases on GitHub", steps: "Install it in four steps" },
  { file: "es/download/index.html", lang: "es", download: "Descargar", all: "Todas las versiones en GitHub", steps: "Instálala en cuatro pasos" },
] as const;
const read = (file: string) => readFileSync(join(DIST, file), "utf8");

test("the download page exists in English and Spanish, indexable, with its own canonical address", () => {
  for (const page of PAGES) {
    assert.ok(existsSync(join(DIST, page.file)), "run `npm run build` first");
    const html = read(page.file);
    assert.match(html, new RegExp(`<html lang="${page.lang}">`));
    assert.doesNotMatch(html, /<meta name="robots" content="noindex">/);
    const path = page.file.replace(/index\.html$/, "");
    assert.match(html, new RegExp(`<link rel="canonical" href="https://tavernledger\\.net/${path}">`));
  }
});

test("one gold key button: a link to the release's installer, plus the releases list in wood", () => {
  for (const page of PAGES) {
    const html = read(page.file);
    const links = [...html.matchAll(/<a class="tb tb-(key|common)" href="([^"]+)"[^>]*>\s*<span class="tb-label">([^<]+)/g)].map((m) => [m[1], m[2], m[3].trim()]);
    assert.deepEqual(links, [
      ["key", installerUrl(), page.download],
      ["common", releasesUrl(), page.all],
    ], page.file);
    assert.equal([...html.matchAll(/class="tb tb-key"/g)].length, 1, `${page.file}: one key action`);
  }
});

test("the page shows the release's own facts: version, file, size, day and SHA-256", () => {
  for (const page of PAGES) {
    const html = read(page.file);
    assert.ok(html.includes(RELEASE.version), `${page.file}: version`);
    assert.ok(html.includes(RELEASE.file), `${page.file}: file name`);
    assert.ok(html.includes(RELEASE.sha256), `${page.file}: hash`);
    assert.match(html, page.lang === "en" ? /6\.5 MB/ : /6,5 MB/, `${page.file}: size`);
    assert.match(html, page.lang === "en" ? /12 Oct 2026/ : /12 oct 2026/, `${page.file}: day`);
    assert.match(html, /Windows 10 (and|y) 11/, `${page.file}: Windows`);
    // The hash and the command are text, never a link or a script.
    assert.ok(html.includes(`Get-FileHash &quot;${RELEASE.file}&quot;`), `${page.file}: command`);
  }
});

test("four steps, and the extras folded shut behind native details", () => {
  for (const page of PAGES) {
    const html = read(page.file);
    const steps = /<ol class="dl-steps"[^>]*>([\s\S]*?)<\/ol>/.exec(html)?.[1] ?? "";
    assert.equal([...steps.matchAll(/<li>/g)].length, 4, `${page.file}: four steps`);
    assert.ok(html.includes(page.steps), page.file);
    const folds = [...html.matchAll(/<details\b[^>]*>/g)].map((m) => m[0]);
    assert.equal(folds.length, 4, `${page.file}: four extras`);
    for (const fold of folds) assert.doesNotMatch(fold, /\sopen\b/, `${page.file}: closed at first`);
    assert.equal([...html.matchAll(/<summary>/g)].length, 4, page.file);
  }
});

test("the only outside links are to the project's GitHub, and the privacy link stays in the page's language", () => {
  for (const page of PAGES) {
    const html = read(page.file);
    const outside = [...html.matchAll(/<a\b[^>]*\shref="(https?:\/\/[^"]+)"/g)].map((m) => m[1]);
    for (const url of outside) assert.match(url, /^https:\/\/github\.com\/KBroneZ\/tavern-ledger(\/|$)/, `${page.file}: ${url}`);
    assert.match(html, new RegExp(`href="${page.lang === "en" ? "" : "/es"}/privacy/"`), page.file);
  }
});

test("the page needs no script of its own, and says plainly that the installer is unsigned", () => {
  for (const page of PAGES) {
    const html = read(page.file);
    const named = [...html.matchAll(/<script\b[^>]*\ssrc="\/_astro\/([^"]+)"/g)].map((m) => m[1]);
    assert.deepEqual(named, [], `${page.file}: no bundle`);
    assert.match(html, page.lang === "en" ? /not signed yet/ : /sin firmar/, page.file);
    assert.match(html, page.lang === "en" ? /More info/ : /Más información/, page.file);
  }
});

test("the menu has a Download link on every page, current on the download page", () => {
  for (const [file, text, target] of [
    ["index.html", "Download", "/download/"],
    ["es/index.html", "Descargar", "/es/download/"],
    ["privacy/index.html", "Download", "/download/"],
    ["es/privacy/index.html", "Descargar", "/es/download/"],
  ]) {
    assert.match(read(file), new RegExp(`<a href="${target}">${text}</a>`), file);
  }
  assert.match(read("download/index.html"), /<a href="\/download\/" aria-current="page">Download<\/a>/);
  assert.match(read("es/download/index.html"), /<a href="\/es\/download\/" aria-current="page">Descargar<\/a>/);
});

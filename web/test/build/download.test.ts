// The download page (T-558), checked on the built files in every build mode:
// both languages, the one gold button pointing at the release's installer,
// the release facts from lib/site/release.ts and nothing else, the four steps
// and the folded extras, and the Download link in the menu.
import { test } from "node:test";
import assert from "node:assert/strict";
import { existsSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { loadEnv } from "vite";
import { readConfig } from "../../src/lib/config.ts";
import { RELEASE, installerUrl, releasesUrl } from "../../src/lib/site/release.ts";

const ROOT = join(import.meta.dirname, "..", "..");
const DIST = join(ROOT, "dist");
const CONFIG = readConfig(loadEnv("production", ROOT, "PUBLIC_"));
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
    assert.match(html, new RegExp(`<dd>${RELEASE.version} \\(beta\\)</dd>`), `${page.file}: version`);
    assert.ok(html.includes(RELEASE.file), `${page.file}: file name`);
    assert.ok(html.includes(RELEASE.sha256), `${page.file}: hash`);
    assert.match(html, page.lang === "en" ? /7\.6 MB/ : /7,6 MB/, `${page.file}: size`);
    assert.match(html, page.lang === "en" ? /16 Oct 2026/ : /16 oct 2026/, `${page.file}: day`);
    assert.match(html, /Windows 10 (and|y) 11/, `${page.file}: Windows`);
    // The hash and the command are text, never a link or a script.
    assert.ok(html.includes(`Get-FileHash &quot;${RELEASE.file}&quot;`), `${page.file}: command`);
  }
});

test("four one-line steps and four extras, each with its Read more folded shut", () => {
  for (const page of PAGES) {
    const html = read(page.file);
    const steps = /<ol class="dl-steps"[^>]*>([\s\S]*?)<\/ol>/.exec(html)?.[1] ?? "";
    assert.equal([...steps.matchAll(/<li>/g)].length, 4, `${page.file}: four steps`);
    assert.equal([...steps.matchAll(/<p class="dl-step-line">/g)].length, 4, `${page.file}: a line each`);
    assert.equal([...steps.matchAll(/<details class="more"/g)].length, 4, `${page.file}: each step has its Read more`);
    assert.ok(html.includes(page.steps), page.file);
    const folds = [...html.matchAll(/<details\b[^>]*>/g)].map((m) => m[0]);
    assert.equal(folds.length, 8, `${page.file}: four steps and four extras`);
    for (const fold of folds) assert.doesNotMatch(fold, /\sopen\b/, `${page.file}: closed at first`);
    assert.equal([...html.matchAll(/<summary>/g)].length, 8, page.file);
    // The shared "Read more" fold (D-130) after a short line that stays in view.
    assert.equal([...html.matchAll(/<details class="more"/g)].length, 8, `${page.file}: Read more`);
    assert.equal([...html.matchAll(/<p class="dl-short">/g)].length, 4, `${page.file}: a short line each`);
  }
});

test("step 4 says how to sign in when accounts are open and that they are not open yet when closed", () => {
  for (const page of PAGES) {
    const html = read(page.file);
    const open = page.lang === "en" ? /Create an account on the website/ : /Crea una cuenta en la web/;
    const closed = page.lang === "en" ? /Accounts on the website are not open yet/ : /Las cuentas de la web aún no están abiertas/;
    assert.match(html, CONFIG ? open : closed, page.file);
    assert.doesNotMatch(html, CONFIG ? closed : open, page.file);
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
    assert.doesNotMatch(html, /<script\b(?![^>]*\ssrc="\/lang\.js")/, `${page.file}: no other script`);
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

test("Updating says the app updates itself from 0.2.1, and 0.1.0 is installed over once", () => {
  for (const page of PAGES) {
    const html = read(page.file);
    const start = html.indexOf(page.lang === "en" ? ">Updating<" : ">Actualizar<");
    assert.ok(start >= 0, `${page.file}: the Updating extra`);
    const updating = html.slice(start);
    const end = updating.indexOf('<div class="dl-extra">');
    assert.ok(end > 0, `${page.file}: the extra after Updating`);
    const block = updating.slice(0, end);
    assert.match(block, page.lang === "en" ? /From version 0\.2\.1 the app updates itself/ : /Desde la versión 0\.2\.1 la aplicación se actualiza sola/, page.file);
    assert.match(block, page.lang === "en" ? /version 0\.1\.0/ : /versión 0\.1\.0/, page.file);
    assert.match(block, page.lang === "en" ? /keeps your games and settings/ : /conserva tus partidas y ajustes/, page.file);
    assert.doesNotMatch(block, page.lang === "en" ? /no automatic updates/i : /no hay actualizaciones automáticas/i, page.file);
  }
});

test("a line before the download button says the memory read is on by default and links to the terms section", () => {
  for (const page of PAGES) {
    const html = read(page.file);
    const prefix = page.lang === "es" ? "/es" : "";
    const section = page.lang === "es" ? "9-el-juego" : "9-the-game";
    const line = new RegExp(`dl-note">[^<]*(?:By default|Por defecto)[^<]*memor[^<]*<a href="${prefix}/terms/#${section}">`);
    const at = html.search(line);
    assert.ok(at >= 0, `${page.file}: the memory line links to the terms section`);
    assert.ok(at < html.indexOf('class="dl-actions"'), `${page.file}: it comes before the download button`);
    assert.doesNotMatch(html, /approved by Blizzard|aprobado por Blizzard/i);
  }
});

test("the terms and the privacy policy say the read is on by default, how to turn it off and who bears the risk", () => {
  const terms = [read("terms/index.html"), read("es/terms/index.html")];
  assert.match(terms[0], /id="9-the-game"/);
  assert.match(terms[0], /By default \(you can turn it off at any time in Settings[\s\S]*The risk to your [\s\S]{0,60}account is yours/);
  assert.match(terms[1], /id="9-el-juego"/);
  assert.match(terms[1], /Por defecto \(puedes desactivarlo cuando quieras en Ajustes[\s\S]*El riesgo para tu cuenta de [\s\S]{0,60}es tuyo/);
  assert.match(read("privacy/index.html"), /Reading from the game \(on by default; you can turn it off\)/);
  assert.match(read("es/privacy/index.html"), /Lectura del juego \(activada por defecto; puedes desactivarla\)/);
});

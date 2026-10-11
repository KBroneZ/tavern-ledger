// The built accuracy page (T-607) and the home page's "What's new" (D-089),
// in every build mode: built in both languages from the committed export,
// indexable, each mode's accuracy in the same row as its blocked share, no
// fetch and no script but the switches, linked from the footer.
import { test } from "node:test";
import assert from "node:assert/strict";
import { existsSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { parseAccuracy } from "../../src/lib/accuracy/data.ts";

const ROOT = join(import.meta.dirname, "..", "..");
const DIST = join(ROOT, "dist");
const PAGES = ["accuracy/index.html", "es/accuracy/index.html"];
const read = (page: string) => readFileSync(join(DIST, page), "utf8");
const data = parseAccuracy(JSON.parse(readFileSync(join(ROOT, "src", "data", "accuracy.json"), "utf8")));
const text = (html: string) => html.replace(/<[^>]+>/g, " ").replace(/\s+/g, " ");

test("the accuracy page is built in English and Spanish and is indexable", () => {
  for (const page of PAGES) {
    assert.ok(existsSync(join(DIST, page)), `${page}: run \`npm run build\` first`);
    const html = read(page);
    assert.ok(!html.includes('<meta name="robots" content="noindex">'), page);
    assert.match(html, /<link rel="canonical" href="https:\/\/tavernledger\.net\/(es\/)?accuracy\/">/, page);
  }
});

test("each mode's tile row holds the accuracy and the blocked share together", () => {
  for (const page of PAGES) {
    const html = read(page);
    const sections = html.split('<section class="data-section"').slice(1, 3);
    assert.equal(sections.length, 2, page);
    for (const section of sections) {
      const tiles = section.slice(section.indexOf('class="tiles"'), section.indexOf("<figure"));
      assert.equal((tiles.match(/class="tile tile-hero"/g) ?? []).length, 1, `${page}: one accuracy headline`);
      assert.equal((tiles.match(/class="acc-blocked"/g) ?? []).length, 1, `${page}: the blocked share beside it`);
    }
  }
});

test("the Brier score, refused count and card ids are Advanced only", () => {
  for (const page of PAGES) {
    const html = read(page);
    const brier = page.startsWith("es/") ? "Puntuación de Brier" : "Brier score";
    const adv = [...html.matchAll(/<div class="adv">([\s\S]*?)<\/div>\s*<\/div>\s*<figure/g)].map((m) => m[1]);
    assert.equal(adv.length, 2, page);
    for (const block of adv) assert.ok(block.includes(brier), page);
    assert.equal(html.split(brier).length - 1, 2, `${page}: the Brier label only inside the Advanced blocks`);
    assert.match(html, /<th scope="col" class="adv">/, page);
    assert.ok(!/<td class="acc-id/.test(html), `${page}: card id cells are Advanced`);
  }
});

test("the page shows the committed numbers: counts, version and date", () => {
  const en = text(read("accuracy/index.html"));
  for (const mode of ["solo", "duos"] as const) {
    const m = data.modes[mode];
    assert.ok(en.includes(`${m.most_likely_right} of ${m.scored} combats`), mode);
    assert.ok(en.includes(`(${m.blocked} of ${m.combats_with_result})`) || m.blocked === 0, mode);
  }
  assert.ok(en.includes(`Engine ${data.engine_version}`));
  for (const card of data.blocking_cards) assert.ok(en.includes(card.card_id), card.card_id);
  const es = text(read("es/accuracy/index.html"));
  assert.ok(es.includes(`${data.modes.duos.most_likely_right} de ${data.modes.duos.scored} combates`));
});

test("the accuracy page fetches nothing and runs no script of its own", () => {
  for (const page of PAGES) {
    const html = read(page);
    const scripts = [...html.matchAll(/<script[^>]*src="([^"]+)"/g)].map((m) => m[1]);
    for (const src of scripts) assert.match(src, /^\/(lang\.js|_astro\/[^"]*\.js)$/, `${page}: ${src}`);
    assert.ok(!html.includes("accuracy.json"), page);
  }
});

test("every page's footer links the accuracy page in its language", () => {
  assert.match(read("index.html"), /<a href="\/accuracy\/">Simulator accuracy<\/a>/);
  assert.match(read("es/index.html"), /<a href="\/es\/accuracy\/">Precisión del simulador<\/a>/);
});

test("the home page has a dated What's new block in both languages", () => {
  const en = read("index.html");
  const es = read("es/index.html");
  assert.match(en, /<h2 id="news-heading" class="sec-h">What&#39;s new<\/h2>|<h2 id="news-heading" class="sec-h">What's new<\/h2>/);
  assert.match(es, /<h2 id="news-heading" class="sec-h">Novedades<\/h2>/);
  assert.match(en, /<time datetime="\d{4}-\d{2}-\d{2}">/);
  assert.match(en, /href="\/accuracy\/">Simulator accuracy</);
  assert.match(es, /href="\/es\/accuracy\/">Precisión del simulador</);
  assert.match(es, /href="\/es\/heroes\/">/);
});

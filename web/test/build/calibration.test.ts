// The built calibration page (T-655), in every build mode: built in both
// languages from the committed file, indexable, linked both ways with the
// accuracy page, no fetch and no script of its own; while no model is
// published it shows the empty state and no number of a model.
import { test } from "node:test";
import assert from "node:assert/strict";
import { existsSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { parseCalibration } from "../../src/lib/calibration/data.ts";

const ROOT = join(import.meta.dirname, "..", "..");
const DIST = join(ROOT, "dist");
const PAGES = ["calibration/index.html", "es/calibration/index.html"];
const read = (page: string) => readFileSync(join(DIST, page), "utf8");
const data = parseCalibration(JSON.parse(readFileSync(join(ROOT, "src", "data", "calibration.json"), "utf8")));
const empty = data.modes.solo === null && data.modes.duos === null;
const text = (html: string) => html.replace(/<[^>]+>/g, " ").replace(/\s+/g, " ");

test("the calibration page is built in English and Spanish and is indexable", () => {
  for (const page of PAGES) {
    assert.ok(existsSync(join(DIST, page)), `${page}: run \`npm run build\` first`);
    const html = read(page);
    assert.ok(!html.includes('<meta name="robots" content="noindex">'), page);
    assert.match(html, /<link rel="canonical" href="https:\/\/tavernledger\.net\/(es\/)?calibration\/">/, page);
  }
  assert.ok(read("calibration/index.html").includes("<title>Expected place calibration · Tavern Ledger</title>"));
  assert.ok(read("es/calibration/index.html").includes("<title>Calibración del puesto esperado · Tavern Ledger</title>"));
});

test("the calibration and accuracy pages link each other in their language", () => {
  assert.match(read("calibration/index.html"), /<a href="\/accuracy\/">Simulator accuracy<\/a>/);
  assert.match(read("es/calibration/index.html"), /<a href="\/es\/accuracy\/">Precisión del simulador<\/a>/);
  assert.match(read("accuracy/index.html"), /<a href="\/calibration\/">Expected place calibration<\/a>/);
  assert.match(read("es/accuracy/index.html"), /<a href="\/es\/calibration\/">Calibración del puesto esperado<\/a>/);
});

test("with no model published, the page says so and shows no model's numbers", { skip: !empty && "a model is published" }, () => {
  for (const page of PAGES) {
    const es = page.startsWith("es/");
    const html = read(page);
    const body = text(html);
    assert.ok(body.includes(es ? "Aún no hay ningún modelo publicado" : "No model is published yet"), page);
    assert.ok(body.includes(es ? "al menos 150 partidas por modo" : "at least 150 games per mode"), page);
    assert.ok(!html.includes("<table"), `${page}: no table without a model`);
    assert.ok(!html.includes('class="tile'), `${page}: no figure without a model`);
  }
});

test("with a model published, each listed build has a table with every group", { skip: empty && "no model is published" }, () => {
  const html = read("calibration/index.html");
  for (const mode of [data.modes.solo, data.modes.duos]) {
    for (const b of mode?.builds ?? []) assert.ok(html.includes(`<strong>Build ${b.build}</strong>`), String(b.build));
  }
});

test("the calibration page fetches nothing and runs no script of its own", () => {
  for (const page of PAGES) {
    const html = read(page);
    const scripts = [...html.matchAll(/<script[^>]*src="([^"]+)"/g)].map((m) => m[1]);
    for (const src of scripts) assert.match(src, /^\/(lang\.js|pwa\.js|_astro\/[^"]*\.js)$/, `${page}: ${src}`);
    assert.ok(!html.includes("calibration.json"), page);
  }
});

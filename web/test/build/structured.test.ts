// Structured data on the download and simulator pages (T-703), checked on the
// built files: one JSON-LD data block with the release's own facts, never a
// rating, review or count the project does not have.
import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { loadEnv } from "vite";
import { readConfig } from "../../src/lib/config.ts";
import { RELEASE, installerUrl } from "../../src/lib/site/release.ts";

const ROOT = join(import.meta.dirname, "..", "..");
const DIST = join(ROOT, "dist");
const CONFIG = readConfig(loadEnv("production", ROOT, "PUBLIC_"));
const read = (file: string) => readFileSync(join(DIST, file), "utf8");

function graphs(html: string): Record<string, unknown>[] {
  const blocks = [...html.matchAll(/<script type="application\/ld\+json">([\s\S]*?)<\/script>/g)].map((m) => JSON.parse(m[1]));
  for (const block of blocks) assert.equal(block["@context"], "https://schema.org");
  return blocks.flatMap((block) => block["@graph"]);
}

const NEVER = ["aggregateRating", "review", "interactionStatistic", "downloads"];

test("the download page describes the app with the release's own facts", () => {
  for (const [file, url] of [
    ["download/index.html", "https://tavernledger.net/download/"],
    ["es/download/index.html", "https://tavernledger.net/es/download/"],
  ]) {
    const nodes = graphs(read(file));
    assert.equal(nodes.length, 1, file);
    const app = nodes[0];
    assert.equal(app["@type"], "SoftwareApplication", file);
    assert.equal(app.name, "Tavern Ledger");
    assert.equal(app.url, url);
    assert.equal(app.softwareVersion, RELEASE.version);
    assert.equal(app.downloadUrl, installerUrl());
    assert.equal(app.operatingSystem, "Windows 10, Windows 11");
    assert.equal(app.isAccessibleForFree, true);
    assert.deepEqual(app.offers, { "@type": "Offer", price: "0", priceCurrency: "EUR" });
    for (const key of NEVER) assert.ok(!(key in app), `${file}: no ${key}`);
    const description = /<meta name="description" content="([^"]+)"/.exec(read(file))?.[1];
    assert.equal(app.description, description?.replaceAll("&#39;", "'"), `${file}: the page's own description`);
  }
});

test("the simulator page carries its structured data only when accounts are open", () => {
  for (const [file, url, name] of [
    ["simulator/index.html", "https://tavernledger.net/simulator/", "Battlegrounds combat simulator"],
    ["es/simulator/index.html", "https://tavernledger.net/es/simulator/", "Simulador de combates de Campos de Batalla"],
  ]) {
    const html = read(file);
    const nodes = graphs(html);
    if (!CONFIG) {
      assert.equal(nodes.length, 0, file);
      continue;
    }
    assert.equal(nodes.length, 1, file);
    const sim = nodes[0];
    assert.equal(sim["@type"], "WebApplication");
    assert.equal(sim.url, url);
    assert.equal(sim.name, name);
    assert.match(html, new RegExp(`<title>${name} · Tavern Ledger</title>`), file);
    for (const key of NEVER) assert.ok(!(key in sim), `${file}: no ${key}`);
  }
});

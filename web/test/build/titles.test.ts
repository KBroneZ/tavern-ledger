// The home and download pages' titles say what the app is, in the words
// players search for (T-708): the search result and the link preview show
// the <title> and og:title, not the page's painted heading.
import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { join } from "node:path";

const DIST = join(import.meta.dirname, "..", "..", "dist");
const read = (file: string) => readFileSync(join(DIST, file), "utf8");

test("home and download titles name the tracker, in both languages, in <title> and og:title", () => {
  for (const [file, title] of [
    ["index.html", "Free Hearthstone Battlegrounds tracker for Windows"],
    ["download/index.html", "Download the free Battlegrounds tracker for Windows"],
    ["es/index.html", "Tracker gratuito de Campos de Batalla de Hearthstone para Windows"],
    ["es/download/index.html", "Descarga el tracker gratuito de Campos de Batalla para Windows"],
  ]) {
    const html = read(file);
    assert.ok(html.includes(`<title>${title} · Tavern Ledger</title>`), file);
    assert.ok(html.includes(`<meta property="og:title" content="${title}">`), file);
  }
});

// Source rules for the site (T-104c): data only reaches the page through
// textContent, and no secret or service key lives in web/.
import { test } from "node:test";
import assert from "node:assert/strict";
import { readdirSync, readFileSync, statSync } from "node:fs";
import { join, relative } from "node:path";

const ROOT = join(import.meta.dirname, "..", "..");
const SKIP = new Set(["node_modules", "dist", ".astro", "test"]);

function files(dir: string): string[] {
  return readdirSync(dir).flatMap((name) => {
    if (SKIP.has(name)) return [];
    const path = join(dir, name);
    return statSync(path).isDirectory() ? files(path) : [path];
  });
}

const SOURCES = files(ROOT).filter((f) => /\.(ts|astro|mjs|js|css|html|json|txt)$|_headers$/.test(f));

test("the scan finds the site's sources", () => {
  const names = SOURCES.map((f) => relative(ROOT, f).replaceAll("\\", "/"));
  assert.ok(names.includes("src/scripts/account.ts"), names.join(", "));
});

test("no HTML is built from strings", () => {
  const banned = /innerHTML|outerHTML|insertAdjacentHTML|document\.write|set:html|createContextualFragment|\beval\(|new Function\(/;
  for (const file of SOURCES) {
    assert.doesNotMatch(readFileSync(file, "utf8"), banned, relative(ROOT, file));
  }
});

test("no service key or secret in the site", () => {
  const banned = /service_role|SERVICE_ROLE|sb_secret_[A-Za-z0-9]|SUPABASE_SERVICE/;
  for (const file of SOURCES) {
    if (file.endsWith("config.ts")) continue; // names them only to refuse them
    assert.doesNotMatch(readFileSync(file, "utf8"), banned, relative(ROOT, file));
  }
});

test("no inline event handlers or inline styles in the pages", () => {
  for (const file of SOURCES.filter((f) => f.endsWith(".astro"))) {
    const text = readFileSync(file, "utf8");
    assert.doesNotMatch(text, /\son[a-z]+=|\sstyle=|<style|javascript:/i, relative(ROOT, file));
  }
});

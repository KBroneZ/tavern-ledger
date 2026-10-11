// The data pages keep every string in one table (T-501, for T-508's Spanish).
// Pictures have their own tests (pictures.test.ts).
import { test } from "node:test";
import assert from "node:assert/strict";
import { readdirSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { STRINGS, t } from "../../src/lib/data/strings.ts";

const ROOT = join(import.meta.dirname, "..", "..");

test("t fills each placeholder and refuses a missing one or an unknown key", () => {
  assert.equal(t("backing.some", { known: 28, total: 31, unit: "games" }), "in 28 of 31 games");
  assert.throws(() => t("backing.some", { known: 28 }), /total/);
  assert.throws(() => t("no.such.key" as keyof typeof STRINGS), /no.such.key/);
});

test("every key the data components name exists in the table", () => {
  const dir = join(ROOT, "src", "components", "data");
  const sources = readdirSync(dir).map((f) => readFileSync(join(dir, f), "utf8"));
  sources.push(readFileSync(join(ROOT, "src", "pages", "style.astro"), "utf8"));
  const keys = sources.flatMap((s) => [...s.matchAll(/\bt\("([a-z0-9.]+)"/g)].map((m) => m[1]));
  assert.ok(keys.length > 10, "the components use the table");
  for (const key of keys) assert.ok(key in STRINGS, key);
});

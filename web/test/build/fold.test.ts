// "Short line + Read more" (T-557, D-130), checked on the built files: the
// three switches that share data keep what they share, that they are off by
// default and that they can be withdrawn in the line that is always in view,
// and every fold is a native <details> with the language's "Read more".
import { test } from "node:test";
import assert from "node:assert/strict";
import { existsSync, readdirSync, readFileSync, statSync } from "node:fs";
import { join, relative } from "node:path";
import { loadEnv } from "vite";
import { readConfig } from "../../src/lib/config.ts";
import { h } from "../../src/lib/view/node.ts";
import { more } from "../../src/lib/view/pieces.ts";
import { toHtml } from "../helpers/html.ts";

const ROOT = join(import.meta.dirname, "..", "..");
const DIST = join(ROOT, "dist");
const CONFIG = readConfig(loadEnv("production", ROOT, "PUBLIC_"));

function files(dir: string): string[] {
  return readdirSync(dir).flatMap((n) => {
    const path = join(dir, n);
    return statSync(path).isDirectory() ? files(path) : [path];
  });
}

const PAGES = (existsSync(DIST) ? files(DIST) : []).filter((f) => f.endsWith(".html")).map((f) => relative(DIST, f).replaceAll("\\", "/"));
const read = (page: string) => readFileSync(join(DIST, page), "utf8");
const textOf = (html: string) => html.replace(/<[^>]+>/g, " ").replace(/&#39;/g, "'").replace(/\s+/g, " ").trim();

/** Each consent switch: the input's id, then what its visible line must say, in each language. */
const CONSENT = [
  {
    id: "is-public",
    en: [/Off by default/, /anyone can find your profile/, /every game you uploaded/, /public leaderboards/, /Turning it off hides you/],
    es: [/Desactivado por defecto/, /cualquiera puede encontrar tu perfil/, /todas las partidas que has subido/, /clasificaciones públicas/, /Si lo desactivas/],
  },
  {
    id: "hero-stats",
    en: [/Off by default/, /count toward the hero stats/, /never with your name/, /turning it off takes them out/],
    es: [/Desactivado por defecto/, /cuentan para las estadísticas de héroes/, /nunca con tu nombre/, /Si lo desactivas/],
  },
  {
    id: "improve-simulator",
    en: [/Off by default/, /boards and combat replays of your uploaded games/, /only as totals over many games/, /never with your name/, /turn it off at any time/],
    es: [/Desactivado por defecto/, /tableros y las repeticiones de los combates/, /solo como totales de muchas partidas/, /nunca con tu nombre/, /desactivarlo cuando quieras/],
  },
];

test("the built pages exist", () => {
  assert.ok(PAGES.length >= 6, "run `npm run build` first");
});

test("the three consent switches say what is shared, that it is off by default and that it can be withdrawn outside the fold", { skip: CONFIG === null }, () => {
  for (const [page, lang] of [["account/index.html", "en"], ["es/account/index.html", "es"]] as const) {
    const html = read(page);
    for (const sw of CONSENT) {
      const input = new RegExp(`<input[^>]*\\bid="${sw.id}"[^>]*aria-describedby="([^"]+)"`).exec(html);
      assert.ok(input, `${page}: the ${sw.id} switch names its short line`);
      assert.match(html, new RegExp(`<label for="${sw.id}">`), `${page}: ${sw.id} has its label`);
      const hint = new RegExp(`<p class="hint" id="${input[1]}">([\\s\\S]*?)</p>`).exec(html);
      assert.ok(hint, `${page}: the short line of ${sw.id}`);
      // Not inside a <details>: count the folds opened and closed before it.
      const before = html.slice(0, hint.index);
      const open = (before.match(/<details\b/g) ?? []).length - (before.match(/<\/details>/g) ?? []).length;
      assert.equal(open, 0, `${page}: the short line of ${sw.id} is outside every fold`);
      const text = textOf(hint[1]);
      for (const needed of sw[lang]) assert.match(text, needed, `${page}: ${sw.id}`);
      // A fold follows the line, so the rest is one click away.
      const after = html.slice(hint.index + hint[0].length, hint.index + hint[0].length + 400);
      assert.match(after, /^\s*<details class="more"/, `${page}: ${sw.id} has its Read more`);
    }
  }
});

test("each consent switch's short line is one the user can read at a glance", { skip: CONFIG === null }, () => {
  for (const page of ["account/index.html", "es/account/index.html"]) {
    const html = read(page);
    for (const sw of CONSENT) {
      const input = new RegExp(`<input[^>]*\\bid="${sw.id}"[^>]*aria-describedby="([^"]+)"`).exec(html);
      assert.ok(input, `${page}: the ${sw.id} switch`);
      const hint = new RegExp(`<p class="hint" id="${input[1]}">([\\s\\S]*?)</p>`).exec(html);
      assert.ok(hint, `${page}: the short line of ${sw.id}`);
      assert.ok(textOf(hint[1]).length <= 400, `${page}: ${sw.id} stays short`);
    }
  }
});

test("every fold is a native <details> with the language's Read more, and says what it is about", () => {
  for (const page of PAGES) {
    const html = read(page);
    const spanish = page.startsWith("es/");
    for (const fold of html.matchAll(/<details class="more"[^>]*>\s*<summary>([\s\S]*?)<\/summary>/g)) {
      const summary = textOf(fold[1]);
      assert.match(summary, spanish ? /^Leer más\s*:\s*\S/ : /^Read more\s*:\s*\S/, page);
    }
  }
});

test("the style page shows the pattern and the browser pages draw the same markup", () => {
  const html = read("style/index.html");
  const demo = /<details class="more">\s*<summary>[\s\S]*?<\/details>/.exec(html.slice(html.indexOf('id="s-more"')));
  assert.ok(demo, "the style page has a Read more");
  const drawn = toHtml(
    more({ context: "Short line and Read more" }, h("p", {}, "The rest of the text sits here, in the same words the page would have shown in full."), h("p", {}, "It can have more than one paragraph, and links.")),
  );
  const squash = (s: string) => s.replace(/>\s+</g, "><").replace(/\s+/g, " ").trim();
  assert.equal(squash(demo[0]), squash(drawn));
});

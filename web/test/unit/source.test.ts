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

/**
 * Every way to turn a string into markup or code: the HTML sinks, `srcdoc`,
 * the HTML parsers, `document.write`, `eval` and its cousins (`Function(`,
 * a timer given a string). Data only reaches the page as text nodes.
 */
const SINKS: [name: string, pattern: RegExp][] = [
  ["innerHTML", /innerHTML/],
  ["outerHTML", /outerHTML/],
  ["insertAdjacentHTML", /insertAdjacentHTML/],
  ["set:html", /set:html/],
  ["document.write", /document\s*\.\s*write(ln)?\b/],
  ["createContextualFragment", /createContextualFragment/],
  ["srcdoc", /srcdoc/i],
  ["DOMParser", /DOMParser/],
  ["setHTMLUnsafe", /setHTMLUnsafe|parseHTMLUnsafe/],
  ["eval", /\beval\s*\(/],
  ["Function constructor", /\bFunction\s*\(/],
  ["timer with a string", /\bset(Timeout|Interval)\s*\(\s*["'`]/],
];

/**
 * The files allowed one of the sinks above, each with the sinks it may use.
 * A new entry is a decision, made in a review, not a fix to a failing test.
 * - JsonLd.astro (T-578, D-134): the minion pages' structured data, a
 *   <script type="application/ld+json"> data block the browser never runs.
 *   Its only input is jsonLd(), JSON.stringify with every "<" escaped, so
 *   no value can end the block (minionView.test.ts, build test).
 */
const ALLOWED: ReadonlyMap<string, readonly string[]> = new Map([["src/components/minions/JsonLd.astro", ["set:html"]]]);

/** The sinks a text uses that its file is not allowed (a file name is relative to web/, with slashes). */
function sinksIn(file: string, text: string): string[] {
  const allowed = ALLOWED.get(file) ?? [];
  return SINKS.filter(([name, pattern]) => pattern.test(text) && !allowed.includes(name)).map(([name]) => name);
}

test("no HTML is built from strings, outside the allow-list", () => {
  for (const file of SOURCES) {
    const name = relative(ROOT, file).replaceAll("\\", "/");
    assert.deepEqual(sinksIn(name, readFileSync(file, "utf8")), [], name);
  }
});

test("the sink scan catches each sink and nothing else", () => {
  const samples: Record<string, string> = {
    innerHTML: "el.innerHTML = x",
    outerHTML: "el.outerHTML = x",
    insertAdjacentHTML: 'el.insertAdjacentHTML("beforeend", x)',
    "set:html": "<div set:html={x} />",
    "document.write": "document.write(x)",
    createContextualFragment: "range.createContextualFragment(x)",
    srcdoc: "frame.srcdoc = x",
    DOMParser: "new DOMParser().parseFromString(x, 'text/html')",
    setHTMLUnsafe: "el.setHTMLUnsafe(x)",
    eval: "eval(x)",
    "Function constructor": "const f = Function(x)",
    "timer with a string": 'setTimeout("run()", 0)',
  };
  assert.deepEqual(Object.keys(samples).toSorted(), SINKS.map(([name]) => name).toSorted(), "a sample for every sink");
  for (const [name, text] of Object.entries(samples)) assert.deepEqual(sinksIn("src/x.ts", text), [name], name);
  assert.deepEqual(sinksIn("src/x.ts", "document . writeln(x)"), ["document.write"]);
  assert.deepEqual(sinksIn("src/x.ts", "setTimeout(() => run(), 0); el.textContent = x"), []);
  assert.deepEqual(sinksIn("src/ok.ts", "el.innerHTML = x"), ["innerHTML"], "not allow-listed here");
  assert.deepEqual([...ALLOWED.keys()], ["src/components/minions/JsonLd.astro"], "only the structured data block is allowed a sink");
  assert.deepEqual(sinksIn("src/components/minions/JsonLd.astro", "<div set:html={x} /> el.innerHTML = x"), ["innerHTML"], "and only set:html");
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

test("the one script the site adds at run time is Cloudflare Turnstile's, on the forms that need it", () => {
  const adders = SOURCES.filter((f) => /createElement\(\s*["'`]script/.test(readFileSync(f, "utf8"))).map((f) =>
    relative(ROOT, f).replaceAll("\\", "/"),
  );
  assert.deepEqual(adders, ["src/scripts/captcha.ts"]);
  const loader = readFileSync(join(ROOT, "src", "scripts", "captcha.ts"), "utf8");
  assert.match(loader, /script\.src = TURNSTILE_SCRIPT;/, "only the published Turnstile address");
  assert.equal(loader.match(/\.src\s*=/g)?.length, 1);
});

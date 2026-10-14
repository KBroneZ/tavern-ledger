// The site's languages (T-508, D-086): every key in English and Spanish,
// the same placeholders, numbers and dates per language, addresses per
// language, the browser's language script, and pages drawn in Spanish with
// no English left.
import { afterEach, test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { runInNewContext } from "node:vm";
import { basePath, href, isLang, langPaths, pathIn, pathLang, placeholders, pluralForm, setLang, TABLES } from "../../src/lib/i18n.ts";
// Each table registers itself when imported.
import { ACCOUNT_STRINGS } from "../../src/lib/account/strings.ts";
import { ACC_STRINGS } from "../../src/lib/accuracy/strings.ts";
import { CH_STRINGS } from "../../src/lib/community/strings.ts";
import { DL_STRINGS } from "../../src/lib/download/strings.ts";
import { STRINGS, t } from "../../src/lib/data/strings.ts";
import { GAME_STRINGS } from "../../src/lib/game/strings.ts";
import { LB_STRINGS } from "../../src/lib/leaderboards/strings.ts";
import { SITE_STRINGS } from "../../src/lib/site/strings.ts";
import { SIM_STRINGS } from "../../src/lib/sim/strings.ts";
import { MIN_STRINGS } from "../../src/lib/minions/strings.ts";
import { average, backing, compact, ordinal, share, trend } from "../../src/lib/data/format.ts";
import { dateText } from "../../src/lib/data/dashboard.ts";
import { formatValue, refreshedText } from "../../src/lib/leaderboards/boards.ts";
import { rate } from "../../src/lib/community/heroes.ts";
import { checkCredentials, signUpOutcome } from "../../src/lib/auth.ts";
import { confirmPhrase, isConfirmed } from "../../src/lib/deleteAccount.ts";
import { gamePath } from "../../src/lib/game/url.ts";
import { publicProfilePath } from "../../src/lib/profile.ts";
import { renderGame } from "../../src/lib/game/render.ts";
import { readRecord } from "../../src/lib/game/report.ts";
import { shapeGame } from "../../src/lib/game/shape.ts";
import { duosReport, INDEX, record, SESSION, soloReport } from "./gameFixtures.ts";
import { dashboardPanels } from "../../src/lib/view/dashboardView.ts";
import { gameRow, heroRow, overview, recordRow, type RecordRow } from "../../src/lib/data/rows.ts";
import { gameRaw, heroRaw, overviewRow, recordRaw } from "../helpers/rows.ts";
import { toHtml } from "../helpers/html.ts";
import type { VNode } from "../../src/lib/view/node.ts";

const ROOT = join(import.meta.dirname, "..", "..");

afterEach(() => setLang("en"));

test("every table is registered: site, account, data, download, game, leaderboards, community, accuracy, simulator, minions", () => {
  assert.deepEqual(TABLES.map((x) => x.name).sort(), ["account", "accuracy", "community", "data", "download", "game", "leaderboards", "minions", "simulator", "site"]);
  const english = [ACCOUNT_STRINGS, ACC_STRINGS, CH_STRINGS, DL_STRINGS, STRINGS, GAME_STRINGS, LB_STRINGS, SITE_STRINGS, SIM_STRINGS, MIN_STRINGS];
  for (const en of english) assert.ok(TABLES.some((x) => x.en === en));
});

test("every key exists in both languages, none is empty, and the placeholders match", () => {
  for (const table of TABLES) {
    const en = Object.keys(table.en);
    const es = Object.keys(table.es);
    assert.deepEqual([...es].sort(), [...en].sort(), `${table.name}: the same keys`);
    for (const key of en) {
      const english = table.en[key];
      const spanish = table.es[key];
      assert.equal(typeof spanish, "string", `${table.name}:${key}`);
      assert.ok(spanish.trim().length > 0 || english.trim().length === 0, `${table.name}:${key} is empty in Spanish`);
      assert.deepEqual(placeholders(spanish), placeholders(english), `${table.name}:${key} placeholders`);
    }
  }
});

test("the Spanish tables are Spanish: no key keeps its English text, but for names and symbols", () => {
  // Same in both: symbols, abbreviations, brand, mode and minion type names.
  const same = new Set(["—", "?", "#", ".", "Solo", "Triples", "APM", "Top 4", "Top 2", "top 4", "top 2", "Top 4 / Top 2", "{n}+", "{hero}", "{hero} · {place}", "{attack}/{health}", "{board}, {mode}, {period}", "{name}: {text}", "{name} ×{count}", "{n}×", "Windows · beta", "{version} (beta)", "{size} MB", "Get-FileHash \"{file}\"", "GitHub", "Elemental", "Naga"]);
  for (const table of TABLES) {
    for (const key of Object.keys(table.en)) {
      const english = table.en[key];
      if (same.has(english)) continue;
      assert.notEqual(table.es[key], english, `${table.name}:${key} is still English`);
    }
  }
});

test("the dash for an unknown value is the same in both languages (kept as a constant)", () => {
  assert.equal(STRINGS["value.none"], "—");
  setLang("es");
  assert.equal(t("value.none"), "—");
});

test("t follows the page's language and refuses a missing placeholder in both", () => {
  assert.equal(t("backing.some", { known: 28, total: 31, unit: "games" }), "in 28 of 31 games");
  setLang("es");
  assert.equal(t("backing.some", { known: 28, total: 31, unit: "partidas" }), "en 28 de 31 partidas");
  assert.throws(() => t("backing.some", { known: 28 }), /total/);
});

test("setLang accepts only our languages", () => {
  assert.equal(setLang("es"), "es");
  assert.equal(setLang("fr"), "en");
  assert.equal(setLang(undefined), "en");
  assert.equal(isLang("es"), true);
  assert.equal(isLang("ES"), false);
});

test("addresses in each language", () => {
  assert.equal(pathIn("/games/?id=1#top", "es"), "/es/games/?id=1#top");
  assert.equal(pathIn("/es/games/?id=1", "en"), "/games/?id=1");
  assert.equal(pathIn("/", "es"), "/es/");
  assert.equal(pathIn("/es/", "en"), "/");
  assert.equal(pathIn("/es/me/", "es"), "/es/me/", "never twice");
  // Not ours to change: other sites, fragments, assets.
  for (const keep of ["https://example.com/", "//example.com/x", "#top", "/_astro/a.js", "/fonts/x.woff2", "/placeholders/hero-1.svg", "/lang.js"]) {
    assert.equal(pathIn(keep, "es"), keep, keep);
  }
  assert.equal(basePath("/es"), "/");
  assert.equal(basePath("/essay/"), "/essay/", "only the /es/ prefix");
  assert.equal(pathLang("/es/terms/"), "es");
  assert.equal(pathLang("/essay/"), "en");
  setLang("es");
  assert.equal(href("/me/"), "/es/me/");
  assert.equal(gamePath("u", "s"), "/es/game/?u=u&id=s");
  assert.equal(publicProfilePath("abc"), "/es/profile/?id=abc");
  assert.deepEqual(langPaths(), [
    { params: { lang: undefined }, props: { lang: "en" } },
    { params: { lang: "es" }, props: { lang: "es" } },
  ]);
});

test("numbers per language: decimals, thousands, compact, percent, signed changes", () => {
  assert.equal(average(2.5), "2.50");
  assert.equal(compact(12_900), "12.9K");
  assert.equal(share(1, 2), "50%");
  setLang("es");
  assert.equal(average(2.5), "2,50");
  assert.equal(compact(9_999), "9999");
  assert.equal(compact(12_900), "12,9 mil");
  assert.equal(share(1, 2), "50 %");
  assert.equal(trend(-0.3, "lower")?.text, "−0,30");
  assert.equal(formatValue({ format: "percent" } as never, 12.5), "12,5 %");
  assert.equal(rate(57), "57 %");
  assert.equal(average(null), "—", "unknown is a dash in both");
});

test("places, plurals and backing lines per language", () => {
  assert.deepEqual([1, 2, 3, 4].map(ordinal), ["1st", "2nd", "3rd", "4th"]);
  setLang("es");
  assert.deepEqual([1, 2, 3, 8].map(ordinal), ["1.º", "2.º", "3.º", "8.º"]);
  assert.equal(pluralForm(1), "one");
  assert.equal(pluralForm(0), "other");
  assert.equal(pluralForm(1_000_000), "other", "Spanish 'many' counts as other");
  assert.equal(backing(1, 1), "en 1 partida");
  assert.equal(backing(28, 31), "en 28 de 31 partidas");
  assert.equal(backing(31, 31), "en 31 de 31 partidas");
  assert.equal(backing(0, 2, "turns"), "sin datos en 2 turnos");
});

test("dates per language, in UTC", () => {
  assert.equal(dateText("2026-10-01"), "Oct 1, 2026");
  assert.equal(refreshedText("2026-10-11T14:20:00Z"), "11 Oct 2026, 14:20 UTC");
  setLang("es");
  assert.equal(dateText("2026-10-01"), "1 oct 2026");
  assert.equal(refreshedText("2026-10-11T14:20:00Z"), "11 oct 2026, 14:20 UTC");
});

test("account messages and the delete phrase follow the language", () => {
  assert.equal(checkCredentials("x", "y"), "Enter a valid email address.");
  setLang("es");
  assert.notEqual(checkCredentials("x", "y"), "Enter a valid email address.");
  assert.match(checkCredentials("a@b.co", "short") ?? "", /10/);
  assert.equal(signUpOutcome(null).kind, "check-email");
  assert.equal(confirmPhrase(), "borrar mi cuenta");
  assert.equal(isConfirmed("borrar mi cuenta"), true);
  assert.equal(isConfirmed("delete my account"), false, "the phrase shown on the page is the one asked for");
});

// ---------- Pages drawn in Spanish ----------

// Words only English text has; ids (TB_…, SAMPLE_…) and numbers are not words.
const ENGLISH_WORDS = /\b(?:the|and|your|you|with|games?|not|this|from|place|heroes?|loading|show|turns?|won|lost|gold|board|unknown)\b/i;

function textOf(node: VNode | string): string {
  if (typeof node === "string") return node;
  const labels = node.attrs.filter(([k]) => k === "aria-label" || k === "title" || k === "alt").map(([, v]) => ` ${v} `);
  if (node.attrs.some(([k, v]) => k === "translate" && v === "no")) return " ";
  return labels.join("") + " " + node.children.map(textOf).join(" ");
}

function assertSpanish(nodes: VNode[], what: string): void {
  const text = nodes.map(textOf).join(" ").replace(/\b[A-Z][A-Za-z0-9]*_[A-Za-z0-9_]+\b/g, " ");
  const hit = ENGLISH_WORDS.exec(text);
  assert.equal(hit, null, `${what}: …${hit ? text.slice(Math.max(0, hit.index - 80), hit.index + 80).replace(/\s+/g, " ") : ""}…`);
}

test("the game viewer draws in Spanish, Solo and Duos", () => {
  setLang("es");
  for (const report of [soloReport(), duosReport()]) {
    const result = readRecord(record(report));
    assert.ok(result.ok);
    const nodes = renderGame(shapeGame(result.record), "2026-10-09");
    assertSpanish(nodes, "game");
    assert.match(toHtml(nodes[0]), /Héroe|Puesto/);
  }
});

test("the dashboard draws in Spanish", () => {
  setLang("es");
  const data = {
    modes: {
      solo: { overview: overview(overviewRow()), places: [3, 1, null, 5], total: 31, lastGames: [gameRow(gameRaw())] },
      duos: { overview: overview(overviewRow({ mode: "duos", avg_place: 2 })), places: [2, 1], total: 31, lastGames: [] },
    },
    heroes: [heroRow(heroRaw())],
    records: [recordRow(recordRaw())].filter((r): r is RecordRow => r !== null),
  };
  const ID = "0f8fad5b-d9cb-469f-a165-70867728950e";
  const panels = dashboardPanels(data, { gamesHref: (m: string) => `/es/games/?mode=${m}`, owner: true, userId: ID });
  assertSpanish(panels, "dashboard");
  for (const [, link] of toHtml(panels[0]).matchAll(/href="(\/[^"]*)"/g)) assert.match(link, /^\/es\//, link);
});

// ---------- The language script (public/lang.js) ----------

const SCRIPT = readFileSync(join(ROOT, "public", "lang.js"), "utf8");

function runScript(path: string, opts: { saved?: string | null; languages?: string[]; search?: string; hash?: string; storage?: "blocked"; keys?: string[] } = {}) {
  const store = new Map<string, string>();
  if (opts.saved) store.set("tl-lang", opts.saved);
  for (const k of opts.keys ?? []) store.set(k, "{}");
  const htmlAttrs: Record<string, string> = {};
  const replaced: string[] = [];
  const listeners: Record<string, (e: unknown) => void> = {};
  const localStorage =
    opts.storage === "blocked"
      ? {
          getItem: () => {
            throw new Error("blocked");
          },
          setItem: () => {
            throw new Error("blocked");
          },
          get length(): number {
            throw new Error("blocked");
          },
        }
      : {
          getItem: (k: string) => store.get(k) ?? null,
          setItem: (k: string, v: string) => void store.set(k, v),
          removeItem: (k: string) => void store.delete(k),
          get length() {
            return store.size;
          },
          key: (i: number) => [...store.keys()][i] ?? null,
        };
  const window = {
    localStorage,
    navigator: { languages: opts.languages ?? ["en-GB"], language: (opts.languages ?? ["en-GB"])[0] },
    location: { pathname: path, search: opts.search ?? "", hash: opts.hash ?? "", href: `https://tavernledger.net${path}${opts.search ?? ""}`, replace: (to: string) => replaced.push(to) },
    document: {
      addEventListener: (name: string, fn: (e: unknown) => void) => (listeners[name] = fn),
      querySelectorAll: () => [],
      documentElement: { setAttribute: (n: string, v: string) => void (htmlAttrs[n] = v) },
    },
  };
  runInNewContext(SCRIPT, { window, URL });
  return { replaced, store, listeners, htmlAttrs };
}

test("lang.js: a first visit follows the browser's language, keeping query and hash", () => {
  assert.deepEqual(runScript("/games/", { languages: ["es-ES", "en"], search: "?id=1", hash: "#x" }).replaced, ["/es/games/?id=1#x"]);
  assert.deepEqual(runScript("/", { languages: ["fr-FR", "es"] }).replaced, ["/es/"], "the first of ours in the list");
  assert.deepEqual(runScript("/", { languages: ["en-US", "es"] }).replaced, []);
  assert.deepEqual(runScript("/", { languages: ["fr-FR"] }).replaced, [], "neither: the page as linked");
  assert.deepEqual(runScript("/es/me/", { languages: ["en-GB"] }).replaced, [], "a Spanish link is followed unless English was chosen");
});

test("lang.js: a saved choice wins over the browser and over the link", () => {
  assert.deepEqual(runScript("/me/", { saved: "en", languages: ["es-ES"] }).replaced, []);
  assert.deepEqual(runScript("/es/me/", { saved: "en", languages: ["es-ES"] }).replaced, ["/me/"]);
  assert.deepEqual(runScript("/account/", { saved: "es", search: "?code=abc" }).replaced, ["/es/account/?code=abc"], "an email link's code goes along");
  assert.deepEqual(runScript("/es/", { saved: "es" }).replaced, []);
  assert.deepEqual(runScript("/me/", { saved: "de", languages: ["en"] }).replaced, [], "an unknown saved value is ignored");
  assert.deepEqual(runScript("/es//evil.example/x", { saved: "en" }).replaced, ["/evil.example/x"], "never a protocol-relative address");
});

test("lang.js: the switch saves the choice; blocked storage still lets the page work", () => {
  const { store, listeners } = runScript("/games/", { search: "?mode=duos" });
  const link = {
    attrs: { href: "/es/games/", "data-lang-switch": "es" } as Record<string, string>,
    getAttribute(n: string) {
      return this.attrs[n];
    },
    setAttribute(n: string, v: string) {
      this.attrs[n] = v;
    },
  };
  listeners.click({ target: { closest: () => link } });
  assert.equal(store.get("tl-lang"), "es");
  assert.equal(link.attrs.href, "/es/games/?mode=duos");
  // Storage blocked: no guess from the browser, as a switch back could not be remembered.
  assert.deepEqual(runScript("/", { storage: "blocked", languages: ["es"] }).replaced, []);
  assert.deepEqual(runScript("/es/games/", { storage: "blocked", languages: ["es"] }).replaced, []);
  listeners.auxclick({ target: { closest: () => ({ ...link, attrs: { href: "/games/", "data-lang-switch": "en" } }) } });
  assert.equal(store.get("tl-lang"), "en", "a middle click saves the choice too");
});

test("lang.js: the menu knows whether a session is stored, from the key's name only (T-513)", () => {
  assert.equal(runScript("/me/", { keys: ["sb-abcdef123-auth-token"] }).htmlAttrs["data-auth"], "in");
  assert.equal(runScript("/me/").htmlAttrs["data-auth"], "out");
  assert.equal(runScript("/me/", { keys: ["sb-abcdef123-auth-token-code-verifier"] }).htmlAttrs["data-auth"], "out", "a sign-in in progress is not a session");
  assert.equal(runScript("/me/", { storage: "blocked" }).htmlAttrs["data-auth"], undefined, "unknown: both links stay");
  assert.equal(runScript("/me/", { languages: ["es"], keys: ["sb-x-auth-token"] }).htmlAttrs["data-auth"], undefined, "a page that redirects marks nothing");
});

// Prepares the simulator (T-606, D-090) before each build, in
// src/lib/sim/generated/ (git-ignored):
// - engine.ts: the committed engine (src/lib/sim/engine.wasm) as a base64
//   string the worker carries, after checking it is the one its stamp names;
// - cards.en.json and cards.es.json: the card data;
// - names.en.json and names.es.json: the names of the other Battlegrounds
//   cards (spells, trinkets...), for the game page (T-559);
// and, from the same lists, in src/lib/minions/generated/ (git-ignored):
// - pool.en.json and pool.es.json: the tavern's pool, for the minion pages
//   (T-578, D-134).
//
//   node tools/prepare-sim.ts                fetch the cards from HearthstoneJSON (the Pages build)
//   node tools/prepare-sim.ts --if-missing   the same, but keep card files already there (dev)
//   SIM_CARDS_DIR=<dir> node tools/prepare-sim.ts   read <dir>/enUS.json and esES.json (CI, tests)
//
// With accounts closed (no Supabase settings) no page has a script, so it
// writes empty lists and fetches nothing (so no minion page is built). A fetch that fails stops the
// build: the site never ships a simulator without its card data, and the
// deployed site stays as it was.
import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { loadEnv } from "vite";
import { readConfig } from "../src/lib/config.ts";
import type { PoolFile } from "../src/lib/minions/poolFile.ts";
import { extractPool } from "./minionPool.ts";
import { type CardNameFile, extractCards, extractNames, type SimCardFile } from "./simCards.ts";
import { sha256, STAMP_PATH, type Stamp, WASM_PATH } from "./simStamp.ts";

const WEB = join(import.meta.dirname, "..");
const OUT = join(WEB, "src", "lib", "sim", "generated");
const POOL_OUT = join(WEB, "src", "lib", "minions", "generated");
const ORIGIN = "https://api.hearthstonejson.com";
const BASE = `${ORIGIN}/v1/latest`;
/** cards.json is about 25 MB uncompressed; far above, it is not the card data. */
const MAX_BYTES = 64 * 1024 * 1024;
const TIMEOUT_MS = 120_000;
const USER_AGENT = "TavernLedger-site-build (+https://tavernledger.net)";

const out = (lang: string) => join(OUT, `cards.${lang}.json`);
const namesOut = (lang: string) => join(OUT, `names.${lang}.json`);
const poolOut = (lang: string) => join(POOL_OUT, `pool.${lang}.json`);

async function fetchList(locale: string): Promise<{ list: unknown; build: string }> {
  const signal = AbortSignal.timeout(TIMEOUT_MS);
  const get = (url: string) => fetch(url, { headers: { "User-Agent": USER_AGENT }, signal, redirect: "manual" });
  let final = new URL(`${BASE}/${locale}/cards.json`);
  let res = await get(final.href);
  // "latest" may redirect once to a build number: followed only on HearthstoneJSON's own host.
  const location = res.headers.get("location");
  if (res.status >= 300 && res.status < 400 && location) {
    final = new URL(location, final);
    if (final.origin !== ORIGIN) throw new Error(`HearthstoneJSON ${locale}: redirected off its host`);
    res = await get(final.href);
  }
  if (!res.ok || !res.body) throw new Error(`HearthstoneJSON ${locale}: HTTP ${res.status}`);
  const reader = res.body.getReader();
  const chunks: Uint8Array[] = [];
  let size = 0;
  for (;;) {
    const { done, value } = await reader.read();
    if (done) break;
    size += value.length;
    if (size > MAX_BYTES) {
      await reader.cancel();
      throw new Error(`HearthstoneJSON ${locale}: larger than ${MAX_BYTES} bytes`);
    }
    chunks.push(value);
  }
  const text = new TextDecoder("utf-8", { fatal: true }).decode(Buffer.concat(chunks));
  // The build number when HearthstoneJSON redirects to it, else the day it was fetched.
  const number = /\/v1\/(\d{1,9})\//.exec(final.pathname)?.[1];
  const build = number ? `build ${number}` : `latest, ${new Date().toISOString().slice(0, 10)}`;
  return { list: JSON.parse(text), build };
}

function write(lang: string, file: SimCardFile, names: CardNameFile, pool: PoolFile): void {
  writeFileSync(out(lang), `${JSON.stringify(file)}\n`);
  writeFileSync(namesOut(lang), `${JSON.stringify(names)}\n`);
  writeFileSync(poolOut(lang), `${JSON.stringify(pool)}\n`);
  const named = Object.keys(names.names).length;
  const kept = `${file.cards.length} minions, ${named} other cards, ${pool.minions.length} in the pool`;
  process.stdout.write(`prepare-sim: ${kept} (${lang}, ${file.source})\n`);
}

/** The engine as a module the worker imports: no fetch, so the CSP needs no connection. */
function writeEngine(): void {
  const bytes = readFileSync(WASM_PATH);
  const stamp = JSON.parse(readFileSync(STAMP_PATH, "utf8")) as Stamp;
  if (sha256(bytes) !== stamp.wasm) throw new Error("engine.wasm is not the one engine.stamp.json names: run tools/build-sim-wasm.ts");
  const text = [
    "// Written by tools/prepare-sim.ts from src/lib/sim/engine.wasm. Do not edit.",
    `export const ENGINE_SHA256 = "${stamp.wasm}";`,
    `export const ENGINE_BASE64 = "${bytes.toString("base64")}";`,
    "",
  ].join("\n");
  writeFileSync(join(OUT, "engine.ts"), text);
}

async function main(): Promise<void> {
  mkdirSync(OUT, { recursive: true });
  mkdirSync(POOL_OUT, { recursive: true });
  writeEngine();
  const ready = (l: string) => [out(l), namesOut(l), poolOut(l)].every((f) => existsSync(f));
  if (process.argv.includes("--if-missing") && ["en", "es"].every(ready)) return;
  const config = readConfig(loadEnv("production", WEB, "PUBLIC_"));
  if (!config) {
    const none = "none (accounts closed)";
    for (const lang of ["en", "es"]) {
      write(lang, { version: 1, source: none, cards: [] }, { version: 1, source: none, names: {}, trinkets: {} }, { version: 1, source: none, minions: [] });
    }
    return;
  }
  const dir = process.env.SIM_CARDS_DIR;
  const read = async (locale: string) =>
    dir ? { list: JSON.parse(readFileSync(join(WEB, dir, `${locale}.json`), "utf8")) as unknown, build: "fixture" } : fetchList(locale);
  const en = await read("enUS");
  const es = await read("esES");
  const source = (build: string) => (build === "fixture" ? build : `HearthstoneJSON (${build})`);
  for (const [lang, local] of [["en", en], ["es", es]] as const) {
    const from = source(local.build);
    // The pool's warnings (a new minion type, a minion with no tier) are the same in both languages: told once.
    const warn = lang === "en" ? (message: string) => process.stderr.write(`prepare-sim: warning: ${message}\n`) : undefined;
    write(lang, extractCards(en.list, local.list, from), extractNames(en.list, local.list, from), extractPool(en.list, local.list, from, warn));
  }
}

main().catch((error: unknown) => {
  process.stderr.write(`prepare-sim: ${error instanceof Error ? error.message : String(error)}\n`);
  process.exit(1);
});

// Prepares the simulator (T-606, D-090) before each build, in
// src/lib/sim/generated/ (git-ignored):
// - engine.ts: the committed engine (src/lib/sim/engine.wasm) as a base64
//   string the worker carries, after checking it is the one its stamp names;
// - cards.en.json and cards.es.json: the card data.
//
//   node tools/prepare-sim.ts                fetch the cards from HearthstoneJSON (the Pages build)
//   node tools/prepare-sim.ts --if-missing   the same, but keep card files already there (dev)
//   SIM_CARDS_DIR=<dir> node tools/prepare-sim.ts   read <dir>/enUS.json and esES.json (CI, tests)
//
// With accounts closed (no Supabase settings) no page has a script, so it
// writes empty lists and fetches nothing. A fetch that fails stops the
// build: the site never ships a simulator without its card data, and the
// deployed site stays as it was.
import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { loadEnv } from "vite";
import { readConfig } from "../src/lib/config.ts";
import { extractCards, type SimCardFile } from "./simCards.ts";
import { sha256, STAMP_PATH, type Stamp, WASM_PATH } from "./simStamp.ts";

const WEB = join(import.meta.dirname, "..");
const OUT = join(WEB, "src", "lib", "sim", "generated");
const BASE = "https://api.hearthstonejson.com/v1/latest";
/** cards.json is about 25 MB uncompressed; far above, it is not the card data. */
const MAX_BYTES = 64 * 1024 * 1024;
const TIMEOUT_MS = 120_000;
const USER_AGENT = "TavernLedger-site-build (+https://tavernledger.net)";

const out = (lang: string) => join(OUT, `cards.${lang}.json`);

async function fetchList(locale: string): Promise<{ list: unknown; build: string }> {
  const url = `${BASE}/${locale}/cards.json`;
  const res = await fetch(url, { headers: { "User-Agent": USER_AGENT }, signal: AbortSignal.timeout(TIMEOUT_MS), redirect: "follow" });
  if (!res.ok || !res.body) throw new Error(`HearthstoneJSON ${locale}: HTTP ${res.status}`);
  // Only HearthstoneJSON's own host, after its redirect to the build number.
  const final = new URL(res.url);
  if (final.origin !== "https://api.hearthstonejson.com") throw new Error(`HearthstoneJSON ${locale}: redirected off its host`);
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

function write(lang: string, file: SimCardFile): void {
  writeFileSync(out(lang), `${JSON.stringify(file)}\n`);
  process.stdout.write(`prepare-sim: ${file.cards.length} minions (${lang}, ${file.source})\n`);
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
  writeEngine();
  if (process.argv.includes("--if-missing") && existsSync(out("en")) && existsSync(out("es"))) return;
  const config = readConfig(loadEnv("production", WEB, "PUBLIC_"));
  if (!config) {
    for (const lang of ["en", "es"]) write(lang, { version: 1, source: "none (accounts closed)", cards: [] });
    return;
  }
  const dir = process.env.SIM_CARDS_DIR;
  const read = async (locale: string) =>
    dir ? { list: JSON.parse(readFileSync(join(WEB, dir, `${locale}.json`), "utf8")) as unknown, build: "fixture" } : fetchList(locale);
  const en = await read("enUS");
  const es = await read("esES");
  const source = (build: string) => (build === "fixture" ? build : `HearthstoneJSON (${build})`);
  write("en", extractCards(en.list, en.list, source(en.build)));
  write("es", extractCards(en.list, es.list, source(es.build)));
}

main().catch((error: unknown) => {
  process.stderr.write(`prepare-sim: ${error instanceof Error ? error.message : String(error)}\n`);
  process.exit(1);
});

// Builds the website's combat engine (T-606, D-090): crates/sim-wasm for
// wasm32-unknown-unknown, copied to src/lib/sim/engine.wasm with its stamp.
// Run by hand after an engine change (needs `rustup target add
// wasm32-unknown-unknown`), then commit both files:
//
//   node tools/build-sim-wasm.ts
//
// The Pages build has no Rust toolchain, so the .wasm is committed; the
// stamp test (test/unit/simEngine.test.ts) fails when the sources change
// without a rebuild. Paths on this PC are remapped and the file is checked
// for them, so no folder or user name ends up in the published file.
import { execFileSync } from "node:child_process";
import { copyFileSync, readFileSync, writeFileSync } from "node:fs";
import { homedir } from "node:os";
import { join, sep } from "node:path";
import { REPO, sha256, sourcesHash, STAMP_PATH, type Stamp, WASM_PATH } from "./simStamp.ts";

const TARGET = "wasm32-unknown-unknown";
const PROFILE = "sim-wasm";
/** CARGO_ENCODED_RUSTFLAGS separates flags with this, so a path with spaces stays whole. */
const FLAG_SEPARATOR = String.fromCharCode(0x1f);

function run(cmd: string, args: string[], env: NodeJS.ProcessEnv = process.env): string {
  return execFileSync(cmd, args, { cwd: REPO, env, encoding: "utf8", stdio: ["ignore", "pipe", "inherit"] });
}

const cargoHome = process.env.CARGO_HOME ?? join(homedir(), ".cargo");
const rustupHome = process.env.RUSTUP_HOME ?? join(homedir(), ".rustup");
const remap = [
  `--remap-path-prefix=${REPO}${sep}=`,
  `--remap-path-prefix=${cargoHome}=/cargo`,
  `--remap-path-prefix=${rustupHome}=/rustup`,
].join(FLAG_SEPARATOR);
run("cargo", ["build", "--locked", "-p", "sim-wasm", "--lib", "--profile", PROFILE, "--target", TARGET], {
  ...process.env,
  CARGO_ENCODED_RUSTFLAGS: remap,
});

const built = join(REPO, "target", TARGET, PROFILE, "sim_wasm.wasm");
const bytes = readFileSync(built);
// No path or name from this PC in a file anyone can download.
const text = bytes.toString("latin1");
for (const leak of [homedir(), REPO, cargoHome, rustupHome, homedir().split(sep).pop() ?? ""]) {
  if (leak.length >= 3 && text.toLowerCase().includes(leak.toLowerCase())) {
    throw new Error(`the built engine still holds a local path or name (${leak.length} characters); not copied`);
  }
}
copyFileSync(built, WASM_PATH);

// The registry crates in the build, by name (the local ones are hashed as files).
const tree = run("cargo", ["tree", "--locked", "-p", "sim-wasm", "-e", "normal", "--target", TARGET, "--prefix", "none"]);
const crates = [...new Set(tree.split("\n").filter((l) => l.trim() && !l.includes("(")).map((l) => l.split(" ")[0]))].sort();
const stamp: Stamp = {
  wasm: sha256(bytes),
  sources: sourcesHash(crates),
  crates,
  rustc: run("rustc", ["--version"]).trim(),
};
writeFileSync(STAMP_PATH, `${JSON.stringify(stamp, null, 2)}\n`);
process.stdout.write(`engine.wasm: ${bytes.length} bytes, sha256 ${stamp.wasm}\n`);

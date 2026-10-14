// The stamp that ties the committed engine (src/lib/sim/engine.wasm) to the
// Rust sources it was built from (T-606, D-090). The build tool writes it;
// a unit test recomputes it, so a change to the engine without a rebuild
// fails the tests (CI and the Pages build) instead of shipping odds that
// differ from the app's. Line endings are normalised, so a Windows checkout
// and a Linux one give the same hash.
import { createHash } from "node:crypto";
import { readdirSync, readFileSync } from "node:fs";
import { join } from "node:path";

export const REPO = join(import.meta.dirname, "..", "..");
export const WASM_PATH = join(REPO, "web", "src", "lib", "sim", "engine.wasm");
export const STAMP_PATH = join(REPO, "web", "src", "lib", "sim", "engine.stamp.json");

export interface Stamp {
  /** sha256 of engine.wasm. */
  wasm: string;
  /** sha256 over the sources that go into it (sourceFiles + lock blocks). */
  sources: string;
  /** The registry crates in the build, as Cargo.lock names them. */
  crates: string[];
  /** The compiler that built it (for the record; not checked). */
  rustc: string;
}

/** Files of a crate's src/ that are compiled into the engine: not tests, not the harness. */
function engineFiles(dir: string): string[] {
  const keep = (name: string) =>
    name.endsWith(".rs") && !name.endsWith("_tests.rs") && name !== "tests.rs" && !name.startsWith("harness") &&
    name !== "combat_validate.rs";
  return readdirSync(join(REPO, dir)).filter(keep).sort().map((name) => `${dir}/${name}`);
}

/** Every repo file the engine is built from, relative to the repo, sorted. */
export function sourceFiles(): string[] {
  return [
    "Cargo.toml",
    "crates/combat-sim/Cargo.toml",
    ...engineFiles("crates/combat-sim/src"),
    "crates/sim-wasm/Cargo.toml",
    ...engineFiles("crates/sim-wasm/src"),
  ];
}

/** The [[package]] blocks of Cargo.lock for the named crates, in order. */
export function lockBlocks(lock: string, crates: readonly string[]): string {
  const blocks = lock.replaceAll("\r\n", "\n").split("\n[[package]]\n");
  return crates
    .map((name) => {
      const block = blocks.find((b) => b.startsWith(`name = "${name}"\n`));
      if (!block) throw new Error(`${name} is not in Cargo.lock`);
      return block.trim();
    })
    .join("\n\n");
}

export const sha256 = (data: string | Uint8Array): string => createHash("sha256").update(data).digest("hex");

/** The hash of the engine's sources as they are in the working tree. */
export function sourcesHash(crates: readonly string[]): string {
  const hash = createHash("sha256");
  for (const file of sourceFiles()) {
    hash.update(`${file}\n`);
    hash.update(readFileSync(join(REPO, file), "utf8").replaceAll("\r\n", "\n"));
    hash.update("\n\0\n");
  }
  hash.update(lockBlocks(readFileSync(join(REPO, "Cargo.lock"), "utf8"), crates));
  return hash.digest("hex");
}

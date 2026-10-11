// The committed engine (src/lib/sim/engine.wasm, T-606): it is the one the
// Rust sources build (stamp), and it gives the Rust answers, which the
// Rust parity test checks against the app's own odds code
// (crates/sim-wasm/tests/parity.rs). So the same boards give the same odds
// in the browser as in the app.
import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { base64Bytes, loadEngine } from "../../src/lib/sim/engine.ts";
import { readAnswer } from "../../src/lib/sim/answer.ts";
import { REPO, sha256, sourcesHash, STAMP_PATH, type Stamp, WASM_PATH } from "../../tools/simStamp.ts";

const PARITY = join(REPO, "crates", "sim-wasm", "tests", "data", "parity.json");
const stamp = JSON.parse(readFileSync(STAMP_PATH, "utf8")) as Stamp;
const wasm = readFileSync(WASM_PATH);

test("the committed engine is the one its stamp names", () => {
  assert.equal(sha256(wasm), stamp.wasm, "engine.wasm changed without `node tools/build-sim-wasm.ts`");
});

test("the engine's sources have not changed since it was built", () => {
  assert.equal(
    sourcesHash(stamp.crates),
    stamp.sources,
    "crates/combat-sim or crates/sim-wasm changed: run `node tools/build-sim-wasm.ts` (web/) and commit engine.wasm and engine.stamp.json",
  );
});

test("the built engine gives the Rust answers on every parity request", async () => {
  const engine = await loadEngine(wasm);
  const { cases } = JSON.parse(readFileSync(PARITY, "utf8")) as { cases: { name: string; request: unknown; answer: unknown }[] };
  assert.ok(cases.length >= 5);
  const kinds = new Set<string>();
  for (const c of cases) {
    const answer = engine.run(JSON.stringify(c.request));
    assert.deepEqual(JSON.parse(answer), c.answer, c.name);
    const read = readAnswer(answer);
    assert.notEqual(read.kind, "unreadable", c.name);
    kinds.add(read.kind);
  }
  assert.deepEqual([...kinds].sort(), ["no_card_data", "not_modelled", "odds"]);
});

test("the engine refuses bad requests with an answer, and keeps working after", async () => {
  const engine = await loadEngine(wasm);
  for (const bad of ["", "{", "null", JSON.stringify({ player: [], opponent: [] }), "x".repeat(70_000)]) {
    const a = readAnswer(engine.run(bad));
    assert.equal(a.kind, "bad_input", bad.slice(0, 20));
  }
  const { cases } = JSON.parse(readFileSync(PARITY, "utf8")) as { cases: { request: unknown; answer: unknown }[] };
  assert.deepEqual(JSON.parse(engine.run(JSON.stringify(cases[0].request))), cases[0].answer);
});

test("the engine imports nothing from the page", async () => {
  const module = await WebAssembly.compile(wasm);
  assert.deepEqual(WebAssembly.Module.imports(module), []);
  const names = WebAssembly.Module.exports(module).map((e) => e.name).sort();
  assert.deepEqual(names.filter((n) => n.startsWith("sim_")), ["sim_alloc", "sim_answer", "sim_run"]);
});

test("no path or user name from the build machine is in the engine", () => {
  const text = wasm.toString("latin1");
  assert.doesNotMatch(text, /[A-Za-z]:\\Users\\|\/home\/|\/Users\//);
});

test("base64 gives the engine's bytes back; anything else is refused", async () => {
  assert.deepEqual([...base64Bytes("AGFzbQ==")], [0, 97, 115, 109]);
  assert.deepEqual(base64Bytes(wasm.toString("base64")), new Uint8Array(wasm));
  assert.throws(() => base64Bytes("https://example.com/x.wasm"));
  assert.throws(() => base64Bytes("AGFzbQ="));
});

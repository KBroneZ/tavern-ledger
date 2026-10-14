// The engine module's calls (crates/sim-wasm lib.rs): write the request
// into its memory, run it, read the answer. The same code runs in the
// page's Web Worker and in the tests (Node), so the test of the built .wasm
// tests what the browser runs.

interface Exports {
  memory: WebAssembly.Memory;
  sim_alloc(len: number): number;
  sim_run(ptr: number, len: number): number;
  sim_answer(): number;
}

export interface Engine {
  /** One request (JSON text) in, the answer (JSON text) out. */
  run(request: string): string;
}

function exportsOf(instance: WebAssembly.Instance): Exports {
  const e = instance.exports as Partial<Exports>;
  const ok = e.memory instanceof WebAssembly.Memory && typeof e.sim_alloc === "function" && typeof e.sim_run === "function" &&
    typeof e.sim_answer === "function";
  if (!ok) throw new Error("not the simulator's engine");
  return e as Exports;
}

/** The engine from the module's bytes. It imports nothing: no host function is reachable from it. */
export async function loadEngine(bytes: BufferSource): Promise<Engine> {
  const { instance } = await WebAssembly.instantiate(bytes, {});
  const e = exportsOf(instance);
  const encoder = new TextEncoder();
  const decoder = new TextDecoder("utf-8", { fatal: true });
  return {
    run(request: string): string {
      const input = encoder.encode(request);
      const ptr = e.sim_alloc(input.length);
      // Views are made after each call: the memory may grow and detach old ones.
      new Uint8Array(e.memory.buffer, ptr, input.length).set(input);
      const len = e.sim_run(ptr, input.length);
      const at = e.sim_answer();
      return decoder.decode(new Uint8Array(e.memory.buffer, at, len).slice());
    },
  };
}

/** The bytes of a base64 string (how the worker's bundle carries the module). */
export function base64Bytes(text: string): Uint8Array<ArrayBuffer> {
  if (!/^[A-Za-z0-9+/]*={0,2}$/.test(text) || text.length % 4 !== 0) throw new Error("not base64");
  const raw = atob(text);
  const out = new Uint8Array(raw.length);
  for (let i = 0; i < raw.length; i++) out[i] = raw.charCodeAt(i);
  return out;
}

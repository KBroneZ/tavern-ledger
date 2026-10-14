/// <reference lib="webworker" />
// The simulator's Web Worker (T-606): runs the engine off the page's main
// thread, so the page never freezes while 10,000 combats play. The engine's
// bytes are inside this script (a base64 string tools/prepare-sim.ts writes
// from the committed engine.wasm), so the worker fetches nothing: the CSP
// needs no connection, only 'wasm-unsafe-eval' on this script alone (D-090).
//
// Messages: { id, request } in; { id, answer } or { id, failed: true } out.
import { ENGINE_BASE64 } from "./generated/engine.ts";
import { base64Bytes, type Engine, loadEngine } from "./engine.ts";

declare const self: DedicatedWorkerGlobalScope;

let engine: Promise<Engine> | null = null;

self.addEventListener("message", (event: MessageEvent<unknown>) => {
  const data = event.data as { id?: unknown; request?: unknown } | null;
  if (!data || typeof data.id !== "number" || typeof data.request !== "string") return;
  const { id, request } = data as { id: number; request: string };
  engine ??= loadEngine(base64Bytes(ENGINE_BASE64));
  engine
    .then((e) => self.postMessage({ id, answer: e.run(request) }))
    .catch(() => {
      // A trap leaves the module's memory in an unknown state: start afresh next time.
      engine = null;
      self.postMessage({ id, failed: true });
    });
});

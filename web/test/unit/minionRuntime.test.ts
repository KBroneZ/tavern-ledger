// The page side of the minions' drawing (art/core.ts): when it loads the
// shared drawing, what it does when the texture set is unreachable or the
// drawing cannot start, and that the page is warned at most once. The real
// wiring (dynamic import, images, the page's events) is in art/runtime.ts and
// is checked in a real browser.
import { test } from "node:test";
import assert from "node:assert/strict";
import { drawnAttrs } from "../../src/lib/art/drawn.ts";
import { type ArtHandle, type CoreEnv, createMinionCore } from "../../src/lib/art/core.ts";

function host(card = "BG25_011") {
  const attrs: Record<string, string | undefined> = { ...drawnAttrs({ cardId: card, atk: 1, health: 2, golden: false, keywords: [] }) };
  const slot: { made: unknown[] } = { made: [] };
  return {
    slot,
    getAttribute: (n: string) => attrs[n] ?? null,
    querySelector: (s: string) =>
      s === ".tlm-slot" ? { querySelector: () => slot.made.at(-1) ?? null, append: (n: unknown) => slot.made.push(n) } : null,
  } as unknown as HTMLElement & { slot: { made: unknown[] } };
}

function setup(over: Partial<CoreEnv> = {}) {
  const log = { warns: [] as string[], loads: 0, destroyed: 0, renders: 0, updates: 0 };
  const handle: ArtHandle = {
    ready: Promise.resolve(),
    render: () => (log.renders++, {} as HTMLElement),
    update: () => void log.updates++,
    destroy: () => void log.destroyed++,
  };
  // The page's minions are whatever was last given to paint().
  let page: HTMLElement[] = [];
  const env: CoreEnv = {
    loadDrawing: async () => (log.loads++, handle),
    warn: (m) => log.warns.push(m),
    hosts: (root) => (page = root as unknown as HTMLElement[]),
    allHosts: () => page,
    onLeave: () => {},
    ...over,
  };
  return { core: createMinionCore(env), log, env };
}
const settle = () => new Promise((r) => setImmediate(r));

test("minions are drawn once the drawing is ready, never before", async () => {
  const { core, log } = setup();
  const h = host();
  core.paint([h] as never);
  assert.equal(log.renders, 0, "loading: the page's own look stays");
  await settle();
  assert.equal(log.renders, 1, "drawn as soon as the drawing is ready, without a second call");
  core.paint([h] as never);
  assert.equal(log.updates, 1, "painting again updates it");
  assert.equal(log.loads, 1, "the drawing is loaded once");
});

test("nothing loads for a page with no minion to draw", async () => {
  const { core, log } = setup();
  core.paint([] as never);
  await settle();
  assert.equal(log.loads, 0);
});

test("an unreachable texture set leaves the page's own look and warns once", async () => {
  const { core, log } = setup({
    loadDrawing: async () => {
      throw new Error("the frame did not load");
    },
  });
  const h = host();
  core.paint([h] as never);
  await settle();
  core.paint([h] as never);
  core.paint([host()] as never);
  await settle();
  assert.equal(h.slot.made.length, 0, "nothing drawn into the slot");
  assert.equal(log.warns.length, 1, "one warning, however many times the page paints");
  assert.match(log.warns[0], /without them/);
});

test("a drawing that cannot start (no usable layout) is the same fallback", async () => {
  const { core, log } = setup({ loadDrawing: async () => null });
  core.paint([host()] as never);
  await settle();
  assert.equal(log.renders, 0);
  assert.equal(log.warns.length, 1);
});

test("the drawing's own trouble reports (a texture, WebGL) add no second warning", async () => {
  const { core, log } = setup();
  core.paint([host()] as never);
  await settle();
  core.warn("texture frame_golden could not be loaded: that layer is left out");
  core.warn("WebGL lost its context");
  assert.equal(log.warns.length, 1, "the first one only");
});

test("leaving the page lets the drawing go, and a later paint starts it again", async () => {
  const { core, log } = setup();
  core.paint([host()] as never);
  await settle();
  core.release();
  assert.equal(log.destroyed, 1);
  core.paint([host()] as never);
  await settle();
  assert.equal(log.loads, 2);
});

test("a load that finishes after the page was left is dropped, not drawn", async () => {
  let finish: (h: ArtHandle | null) => void = () => {};
  const { core, log } = setup({ loadDrawing: () => new Promise((r) => (finish = r)) });
  const h = host();
  core.paint([h] as never);
  core.release();
  finish({ ready: Promise.resolve(), render: () => ({}) as HTMLElement, update: () => {}, destroy: () => void log.destroyed++ });
  await settle();
  assert.equal(log.destroyed, 1, "the late drawing is let go");
  assert.equal(h.slot.made.length, 0);
});

// afterPaint (T-554): a page script waits for the first paint before it asks
// for the network. Browser globals are stubbed; each test puts them back.
import { afterEach, test } from "node:test";
import assert from "node:assert/strict";
import { afterPaint } from "../../src/lib/view/afterPaint.ts";

const g = globalThis as Record<string, unknown>;
const saved = { document: g.document, PerformanceObserver: g.PerformanceObserver, requestAnimationFrame: g.requestAnimationFrame };

type PaintCallback = (list: { getEntriesByName: (name: string) => unknown[] }, observer: { disconnect: () => void }) => void;

/** A PerformanceObserver that records its callback so the test can report a paint when it likes. */
function stubObserver(): { report: (names: string[]) => void; disconnected: () => boolean } {
  let callback: PaintCallback | null = null;
  let disconnected = false;
  g.PerformanceObserver = class {
    constructor(cb: PaintCallback) {
      callback = cb;
    }
    observe(): void {}
    disconnect(): void {
      disconnected = true;
    }
  };
  return {
    report: (names) =>
      callback?.({ getEntriesByName: (name) => names.filter((n) => n === name) }, { disconnect: () => (disconnected = true) }),
    disconnected: () => disconnected,
  };
}

afterEach(() => {
  g.document = saved.document;
  g.PerformanceObserver = saved.PerformanceObserver;
  g.requestAnimationFrame = saved.requestAnimationFrame;
});

test("a tab in the background starts at once: it paints nothing", async () => {
  g.document = { visibilityState: "hidden" };
  await afterPaint();
});

test("it waits for the first-contentful-paint report, not for any other paint", async () => {
  g.document = { visibilityState: "visible" };
  const observer = stubObserver();
  let done = false;
  const waiting = afterPaint().then(() => (done = true));
  observer.report(["first-paint"]);
  await new Promise((r) => setTimeout(r, 20));
  assert.equal(done, false, "first-paint alone is not the first contentful paint");
  observer.report(["first-paint", "first-contentful-paint"]);
  await waiting;
  assert.equal(done, true);
  assert.equal(observer.disconnected(), true);
});

test("a browser without paint timing falls back to the next animation frame", async () => {
  g.document = { visibilityState: "visible" };
  g.PerformanceObserver = class {
    constructor() {
      throw new TypeError("unsupported");
    }
  };
  let frames = 0;
  g.requestAnimationFrame = (cb: () => void) => {
    frames += 1;
    setTimeout(cb, 0);
    return frames;
  };
  await afterPaint();
  assert.equal(frames, 1);
});

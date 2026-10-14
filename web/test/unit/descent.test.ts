// The home page's backdrop (T-530, D-105): how far the tall painting has
// moved for a given scroll, slower while the five-step scene is pinned.
import { test } from "node:test";
import assert from "node:assert/strict";
import { backdropOffset, PIN_SPEED, type Descent } from "../../src/lib/home/descent.ts";

const page = (over: Partial<Descent> = {}): Descent => ({
  maxScroll: 4000,
  travel: 2000,
  pinStart: null,
  pinEnd: null,
  ...over,
});

test("without a pinned scene the painting moves evenly from the door to the cellar", () => {
  const d = page();
  assert.equal(backdropOffset(0, d), 0);
  assert.equal(backdropOffset(1000, d), 500);
  assert.equal(backdropOffset(2000, d), 1000);
  assert.equal(backdropOffset(4000, d), 2000);
});

test("the painting ends at the cellar exactly when the page ends, pinned scene or not", () => {
  assert.equal(backdropOffset(4000, page({ pinStart: 800, pinEnd: 2150 })), 2000);
  assert.equal(backdropOffset(0, page({ pinStart: 800, pinEnd: 2150 })), 0);
});

test("during the pinned scene it moves at a tenth of its speed outside", () => {
  assert.equal(PIN_SPEED, 0.1);
  const d = page({ pinStart: 1000, pinEnd: 2000 });
  // Outside: 2000 px of painting over 3000 px of normal scroll plus 1000 px at a tenth.
  const outside = 2000 / (3000 + 100);
  const before = backdropOffset(500, d);
  assert.ok(Math.abs(before - 500 * outside) < 1e-9);
  const inScene = backdropOffset(1600, d) - backdropOffset(1500, d);
  const afterScene = backdropOffset(2600, d) - backdropOffset(2500, d);
  assert.ok(Math.abs(inScene - 100 * outside * PIN_SPEED) < 1e-9);
  assert.ok(Math.abs(afterScene - 100 * outside) < 1e-9);
});

test("it never runs backwards", () => {
  const d = page({ pinStart: 900, pinEnd: 2400 });
  let last = -1;
  for (let s = 0; s <= 4000; s += 37) {
    const y = backdropOffset(s, d);
    assert.ok(y >= last, `at ${s}`);
    last = y;
  }
});

test("a scroll past either end (overscroll bounce) stays at the door or the cellar", () => {
  const d = page({ pinStart: 1000, pinEnd: 2000 });
  assert.equal(backdropOffset(-120, d), 0);
  assert.equal(backdropOffset(4300, d), 2000);
});

test("a scene cut off by the end of the page only slows the part that is scrolled", () => {
  const d = page({ pinStart: 3500, pinEnd: 5000 });
  assert.equal(backdropOffset(4000, d), 2000);
  const outside = 2000 / (3500 + 500 * PIN_SPEED);
  assert.ok(Math.abs(backdropOffset(3500, d) - 3500 * outside) < 1e-9);
});

test("a page that does not scroll, or a painting no taller than the screen, stays still", () => {
  assert.equal(backdropOffset(0, page({ maxScroll: 0 })), 0);
  assert.equal(backdropOffset(300, page({ travel: 0 })), 0);
  assert.equal(backdropOffset(300, page({ travel: -50 })), 0);
});

test("broken measurements (NaN, a scene that ends before it starts) fall back to even motion or stillness", () => {
  assert.equal(backdropOffset(1000, page({ pinStart: 2000, pinEnd: 1000 })), 500);
  assert.equal(backdropOffset(1000, page({ pinStart: Number.NaN, pinEnd: 2000 })), 500);
  assert.equal(backdropOffset(Number.NaN, page()), 0);
  assert.equal(backdropOffset(1000, page({ maxScroll: Number.NaN })), 0);
});

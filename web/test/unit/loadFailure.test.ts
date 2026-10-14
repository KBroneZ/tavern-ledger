// The error a page shows when its own script could not be loaded (T-554): a
// visible block with a reload button. A tiny fake stands in for the DOM.
import { afterEach, test } from "node:test";
import assert from "node:assert/strict";
import { showLoadFailure } from "../../src/lib/view/loadFailure.ts";

class FakeNode {
  className = "";
  textContent = "";
  type = "";
  children: FakeNode[] = [];
  listeners = new Map<string, () => void>();
  tag: string;
  constructor(tag: string) {
    this.tag = tag;
  }
  append(...nodes: FakeNode[]) {
    this.children.push(...nodes);
  }
  replaceChildren(...nodes: FakeNode[]) {
    this.children = nodes;
  }
  addEventListener(name: string, fn: () => void) {
    this.listeners.set(name, fn);
  }
}

const g = globalThis as Record<string, unknown>;
const saved = { document: g.document, location: g.location };
afterEach(() => {
  g.document = saved.document;
  g.location = saved.location;
});

function install(): { reloads: () => number } {
  let reloads = 0;
  g.document = { createElement: (tag: string) => new FakeNode(tag) };
  g.location = { reload: () => void (reloads += 1) };
  return { reloads: () => reloads };
}

test("the block has a title, the text, and a button that reloads the page", () => {
  const page = install();
  const target = new FakeNode("div");
  target.children = [new FakeNode("skeleton")];
  showLoadFailure(target as unknown as HTMLElement, "Could not load", "Check your connection.", "Try again");
  assert.equal(target.children.length, 1, "it replaces what was there");
  const block = target.children[0];
  assert.equal(block.className, "state state-error");
  assert.deepEqual(block.children.map((c) => [c.tag, c.textContent]), [
    ["p", "Could not load"],
    ["p", "Check your connection."],
    ["button", "Try again"],
  ]);
  assert.equal(block.children[0].className, "state-title");
  assert.equal(block.children[2].type, "button");
  // The small stone, as every retry button on the site (buttons.css).
  assert.equal(block.children[2].className, "tb tb-small");
  block.children[2].listeners.get("click")?.();
  assert.equal(page.reloads(), 1);
});

test("an empty body adds no empty paragraph", () => {
  install();
  const target = new FakeNode("div");
  showLoadFailure(target as unknown as HTMLElement, "The simulator stopped unexpectedly.", "", "Try again");
  assert.deepEqual(target.children[0].children.map((c) => c.tag), ["p", "button"]);
});

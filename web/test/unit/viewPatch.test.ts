// patch() redraws a region in place (T-633): the result is the same tree a
// fresh mount would give, but elements that stay keep their identity, so a
// list the player has open or is typing in (the browser's type-ahead) is not
// closed or restarted by a redraw.
import { test } from "node:test";
import assert from "node:assert/strict";
import { h, toDom, type VNode } from "../../src/lib/view/node.ts";
import { patch } from "../../src/lib/view/patch.ts";
import { FakeDocument, FakeElement, FakeText, serialize } from "../helpers/fakeDom.ts";

function mounted(...nodes: (VNode | string)[]): { doc: FakeDocument; root: FakeElement } {
  const doc = new FakeDocument();
  const root = doc.createElement("div");
  for (const n of nodes) root.appendChild(toDom(n, doc.as()) as unknown as FakeElement);
  return { doc, root };
}

function patched(root: FakeElement, doc: FakeDocument, nodes: (VNode | string)[], live: FakeElement | null = null): void {
  (root as unknown as { ownerDocument: unknown }).ownerDocument = doc;
  patch(root as unknown as Element, nodes, live as unknown as Element | null);
}

function fresh(nodes: (VNode | string)[]): string {
  return serialize(mounted(...nodes).root);
}

const list = (selected: string, extra: string[] = []) =>
  h(
    "select",
    { id: "kit", class: extra.length ? "many" : "few" },
    h("option", { value: "" }, "None"),
    ["a", "b", "c", ...extra].map((v) => h("option", { value: v, selected: v === selected }, `Card ${v}`)),
  );

test("patch gives the tree a fresh mount would give", () => {
  const before = [h("p", { class: "x", title: "old" }, "one"), h("ul", {}, h("li", {}, "a"), h("li", {}, "b")), "tail"];
  const after = [h("p", { class: "y" }, "two"), h("ul", {}, h("li", {}, "a"), h("li", {}, "c"), h("li", {}, "d")), h("span", {}, "new")];
  const { doc, root } = mounted(...before);
  patched(root, doc, after);
  assert.equal(serialize(root), fresh(after));
});

test("patch drops what the new tree no longer has", () => {
  const { doc, root } = mounted(h("p", { a: "1", b: "2" }, "x"), h("p", {}, "y"), h("p", {}, "z"));
  patched(root, doc, [h("p", { b: "2" }, "x")]);
  assert.equal(serialize(root), fresh([h("p", { b: "2" }, "x")]));
  assert.equal(root.childNodes.length, 1);
});

test("elements and text that stay are the same objects", () => {
  const { doc, root } = mounted(h("div", { class: "a" }, h("button", {}, "go"), "text"));
  const div = root.childNodes[0] as FakeElement;
  const button = div.childNodes[0] as FakeElement;
  const text = div.childNodes[1] as FakeText;
  patched(root, doc, [h("div", { class: "b" }, h("button", {}, "stop"), "text 2")]);
  assert.equal(root.childNodes[0], div);
  assert.equal(div.childNodes[0], button);
  assert.equal(div.childNodes[1], text);
  assert.equal(text.nodeValue, "text 2");
  assert.equal(div.getAttribute("class"), "b");
});

test("a different tag at the same place is replaced", () => {
  const { doc, root } = mounted(h("p", {}, "x"));
  const old = root.childNodes[0];
  patched(root, doc, [h("h2", {}, "x")]);
  assert.notEqual(root.childNodes[0], old);
  assert.equal(serialize(root), fresh([h("h2", {}, "x")]));
});

test("the list in use is left alone: same element, same choice, same options", () => {
  const { doc, root } = mounted(h("div", {}, list("a")));
  const select = (root.childNodes[0] as FakeElement).childNodes[0] as FakeElement;
  const options = [...select.childNodes];
  select.selectedIndex = 2; // the player moved to "Card b" with the keyboard
  doc.activeElement = select;
  patched(root, doc, [h("div", { class: "redrawn" }, list("a"))], select);
  assert.equal((root.childNodes[0] as FakeElement).childNodes[0], select);
  assert.equal(select.selectedIndex, 2, "the player's choice stays");
  assert.deepEqual([...select.childNodes], options, "options stay the same objects");
  assert.equal((root.childNodes[0] as FakeElement).getAttribute("class"), "redrawn");
});

test("the list in use takes new options when they change, and the choice the page says", () => {
  const { doc, root } = mounted(h("div", {}, list("a")));
  const select = (root.childNodes[0] as FakeElement).childNodes[0] as FakeElement;
  doc.activeElement = select;
  patched(root, doc, [h("div", {}, list("c", ["d"]))], select);
  assert.equal(serialize(root), fresh([h("div", {}, list("c", ["d"]))]));
  assert.equal(select.selectedIndex, 3);
});

test("a list not in use shows the choice the page says", () => {
  const { doc, root } = mounted(h("div", {}, list("a")));
  const select = (root.childNodes[0] as FakeElement).childNodes[0] as FakeElement;
  select.selectedIndex = 2;
  patched(root, doc, [h("div", {}, list("c"))]);
  assert.equal(select.selectedIndex, 3);
  patched(root, doc, [h("div", {}, list(""))]);
  assert.equal(select.selectedIndex, 0, "no option marked: the first (None)");
});

test("a text box not in use takes the page's value; one in use keeps what was typed", () => {
  const box = (v: string) => h("input", { type: "search", value: v });
  const { doc, root } = mounted(box("old"));
  const input = root.childNodes[0] as FakeElement;
  patched(root, doc, [box("new")]);
  assert.equal(input.value, "new");
  input.type("typed");
  doc.activeElement = input;
  patched(root, doc, [box("other")], input);
  assert.equal(input.value, "typed");
});

test("nested elements are matched by place and tag", () => {
  const { doc, root } = mounted(h("div", {}, h("a", {}, "x")));
  patched(root, doc, [h("div", {}, h("a", {}, "y"))]);
  assert.equal(serialize(root), fresh([h("div", {}, h("a", {}, "y"))]));
});

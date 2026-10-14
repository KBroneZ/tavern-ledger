// The element tree the browser pages draw with (T-503, D-073): text stays
// text, no event handler can be set, and toDom builds real elements only
// through createElement, createElementNS and text nodes.
import { test } from "node:test";
import assert from "node:assert/strict";
import { cls, h, mount, s, toDom, type DomDocument } from "../../src/lib/view/node.ts";
import { toHtml } from "../helpers/html.ts";

test("h: attributes in order, true is bare, false, null and undefined are left out", () => {
  const node = h("details", { class: "fold", open: true, hidden: false, title: null, id: undefined, "data-size": 24 });
  assert.deepEqual(node.attrs, [["class", "fold"], ["open", ""], ["data-size", "24"]]);
});

test("h: children flatten one level and skip null, undefined and false; numbers become text", () => {
  const node = h("p", {}, "a", [h("b", {}, "x"), null], false, undefined, 3);
  assert.equal(toHtml(node), "<p>a<b>x</b>3</p>");
});

test("text is text: markup in data is escaped, never parsed", () => {
  assert.equal(toHtml(h("td", {}, "<img src=x onerror=alert(1)>")), "<td>&lt;img src=x onerror=alert(1)&gt;</td>");
  assert.equal(toHtml(h("a", { href: '/x" onclick="y' }, "go")), '<a href="/x&quot; onclick=&quot;y">go</a>');
});

test("no event handler attribute and no odd tag or attribute name can be made", () => {
  assert.throws(() => h("a", { onclick: "x" }), /attribute/);
  assert.throws(() => h("a", { OnMouseOver: "x" }), /attribute/);
  assert.throws(() => h("a", { "x y": "1" }), /attribute/);
  assert.throws(() => h("script>", {}), /tag/);
});

test("cls joins like Astro's class:list", () => {
  assert.equal(cls("tile", { "tile-hero": true, off: false }, null, ""), "tile tile-hero");
  assert.equal(cls({ more: false }), "");
});

/** A stand-in document that records what toDom asks for. */
function fakeDocument() {
  const calls: string[] = [];
  const element = (tag: string, ns: string | null) => {
    const el = {
      tag,
      ns,
      attrs: new Map<string, string>(),
      children: [] as unknown[],
      setAttribute(name: string, value: string) {
        this.attrs.set(name, value);
      },
      appendChild(child: unknown) {
        this.children.push(child);
        return child;
      },
    };
    return el;
  };
  const doc = {
    createElement(tag: string) {
      calls.push(`el ${tag}`);
      return element(tag, null);
    },
    createElementNS(ns: string, tag: string) {
      calls.push(`ns ${tag}`);
      return element(tag, ns);
    },
    createTextNode(text: string) {
      calls.push(`text ${text}`);
      return { text };
    },
  } as unknown as DomDocument;
  return { doc, calls };
}

test("toDom: HTML and SVG elements in their namespaces, text as text nodes", () => {
  const { doc, calls } = fakeDocument();
  const el = toDom(h("figure", { class: "chart" }, s("svg", { viewBox: "0 0 1 1" }, s("path", { d: "M0 0" })), "<b>"), doc) as unknown as {
    attrs: Map<string, string>;
    children: { ns?: string; text?: string }[];
  };
  assert.deepEqual(calls, ["el figure", "ns svg", "ns path", "text <b>"]);
  assert.equal(el.attrs.get("class"), "chart");
  assert.equal(el.children[0].ns, "http://www.w3.org/2000/svg");
  assert.equal(el.children[1].text, "<b>");
});

test("mount replaces the target's children with the trees", () => {
  const { doc } = fakeDocument();
  let replaced: unknown[] = [];
  const target = { ownerDocument: doc, replaceChildren: (...nodes: unknown[]) => (replaced = nodes) } as unknown as Element;
  mount(target, h("p", {}, "a"), "b");
  assert.equal(replaced.length, 2);
});

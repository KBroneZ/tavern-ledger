// A small stand-in for the DOM, for the tests of lib/view/patch.ts: elements
// with attributes, children, and the form state a patch must keep or set
// (value, selectedIndex). Only what patch.ts and toDom ask for.
import type { DomDocument } from "../../src/lib/view/node.ts";

const SVG_NS = "http://www.w3.org/2000/svg";

export class FakeNode {
  parentNode: FakeElement | null = null;
  readonly nodeType: number;
  nodeName: string;
  constructor(nodeType: number, nodeName: string) {
    this.nodeType = nodeType;
    this.nodeName = nodeName;
  }
}

export class FakeText extends FakeNode {
  nodeValue: string;
  constructor(nodeValue: string) {
    super(3, "#text");
    this.nodeValue = nodeValue;
  }
}

export class FakeElement extends FakeNode {
  readonly attrs = new Map<string, string>();
  readonly childNodes: FakeNode[] = [];
  selectedIndex = 0;
  private current = "";
  private dirty = false;
  /** As a browser: once the value is set (or typed), the markup's value no longer changes it. */
  get value(): string {
    return this.current;
  }
  set value(v: string) {
    this.current = v;
    this.dirty = true;
  }
  /** What a person typing does. */
  type(v: string): void {
    this.value = v;
  }
  readonly namespaceURI: string | null;
  constructor(tag: string, namespaceURI: string | null) {
    super(1, namespaceURI === SVG_NS ? tag : tag.toUpperCase());
    this.namespaceURI = namespaceURI;
  }
  get textContent(): string {
    return this.childNodes.map((c) => (c instanceof FakeText ? c.nodeValue : (c as FakeElement).textContent)).join("");
  }
  getAttributeNames(): string[] {
    return [...this.attrs.keys()];
  }
  getAttribute(name: string): string | null {
    return this.attrs.get(name) ?? null;
  }
  setAttribute(name: string, value: string): void {
    this.attrs.set(name, value);
    // As a browser: a fresh input or select takes its value from the markup.
    if (name === "value" && this.nodeName === "INPUT" && !this.dirty) this.current = value;
  }
  removeAttribute(name: string): void {
    this.attrs.delete(name);
  }
  appendChild<T extends FakeNode>(child: T): T {
    child.parentNode = this;
    this.childNodes.push(child);
    return child;
  }
  replaceChild<T extends FakeNode>(child: T, old: FakeNode): FakeNode {
    const at = this.childNodes.indexOf(old);
    if (at < 0) throw new Error("replaceChild: not a child");
    old.parentNode = null;
    child.parentNode = this;
    this.childNodes[at] = child;
    return old;
  }
  removeChild<T extends FakeNode>(child: T): T {
    const at = this.childNodes.indexOf(child);
    if (at < 0) throw new Error("removeChild: not a child");
    this.childNodes.splice(at, 1);
    child.parentNode = null;
    return child;
  }
}

export class FakeDocument {
  activeElement: FakeElement | null = null;
  createElement(tag: string): FakeElement {
    return new FakeElement(tag, null);
  }
  createElementNS(ns: string, tag: string): FakeElement {
    return new FakeElement(tag, ns);
  }
  createTextNode(text: string): FakeText {
    return new FakeText(text);
  }
  as(): DomDocument {
    return this as unknown as DomDocument;
  }
}

/** Markup-like text of a fake tree (attributes sorted), to compare two trees. */
export function serialize(node: FakeNode): string {
  if (node instanceof FakeText) return node.nodeValue;
  const el = node as FakeElement;
  const attrs = [...el.attrs.entries()]
    .sort(([a], [b]) => a.localeCompare(b))
    .map(([k, v]) => ` ${k}="${v}"`)
    .join("");
  return `<${el.nodeName.toLowerCase()}${attrs}>${el.childNodes.map(serialize).join("")}</${el.nodeName.toLowerCase()}>`;
}

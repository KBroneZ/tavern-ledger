// A tiny element tree for the data pages that load in the browser (T-503,
// D-073). The pieces in lib/view/pieces.ts describe the same markup as the
// Astro components of T-501; toDom() turns a tree into real elements with
// createElement and text nodes only, so data never goes through an HTML
// parser (test/unit/source.test.ts bans the string-to-HTML calls).

export type AttrValue = string | number | boolean | null | undefined;
export type Attrs = Record<string, AttrValue>;
export type Child = VNode | string | number | null | undefined | false;

export interface VNode {
  tag: string;
  svg: boolean;
  /** Attribute name → value; "" is a bare attribute (hidden, open, alt). */
  attrs: [string, string][];
  children: (VNode | string)[];
}

const TAG = /^[a-z][a-z0-9]*$/i;
const ATTR = /^[a-z][a-z0-9:-]*$/i;

function build(tag: string, svg: boolean, attrs: Attrs, children: (Child | Child[])[]): VNode {
  if (!TAG.test(tag)) throw new Error(`bad tag ${tag}`);
  const list: [string, string][] = [];
  for (const [name, value] of Object.entries(attrs)) {
    if (!ATTR.test(name) || /^on/i.test(name)) throw new Error(`bad attribute ${name}`);
    if (value === false || value === null || value === undefined) continue;
    list.push([name, value === true ? "" : String(value)]);
  }
  const kids: (VNode | string)[] = [];
  for (const child of children.flat()) {
    if (child === null || child === undefined || child === false) continue;
    kids.push(typeof child === "number" ? String(child) : child);
  }
  return { tag, svg, attrs: list, children: kids };
}

/** An HTML element. */
export function h(tag: string, attrs: Attrs = {}, ...children: (Child | Child[])[]): VNode {
  return build(tag, false, attrs, children);
}

/** An SVG element. */
export function s(tag: string, attrs: Attrs = {}, ...children: (Child | Child[])[]): VNode {
  return build(tag, true, attrs, children);
}

/** Class names joined like Astro's class:list: strings, and object keys whose value is true. */
export function cls(...parts: (string | false | null | undefined | Record<string, boolean | undefined>)[]): string {
  const names: string[] = [];
  for (const part of parts) {
    if (!part) continue;
    if (typeof part === "string") names.push(part);
    else for (const [name, on] of Object.entries(part)) if (on) names.push(name);
  }
  return names.join(" ");
}

const SVG_NS = "http://www.w3.org/2000/svg";

/** The DOM document's few calls this needs, so tests can pass a stand-in. */
export type DomDocument = Pick<Document, "createElement" | "createElementNS" | "createTextNode">;

export function toDom(node: VNode | string, doc: DomDocument = document): Node {
  if (typeof node === "string") return doc.createTextNode(node);
  const el = node.svg ? doc.createElementNS(SVG_NS, node.tag) : doc.createElement(node.tag);
  for (const [name, value] of node.attrs) el.setAttribute(name, value);
  for (const child of node.children) el.appendChild(toDom(child, doc));
  return el;
}

/** Puts the trees in place of the element's children. */
export function mount(target: Element, ...nodes: (VNode | string)[]): void {
  target.replaceChildren(...nodes.map((n) => toDom(n, target.ownerDocument)));
}

// Redraws a region in place (T-633). mount() throws the old elements away, so
// a list the player has open, or is typing in (the browser keeps the letters
// typed so far), is closed or restarted by every redraw. patch() turns the
// region into the same tree toDom() would build but keeps the elements that
// stay, the one in use (`live`) above all: its choice and options are left
// alone unless the page's own options changed.

import { toDom, type DomDocument, type VNode } from "./node.ts";

const ELEMENT = 1;
const TEXT = 3;
const SVG_NS = "http://www.w3.org/2000/svg";

/** Puts the trees in place of the element's children, reusing what already matches. */
export function patch(target: Element, nodes: (VNode | string)[], live: Element | null = null): void {
  syncChildren(target, nodes, target.ownerDocument, live);
}

function sameKind(have: Node, want: VNode | string): boolean {
  if (typeof want === "string") return have.nodeType === TEXT;
  if (have.nodeType !== ELEMENT) return false;
  const el = have as Element;
  const svg = el.namespaceURI === SVG_NS;
  return svg === want.svg && el.nodeName.toLowerCase() === want.tag.toLowerCase();
}

function syncChildren(parent: Node, wanted: (VNode | string)[], doc: DomDocument, live: Element | null): void {
  const have = Array.from(parent.childNodes);
  wanted.forEach((want, i) => {
    const current = have[i];
    if (!current) {
      parent.appendChild(toDom(want, doc));
    } else if (!sameKind(current, want)) {
      parent.replaceChild(toDom(want, doc), current);
    } else if (typeof want === "string") {
      if (current.nodeValue !== want) current.nodeValue = want;
    } else {
      morph(current as Element, want, doc, live);
    }
  });
  for (let i = have.length - 1; i >= wanted.length; i--) parent.removeChild(have[i]);
}

function morph(el: Element, want: VNode, doc: DomDocument, live: Element | null): void {
  const attrs = new Map(want.attrs);
  for (const name of el.getAttributeNames()) if (!attrs.has(name)) el.removeAttribute(name);
  for (const [name, value] of attrs) if (el.getAttribute(name) !== value) el.setAttribute(name, value);
  const inUse = el === live;
  const tag = want.tag.toLowerCase();
  if (tag === "select") {
    // The list in use keeps its options (a redrawn popup closes) unless they changed.
    const rewrite = !inUse || !sameOptions(el, want);
    if (rewrite) syncChildren(el, want.children, doc, live);
    if (rewrite) (el as HTMLSelectElement).selectedIndex = chosenIndex(want);
  } else {
    syncChildren(el, want.children, doc, live);
    // A text box takes the page's value, unless it is the one being typed in.
    if (tag === "input" && !inUse) (el as HTMLInputElement).value = attrs.get("value") ?? "";
  }
}

/** The option the tree marks as chosen, or the first. */
function chosenIndex(select: VNode): number {
  const marked = select.children.findIndex((c) => typeof c !== "string" && c.attrs.some(([name]) => name === "selected"));
  return Math.max(marked, 0);
}

/** True when the select already holds exactly these options, in this order. */
function sameOptions(select: Element, want: VNode): boolean {
  const have = Array.from(select.childNodes);
  if (have.length !== want.children.length) return false;
  return want.children.every((c, i) => {
    const node = have[i];
    if (typeof c === "string" || node.nodeType !== ELEMENT) return false;
    const option = node as Element;
    const value = c.attrs.find(([name]) => name === "value")?.[1] ?? null;
    return option.getAttribute("value") === value && (option.textContent ?? "") === c.children.filter((t) => typeof t === "string").join("");
  });
}

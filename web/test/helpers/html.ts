// Serializes a lib/view element tree the way Astro writes its markup, for
// the tests only (the site itself never builds HTML from strings): bare
// attributes for "", void elements without a closing tag, Astro's escaping.
import type { VNode } from "../../src/lib/view/node.ts";

const VOID = new Set(["img", "input", "br", "hr", "meta", "link"]);

function escape(text: string): string {
  return text.replaceAll("&", "&amp;").replaceAll("<", "&lt;").replaceAll(">", "&gt;").replaceAll('"', "&quot;").replaceAll("'", "&#39;");
}

export function toHtml(node: VNode | string): string {
  if (typeof node === "string") return escape(node);
  const attrs = node.attrs.map(([name, value]) => (value === "" ? ` ${name}` : ` ${name}="${escape(value)}"`)).join("");
  if (!node.svg && VOID.has(node.tag)) return `<${node.tag}${attrs}>`;
  return `<${node.tag}${attrs}>${node.children.map(toHtml).join("")}</${node.tag}>`;
}

/** Visible text only, for "no name anywhere" checks. */
export function textOf(node: VNode | string): string {
  return typeof node === "string" ? node : node.children.map(textOf).join(" ");
}

/** Every element in the tree, depth first. */
export function elements(node: VNode | string): VNode[] {
  return typeof node === "string" ? [] : [node, ...node.children.flatMap(elements)];
}

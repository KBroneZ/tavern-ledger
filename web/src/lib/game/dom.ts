// Small DOM builders for the game viewer (T-504): the T-501 pieces drawn in
// the browser with the same classes as their Astro versions. Text always goes
// in as text nodes (never HTML), so nothing from the record can become markup.
import { ordinal, placeGroup, type Mode, type Result } from "../data/format.ts";
import { FALLBACK_ATTR, pictureFor, type PictureKind } from "../data/pictures.ts";
import { t } from "../data/strings.ts";

type Child = Node | string | null | undefined | false;
type Attrs = Record<string, string | number | boolean | null | undefined>;

export function el<K extends keyof HTMLElementTagNameMap>(tag: K, attrs: Attrs = {}, ...children: Child[]): HTMLElementTagNameMap[K] {
  const node = document.createElement(tag);
  for (const [name, value] of Object.entries(attrs)) {
    if (value === null || value === undefined || value === false) continue;
    node.setAttribute(name, value === true ? "" : String(value));
  }
  append(node, children);
  return node;
}

export function append(node: Node, children: Child[]): void {
  for (const child of children) {
    if (child === null || child === undefined || child === false) continue;
    node.appendChild(typeof child === "string" ? document.createTextNode(child) : child);
  }
}

const SVG = "http://www.w3.org/2000/svg";

export function svg(tag: string, attrs: Attrs = {}, ...children: Child[]): SVGElement {
  const node = document.createElementNS(SVG, tag);
  for (const [name, value] of Object.entries(attrs)) {
    if (value !== null && value !== undefined && value !== false) node.setAttribute(name, String(value));
  }
  append(node, children);
  return node;
}

export function hidden(text: string): HTMLElement {
  return el("span", { class: "visually-hidden" }, text);
}

/** A hero portrait or card picture, cropped in a circle; a failed one falls back to a placeholder. */
export function picture(kind: PictureKind, id: string | null, name: string, size: 24 | 32 | 40 | 56, decorative = false): HTMLElement {
  let pic: ReturnType<typeof pictureFor> | null = null;
  try {
    pic = id ? pictureFor(kind, id) : null;
  } catch {
    pic = null;
  }
  if (!pic) {
    return el("span", {
      class: `pic pic-${kind} pic-missing`,
      "data-size": size,
      role: decorative ? null : "img",
      "aria-label": decorative ? null : `${name}: ${t("picture.missing")}`,
    });
  }
  return el(
    "span",
    { class: `pic pic-${kind}`, "data-size": size },
    el("img", {
      src: pic.src,
      [FALLBACK_ATTR]: pic.fallback,
      alt: decorative ? "" : name,
      width: size,
      height: size,
      loading: "lazy",
      decoding: "async",
      referrerpolicy: "no-referrer",
    }),
  );
}

export function placeBadge(place: number | null, mode: Mode, large = false): HTMLElement {
  const group = placeGroup(place, mode);
  const label = group === "unknown" ? t("place.unknown") : t("place.label", { place: ordinal(place as number) });
  return el(
    "span",
    { class: `place place-${group}${large ? " place-lg" : ""}` },
    el("span", { "aria-hidden": "true" }, group === "unknown" ? "?" : String(place)),
    hidden(label),
  );
}

export function sourceMark(kind: "inferred" | "unknown"): HTMLElement {
  return el(
    "span",
    { class: `mark-src mark-${kind}`, title: t(kind === "inferred" ? "source.inferred.tip" : "source.unknown.tip") },
    t(kind === "inferred" ? "source.inferred" : "source.unknown"),
  );
}

export function resultChip(result: Result, inferred: boolean): HTMLElement {
  return el("span", {}, el("span", { class: `res-${result}` }, t(`result.${result}`)), inferred ? " " : null, inferred ? sourceMark("inferred") : null);
}

/** A native fold: Enter and Space open it, screen readers say its state. */
export function fold(summary: string, body: Child[], opts: { hint?: string; context?: string; compact?: boolean; cls?: string } = {}): HTMLElement {
  return el(
    "details",
    { class: ["fold", opts.compact ? "fold-compact" : "", opts.cls ?? ""].filter(Boolean).join(" ") },
    el("summary", {}, el("span", {}, summary, opts.context ? hidden(`: ${opts.context}`) : null), opts.hint ? el("span", { class: "fold-hint" }, opts.hint) : null),
    el("div", { class: "fold-body" }, ...body),
  );
}

/** A stat tile: label, value (a dash reads as "unknown"), the line of what backs it. */
export function tile(label: string, value: string, opts: { backing?: string; source?: "inferred" | "unknown"; hero?: boolean; cls?: string } = {}): HTMLElement {
  const unknown = value === t("value.none");
  return el(
    "div",
    { class: ["tile", opts.hero ? "tile-hero" : "", opts.cls ?? ""].filter(Boolean).join(" ") },
    el("p", { class: "tile-label" }, label),
    el(
      "p",
      { class: `tile-value${unknown ? " is-unknown" : ""}` },
      unknown ? el("span", { "aria-hidden": "true" }, value) : el("span", {}, value),
      unknown ? hidden(t("result.unknown")) : null,
      opts.source ? " " : null,
      opts.source ? sourceMark(opts.source) : null,
    ),
    opts.backing ? el("p", { class: "tile-back" }, opts.backing) : null,
  );
}

export function section(id: string, title: string, children: Child[], advanced = false): HTMLElement {
  return el(
    "section",
    { class: `data-section${advanced ? " adv" : ""}`, "aria-labelledby": `${id}-title` },
    el("h2", { id: `${id}-title` }, title),
    ...children,
  );
}

export function note(text: string): HTMLElement {
  return el("p", { class: "data-note" }, text);
}

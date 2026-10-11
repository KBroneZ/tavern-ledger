// The T-501 pieces as element trees, for pages that draw live data in the
// browser (T-503, D-073). Each function gives the same markup as its Astro
// component in components/data/ (test/build checks it against the built
// /style/ page), so the browser pages look exactly like the design system.
import {
  barScale,
  columnPath,
  ordinal,
  placeDistribution,
  placeGroup,
  share,
  sparkline,
  topN,
  type Mode,
  type Trend,
} from "../data/format.ts";
import { FALLBACK_ATTR, type Picture, pictureFor, type PictureKind } from "../data/pictures.ts";
import { t } from "../data/strings.ts";
import { heroLabel } from "../heroes/names.ts";
import { cls, h, s, type VNode } from "./node.ts";

// The same in every language (test/unit/i18n.test.ts), so it can be a constant.
const DASH = "—";

export function sourceMark(kind: "inferred" | "unknown"): VNode {
  return h("span", { class: `mark-src mark-${kind}` }, kind === "inferred" ? t("source.inferred") : t("source.unknown"));
}

export function placeBadge(place: number | null, mode: Mode, size: "md" | "lg" = "md"): VNode {
  const group = placeGroup(place, mode);
  const label = group === "unknown" ? t("place.unknown") : t("place.label", { place: ordinal(place as number) });
  return h(
    "span",
    { class: cls("place", `place-${group}`, { "place-lg": size === "lg" }) },
    h("span", { "aria-hidden": "true" }, group === "unknown" ? "?" : String(place)),
    h("span", { class: "visually-hidden" }, label),
  );
}

/** A dash that screen readers read as "Unknown" (the tiles' rule). */
export function unknownValue(): VNode[] {
  return [h("span", { "aria-hidden": "true" }, DASH), h("span", { class: "visually-hidden" }, t("result.unknown"))];
}

export interface FoldProps {
  summary: string;
  context?: string;
  hint?: string;
  open?: boolean;
  compact?: boolean;
}

export function fold(props: FoldProps, ...body: VNode[]): VNode {
  const { summary, context, hint, open = false, compact = false } = props;
  return h(
    "details",
    { class: cls("fold", { "fold-compact": compact }), open },
    h(
      "summary",
      {},
      h("span", {}, summary, context ? h("span", { class: "visually-hidden" }, `: ${context}`) : null),
      hint ? h("span", { class: "fold-hint" }, hint) : null,
    ),
    h("div", { class: "fold-body" }, ...body),
  );
}

export interface StateProps {
  kind: "empty" | "unknown" | "loading" | "error";
  title?: string;
  body?: string;
  lines?: number;
}

const STATE_TEXT = {
  empty: ["state.empty", "state.empty.body"],
  unknown: ["state.unknown", "state.unknown.body"],
  error: ["state.error", "state.error.body"],
} as const;

export function stateBlock(props: StateProps, ...extra: VNode[]): VNode {
  const { kind, lines = 3 } = props;
  if (kind === "loading") {
    return h(
      "div",
      { class: "state state-loading", role: "status" },
      h("span", { class: "visually-hidden" }, props.title ?? t("state.loading")),
      h("div", { class: "skeleton", "aria-hidden": "true" }, Array.from({ length: lines }, () => h("span"))),
    );
  }
  const heading = props.title ?? t(STATE_TEXT[kind][0]);
  const text = props.body ?? t(STATE_TEXT[kind][1]);
  return h("div", { class: `state state-${kind}` }, h("p", { class: "state-title" }, heading), text ? h("p", {}, text) : null, ...extra);
}

export interface SparkProps {
  values: (number | null)[];
  min: number;
  max: number;
  invert?: boolean;
}

export function sparkTile({ values, min, max, invert = false }: SparkProps): VNode {
  const line = sparkline(values, { width: 120 - 4 - 6, height: 28, min, max, invert });
  const last = line.points.at(-1)?.index === values.length - 1 ? line.points.at(-1) : undefined;
  return s(
    "svg",
    { class: "tile-spark", viewBox: "0 0 120 28", width: 120, height: 28, "aria-hidden": "true", focusable: "false" },
    s(
      "g",
      { transform: "translate(4 0)" },
      s("path", { class: "spark-line", d: line.path }),
      last ? s("circle", { class: "spark-end", cx: last.x, cy: last.y, r: "4" }) : null,
    ),
  );
}

export interface TileProps {
  label: string;
  value: string;
  backing?: string;
  trend?: Trend | null;
  spark?: SparkProps;
  source?: "inferred" | "unknown";
  hero?: boolean;
}

export function statTile(props: TileProps): VNode {
  const { label, value, backing, trend = null, spark, source, hero = false } = props;
  const unknown = value === DASH;
  const arrow = !trend || trend.sign === 0 ? "▬" : trend.sign < 0 ? "▼" : "▲";
  return h(
    "div",
    { class: cls("tile", { "tile-hero": hero }) },
    h("p", { class: "tile-label" }, label),
    h(
      "p",
      { class: cls("tile-value", { "is-unknown": unknown }) },
      unknown ? unknownValue() : h("span", {}, value),
      source ? sourceMark(source) : null,
    ),
    trend
      ? h(
          "p",
          { class: cls("tile-trend", trend.direction) },
          h("span", { "aria-hidden": "true" }, arrow),
          h("span", {}, trend.text),
          h("span", { class: "visually-hidden" }, t(`trend.${trend.direction}`)),
        )
      : null,
    backing ? h("p", { class: "tile-back" }, backing) : null,
    spark ? sparkTile(spark) : null,
  );
}

export function picture(kind: PictureKind, id: string | null, name: string, size: 24 | 32 | 40 | 56 = 32, decorative = false): VNode {
  let pic: Picture | null = null;
  try {
    pic = id ? pictureFor(kind, id) : null;
  } catch {
    pic = null; // not shaped like a card id: the plain circle, never a broken address
  }
  if (!pic) {
    return h("span", {
      class: `pic pic-${kind} pic-missing`,
      "data-size": size,
      role: decorative ? undefined : "img",
      "aria-label": decorative ? undefined : `${name}: ${t("picture.missing")}`,
    });
  }
  return h(
    "span",
    { class: `pic pic-${kind}`, "data-size": size },
    // A picture that fails falls back to its local placeholder (watchPictureFallback, D-074).
    h("img", { src: pic.src, [FALLBACK_ATTR]: pic.fallback, alt: decorative ? "" : name, width: size, height: size, loading: "lazy", decoding: "async" }),
  );
}

function dataTable(head: VNode, rows: VNode[], caption?: string): VNode {
  return h("table", { class: "dtable" }, caption ? h("caption", {}, caption) : null, head, h("tbody", {}, rows));
}

function headRow(labels: [string, boolean][] | Column[]): VNode {
  const columns: Column[] = labels.map((l) => (Array.isArray(l) ? { label: l[0], numeric: l[1] } : l));
  return h(
    "thead",
    {},
    h("tr", {}, columns.map((c) => h("th", { scope: "col", class: cls({ num: !!c.numeric, adv: !!c.advanced }) || undefined }, c.label))),
  );
}

export type Cell =
  | string
  | number
  | { place: number | null; mode: Mode }
  /**
   * With href, the hero's name links there (the heroes page's link to that
   * hero's games). unknownId: an id the name table does not know, shown in
   * Advanced only (T-510).
   */
  | { hero: string | null; name: string; href?: string; unknownId?: string | null }
  /** Anything else (a value with a mark); the client pages only. */
  | { node: VNode | VNode[] };

/** The portrait size in hero lists (T-510): picture first, big enough to tell heroes apart. */
export const HERO_PIC = 56;

/** A hero's table cell: its name in the page's language, never the bare card id (T-510). */
export function heroCell(id: string | null, href?: string): Cell {
  const label = heroLabel(id, t("hero.unknown"), t("hero.unknownId"));
  return { hero: id, name: label.name, href, unknownId: label.unknownId };
}

export interface Column {
  label: string;
  numeric?: boolean;
  /** Shown in Advanced only (the .adv rule). */
  advanced?: boolean;
  /** Text cells of this column are the row's header (th scope="row"). */
  rowHeader?: boolean;
}

function tableCell(cell: Cell, column: Column | undefined): VNode {
  if (typeof cell === "object" && "hero" in cell) {
    const name = cell.href ? h("a", { href: cell.href }, cell.name) : h("span", {}, cell.name);
    const id = cell.unknownId ? h("span", { class: "adv cell-note hero-id" }, cell.unknownId) : null;
    return h(
      "th",
      { scope: "row", class: cls({ adv: !!column?.advanced }) || undefined },
      h("span", { class: "cell-hero" }, picture("hero", cell.hero, cell.name, HERO_PIC, true), h("span", { class: "hero-name" }, name, id)),
    );
  }
  const klass = cls({ num: !!column?.numeric, adv: !!column?.advanced }) || undefined;
  if (typeof cell === "object" && "place" in cell) return h("td", { class: klass }, placeBadge(cell.place, cell.mode));
  if (typeof cell === "object") return h("td", { class: klass }, cell.node);
  if (column?.rowHeader) return h("th", { scope: "row", class: klass }, String(cell));
  return h("td", { class: klass }, String(cell));
}

export interface TopTableProps {
  caption: string;
  columns: Column[];
  rows: Cell[][];
  top?: number;
  label?: string;
}

export function topTable({ caption, columns, rows, top = 5, label = caption }: TopTableProps): VNode {
  const { shown, rest } = topN(rows, top);
  const all = [...shown.map((r) => ({ cells: r, more: false })), ...rest.map((r) => ({ cells: r, more: true }))];
  return h(
    "div",
    { class: "topn" },
    h(
      "div",
      { class: "scroll-x", role: "region", "aria-label": label, tabindex: "0" },
      dataTable(
        headRow(columns),
        all.map(({ cells, more }) => h("tr", { class: cls({ more }) || undefined }, cells.map((c, i) => tableCell(c, columns[i])))),
        caption,
      ),
    ),
    rest.length > 0
      ? h(
          "details",
          { class: "topn-toggle" },
          h(
            "summary",
            {},
            h("span", { class: "when-closed" }, t("table.showAll", { count: rows.length })),
            h("span", { class: "visually-hidden" }, `: ${caption}`),
            h("span", { class: "when-open" }, t("table.showTop", { count: top })),
          ),
        )
      : null,
  );
}

function chartHead(id: string, title: string, desc: string): VNode {
  return h(
    "figcaption",
    {},
    h("p", { class: "chart-title", id: `${id}-title` }, title),
    h("p", { class: "chart-desc", id: `${id}-desc` }, desc),
  );
}

export function placeChart(places: (number | null)[], mode: Mode, id: string): VNode {
  if (places.length === 0) return stateBlock({ kind: "empty", title: t("chart.places.title") });
  const { bars, unknown, total } = placeDistribution(places, mode);
  const [SLOT, BAR, TOP, BASE, HEIGHT] = [mode === "solo" ? 44 : 64, 24, 18, 104, 124];
  const width = bars.length * SLOT;
  const scale = barScale(bars.map((b) => b.count), BASE - TOP);
  const modeName = mode === "solo" ? t("mode.solo") : t("mode.duos");
  const topLabel = mode === "solo" ? t("place.group.top.solo") : t("place.group.top.duos");
  const keyItem = (key: string, text: string) => h("li", {}, h("i", { class: `key-${key}`, "aria-hidden": "true" }), text);
  const marks = bars.map((b, i) => {
    const x = i * SLOT + (SLOT - BAR) / 2;
    const length = scale(b.count);
    return s(
      "g",
      { class: "mark" },
      s("title", {}, `${ordinal(b.place)}: ${b.count} (${share(b.count, total)})`),
      s("rect", { class: "hit", x: i * SLOT, y: "0", width: SLOT, height: HEIGHT }),
      length > 0 ? s("path", { class: `fill-${b.group}`, d: columnPath(x, BASE, BAR, length, "up") }) : null,
      s("text", { x: x + BAR / 2, y: BASE - length - 5, "text-anchor": "middle", class: cls({ "label-strong": b.count > 0 }) || undefined }, String(b.count)),
      s("text", { x: x + BAR / 2, y: BASE + 15, "text-anchor": "middle" }, ordinal(b.place)),
    );
  });
  return h(
    "figure",
    { class: "chart chart-places" },
    chartHead(id, t("chart.places.title"), t("chart.places.desc", { mode: modeName })),
    h("ul", { class: "chart-key", "aria-hidden": "true" }, keyItem("first", t("place.group.first")), keyItem("top", topLabel), keyItem("bottom", t("place.group.bottom"))),
    s(
      "svg",
      { viewBox: `0 0 ${width} ${HEIGHT}`, width, height: HEIGHT, role: "img", "aria-labelledby": `${id}-title`, "aria-describedby": `${id}-desc` },
      s("line", { class: "axis", x1: "0", x2: width, y1: BASE, y2: BASE }),
      marks,
    ),
    unknown > 0
      ? h("p", { class: "chart-desc" }, unknown === 1 ? t("chart.places.unknown.one") : t("chart.places.unknown.other", { count: unknown }))
      : null,
    fold(
      { summary: t("chart.table"), context: `${t("chart.places.title")}, ${modeName}`, compact: true },
      dataTable(
        headRow([[t("chart.places.col.place"), false], [t("chart.places.col.games"), true], [t("chart.places.col.share"), true]]),
        bars.map((b) => h("tr", {}, h("td", {}, ordinal(b.place)), h("td", { class: "num" }, String(b.count)), h("td", { class: "num" }, share(b.count, total)))),
      ),
    ),
  );
}

/** Place over time (the Sparkline component's chart variant, with places). */
export function placeTrend(values: (number | null)[], max: number, id: string): VNode {
  if (values.length === 0) return stateBlock({ kind: "empty", title: t("chart.trend.title") });
  const [width, height, left, right, min] = [300, 72, 30, 6, 1];
  const line = sparkline(values, { width: width - left - right, height, min, max, invert: true });
  const last = line.points.at(-1)?.index === values.length - 1 ? line.points.at(-1) : undefined;
  return h(
    "figure",
    { class: "chart chart-trend" },
    chartHead(id, t("chart.trend.title"), t("chart.trend.desc", { count: values.length })),
    s(
      "svg",
      { viewBox: `0 0 ${width} ${height}`, width, height, role: "img", "aria-labelledby": `${id}-title`, "aria-describedby": `${id}-desc` },
      s("line", { class: "grid", x1: left, x2: width - right, y1: "1", y2: "1" }),
      s("line", { class: "grid", x1: left, x2: width - right, y1: height - 1, y2: height - 1 }),
      s("text", { x: left - 6, y: "5", "text-anchor": "end" }, ordinal(min)),
      s("text", { x: left - 6, y: height + 3, "text-anchor": "end" }, ordinal(max)),
      s(
        "g",
        { transform: `translate(${left} 0)` },
        s("path", { class: "spark-line", d: line.path }),
        line.points.map((p) =>
          s("g", { class: "mark" }, s("title", {}, `${t("chart.trend.col.game")} ${p.index + 1}: ${ordinal(p.value)}`), s("circle", { class: "hit", cx: p.x, cy: p.y, r: "8" })),
        ),
        last ? s("circle", { class: "spark-end", cx: last.x, cy: last.y, r: "4.5" }) : null,
      ),
    ),
    fold(
      { summary: t("chart.table"), context: t("chart.trend.title"), compact: true },
      dataTable(
        headRow([[t("chart.trend.col.game"), false], [t("chart.trend.col.place"), true]]),
        values.map((v, i) => h("tr", {}, h("td", {}, String(i + 1)), h("td", { class: "num" }, v === null ? DASH : ordinal(v)))),
      ),
    ),
  );
}

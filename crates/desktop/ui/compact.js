// Compact lists (T-323): every long table in the main window shows its top
// rows, ranked the same way everywhere (most games first, ties broken by the
// better average place, then by name), with "Show all (N)" for the rest and,
// where it helps, headings that sort. One helper, so every list behaves alike.
// The ranking and the view are plain data in, plain data out
// (ui-tests/compact.test.js); `table` draws them with the window's `ui`.
"use strict";

(() => {
  const LIMIT = 6;
  const known = (v) => v !== null && v !== undefined && !(typeof v === "number" && Number.isNaN(v));
  const collator = new Intl.Collator("en", { sensitivity: "base", numeric: true });

  // Unknown values always go after known ones, whichever way a list is sorted.
  function compareKnown(a, b, dir) {
    if (!known(a) || !known(b)) return known(a) ? -1 : known(b) ? 1 : 0;
    if (typeof a === "string" || typeof b === "string") return dir * collator.compare(String(a), String(b));
    return dir * (a - b);
  }

  // A new list: most games first, then the better (lower) average place, then the name.
  function rank(rows, by) {
    return rows.slice().sort((a, b) =>
      compareKnown(by.games(a), by.games(b), -1)
      || (by.place ? compareKnown(by.place(a), by.place(b), 1) : 0)
      || compareKnown(by.name(a), by.name(b), 1));
  }

  // Best first: high numbers, low places (`low`), A to Z for text.
  const bestDir = (column) => (column.text || column.low ? 1 : -1);

  function sorted(rows, columns, sort) {
    if (!sort || !columns || !columns[sort.column] || !columns[sort.column].sort) return rows;
    const column = columns[sort.column];
    const dir = bestDir(column) * (sort.dir === "worst" ? -1 : 1);
    // A stable sort, so ties keep the default ranking.
    return rows.slice().sort((a, b) => compareKnown(column.sort(a), column.sort(b), dir));
  }

  function ariaSort(column, index, sort) {
    if (!sort || sort.column !== index) return "none";
    const ascending = (bestDir(column) === 1) === (sort.dir !== "worst");
    return ascending ? "ascending" : "descending";
  }

  // What to draw: the rows, the button (null when every row fits) and each heading's state.
  function view({ rows, limit = LIMIT, expanded = false, columns = [], sort = null }) {
    const all = sorted(rows, columns, sort);
    const folds = all.length > limit;
    return {
      rows: folds && !expanded ? all.slice(0, limit) : all,
      button: folds ? { text: expanded ? `Show top ${limit}` : `Show all (${all.length})`, expanded } : null,
      headings: columns.map((c, i) => ({ label: c.label, sortable: Boolean(c.sort), ariaSort: c.sort ? ariaSort(c, i, sort) : "none" })),
    };
  }

  // A heading pressed: best first, then worst first, then the default order.
  function nextSort(sort, column) {
    if (!sort || sort.column !== column) return { column, dir: "best" };
    return sort.dir === "best" ? { column, dir: "worst" } : null;
  }

  // Expanded and sorted, per list, kept across redraws for the whole session.
  const states = new Map();
  function state(key) {
    if (!states.has(key)) states.set(key, { expanded: false, sort: null });
    return states.get(key);
  }

  // --- Drawing (the page only). `ui` is the window's el/withSource/sourceMark/addTip.

  const keep = (redraw) => (typeof window !== "undefined" && window.TLFocus ? window.TLFocus.keep(document, redraw) : redraw());
  const safeKey = (key) => key.replace(/[^A-Za-z0-9_-]/g, "-");

  // A folded block, closed until opened (Enter or Space on its summary) and
  // kept open across redraws. `title` is text or nodes.
  const openFolds = new Set();
  function fold(ui, key, title, className) {
    const box = ui.el("details", className ? `fold ${className}` : "fold");
    const summary = ui.el("summary");
    summary.append(...[].concat(title));
    summary.setAttribute("data-focus", `fold:${key}`);
    box.append(summary);
    box.open = openFolds.has(key);
    box.addEventListener("toggle", () => (box.open ? openFolds.add(key) : openFolds.delete(key)));
    return box;
  }

  // A long explanation, out of the way: "About this" opens it.
  function about(ui, key, text, label = "About this") {
    const box = fold(ui, `about:${key}`, label, "about");
    box.append(ui.el("p", "note", text));
    return box;
  }

  function headCell(ui, column, heading, index, st, redraw) {
    const th = ui.el("th", column.num ? "num" : null);
    th.setAttribute("scope", "col");
    if (!heading.sortable) {
      th.append(column.head ? column.head() : column.label);
      return th;
    }
    th.setAttribute("aria-sort", heading.ariaSort);
    const button = ui.el("button", "sort-btn", column.label);
    button.type = "button";
    button.setAttribute("data-focus", `sort:${redraw.key}:${index}`);
    const arrow = heading.ariaSort === "ascending" ? "▲" : heading.ariaSort === "descending" ? "▼" : "";
    if (arrow) {
      // aria-sort on the heading already says it; the arrow is for the eye.
      const mark = ui.el("span", "sort-arrow", arrow);
      mark.setAttribute("aria-hidden", "true");
      button.append(mark);
    }
    button.title = `Sort by ${column.label.toLowerCase()}`;
    button.addEventListener("click", () => {
      st.sort = nextSort(st.sort, index);
      redraw();
    });
    th.append(button);
    if (column.head) th.append(column.head());
    return th;
  }

  // A table that shows its top rows. `columns`: { label, num, sort, text, low,
  // head }; `row(r)` returns the <tr>. Ranked by the caller (see `rank`).
  function table(ui, { key, columns, rows, row, limit = LIMIT, className, label }) {
    const wrap = ui.el("div", "compact");
    const tableId = `compact-${safeKey(key)}`;
    const st = state(key);
    const draw = () => {
      const v = view({ rows, limit, expanded: st.expanded, columns, sort: st.sort });
      const head = ui.el("tr");
      head.append(...columns.map((c, i) => headCell(ui, c, v.headings[i], i, st, redraw)));
      const thead = ui.el("thead");
      thead.append(head);
      const tbody = ui.el("tbody");
      tbody.append(...v.rows.map(row));
      const node = ui.el("table", className || null);
      node.id = tableId;
      if (label) node.setAttribute("aria-label", label);
      node.append(thead, tbody);
      const parts = [node];
      if (v.button) {
        const button = ui.el("button", "show-all", v.button.text);
        button.type = "button";
        button.setAttribute("aria-expanded", String(v.button.expanded));
        button.setAttribute("aria-controls", tableId);
        button.setAttribute("data-focus", `showall:${key}`);
        button.addEventListener("click", () => {
          st.expanded = !st.expanded;
          redraw();
        });
        parts.push(button);
      }
      wrap.replaceChildren(...parts);
    };
    const redraw = () => keep(draw);
    redraw.key = safeKey(key);
    draw();
    return wrap;
  }

  const api = { LIMIT, rank, view, nextSort, state, fold, about, table };
  if (typeof module !== "undefined" && module.exports) module.exports = api;
  else window.TLCompact = api;
})();

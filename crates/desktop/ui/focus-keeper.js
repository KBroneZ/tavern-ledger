// Keeps keyboard focus where the user left it when a page redraws (T-317).
// Redrawing replaces elements, and a replaced or hidden element drops focus to
// the page: a keyboard or screen reader user would start again from the top.
// `keep(doc, redraw)` notes the focused control, runs the redraw and puts focus
// back on the same control, or on a sensible neighbour when it is gone.
//
// A control is found again by its `id`, or by a `data-focus` key set by the
// code that builds it. Two optional attributes say where to go if it is gone:
//   data-focus-next="id other-id"   controls to try, in order
//   data-focus-scope                on the list or table that holds it; the
//                                   control in the same position is used
"use strict";

(() => {
  const FOCUSABLE = "button, input, select, textarea, a[href], [tabindex]";

  const isUsable = (node) => Boolean(node) && !node.disabled && node.isConnected !== false && !node.closest("[hidden]");

  const byKey = (doc, key) =>
    [...doc.querySelectorAll("[data-focus]")].find((n) => n.getAttribute("data-focus") === key) || null;

  const scopeItems = (scope) => [...scope.querySelectorAll(FOCUSABLE)].filter(isUsable);

  // What is needed to find the focused control again, or null when nothing is focused.
  function note(doc) {
    const node = doc.activeElement;
    if (!node || node === doc.body || node === doc.documentElement) return null;
    const scope = node.closest("[data-focus-scope]");
    return {
      id: node.id || null,
      key: node.getAttribute("data-focus"),
      next: (node.getAttribute("data-focus-next") || "").split(/\s+/).filter(Boolean),
      scopeId: scope && scope.id ? scope.id : null,
      index: scope ? scopeItems(scope).indexOf(node) : -1,
    };
  }

  function find(doc, saved) {
    if (saved.id) {
      const node = doc.getElementById(saved.id);
      if (isUsable(node)) return node;
    }
    if (saved.key) {
      const node = byKey(doc, saved.key);
      if (isUsable(node)) return node;
    }
    for (const id of saved.next) {
      const node = doc.getElementById(id);
      if (isUsable(node)) return node;
    }
    const scope = saved.scopeId && doc.getElementById(saved.scopeId);
    if (scope && saved.index >= 0) {
      const items = scopeItems(scope);
      if (items.length) return items[Math.min(saved.index, items.length - 1)];
    }
    return null;
  }

  // Puts focus back. True when it ended up on a control.
  function restore(doc, saved) {
    if (!saved) return false;
    const target = find(doc, saved);
    if (!target) return false;
    if (doc.activeElement !== target) target.focus({ preventScroll: true });
    return doc.activeElement === target;
  }

  function keep(doc, redraw) {
    const saved = note(doc);
    try {
      return redraw();
    } finally {
      restore(doc, saved);
    }
  }

  const api = { note, restore, keep };
  if (typeof module !== "undefined" && module.exports) module.exports = api;
  else window.TLFocus = api;
})();

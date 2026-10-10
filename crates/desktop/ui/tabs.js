// Tabs of the main window (T-216): an ARIA tab list. Only the selected tab is
// in the Tab order; the arrow keys, Home and End move between tabs and show
// the one they land on, as screen readers and keyboard users expect.
"use strict";

(() => {
  // The tab to go to for a key, or null when the key is not one of ours.
  function nextIndex(key, current, count) {
    if (count < 1) return null;
    switch (key) {
      case "ArrowRight": return (current + 1) % count;
      case "ArrowLeft": return (current - 1 + count) % count;
      case "Home": return 0;
      case "End": return count - 1;
      default: return null;
    }
  }

  // Shows tab `index` and its panel and hides the others; focuses it if asked.
  function select(tabs, panels, index, focus) {
    tabs.forEach((tab, i) => {
      const on = i === index;
      tab.setAttribute("aria-selected", String(on));
      tab.setAttribute("tabindex", on ? "0" : "-1");
      panels[i].hidden = !on;
    });
    if (focus) tabs[index].focus();
  }

  // Wires a tab list: `onChange(index)` runs after a tab is shown.
  function setup(doc, listId, onChange) {
    const list = doc.getElementById(listId);
    const tabs = [...list.querySelectorAll('[role="tab"]')];
    const panels = tabs.map((t) => doc.getElementById(t.getAttribute("aria-controls")));
    const current = () => Math.max(0, tabs.findIndex((t) => t.getAttribute("aria-selected") === "true"));
    const show = (i, focus) => {
      select(tabs, panels, i, focus);
      if (onChange) onChange(i);
    };
    tabs.forEach((tab, i) => tab.addEventListener("click", () => show(i, false)));
    list.addEventListener("keydown", (e) => {
      const next = nextIndex(e.key, current(), tabs.length);
      if (next === null) return;
      e.preventDefault();
      show(next, true);
    });
    select(tabs, panels, current(), false);
    return { show, current };
  }

  const api = { nextIndex, select, setup };
  if (typeof module !== "undefined" && module.exports) module.exports = api;
  else window.TLTabs = api;
})();

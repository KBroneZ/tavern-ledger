// The overlay page for the workbench: the real overlay.html as it is on disk,
// with the gallery's stand-in for the app's bridge, the theme colours and one
// scenario's state in a script placed before the page's first script, and a
// small live-reload script after it. Nothing is copied or built.
"use strict";

const fs = require("node:fs");
const path = require("node:path");

const THEMES = ["glass", "contrast"];
const FIRST_SCRIPT = /<script src="/;

// Changed stylesheets are swapped in place; anything else reloads the page.
// A server that comes back after a stop reloads it too.
const LIVE_RELOAD = `(() => {
  if (!window.EventSource) return;
  const source = new EventSource("/events");
  let dropped = false;
  source.addEventListener("css", (event) => {
    let swapped = false;
    for (const link of document.querySelectorAll('link[rel="stylesheet"]')) {
      const url = new URL(link.href);
      if (url.pathname !== "/ui/" + event.data) continue;
      url.searchParams.set("v", String(Date.now()));
      link.href = url.href;
      swapped = true;
    }
    if (!swapped) location.reload();
  });
  source.addEventListener("reload", () => location.reload());
  source.onerror = () => { dropped = true; };
  source.onopen = () => { if (dropped) location.reload(); };
})();`;

const read = (file) => fs.readFileSync(file, "utf8");
// Inside an inline script the text `</script` would end it.
const inScript = (text) => text.replace(/<\/script/gi, "<\\/script");
// JSON that is safe inside a script element.
const jsonForScript = (value) => JSON.stringify(value).replace(/</g, "\\u003c");

// The scenario's state with the workbench's own choices on top. The hover
// card is placed by the stand-in for the frame's own size (default-layout.js).
function stateFor(scenario, { theme }) {
  const settings = { ...scenario.state.settings };
  if (THEMES.includes(theme)) settings.theme = theme;
  return { ...scenario.state, settings };
}

function overlayDocument({ html, galleryDir, scenario, lang, theme, bare }) {
  if (!FIRST_SCRIPT.test(html)) throw new Error("overlay.html has no script to put the stand-in before");
  const state = stateFor(scenario, { theme });
  const bootstrap = [
    read(path.join(galleryDir, "theme-vars.js")),
    read(path.join(galleryDir, "default-layout.js")),
    read(path.join(galleryDir, "bridge.js")),
    `TLGalleryBridge.install(window, { state: ${jsonForScript(state)} }, ${JSON.stringify(lang === "es" ? "es" : "en")});`,
  ].join("\n");
  const backdrop = bare ? "<style>html { background: #1b1b1f; }</style>\n" : "";
  return html.replace(FIRST_SCRIPT, (first) => `${backdrop}<script>\n${inScript(bootstrap)}\n</script>\n<script>\n${LIVE_RELOAD}\n</script>\n${first}`);
}

module.exports = { overlayDocument, stateFor, THEMES };

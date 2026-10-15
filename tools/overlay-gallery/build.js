// Builds the overlay gallery: one self-contained HTML file that shows the
// real overlay page (crates/desktop/ui/overlay.html and the files it loads)
// in every state, with example data and a short explanation of each.
//
//   node tools/overlay-gallery/build.js [--out <file>]
//
// The default file is .local/overlay-gallery/overlay-gallery.html (never
// committed, never in the installer). The overlay page is read, never edited:
// the stand-in for the app's bridge is put in before its scripts, its
// stylesheet and fonts are inlined, and it is stored once, as one encoded
// block, that the gallery page opens in a frame per scenario. Scenarios are
// chosen by the page's address: ?scenario=<id> and ?lang=es.
"use strict";

const fs = require("node:fs");
const path = require("node:path");

const { scenariosFor } = require("./scenarios/index.js");

const REPO = path.join(__dirname, "..", "..");
const DEFAULT_UI = path.join(REPO, "crates", "desktop", "ui");
const DEFAULT_OUT = path.join(REPO, ".local", "overlay-gallery", "overlay-gallery.html");
const MAX_BYTES = 2 * 1024 * 1024;

const read = (file) => fs.readFileSync(file, "utf8");
// Inside an inline script the text `</script` would end it.
const inScript = (text) => text.replace(/<\/script/gi, "<\\/script");
// JSON that is safe inside a script element.
const jsonForScript = (value) => JSON.stringify(value).replace(/</g, "\\u003c");

function inlineFonts(css, uiDir) {
  return css.replace(/url\("(fonts\/[^"]+)"\)/g, (_, file) => {
    const bytes = fs.readFileSync(path.join(uiDir, file));
    return `url("data:font/woff2;base64,${bytes.toString("base64")}")`;
  });
}

// A module script and what it imports as one script: every file in the
// graph, the dependencies first, with the import lines and the `export`
// keywords taken out (the files' names are unique, so they share one scope).
function bundleModule(file, seen = new Set(), parts = []) {
  if (seen.has(file)) return parts;
  seen.add(file);
  const text = read(file);
  const IMPORT = /^import\s[^;]*?from\s+"([^"]+)";?[ \t]*$/gm;
  for (const m of text.matchAll(IMPORT)) {
    if (!m[1].startsWith(".")) throw new Error(`${file} imports ${m[1]}: only the app's own files are inlined`);
    bundleModule(path.resolve(path.dirname(file), m[1]), seen, parts);
  }
  parts.push(text.replace(IMPORT, "").replace(/^export\s+(?=(?:async\s+)?(?:function|const|let|class)\b)/gm, ""));
  return parts;
}

// The overlay page as one document: stylesheet inline, scripts inline, the
// stand-in and the scenarios' states before the first script.
function overlayDocument(uiDir, scenarios) {
  const html = read(path.join(uiDir, "overlay.html"));
  const states = {};
  for (const s of scenarios) states[s.id] = { ...s.state, names: undefined };
  const names = scenarios[0].state.names;
  const bootstrap = [
    "/*__GALLERY_CONFIG__*/",
    read(path.join(__dirname, "theme-vars.js")),
    read(path.join(__dirname, "default-layout.js")),
    read(path.join(__dirname, "bridge.js")),
    `(() => { const states = ${jsonForScript(states)}; const names = ${jsonForScript(names)};` +
      " const cfg = window.__GALLERY__ || { scenario: Object.keys(states)[0], lang: \"en\" };" +
      " TLGalleryBridge.install(window, { state: { ...states[cfg.scenario], names } }, cfg.lang); })();",
  ].join("\n");
  let first = true;
  return html
    .replace(/<link rel="stylesheet" href="([^"]+)">/g, (_, href) => `<style>\n${inlineFonts(read(path.join(uiDir, href)), uiDir)}\n</style>`)
    .replace(/<script type="module" src="([^"]+)"><\/script>/g, (_, src) => {
      const code = bundleModule(path.join(uiDir, src)).join("\n");
      return `<script type="module">\n${inScript(code)}\n</script>`;
    })
    .replace(/<script src="([^"]+)"><\/script>/g, (_, src) => {
      const own = `<script>\n${inScript(read(path.join(uiDir, src)))}\n</script>`;
      if (!first) return own;
      first = false;
      return `<script>\n${inScript(bootstrap)}\n</script>\n${own}`;
    });
}

// The page must have come out whole: a tag the inlining did not recognise
// would make the frame ask for a file that is not there.
function checkSelfContained(doc) {
  // A CSS url() matters in the styles and the markup, not in a script's own text.
  const withoutScripts = doc.replace(/<script(?![^>]*\ssrc=)[^>]*>[\s\S]*?<\/script>/gi, "");
  const loose = withoutScripts.match(/<script[^>]*\ssrc=[^>]*>|<link[^>]*>|url\(\s*(?!["']?data:)[^)]*\)/i);
  if (loose) throw new Error(`overlay.html has something the gallery does not inline: ${loose[0].slice(0, 80)}`);
}

function build({ uiDir = DEFAULT_UI, outFile = DEFAULT_OUT } = {}) {
  const scenarios = scenariosFor(read(path.join(uiDir, "overlay.html")));
  const doc = overlayDocument(uiDir, scenarios);
  checkSelfContained(doc);
  const gallery = {
    html: read(path.join(__dirname, "gallery.html")),
    css: read(path.join(__dirname, "gallery.css")),
    js: read(path.join(__dirname, "gallery.js")),
  };
  const data = { scenarios: scenarios.map((s) => ({ id: s.id, focus: s.focus, caption: s.caption })) };
  const fill = { "{{CSS}}": gallery.css, "{{DATA}}": jsonForScript(data), "{{JS}}": inScript(gallery.js), "{{DOC}}": Buffer.from(doc, "utf8").toString("base64") };
  const text = gallery.html.replace(/\{\{(CSS|DATA|JS|DOC)\}\}/g, (token) => fill[token]);
  const bytes = Buffer.byteLength(text);
  if (bytes > MAX_BYTES) throw new Error(`the gallery is ${bytes} bytes, over the ${MAX_BYTES} limit`);
  fs.mkdirSync(path.dirname(outFile), { recursive: true });
  fs.writeFileSync(outFile, text);
  return { file: outFile, bytes, scenarios: scenarios.map((s) => s.id) };
}

if (require.main === module) {
  const at = process.argv.indexOf("--out");
  const result = build(at > 0 ? { outFile: path.resolve(process.argv[at + 1]) } : {});
  console.log(`${result.file} (${(result.bytes / 1024).toFixed(0)} KB, ${result.scenarios.length} scenarios)`);
}

module.exports = { build, inScript, jsonForScript, checkSelfContained, bundleModule };

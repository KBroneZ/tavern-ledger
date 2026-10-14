// The overlay workbench: the real overlay page on localhost, with the
// gallery's example data, to work on it and see the changes.
//
//   node tools/overlay-dev/serve.js [--port 5178]
//
// Open http://127.0.0.1:5178/ (the page in a game-window-sized frame with
// controls) or http://127.0.0.1:5178/bare (only the overlay page, full size).
// The server listens on 127.0.0.1 only and serves crates/desktop/ui/ straight
// from disk: no copy, no build step. Only four folders are served, nothing is
// written to disk, and nothing is fetched from the network. Saving a file
// there swaps a changed stylesheet in place and reloads the page for anything
// else (Server-Sent Events). Node built-ins only.
"use strict";

const fs = require("node:fs");
const http = require("node:http");
const path = require("node:path");

const { resolveInside } = require("./paths.js");
const { overlayDocument } = require("./page.js");
const { watchFolders } = require("./watcher.js");

const REPO = path.join(__dirname, "..", "..");
const DEFAULT_PORT = 5178;
const HOST = "127.0.0.1";

const TYPES = {
  ".html": "text/html; charset=utf-8",
  ".css": "text/css; charset=utf-8",
  ".js": "text/javascript; charset=utf-8",
  ".json": "application/json; charset=utf-8",
  ".svg": "image/svg+xml",
  ".png": "image/png",
  ".jpg": "image/jpeg",
  ".jpeg": "image/jpeg",
  ".webp": "image/webp",
  ".gif": "image/gif",
  ".woff2": "font/woff2",
  ".woff": "font/woff",
};
const IMAGE = /\.(png|jpe?g|webp|gif)$/i;

// What `/ui/`, `/dev/`, `/gallery/` and `/backgrounds/` stand for.
const prefixes = (dirs) => [
  ["/ui/", dirs.uiDir],
  ["/dev/", dirs.devDir],
  ["/gallery/", dirs.galleryDir],
  ["/backgrounds/", dirs.backgroundsDir, IMAGE],
];

// The overlay page by any spelling that reaches the same file (encoded or
// differently cased letters), so it never comes without the stand-in.
function isOverlayPage(raw) {
  try {
    return decodeURIComponent(raw).toLowerCase() === "/ui/overlay.html";
  } catch {
    return false;
  }
}

function createDevServer(options = {}) {
  const dirs = {
    uiDir: options.uiDir || path.join(REPO, "crates", "desktop", "ui"),
    devDir: options.devDir || __dirname,
    galleryDir: options.galleryDir || path.join(REPO, "tools", "overlay-gallery"),
    backgroundsDir: options.backgroundsDir || path.join(REPO, ".local", "overlay-dev", "backgrounds"),
  };
  const roots = prefixes(dirs);
  const clients = new Set();
  let allowedHosts = [];
  let stopWatching = () => {};
  // The scenarios are code: after a change in the gallery's folder they are
  // loaded again.
  let stale = false;

  function scenariosFor(html) {
    const index = path.join(dirs.galleryDir, "scenarios", "index.js");
    if (stale) {
      for (const key of Object.keys(require.cache)) if (key.startsWith(dirs.galleryDir)) delete require.cache[key];
      stale = false;
    }
    return require(index).scenariosFor(html);
  }

  const send = (res, status, type, body, head) => {
    res.writeHead(status, { "content-type": type, "cache-control": "no-store", "x-content-type-options": "nosniff" });
    res.end(head ? undefined : body);
  };
  const notFound = (res) => send(res, 404, "text/plain; charset=utf-8", "Not found");
  const json = (res, value, head) => send(res, 200, TYPES[".json"], JSON.stringify(value), head);

  function overlayPage(res, query, head) {
    const html = fs.readFileSync(path.join(dirs.uiDir, "overlay.html"), "utf8");
    const scenarios = scenariosFor(html);
    const id = query.get("scenario");
    const scenario = id ? scenarios.find((s) => s.id === id) : scenarios[0];
    if (!scenario) return notFound(res);
    const body = overlayDocument({
      html,
      galleryDir: dirs.galleryDir,
      scenario,
      lang: query.get("lang"),
      theme: query.get("theme"),
      width: Number(query.get("width")),
      bare: query.get("bare") === "1",
    });
    send(res, 200, TYPES[".html"], body, head);
  }

  function scenarioList(res, head) {
    const html = fs.readFileSync(path.join(dirs.uiDir, "overlay.html"), "utf8");
    const scenarios = scenariosFor(html).map((s) => ({
      id: s.id,
      caption: s.caption,
      settings: s.state.settings,
      hover: s.state.hover || null,
      focus: s.focus,
    }));
    json(res, { scenarios }, head);
  }

  function backgroundList(res, head) {
    let files = [];
    try {
      files = fs.readdirSync(dirs.backgroundsDir).filter((name) => IMAGE.test(name)).sort();
    } catch {
      // No folder yet: no backgrounds.
    }
    json(res, { files }, head);
  }

  function events(req, res) {
    res.writeHead(200, { "content-type": "text/event-stream", "cache-control": "no-store", connection: "keep-alive" });
    res.write("retry: 1000\n\n");
    clients.add(res);
    req.on("close", () => clients.delete(res));
  }

  function broadcast(name, data) {
    for (const client of clients) client.write(`event: ${name}\ndata: ${data}\n\n`);
  }

  // What the page should do for a batch of saved files: a lone-stylesheet
  // change is swapped in place, anything else reloads.
  function onChange(changes) {
    if (changes.some((c) => c.tag === "gallery")) stale = true;
    const sheets = changes.filter((c) => c.tag === "ui" && c.file.endsWith(".css"));
    if (sheets.length === changes.length) {
      for (const file of new Set(sheets.map((c) => c.file))) broadcast("css", file);
    } else {
      broadcast("reload", changes[0].file);
    }
  }

  function staticFile(res, url, head) {
    for (const [prefix, root, only] of roots) {
      if (!url.startsWith(prefix)) continue;
      const file = resolveInside(root, url.slice(prefix.length));
      if (!file || (only && !only.test(file))) return notFound(res);
      return send(res, 200, TYPES[path.extname(file).toLowerCase()] || "application/octet-stream", fs.readFileSync(file), head);
    }
    return notFound(res);
  }

  function route(req, res) {
    const head = req.method === "HEAD";
    const host = String(req.headers.host || "");
    if (!allowedHosts.includes(host)) return send(res, 403, "text/plain; charset=utf-8", "Forbidden");
    if (req.method !== "GET" && !head) {
      res.setHeader("allow", "GET, HEAD");
      return send(res, 405, "text/plain; charset=utf-8", "Method not allowed");
    }
    if (!req.url.startsWith("/") || req.url.startsWith("//")) return notFound(res);
    const [raw, queryText = ""] = req.url.split("#")[0].split("?");
    const query = new URLSearchParams(queryText);
    if (raw === "/") return staticFile(res, "/dev/workbench.html", head);
    if (raw === "/bare") {
      query.set("bare", "1");
      res.writeHead(302, { location: `/ui/overlay.html?${query}` });
      return res.end();
    }
    if (raw === "/events") return head ? notFound(res) : events(req, res);
    if (raw === "/api/scenarios") return scenarioList(res, head);
    if (raw === "/api/backgrounds") return backgroundList(res, head);
    if (isOverlayPage(raw)) return overlayPage(res, query, head);
    return staticFile(res, raw, head);
  }

  const server = http.createServer((req, res) => {
    try {
      route(req, res);
    } catch (error) {
      console.error(`overlay-dev: ${req.method} ${req.url}: ${error.message}`);
      if (!res.headersSent) send(res, 500, "text/plain; charset=utf-8", "Something went wrong; see the terminal");
      else res.end();
    }
  });

  function listen(port = DEFAULT_PORT) {
    return new Promise((resolve, reject) => {
      server.once("error", reject);
      server.listen(port, HOST, () => {
        server.off("error", reject);
        const address = server.address();
        allowedHosts = [`${HOST}:${address.port}`, `localhost:${address.port}`];
        stopWatching = watchFolders(
          [
            { dir: dirs.uiDir, tag: "ui" },
            { dir: dirs.galleryDir, tag: "gallery" },
          ],
          onChange,
        );
        resolve(address);
      });
    });
  }

  function close() {
    stopWatching();
    for (const client of clients) client.end();
    clients.clear();
    return new Promise((resolve) => {
      if (!server.listening) return resolve();
      server.close(() => resolve());
      server.closeAllConnections();
    });
  }

  return { listen, close, clients: () => clients.size };
}

function portFromArgs(argv) {
  const at = argv.indexOf("--port");
  if (at < 0) return DEFAULT_PORT;
  const port = Number(argv[at + 1]);
  if (!Number.isInteger(port) || port < 0 || port > 65535) throw new Error(`--port needs a number from 0 to 65535, not "${argv[at + 1]}"`);
  return port;
}

async function main() {
  const dev = createDevServer();
  const address = await dev.listen(portFromArgs(process.argv.slice(2)));
  console.log(`Overlay workbench: http://${HOST}:${address.port}/`);
  console.log(`Only the overlay page, full size: http://${HOST}:${address.port}/bare`);
  console.log("Backgrounds: drop images in .local/overlay-dev/backgrounds/ (never committed). Ctrl+C stops.");
}

if (require.main === module) {
  main().catch((error) => {
    console.error(`overlay-dev: ${error.message}`);
    process.exitCode = 1;
  });
}

module.exports = { createDevServer, DEFAULT_PORT };

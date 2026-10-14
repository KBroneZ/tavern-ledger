"use strict";
// The overlay workbench (tools/overlay-dev): it serves the real overlay page
// from disk on 127.0.0.1 only, puts the gallery's stand-in before the page's
// first script, answers every gallery scenario, refuses paths that leave the
// served folders, and tells the page when a watched file changes. The
// server's own folders are a temporary copy: nothing is written into the
// repository, and every server is closed at the end of its test.
const test = require("node:test");
const assert = require("node:assert/strict");
const fs = require("node:fs");
const http = require("node:http");
const os = require("node:os");
const path = require("node:path");

const TOOL = path.join(__dirname, "..", "..", "..", "tools", "overlay-dev");
const GALLERY = path.join(__dirname, "..", "..", "..", "tools", "overlay-gallery");
const REAL_UI = path.join(__dirname, "..", "ui");
const { createDevServer } = require(path.join(TOOL, "serve.js"));
const { safeSegments } = require(path.join(TOOL, "paths.js"));
const { SCENARIOS } = require(path.join(GALLERY, "scenarios", "index.js"));

const PAGE = `<!doctype html><html><head><link rel="stylesheet" href="overlay.css"></head>
<body><script src="first.js"></script><script src="second.js"></script></body></html>`;

// A small ui folder, a backgrounds folder and a file outside both.
function workspace() {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), "overlay-dev-"));
  const ui = path.join(root, "ui");
  const backgrounds = path.join(root, "backgrounds");
  fs.mkdirSync(path.join(ui, "sub"), { recursive: true });
  fs.mkdirSync(backgrounds);
  fs.writeFileSync(path.join(ui, "overlay.html"), PAGE);
  fs.writeFileSync(path.join(ui, "overlay.css"), "body { color: red; }");
  fs.writeFileSync(path.join(ui, "first.js"), "window.first = 1;");
  fs.writeFileSync(path.join(ui, "second.js"), "window.second = 2;");
  fs.writeFileSync(path.join(ui, "sub", "inner.js"), "window.inner = 3;");
  fs.writeFileSync(path.join(backgrounds, "table.png"), "png");
  fs.writeFileSync(path.join(backgrounds, "notes.txt"), "not an image");
  fs.writeFileSync(path.join(root, "secret.txt"), "outside");
  return { root, ui, backgrounds };
}

// Runs `body` with a started server; always closes it and removes the folders.
async function withServer(options, body) {
  const space = workspace();
  const dev = createDevServer({ uiDir: space.ui, backgroundsDir: space.backgrounds, ...options });
  const address = await dev.listen(0);
  const origin = `http://127.0.0.1:${address.port}`;
  try {
    await body({ dev, origin, space, address });
  } finally {
    await dev.close();
    fs.rmSync(space.root, { recursive: true, force: true });
  }
}

// A plain GET with the path sent exactly as written (fetch would tidy `..`).
function get(origin, rawPath, headers = {}) {
  return new Promise((resolve, reject) => {
    const url = new URL(origin);
    const req = http.request({ host: url.hostname, port: url.port, path: rawPath, headers, method: "GET", agent: false }, (res) => {
      const chunks = [];
      res.on("data", (c) => chunks.push(c));
      res.on("end", () => resolve({ status: res.statusCode, headers: res.headers, body: Buffer.concat(chunks).toString("utf8") }));
    });
    req.on("error", reject);
    req.end();
  });
}

test("serves files straight from the ui folder, as they are on disk", async () => {
  await withServer({}, async ({ origin, space }) => {
    const css = await get(origin, "/ui/overlay.css");
    assert.equal(css.status, 200);
    assert.match(css.headers["content-type"], /text\/css/);
    assert.equal(css.headers["cache-control"], "no-store");
    assert.equal(css.body, "body { color: red; }");
    fs.writeFileSync(path.join(space.ui, "overlay.css"), "body { color: blue; }");
    assert.equal((await get(origin, "/ui/overlay.css")).body, "body { color: blue; }", "no copy, no cache");
    assert.equal((await get(origin, "/ui/sub/inner.js")).status, 200);
  });
});

test("listens on 127.0.0.1 only", async () => {
  await withServer({}, async ({ address }) => {
    assert.equal(address.address, "127.0.0.1");
  });
});

test("refuses a request whose Host is not the local server (DNS rebinding)", async () => {
  await withServer({}, async ({ origin, address }) => {
    assert.equal((await get(origin, "/api/scenarios", { host: "evil.example" })).status, 403);
    assert.equal((await get(origin, "/api/scenarios", { host: `localhost:${address.port}` })).status, 200);
  });
});

const REFUSED = [
  "/ui/../secret.txt",
  "/ui/%2e%2e/secret.txt",
  "/ui/%2E%2E/secret.txt",
  "/ui/..%2fsecret.txt",
  "/ui/sub/../../secret.txt",
  "/ui//secret.txt",
  "/ui/C:/Windows/win.ini",
  "/ui/..\\secret.txt",
  "/ui/%5c..%5csecret.txt",
  "/ui/sub\\..\\..\\secret.txt",
  "/ui/%252e%252e/secret.txt",
  "/ui/overlay.css%00.png",
  "/ui/%2fsecret.txt",
  "/ui/sub/",
  "/ui/sub",
  "/ui/",
  "/ui",
  "/backgrounds/../secret.txt",
  "/backgrounds/%2e%2e%2fsecret.txt",
  "/gallery/../../secret.txt",
  "/dev/../ui/overlay.css/..",
  "/package.json",
  "/etc/passwd",
  "/%",
];

test("path traversal, absolute and backslash paths, folders and other roots answer 404", async () => {
  await withServer({}, async ({ origin }) => {
    for (const raw of REFUSED) {
      const res = await get(origin, raw);
      assert.equal(res.status, 404, `${raw} must be refused`);
      assert.doesNotMatch(res.body, /outside/, `${raw} leaked a file`);
    }
  });
});

test("a symbolic link out of a served folder is not followed", async (t) => {
  await withServer({}, async ({ origin, space }) => {
    try {
      fs.symlinkSync(path.join(space.root, "secret.txt"), path.join(space.ui, "link.txt"));
    } catch {
      t.skip("symbolic links are not allowed here");
      return;
    }
    assert.equal((await get(origin, "/ui/link.txt")).status, 404);
  });
});

test("segments check: plain names pass, anything that climbs or names a drive does not", () => {
  assert.deepEqual(safeSegments("sub/inner.js"), ["sub", "inner.js"]);
  for (const bad of ["..", "a/../b", "a//b", "a/", "/a", "a\\b", "c:/x", "a/%2e%2e/b", "%zz", "a%00b"]) {
    assert.equal(safeSegments(bad), null, bad);
  }
});

test("other methods are refused and the server writes nothing", async () => {
  await withServer({}, async ({ origin, space }) => {
    const before = fs.readdirSync(space.root).sort();
    const res = await fetch(`${origin}/ui/overlay.css`, { method: "PUT", body: "x" });
    assert.equal(res.status, 405);
    assert.deepEqual(fs.readdirSync(space.root).sort(), before);
    assert.equal(fs.readFileSync(path.join(space.ui, "overlay.css"), "utf8"), "body { color: red; }");
  });
});

test("the overlay page gets the stand-in before its first script, and the scenario's state", async () => {
  await withServer({}, async ({ origin }) => {
    const res = await get(origin, "/ui/overlay.html?scenario=solo-advanced&lang=es&theme=contrast");
    assert.equal(res.status, 200);
    const install = res.body.indexOf("TLGalleryBridge.install(");
    const themes = res.body.indexOf("TLGalleryThemes");
    const first = res.body.indexOf('<script src="first.js">');
    const second = res.body.indexOf('<script src="second.js">');
    assert.ok(themes > -1 && install > themes, "theme colours come before the stand-in is installed");
    assert.ok(first > install, "the stand-in is installed before the page's first script");
    assert.ok(second > first, "the page's scripts keep their order");
    assert.match(res.body, /"theme":"contrast"/, "the chosen theme is in the state");
    assert.match(res.body, /"detail":"advanced"/, "the scenario's own mode stays as it is");
    assert.match(res.body, /, "es"\);/, "the language is passed");
    assert.match(res.body, /<link rel="stylesheet" href="overlay.css">/, "the page's own tags are kept");
  });
});

test("a hover placed for 1920 pixels is scaled to the window's width", async () => {
  await withServer({ uiDir: REAL_UI }, async ({ origin }) => {
    const res = await get(origin, "/ui/overlay.html?scenario=hover-solo&width=960");
    for (const spelling of ["/ui/%6Fverlay.html", "/ui/Overlay.html"]) {
      assert.ok((await get(origin, spelling)).body.includes("TLGalleryBridge.install("), `${spelling} gets the stand-in`);
    }
    const base = SCENARIOS.find((s) => s.id === "hover-solo").state.hover;
    assert.ok(res.body.includes(`"width":${Math.round(base.card.width / 2)}`), "the card is half as wide at half the width");
    assert.ok(res.body.includes(`"x":${Math.round(base.slot.x / 2)}`));
  });
});

test("the real overlay page: the stand-in is before every one of its scripts", async () => {
  await withServer({ uiDir: REAL_UI }, async ({ origin }) => {
    const res = await get(origin, "/ui/overlay.html");
    assert.equal(res.status, 200);
    const install = res.body.indexOf("TLGalleryBridge.install(");
    const scripts = [...res.body.matchAll(/<script src="/g)].map((m) => m.index);
    assert.ok(scripts.length > 5);
    assert.ok(scripts.every((at) => at > install));
  });
});

test("every gallery scenario answers, in both languages, and an unknown one is 404", async () => {
  await withServer({ uiDir: REAL_UI }, async ({ origin }) => {
    const list = JSON.parse((await get(origin, "/api/scenarios")).body);
    const ids = list.scenarios.map((s) => s.id);
    for (const s of SCENARIOS.filter((x) => x.id !== "counters")) assert.ok(ids.includes(s.id), `${s.id} is listed`);
    assert.ok(ids.length >= 19);
    for (const id of ids) {
      for (const lang of ["en", "es"]) {
        const res = await get(origin, `/ui/overlay.html?scenario=${id}&lang=${lang}`);
        assert.equal(res.status, 200, `${id} ${lang}`);
        assert.ok(res.body.includes("TLGalleryBridge.install("), `${id} ${lang}`);
      }
    }
    assert.equal((await get(origin, "/ui/overlay.html?scenario=nope")).status, 404);
    const hover = list.scenarios.find((s) => s.id === "hover-solo");
    assert.ok(hover.hover && hover.hover.card, "a scenario with a hover lists its payload");
    assert.equal(list.scenarios.find((s) => s.id === "solo-simple").hover, null);
    for (const s of list.scenarios) assert.ok(s.focus && s.focus.w > 0 && s.focus.h > 0, `${s.id} has a close-up region`);
  });
});

test("the bare address goes to the overlay page at full size, keeping the choices", async () => {
  await withServer({}, async ({ origin }) => {
    const res = await get(origin, "/bare?scenario=duos&lang=es");
    assert.equal(res.status, 302);
    assert.equal(res.headers.location, "/ui/overlay.html?scenario=duos&lang=es&bare=1");
    const page = await get(origin, "/ui/overlay.html?bare=1");
    assert.match(page.body, /html \{ background/, "the bare page has a backdrop of its own");
  });
});

test("the workbench page and its files are served; the backgrounds list shows images only", async () => {
  await withServer({}, async ({ origin }) => {
    const home = await get(origin, "/");
    assert.equal(home.status, 200);
    assert.match(home.headers["content-type"], /text\/html/);
    assert.equal((await get(origin, "/dev/workbench.js")).status, 200);
    assert.equal((await get(origin, "/dev/workbench.css")).status, 200);
    const list = JSON.parse((await get(origin, "/api/backgrounds")).body);
    assert.deepEqual(list.files, ["table.png"]);
    assert.equal((await get(origin, "/backgrounds/table.png")).status, 200);
    assert.equal((await get(origin, "/backgrounds/notes.txt")).status, 404, "only images are served from there");
  });
});

test("a missing backgrounds folder is an empty list, and is not created", async () => {
  const missing = path.join(os.tmpdir(), "overlay-dev-no-such-folder");
  await withServer({ backgroundsDir: missing }, async ({ origin }) => {
    assert.deepEqual(JSON.parse((await get(origin, "/api/backgrounds")).body).files, []);
    assert.equal(fs.existsSync(missing), false);
  });
});

// Reads the event stream until `wanted` events have come or the time is up.
function listen(origin, wanted, ms = 5000) {
  return new Promise((resolve, reject) => {
    const url = new URL(origin);
    const events = [];
    let buffer = "";
    const limit = setTimeout(() => {
      req.destroy();
      reject(new Error(`only ${events.length} of ${wanted} events came`));
    }, ms);
    const req = http.get({ host: url.hostname, port: url.port, path: "/events", agent: false }, (res) => {
      assert.match(res.headers["content-type"], /text\/event-stream/);
      res.setEncoding("utf8");
      res.on("data", (chunk) => {
        buffer += chunk;
        let at;
        while ((at = buffer.indexOf("\n\n")) >= 0) {
          const block = buffer.slice(0, at);
          buffer = buffer.slice(at + 2);
          const name = /^event: (.*)$/m.exec(block);
          const data = /^data: (.*)$/m.exec(block);
          if (name) events.push({ name: name[1], data: data ? data[1] : "" });
        }
        if (events.length >= wanted) {
          clearTimeout(limit);
          req.destroy();
          resolve(events);
        }
      });
    });
    req.on("error", () => {});
  });
}

const pause = (ms) => new Promise((resolve) => setTimeout(resolve, ms));

// Waits until the server has exactly this one stream open, then changes the file.
async function changeWhenListening(dev, file, text) {
  for (let i = 0; i < 150 && dev.clients() !== 1; i++) await pause(20);
  assert.equal(dev.clients(), 1, "the event stream is open");
  fs.writeFileSync(file, text);
}

// A destroyed stream leaves the server's set a moment later.
async function noStreams(dev) {
  for (let i = 0; i < 150 && dev.clients() !== 0; i++) await pause(20);
  assert.equal(dev.clients(), 0);
}

test("the event stream sends a css event for a stylesheet and a reload for anything else", async () => {
  await withServer({}, async ({ dev, origin, space }) => {
    const css = listen(origin, 1);
    await changeWhenListening(dev, path.join(space.ui, "overlay.css"), "body { color: green; }");
    assert.deepEqual(await css, [{ name: "css", data: "overlay.css" }]);

    await noStreams(dev);
    const js = listen(origin, 1);
    await changeWhenListening(dev, path.join(space.ui, "sub", "inner.js"), "window.inner = 4;");
    const [reload] = await js;
    assert.equal(reload.name, "reload");
    assert.match(reload.data, /^sub(\/inner\.js)?$/, "the folder or the file, whichever the system reports");

    await noStreams(dev);
    const mixed = listen(origin, 1);
    await changeWhenListening(dev, path.join(space.ui, "overlay.css"), "body { color: pink; }");
    fs.writeFileSync(path.join(space.ui, "first.js"), "window.first = 9;");
    const [event] = await mixed;
    assert.equal(event.name, "reload", "a css change that comes with a script change reloads");
  });
});

test("close ends open event streams and stops the watchers", async () => {
  const space = workspace();
  const dev = createDevServer({ uiDir: space.ui, backgroundsDir: space.backgrounds });
  const address = await dev.listen(0);
  const ended = new Promise((resolve) => {
    const req = http.get({ host: "127.0.0.1", port: address.port, path: "/events", agent: false }, (res) => {
      res.on("data", () => {});
      res.on("end", resolve);
      res.on("close", resolve);
    });
    req.on("error", resolve);
  });
  for (let i = 0; i < 100 && dev.clients() === 0; i++) await pause(20);
  assert.equal(dev.clients(), 1);
  await dev.close();
  await ended;
  assert.equal(dev.clients(), 0);
  fs.rmSync(space.root, { recursive: true, force: true });
});

test("the workbench has the controls the task asks for", () => {
  const html = fs.readFileSync(path.join(TOOL, "workbench.html"), "utf8");
  for (const id of ["scenario", "lang", "theme", "size", "backdrop", "view", "scale"]) {
    assert.match(html, new RegExp(`id="${id}"`), `control ${id}`);
  }
  const js = fs.readFileSync(path.join(TOOL, "workbench.js"), "utf8");
  for (const name of ["live-changed", "overlay-hover", "lobby-tribes-changed", "cards-changed", "overlay-settings-changed", "app-settings-changed"]) {
    assert.ok(js.includes(`"${name}"`), `button for ${name}`);
  }
  for (const preset of ["1920x1080", "2560x1440", "1280x720"]) assert.ok(html.includes(preset), preset);
});

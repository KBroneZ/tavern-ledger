"use strict";
// The overlay gallery (tools/overlay-gallery): the stand-in for the app's
// bridge answers everything the overlay page asks, its theme colours are the
// app's, every scenario is well formed, and the built file is one small page.
// Nothing here writes into the repository: builds go to a temporary folder.
// No DOM library: the page itself is checked by opening the built file.
const test = require("node:test");
const assert = require("node:assert/strict");
const fs = require("node:fs");
const os = require("node:os");
const path = require("node:path");

const UI = path.join(__dirname, "..", "ui");
const SRC = path.join(__dirname, "..", "src");
const GALLERY = path.join(__dirname, "..", "..", "..", "tools", "overlay-gallery");

require("../ui/i18n.js");
require("../ui/strings/common.js");
require("../ui/strings/overlay.js");
const { oddsView } = require("../ui/odds-words.js");
const bridge = require(path.join(GALLERY, "bridge.js"));
const { THEME_VARS } = require(path.join(GALLERY, "theme-vars.js"));
const { SCENARIOS, scenariosFor } = require(path.join(GALLERY, "scenarios", "index.js"));
const { build, inScript, jsonForScript, checkSelfContained } = require(path.join(GALLERY, "build.js"));

const read = (file) => fs.readFileSync(file, "utf8");
const html = read(path.join(UI, "overlay.html"));
const pageScripts = [...html.matchAll(/<script src="([^"]+)"/g)].map((m) => m[1]);

// Every command and event name written as a literal in the page's scripts.
function literalNames() {
  const commands = new Set();
  const events = new Set();
  for (const file of pageScripts) {
    const text = read(path.join(UI, file));
    for (const m of text.matchAll(/\b(invoke|call|send)\(\s*"([A-Za-z0-9_-]+)"/g)) commands.add(m[2]);
    for (const m of text.matchAll(/\b(listen|onEvent)\(\s*"([A-Za-z0-9_-]+)"/g)) events.add(m[2]);
  }
  return { commands, events };
}

test("the page loads its scripts from overlay.html, and the scan finds the bridge calls", () => {
  assert.ok(pageScripts.includes("overlay.js"));
  const { commands, events } = literalNames();
  for (const name of ["overlay_state", "overlay_scene", "overlay_settings", "card_names", "app_settings", "log_ui_error"]) {
    assert.ok(commands.has(name), `the scan lost ${name}`);
  }
  for (const name of ["live-changed", "overlay-hover", "overlay-settings-changed", "app-settings-changed"]) {
    assert.ok(events.has(name), `the scan lost ${name}`);
  }
});

test("the stand-in answers every command the page's scripts call", () => {
  const { commands } = literalNames();
  const missing = [...commands].filter((name) => !bridge.COMMANDS.includes(name));
  assert.deepEqual(missing, [], `no answer for: ${missing.join(", ")}`);
});

test("the stand-in knows every event the page's scripts listen to", () => {
  const { events } = literalNames();
  const missing = [...events].filter((name) => !bridge.EVENTS.includes(name));
  assert.deepEqual(missing, [], `not known: ${missing.join(", ")}`);
});

test("the stand-in answers the overlay_set_ calls with the settings view and the data commands with data", async () => {
  const scenario = SCENARIOS.find((s) => s.id === "solo-simple");
  const tauri = bridge.createTauri(scenario, "en");
  const { invoke } = tauri.core;
  const view = await invoke("overlay_set_detail", { detail: "advanced" });
  assert.equal(view.detail, "simple", "a setter changes nothing: the frame stays as the scenario says");
  assert.deepEqual(await invoke("overlay_settings"), view);
  assert.equal(await invoke("overlay_state"), scenario.state.game);
  assert.deepEqual(await invoke("app_settings"), { language: "en" });
  assert.equal(await invoke("log_ui_error", { kind: "x", message: "y", place: "z" }), null);
  await assert.rejects(() => invoke("not_a_command"), /not_a_command/);
});

test("card_names answers each id from the scenario's names, in the language, and null for the rest", async () => {
  const scenario = SCENARIOS.find((s) => s.id === "solo-advanced");
  const [id, names] = Object.entries(scenario.state.names)[0];
  const en = await bridge.createTauri(scenario, "en").core.invoke("card_names", { ids: [id, "EX_NOT_A_CARD"], lang: "en" });
  const es = await bridge.createTauri(scenario, "es").core.invoke("card_names", { ids: [id], lang: "es" });
  assert.equal(en[id], names.en);
  assert.equal(es[id], names.es);
  assert.equal(en.EX_NOT_A_CARD, null);
});

test("overlay-hover is sent to a listener once it listens, when the scenario has a hover", async () => {
  const scenario = SCENARIOS.find((s) => s.id === "hover-solo");
  const tauri = bridge.createTauri(scenario, "en");
  const got = [];
  await tauri.event.listen("overlay-hover", (event) => got.push(event.payload));
  await new Promise((resolve) => setTimeout(resolve, 0));
  assert.equal(got.length, 1);
  assert.deepEqual(Object.keys(got[0]).sort(), ["card", "member", "place", "slot"]);
  assert.deepEqual(Object.keys(got[0].card).sort(), ["max_height", "width", "x", "y"]);
  const quiet = bridge.createTauri(SCENARIOS.find((s) => s.id === "solo-simple"), "en");
  const none = [];
  await quiet.event.listen("overlay-hover", (event) => none.push(event));
  await new Promise((resolve) => setTimeout(resolve, 0));
  assert.deepEqual(none, []);
});

// The theme colours: the page has no colours without them, and the app's
// (overlay_layout.rs) are the truth. Read from the Rust source.
function rustThemes() {
  const text = read(path.join(SRC, "overlay_layout.rs"));
  const hex = (r, g, b) => `#${[r, g, b].map((v) => v.toString(16).padStart(2, "0")).join("")}`;
  const palette = (name) => {
    const body = text.match(new RegExp(`Theme::${name} => Palette \\{([^}]*)\\}`))[1];
    const out = {};
    for (const m of body.matchAll(/(\w+): \((0x[0-9a-f]{2}), (0x[0-9a-f]{2}), (0x[0-9a-f]{2})\)/g)) {
      out[m[1]] = { rgb: [m[2], m[3], m[4]].map(Number), hex: hex(Number(m[2]), Number(m[3]), Number(m[4])) };
    }
    return out;
  };
  const cssBody = text.slice(text.indexOf("pub fn css_vars"), text.indexOf("#[cfg(test)]", text.indexOf("pub fn css_vars")));
  const names = [...cssBody.matchAll(/"(--[a-z-]+)"/g)].map((m) => m[1]);
  const opacity = Number(text.match(/DEFAULT_OPACITY: f64 = ([0-9.]+)/)[1]);
  return { palette, names: [...new Set(names)].sort(), opacity };
}

test("the copied theme colours have the app's names and values", () => {
  const rust = rustThemes();
  for (const [theme, rustName] of [["glass", "Glass"], ["contrast", "Contrast"]]) {
    const p = rust.palette(rustName);
    const expected = {
      "--panel-bg": `rgba(${p.panel.rgb.join(", ")}, ${rust.opacity.toFixed(2)})`,
      "--panel-solid": p.panel.hex,
      "--text": p.text.hex,
      "--dim": p.dim.hex,
      "--accent": p.accent.hex,
      "--won": p.won.hex,
      "--lost": p.lost.hex,
      "--amber": p.amber.hex,
      "--info": p.info.hex,
      "--muted": p.muted.hex,
      "--danger": p.danger.hex,
      "--border": p.border.hex,
    };
    assert.deepEqual(THEME_VARS[theme], expected, `${theme} differs from overlay_layout.rs`);
    assert.deepEqual(Object.keys(THEME_VARS[theme]).sort(), rust.names, `${theme} names differ from css_vars`);
  }
  assert.deepEqual(Object.keys(THEME_VARS).sort(), ["contrast", "glass"]);
});

// The scenarios.
const FORBIDDEN = [/\b[DT]-\d+\b/, /\bsession\b/i, /\bBACON_/, /\bBG\d+/, /\bTAG_/, /\bhub\b/i, /\bclaude\b/i];

test("scenario ids are unique and the list covers what the task asks for", () => {
  const ids = SCENARIOS.map((s) => s.id);
  assert.equal(new Set(ids).size, ids.length);
  for (const id of [
    "waiting", "lobby", "solo-simple", "solo-advanced", "duos", "odds", "gap-no-board-seen", "gap-no-card-data",
    "gap-not-modelled", "gap-bad-board", "gap-order", "gap-missing-board", "hover-solo", "hover-duos", "damage-cap",
    "unreadable", "game-over", "high-contrast",
  ]) {
    assert.ok(ids.includes(id), `missing scenario ${id}`);
  }
});

test("every scenario is captioned in English and Spanish, in public wording", () => {
  for (const s of SCENARIOS) {
    for (const lang of ["en", "es"]) {
      const caption = s.caption[lang];
      for (const field of ["title", "sees", "source", "rule"]) {
        assert.equal(typeof caption[field], "string", `${s.id} ${lang} ${field}`);
        assert.ok(caption[field].trim().length > (field === "title" ? 3 : 10), `${s.id} ${lang} ${field} is empty`);
        for (const pattern of FORBIDDEN) assert.doesNotMatch(caption[field], pattern, `${s.id} ${lang} ${field}`);
      }
    }
    assert.notEqual(s.caption.en.sees, s.caption.es.sees, `${s.id} is not translated`);
  }
});

test("every scenario names the part of the game window it is about, inside the window", () => {
  for (const s of SCENARIOS) {
    const f = s.focus;
    assert.ok(f && [f.x, f.y, f.w, f.h].every(Number.isFinite), `${s.id} has no focus`);
    assert.ok(f.x >= 0 && f.y >= 0 && f.w > 0 && f.h > 0 && f.x + f.w <= 1920 && f.y + f.h <= 1080, `${s.id} focus is outside the window`);
  }
});

test("every card and hero id is an invented one the page accepts, and has a name in both languages", () => {
  const ID = /^[A-Za-z0-9_]{1,64}$/;
  for (const s of SCENARIOS) {
    const g = s.state.game;
    if (!g) continue;
    const ids = new Set();
    for (const h of [g.hero.value, g.teammate_hero.value]) if (h) ids.add(h.id);
    for (const o of g.opponents) {
      o.heroes.forEach((h) => ids.add(h.id));
      o.boards.forEach((b) => b.minions.forEach((m) => ids.add(m.card_id)));
      o.shown.forEach((seat) => seat.trinkets.forEach((t) => t.card_id && ids.add(t.card_id)));
      o.possible.forEach((p) => p.cards.forEach((c) => ids.add(c.id)));
    }
    for (const id of ids) {
      assert.match(id, ID, `${s.id}: ${id}`);
      assert.match(id, /^EX_/, `${s.id}: ${id} is not an invented id`);
      assert.ok(s.state.names[id] && s.state.names[id].en && s.state.names[id].es, `${s.id}: no names for ${id}`);
    }
  }
});

test("the odds scenarios give odds or the note the page says in words", () => {
  const view = (id, advanced) => oddsView(SCENARIOS.find((s) => s.id === id).state.game.odds, advanced, () => null);
  assert.equal(view("odds", false).kind, "odds");
  assert.equal(view("odds", true).kind, "odds");
  assert.ok(view("odds", true).lines.length > 0, "Advanced adds the damage lines");
  for (const gap of ["no-board-seen", "no-card-data", "not-modelled", "bad-board", "order", "missing-board"]) {
    assert.equal(view(`gap-${gap}`, false).kind, "note", gap);
  }
});

test("a hover scenario hovers a place that is an opponent's and not the player's", () => {
  for (const id of ["hover-solo", "hover-duos", "hover-advanced"]) {
    const s = SCENARIOS.find((x) => x.id === id);
    const g = s.state.game;
    const target = g.opponents.find((o) => o.place.value === s.state.hover.place);
    assert.ok(target, `${id}: nobody is at the hovered place`);
    assert.notEqual(g.own_place.value, s.state.hover.place);
    if (id !== "hover-advanced") assert.ok(target.boards.length > 0);
  }
  assert.equal(SCENARIOS.find((s) => s.id === "hover-duos").state.hover.member, 0);
  assert.equal(SCENARIOS.find((s) => s.id === "hover-solo").state.hover.member, null);
});

test("a frame is built per scenario and example data stays out of the live shapes", () => {
  const solo = SCENARIOS.find((s) => s.id === "solo-simple").state;
  assert.equal(solo.settings.detail, "simple");
  assert.equal(SCENARIOS.find((s) => s.id === "solo-advanced").state.settings.detail, "advanced");
  assert.equal(SCENARIOS.find((s) => s.id === "high-contrast").state.settings.theme, "contrast");
  assert.equal(SCENARIOS.find((s) => s.id === "waiting").state.game, null);
  assert.equal(SCENARIOS.find((s) => s.id === "unreadable").state.game.phase, "unreadable");
  assert.equal(SCENARIOS.find((s) => s.id === "game-over").state.game.phase, "over");
  assert.equal(SCENARIOS.find((s) => s.id === "lobby").state.lobby.in_lobby, true);
});

test("the counters scenario is added only once the page has a counters slot", () => {
  const without = scenariosFor('<div class="slot" data-panel="status"></div>').map((s) => s.id);
  const withSlot = scenariosFor('<div class="slot" data-panel="counters" data-focus="slot:counters"></div>').map((s) => s.id);
  assert.ok(!without.includes("counters"));
  assert.ok(withSlot.includes("counters"));
  assert.equal(withSlot.length, without.length + 1);
  const counters = SCENARIOS.find((s) => s.id === "counters");
  assert.ok(Array.isArray(counters.state.game.counters) && counters.state.game.counters.length >= 3);
});

// The built file.
function buildToTemp() {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), "overlay-gallery-test-"));
  const out = path.join(dir, "gallery.html");
  const result = build({ uiDir: UI, outFile: out });
  return { dir, out, result, text: read(out) };
}

test("the build writes one self-contained file under 2 MB, to the place it is told", () => {
  const { dir, out, result, text } = buildToTemp();
  try {
    assert.deepEqual(fs.readdirSync(dir), ["gallery.html"]);
    assert.equal(result.file, out);
    assert.ok(Buffer.byteLength(text) < 2 * 1024 * 1024, `${Buffer.byteLength(text)} bytes`);
    for (const [what, page] of [["the gallery page", text], ["the overlay page", decodeDoc(text)]]) {
      assert.doesNotMatch(page, /<script[^>]*\ssrc=/, `${what}: no script is loaded from a file`);
      assert.doesNotMatch(page, /<link[^>]*>/, `${what}: no stylesheet is loaded from a file`);
      assert.doesNotMatch(page, /url\(["']?fonts\//, `${what}: the fonts are inside the file`);
    }
    const doc = decodeDoc(text);
    assert.equal((doc.match(/<script>/g) || []).length, pageScripts.length + 1, "every script is inline, plus the stand-in");
    assert.match(doc, /<style>/);
    assert.match(doc, /url\("data:font\/woff2;base64,/);
  } finally {
    fs.rmSync(dir, { recursive: true, force: true });
  }
});

test("the page is in the file once, not once per scenario, and every scenario is reachable by its id", () => {
  const { dir, text } = buildToTemp();
  try {
    // The page is stored as one encoded block: its own code is not in the text.
    assert.equal(text.split("function renderOpponents").length - 1, 0);
    const doc = decodeDoc(text);
    assert.equal(doc.split("function renderOpponents").length - 1, 1);
    assert.equal(doc.split("function renderOdds").length - 1, 1);
    for (const s of scenariosFor(html)) assert.ok(text.includes(`"${s.id}"`), s.id);
    assert.ok(doc.indexOf("TLGalleryBridge") < doc.indexOf("function renderOpponents"), "the stand-in comes before the page's scripts");
    assert.ok(doc.indexOf("TLGalleryBridge") < doc.indexOf("TLI18n"), "the stand-in comes before the language script");
  } finally {
    fs.rmSync(dir, { recursive: true, force: true });
  }
});

test("every frame says it is an example, in both languages, and the page takes ?lang and ?scenario", () => {
  const { dir, text } = buildToTemp();
  try {
    assert.match(text, /Example/);
    assert.match(text, /Ejemplo/);
    assert.match(text, /URLSearchParams/);
    assert.match(text, /"lang"/);
    assert.match(text, /"scenario"/);
    assert.match(text, /name="viewport"/);
  } finally {
    fs.rmSync(dir, { recursive: true, force: true });
  }
});

test("text that would end a script early is escaped, and an unrecognised tag stops the build", () => {
  assert.equal(inScript("a</script>b</SCRIPT"), "a<\\/script>b<\\/script");
  assert.ok(!jsonForScript({ t: "</script><!--" }).includes("<"));
  assert.deepEqual(JSON.parse(jsonForScript({ t: "</script><!--" })), { t: "</script><!--" });
  assert.throws(() => checkSelfContained('<script src="x.js" defer></script>'), /does not inline/);
  assert.throws(() => checkSelfContained('<link rel="stylesheet" href="a.css" />'), /does not inline/);
  assert.throws(() => checkSelfContained("a{src:url(fonts/a.woff2)}"), /does not inline/);
  checkSelfContained('<script>x</script><style>a{src:url("data:font/woff2;base64,AA")}</style>');
});

test("every frame of the gallery page gets an Example badge, in both languages", () => {
  const js = read(path.join(GALLERY, "gallery.js"));
  const shot = js.slice(js.indexOf("function shot("), js.indexOf("document.documentElement.lang"));
  assert.match(shot, /el\("span", "badge", words\.example\)/, "shot() adds the badge");
  assert.match(js, /example: "Example"/);
  assert.match(js, /example: "Ejemplo"/);
});

test("the build does not touch the page's own files", () => {
  const before = pageScripts.map((f) => read(path.join(UI, f)));
  const { dir } = buildToTemp();
  fs.rmSync(dir, { recursive: true, force: true });
  assert.deepEqual(pageScripts.map((f) => read(path.join(UI, f))), before);
});

test("the counters scenario shows in the build only when the page has the slot", () => {
  const { dir, text } = buildToTemp();
  try {
    assert.equal(text.includes('"counters"'), html.includes('data-panel="counters"'));
  } finally {
    fs.rmSync(dir, { recursive: true, force: true });
  }
});

// The page is stored base64-encoded in the built file (so its scripts are not
// scripts of the gallery's own page); the gallery page reads it back the same way.
function decodeDoc(text) {
  const m = text.match(/<script type="text\/plain" id="overlay-doc">([^<]*)<\/script>/);
  assert.ok(m, "the encoded page is in the file");
  return Buffer.from(m[1].trim(), "base64").toString("utf8");
}

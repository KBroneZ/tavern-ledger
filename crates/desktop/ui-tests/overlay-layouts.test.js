"use strict";
// The overlay gallery's layout round (tools/overlay-gallery/layouts): every
// variant shows the same example games, in Solo and Duos, a Recruit phase and
// combat, at both window sizes; a variant changes the page only inside the
// gallery's frame; and the built file is one self-contained page. Builds go
// to a temporary folder, never into the repository.
const test = require("node:test");
const assert = require("node:assert/strict");
const fs = require("node:fs");
const os = require("node:os");
const path = require("node:path");

const UI = path.join(__dirname, "..", "ui");
const GALLERY = path.join(__dirname, "..", "..", "..", "tools", "overlay-gallery");

const { SCENARIOS, PAGE, VARIANTS, FRAMES } = require(path.join(GALLERY, "layouts", "scenarios.js"));
const layouts = require(path.join(GALLERY, "layouts", "variants.js"));
const backdrop = require(path.join(GALLERY, "layouts", "backdrop.js"));
const defaults = require(path.join(GALLERY, "default-layout.js"));
const { build } = require(path.join(GALLERY, "build.js"));

const read = (file) => fs.readFileSync(file, "utf8");
const FORBIDDEN = [/\b[DT]-\d+\b/, /\bsession\b/i, /\bBACON_/, /\bTAG_/, /\bhub\b/i, /\bclaude\b/i];

test("every variant has the same frames with the same example games", () => {
  const ids = SCENARIOS.map((s) => s.id);
  assert.equal(new Set(ids).size, ids.length);
  assert.deepEqual(VARIANTS.map((v) => v.layout.place), ["above", "below", "beside"]);
  for (const v of VARIANTS) {
    const frames = SCENARIOS.filter((s) => s.id.startsWith(`${v.id}-`));
    assert.equal(frames.length, FRAMES.length, v.id);
    frames.forEach((s, i) => {
      const first = SCENARIOS.filter((x) => x.id.startsWith(`${VARIANTS[0].id}-`))[i];
      assert.deepEqual(s.state.game, first.state.game, `${s.id} shows the same game as ${first.id}`);
      assert.deepEqual(s.screen, first.screen);
      assert.deepEqual(s.state.layout, v.layout);
    });
  }
});

test("the frames cover Solo and Duos, a Recruit phase and combat, 1920x1080 and 2560x1440", () => {
  const seen = (pick) => new Set(FRAMES.map(pick));
  assert.deepEqual([...seen((f) => f.mode)].sort(), ["duos", "solo"]);
  assert.deepEqual([...seen((f) => f.phase)].sort(), ["combat", "recruit"]);
  const screens = new Set(SCENARIOS.map((s) => s.screen.join("x")));
  assert.deepEqual([...screens].sort(), ["1920x1080", "2560x1440"]);
  for (const s of SCENARIOS) {
    const recruit = s.state.backdrop.phase === "recruit";
    assert.equal(s.state.game.odds !== null, recruit, `${s.id}: odds only in a Recruit phase`);
    assert.equal(s.state.game.is_duos, s.state.backdrop.duos, s.id);
    assert.ok(s.state.game.counters.length > 0, s.id);
  }
});

test("today's frame shows the page as it ships and the hide-all frame hides it", () => {
  assert.equal(SCENARIOS.find((s) => s.id.startsWith("today-")).state.layout, null);
  assert.equal(SCENARIOS.find((s) => s.id.startsWith("hide-all-")).state.layout.hideAll, true);
});

test("every frame is captioned in English and Spanish with what changes and why, in public wording", () => {
  assert.deepEqual(PAGE.fields.map((f) => f.key), ["change", "why"]);
  for (const s of SCENARIOS) {
    for (const lang of ["en", "es"]) {
      for (const field of ["title", "change", "why"]) {
        const text = s.caption[lang][field];
        assert.ok(typeof text === "string" && text.trim().length > 10, `${s.id} ${lang} ${field}`);
        for (const pattern of FORBIDDEN) assert.doesNotMatch(text, pattern, `${s.id} ${lang} ${field}`);
      }
    }
    assert.notEqual(s.caption.en.change, s.caption.es.change, `${s.id} is not translated`);
  }
  for (const lang of ["en", "es"]) {
    for (const key of ["title", "intro", "foot"]) {
      assert.ok(PAGE[lang][key].length > 10, `${lang} ${key}`);
      for (const pattern of FORBIDDEN) assert.doesNotMatch(PAGE[lang][key], pattern, `page ${lang} ${key}`);
    }
  }
});

test("every frame's zoomed region is inside its window", () => {
  for (const s of SCENARIOS) {
    const [w, h] = s.screen;
    const f = s.focus;
    assert.ok(f.x >= 0 && f.y >= 0 && f.x + f.w <= w && f.y + f.h <= h, s.id);
  }
});

test("the stack order puts the counters above, below or beside the opponents, the tribes always just before them", () => {
  const ids = ["status", "counters", "tribes", "opponents", "odds", "legend", "lobby", "heroes"];
  const stack = (place) => layouts.ordered(ids, layouts.ORDERS[place]).filter((id) => ["counters", "tribes", "opponents"].includes(id));
  assert.deepEqual(stack("above"), ["counters", "tribes", "opponents"]);
  assert.deepEqual(stack("below"), ["tribes", "opponents", "counters"]);
  for (const place of layouts.PLACES) assert.deepEqual([...layouts.ordered(ids, layouts.ORDERS[place])].sort(), [...ids].sort(), place);
  assert.match(layouts.css({ place: "beside" }), /grid-template-columns/);
  assert.doesNotMatch(layouts.css({ place: "above" }), /grid-template-columns/);
  assert.match(layouts.css({ place: "above", hideAll: true }), /visibility: hidden/);
});

// Just enough of a window for apply().
function fakeWindow() {
  const slot = { hidden: true, removeAttribute(name) { if (name === "hidden") this.hidden = false; } };
  const added = { head: [], body: [] };
  const element = (tag) => ({ tag, className: "", textContent: "" });
  const document = {
    querySelector: (sel) => (sel === '.slot[data-panel="counters"]' ? slot : null),
    createElement: element,
    head: { append: (n) => added.head.push(n) },
    body: { append: (n) => added.body.push(n) },
  };
  return { win: { document }, slot, added };
}

test("a variant turns the counters panel on as the page defines it, in its order, and adds its styles", () => {
  const { win, slot, added } = fakeWindow();
  layouts.apply(win, { place: "below" }, "es");
  const pageApi = { ENABLED: false, rows: () => [], panelIds: (ids) => ids.filter((id) => id !== "counters") };
  win.TLCounters = pageApi;
  assert.equal(win.TLCounters.ENABLED, true);
  assert.equal(win.TLCounters.rows, pageApi.rows);
  assert.deepEqual(win.TLCounters.panelIds(["status", "counters", "tribes", "opponents"]), ["status", "tribes", "opponents", "counters"]);
  assert.equal(slot.hidden, false);
  assert.equal(added.head.length, 1);
  assert.equal(added.body.length, 0);
});

test("the hide-all frame says the key in the frame's language; no layout changes nothing", () => {
  const es = fakeWindow();
  layouts.apply(es.win, { place: "above", hideAll: true }, "es");
  assert.match(es.added.body[0].textContent, /Ctrl\+Mayús\+H/);
  const en = fakeWindow();
  layouts.apply(en.win, { place: "above", hideAll: true }, "en");
  assert.match(en.added.body[0].textContent, /Ctrl\+Shift\+H/);
  const none = fakeWindow();
  layouts.apply(none.win, null, "en");
  assert.equal(none.win.TLCounters, undefined);
  assert.equal(none.slot.hidden, true);
  assert.throws(() => layouts.apply(fakeWindow().win, { place: "left" }, "en"), /unknown counters place/);
  const noSlot = fakeWindow();
  noSlot.win.document.querySelector = () => null;
  assert.throws(() => layouts.apply(noSlot.win, { place: "above" }, "en"), /no counters slot/);
});

test("the overlay page has the counters slot the variants show", () => {
  assert.match(read(path.join(UI, "overlay.html")), /<div class="slot"[^>]*data-panel="counters"/);
});

// The page's own counters-words.js, run in a window after a variant: the
// panel's switch and list are the ones the page's scripts then read.
test("with a variant, the page's own counters script turns the panel on and lists it in the variant's order", () => {
  const vm = require("node:vm");
  const { win } = fakeWindow();
  layouts.apply(win, { place: "below" }, "en");
  win.TLI18n = require("../ui/i18n.js");
  vm.runInNewContext(read(path.join(UI, "counters-words.js")), { window: win });
  assert.equal(win.TLCounters.ENABLED, true);
  assert.equal(typeof win.TLCounters.rows, "function");
  // The list overlay-layout.js asks for (PANEL_IDS), as the page writes it.
  const asked = read(path.join(UI, "overlay-layout.js")).match(/const PANEL_IDS = window\.TLCounters\.panelIds\((\[[^\]]*\])\)/);
  assert.ok(asked, "overlay-layout.js builds its panel list through TLCounters.panelIds");
  const ids = JSON.parse(asked[1]);
  const stack = win.TLCounters.panelIds(ids).filter((id) => ["counters", "tribes", "opponents"].includes(id));
  assert.deepEqual(stack, ["tribes", "opponents", "counters"]);
  // The stack is laid out in that list's order.
  assert.match(read(path.join(UI, "overlay-layout.js")), /for \(const id of PANEL_IDS\)[\s\S]*?stack\.append\(\.\.\.inStack\)/);
});

test("the backdrop draws the leaderboard where the app measures it, in pairs in Duos", () => {
  const area = defaults.leaderboardArea(1600, 900);
  const solo = backdrop.svg({ phase: "recruit", duos: false });
  assert.ok(solo.includes(`x="${area.x + 3}"`), "the leaderboard's slots start at the measured area");
  assert.equal((solo.match(/<rect x="[^"]+" y="[^"]+" width="[^"]+" height="[^"]+" rx="10" fill="#352c24"/g) || []).length, 8);
  const duos = backdrop.svg({ phase: "combat", duos: true });
  assert.equal((duos.match(/rx="12" fill="none"/g) || []).length, 4);
  assert.doesNotMatch(solo + duos, /<image|href=/, "shapes only, no pictures");
});

test("the layout round builds to one self-contained file and leaves the page's own files as they are", () => {
  const html = read(path.join(UI, "overlay.html"));
  const scripts = [...html.matchAll(/<script (?:type="module" )?src="([^"]+)"/g)].map((m) => m[1]);
  const before = [html, ...scripts.map((f) => read(path.join(UI, f)))];
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), "overlay-layouts-test-"));
  try {
    const out = path.join(dir, "layouts.html");
    const result = build({ uiDir: UI, outFile: out, layouts: true });
    assert.deepEqual(fs.readdirSync(dir), ["layouts.html"]);
    assert.deepEqual(result.scenarios, SCENARIOS.map((s) => s.id));
    const text = read(out);
    assert.ok(Buffer.byteLength(text) < 2 * 1024 * 1024);
    const doc = Buffer.from(text.match(/<script type="text\/plain" id="overlay-doc">([^<]*)<\/script>/)[1].trim(), "base64").toString("utf8");
    assert.doesNotMatch(doc, /<script[^>]*\ssrc=|<link[^>]*>/);
    assert.ok(doc.indexOf("TLGalleryLayouts.apply") < doc.indexOf("window.TLCounters = api"), "the variant comes before the counters script");
    assert.match(text, /Overlay layouts: pick one/);
    assert.match(text, /"screen":\[2560,1440\]/);
  } finally {
    fs.rmSync(dir, { recursive: true, force: true });
  }
  assert.deepEqual([html, ...scripts.map((f) => read(path.join(UI, f)))], before);
});

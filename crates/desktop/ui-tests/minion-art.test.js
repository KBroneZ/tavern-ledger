"use strict";
// The shared minion drawing (ui/minion-art/): the layout reader, the layer
// order, and a minion built on a small fake page. The WebGL effects are
// checked in a real browser (the workbench), here a stand-in takes their place.
const test = require("node:test");
const assert = require("node:assert/strict");
const path = require("node:path");
const { pathToFileURL } = require("node:url");

const dir = path.join(__dirname, "..", "ui", "minion-art");
const load = (file) => import(pathToFileURL(path.join(dir, file)).href);

// The shape of the texture set's layout.json, with round made-up numbers.
const LAYOUT = {
  canvas: { width: 400, height: 500 },
  frame: { box: [0, 0, 400, 500] },
  taunt: { box: [-20, 0, 420, 600] },
  parts: {
    gem_attack: { box: [10, 300, 150, 450] },
    gem_health: { box: [280, 300, 380, 450] },
    skull: { box: [120, 400, 280, 540] },
    trigger: { box: [160, 420, 260, 540] },
    rally: { box: [120, 420, 280, 520] },
    venomous: { box: [170, -40, 240, 90] },
    poisonous: { box: [150, 400, 250, 540] },
    divine_shield: { box: [-90, -60, 500, 650] },
    reborn_glow: { box: [-150, -80, 540, 610] },
  },
  windfury: { box_canvas_px: [-30, 100, 430, 420], size: [230, 160], umin: -0.3, umax: 1.8 },
  stealth: { box_canvas_px: [20, 20, 380, 500], size: [180, 240], u: [0.002, -0.08], v: [-0.002, 1.2] },
  fx_canvas_pad_px: 100,
};
const ALL = [
  "frame_normal", "frame_golden", "oval_mask", "taunt_normal", "taunt_golden", "divine_shield", "reborn_glow",
  "venomous", "poisonous", "skull", "gem_attack", "gem_health", "trigger", "rally",
];
const has = (list) => (name) => list.includes(name);
const m = (keywords = [], extra = {}) => ({ card_id: "C_1", atk: 3, health: 4, golden: false, keywords, ...extra });

test("the layout is read into boxes, and what is not a box is left out", async () => {
  const { readLayout } = await load("layout.js");
  const L = readLayout(LAYOUT);
  assert.deepEqual([L.width, L.height], [400, 500]);
  assert.deepEqual(L.boxes.gem_attack, [10, 300, 150, 450]);
  assert.deepEqual(L.boxes.taunt_normal, [-20, 0, 420, 600]);
  assert.deepEqual(L.boxes.taunt_golden, [-20, 0, 420, 600]);
  assert.deepEqual(L.boxes.frame, [0, 0, 400, 500]);
  assert.equal(L.windfury.size[0], 230);
  assert.equal(L.stealth.pad, 100);
  const broken = structuredClone(LAYOUT);
  broken.parts.skull.box = [0, 0, "x", 3];
  broken.parts.rally.box = [10, 10, 5, 5];
  delete broken.parts.trigger;
  broken.windfury.size = [0, 100];
  const B = readLayout(broken);
  for (const name of ["skull", "rally", "trigger"]) assert.equal(B.boxes[name], undefined, name);
  assert.equal(B.windfury, null);
  assert.ok(B.stealth, "the other effect is kept");
});

test("sizes and boxes nobody drew are refused: a host cannot make a minion cover the screen or fill the GPU", async () => {
  const { readLayout } = await load("layout.js");
  const huge = structuredClone(LAYOUT);
  huge.parts.skull.box = [-5000, 0, 5000, 5000];
  huge.parts.rally.box = [0, 0, 1e9, 10];
  huge.windfury.size = [4096, 4096];
  huge.stealth.box_canvas_px = [0, 0, 40000, 500];
  const L = readLayout(huge);
  assert.equal(L.boxes.skull, undefined);
  assert.equal(L.boxes.rally, undefined);
  assert.ok(L.boxes.gem_attack, "ordinary parts stay");
  assert.equal(L.windfury, null);
  assert.equal(L.stealth, null);
  // What the game really draws reaches past the frame (shield, bubble, glow) and is kept.
  assert.ok(L.boxes.divine_shield && L.boxes.reborn_glow && L.boxes.taunt_normal);
});

test("a layout without a canvas, or a pile of nonsense, is no layout", async () => {
  const { readLayout } = await load("layout.js");
  for (const raw of [null, undefined, 5, "x", [], {}, { canvas: { width: 0, height: 5 } }, { canvas: { width: "4", height: 5 } }]) {
    assert.equal(readLayout(raw), null, JSON.stringify(raw));
  }
  assert.equal(readLayout({ canvas: { width: 1e9, height: 5 } }), null, "a size nobody drew");
});

const names = (ops) => ops.map((o) => (o.kind === "img" ? o.name : o.kind === "fx" ? `fx:${o.fx}` : o.kind === "number" ? `#${o.which}` : o.kind));

test("every layer of a minion with every keyword, back to front, as the game stacks them", async () => {
  const { readLayout, planLayers } = await load("layout.js");
  const L = readLayout(LAYOUT);
  const kws = ["taunt", "divine_shield", "reborn", "venomous", "poisonous", "windfury", "stealth", "deathrattle"];
  const ops = planLayers(m(kws), L, has(ALL));
  assert.deepEqual(names(ops), [
    "reborn_glow", "taunt_normal", "art", "frame_normal", "fx:stealth", "fx:windfury",
    "gem_attack", "gem_health", "#attack", "#health", "skull", "venomous", "poisonous", "divine_shield",
  ]);
});

test("golden minions use the golden frame and the golden Taunt", async () => {
  const { readLayout, planLayers } = await load("layout.js");
  const L = readLayout(LAYOUT);
  const plain = names(planLayers(m(["taunt"], { golden: true }), L, has(ALL)));
  assert.deepEqual(plain.filter((n) => /frame|taunt/.test(n)), ["taunt_golden", "frame_golden"]);
  assert.ok(!plain.includes("frame_normal"));
});

test("Venomous sits above every effect but below Poisonous and Divine Shield", async () => {
  const { readLayout, planLayers } = await load("layout.js");
  const L = readLayout(LAYOUT);
  const order = names(planLayers(m(["venomous", "divine_shield", "poisonous", "deathrattle", "windfury", "stealth"]), L, has(ALL)));
  const at = (n) => order.indexOf(n);
  for (const below of ["fx:stealth", "fx:windfury", "gem_attack", "gem_health", "#attack", "#health", "skull"]) {
    assert.ok(at(below) < at("venomous"), below);
  }
  assert.ok(at("venomous") < at("poisonous"));
  assert.ok(at("poisonous") < at("divine_shield"));
});

test("one bottom icon at a time: the skull, else the trigger; Rally is not drawn (its use is not decided)", async () => {
  const { readLayout, planLayers } = await load("layout.js");
  const L = readLayout(LAYOUT);
  const icon = (kws) => names(planLayers(m(kws), L, has(ALL))).filter((n) => ["skull", "trigger", "rally"].includes(n));
  assert.deepEqual(icon(["deathrattle", "rally", "trigger"]), ["skull"]);
  assert.deepEqual(icon(["rally", "trigger"]), ["trigger"]);
  assert.deepEqual(icon(["rally"]), [], "the texture stays in the set, the layer is not drawn");
  assert.deepEqual(icon([]), []);
});

test("a keyword nobody drew, or a missing texture, leaves only that layer out", async () => {
  const { readLayout, planLayers } = await load("layout.js");
  const L = readLayout(LAYOUT);
  const without = ALL.filter((n) => n !== "skull" && n !== "frame_normal");
  const ops = names(planLayers(m(["deathrattle", "taunt", "cleave", "magnetic"]), L, has(without)));
  assert.ok(!ops.includes("skull"));
  assert.ok(!ops.includes("frame_normal"));
  assert.deepEqual(ops.filter((n) => ["taunt_normal", "art", "gem_attack", "#attack"].includes(n)), ["taunt_normal", "art", "gem_attack", "#attack"]);
});

test("keywords may be missing, odd or repeated without breaking the plan", async () => {
  const { readLayout, planLayers } = await load("layout.js");
  const L = readLayout(LAYOUT);
  for (const keywords of [undefined, null, "taunt", [null, 4, {}], ["taunt", "taunt"]]) {
    const ops = names(planLayers({ card_id: "C_1", atk: 1, health: 1, keywords }, L, has(ALL)));
    assert.ok(ops.includes("art") && ops.includes("frame_normal"), JSON.stringify(keywords));
    assert.ok(ops.filter((n) => n === "taunt_normal").length <= 1);
  }
});

test("mega Windfury shows the Windfury curls", async () => {
  const { readLayout, planLayers } = await load("layout.js");
  const L = readLayout(LAYOUT);
  assert.ok(names(planLayers(m(["mega_windfury"]), L, has(ALL))).includes("fx:windfury"));
});

// --- the minion on a fake page ------------------------------------------

class Node {
  constructor(tag) {
    this.tag = tag;
    this.className = "";
    this.attrs = {};
    this.children = [];
    this.listeners = {};
    this.style = { setProperty: (k, v) => { this.style[k] = v; } };
    this.isConnected = true;
    this.textContent = "";
  }
  setAttribute(k, v) { this.attrs[k] = String(v); }
  append(...nodes) { nodes.forEach((n) => { n.parent = this; this.children.push(n); }); }
  addEventListener(type, fn) { (this.listeners[type] = this.listeners[type] || []).push(fn); }
  fire(type) { (this.listeners[type] || []).forEach((fn) => fn({ type })); }
  replaceChildren(...nodes) { this.children = []; this.append(...nodes); }
  remove() { if (this.parent) this.parent.children = this.parent.children.filter((c) => c !== this); this.removed = true; }
  getContext() { return {}; }
  find(cls) { return this.children.filter((c) => c.className.split(" ").includes(cls)); }
}
const doc = { createElement: (tag) => new Node(tag) };
const URLS = (name) => (ALL.includes(name) || name.startsWith("wf") || name.startsWith("Stealth") || name === "wind1" || name === "st_cover" ? `tex://${name}.webp` : null);

async function make(extra = {}) {
  const { createMinionArt } = await load("minion-art.js");
  const logged = [];
  const art = createMinionArt({
    layout: LAYOUT,
    imageUrl: URLS,
    artImage: (id) => { const img = new Node("img"); img.className = "tlm-art-img"; img.src = `art://${id}`; return img; },
    formatStat: (v) => (v === null || v === undefined ? "?" : String(v)),
    markText: (k) => `word:${k}`,
    log: (message) => logged.push(message),
    document: doc,
    fx: extra.fx === undefined ? { supported: () => true, attach: () => true, setActive() {}, destroy() {} } : extra.fx,
    preload: false,
  });
  return { art, logged };
}

test("a minion is a stack of images placed as a share of the canvas, with its numbers", async () => {
  const { art } = await make();
  const el = art.render(m(["taunt", "divine_shield"], { atk: 12, health: 7 }));
  assert.match(el.className, /\btlm\b/);
  const frame = el.children.find((c) => c.src === "tex://frame_normal.webp");
  assert.equal(frame.style.left, "0%");
  assert.equal(frame.style.width, "100%");
  assert.equal(frame.crossOrigin, "anonymous");
  const shield = el.children.find((c) => c.src === "tex://divine_shield.webp");
  assert.equal(shield.style.left, "-22.5%");
  assert.equal(shield.style.top, "-12%");
  const nums = el.children.filter((c) => c.className.includes("tlm-num"));
  assert.deepEqual(nums.map((n) => n.textContent), ["12", "7"]);
  assert.equal(el.attrs["aria-hidden"], "true");
  assert.ok(el.children.find((c) => c.className.includes("tlm-art")), "the art box");
});

test("the art sits under the oval mask and the card art picture fills it", async () => {
  const { art } = await make();
  const el = art.render(m());
  const box = el.children.find((c) => c.className.includes("tlm-art"));
  assert.match(box.style.maskImage, /tex:\/\/oval_mask\.webp/);
  assert.match(box.style.webkitMaskImage, /tex:\/\/oval_mask\.webp/);
  assert.equal(box.children[0].src, "art://C_1");
});

test("the card art picture carries the art class once", async () => {
  const { art } = await make();
  const picture = art.render(m()).children.find((c) => c.className.includes("tlm-art")).children[0];
  assert.equal(picture.className.split(" ").filter((c) => c === "tlm-art-img").length, 1);
});

test("without the oval mask the art falls back to a plain oval, never an unmasked square", async () => {
  const { createMinionArt } = await load("minion-art.js");
  const art = createMinionArt({
    layout: LAYOUT, imageUrl: (n) => (n === "oval_mask" ? null : URLS(n)), artImage: () => new Node("img"),
    formatStat: String, markText: String, log() {}, document: doc, fx: null, preload: false,
  });
  const box = art.render(m()).children.find((c) => c.className.includes("tlm-art"));
  assert.match(box.className, /tlm-art-plain/);
  assert.equal(box.style.maskImage, undefined);
});

test("a texture that fails to load is dropped and told once; the rest stays", async () => {
  const { art, logged } = await make();
  const first = art.render(m(["taunt"]));
  const second = art.render(m(["taunt"]));
  first.children.find((c) => c.src === "tex://taunt_normal.webp").fire("error");
  second.children.find((c) => c.src === "tex://taunt_normal.webp").fire("error");
  assert.ok(first.children.every((c) => c.src !== "tex://taunt_normal.webp" || c.removed));
  assert.ok(first.children.some((c) => c.src === "tex://frame_normal.webp" && !c.removed));
  assert.equal(logged.length, 1, "one line, not one per minion");
  assert.match(logged[0], /taunt_normal/);
  // The next minion no longer asks for it.
  assert.ok(!art.render(m(["taunt"])).children.some((c) => c.src === "tex://taunt_normal.webp"));
});

test("Windfury and Stealth are canvases when the effects can run", async () => {
  const attached = [];
  const { art } = await make({ fx: { supported: () => true, attach: (kind, canvas) => { attached.push([kind, canvas.width, canvas.height]); return true; }, setActive() {}, destroy() {} } });
  const el = art.render(m(["windfury", "stealth"]));
  assert.deepEqual(attached.sort(), [["stealth", 180, 240], ["windfury", 230, 160]]);
  const canvases = el.children.filter((c) => c.tag === "canvas");
  assert.equal(canvases.length, 2);
  assert.equal(canvases[0].style.left !== undefined, true);
  assert.equal(el.find("tlm-mark").length, 0);
});

test("Windfury and Stealth fall back to words when WebGL cannot run", async () => {
  for (const fx of [null, { supported: () => false, attach: () => false, setActive() {}, destroy() {} }]) {
    const { art } = await make({ fx });
    const el = art.render(m(["windfury", "stealth", "taunt"]));
    assert.equal(el.children.filter((c) => c.tag === "canvas").length, 0);
    const marks = el.children.find((c) => c.className.includes("tlm-marks"));
    assert.deepEqual(marks.children.map((c) => c.textContent), ["word:stealth", "word:windfury"]);
    assert.ok(el.children.some((c) => c.src === "tex://frame_normal.webp"), "the static layers are drawn");
  }
});

test("an effect that loses its context after drawing is swapped for its word", async () => {
  let lost;
  const { art } = await make({ fx: { supported: () => true, attach: (kind, canvas, onLost) => { lost = onLost; return true; }, setActive() {}, destroy() {} } });
  const el = art.render(m(["windfury"]));
  assert.equal(el.children.filter((c) => c.tag === "canvas").length, 1);
  lost();
  assert.equal(el.children.filter((c) => c.tag === "canvas" && !c.removed).length, 0);
  const marks = el.children.find((c) => c.className.includes("tlm-marks"));
  assert.deepEqual(marks.children.map((c) => c.textContent), ["word:windfury"]);
});

test("an effect whose own textures are missing shows its word instead of nothing", async () => {
  const { createMinionArt } = await load("minion-art.js");
  const art = createMinionArt({
    layout: LAYOUT, imageUrl: (n) => (n === "wind1" ? null : URLS(n)), artImage: () => new Node("img"),
    formatStat: String, markText: (k) => `word:${k}`, log() {}, document: doc,
    fx: { supported: () => true, attach: () => true, setActive() {}, destroy() {} }, preload: false,
  });
  const el = art.render(m(["windfury", "stealth"]));
  const marks = el.children.find((c) => c.className.includes("tlm-marks"));
  assert.deepEqual(marks.children.map((c) => c.textContent), ["word:windfury"]);
  assert.equal(el.children.filter((c) => c.tag === "canvas").length, 1, "Stealth still runs");
});

test("a layout without an effect's data gives that effect's word, not an error", async () => {
  const { createMinionArt } = await load("minion-art.js");
  const layout = structuredClone(LAYOUT);
  delete layout.windfury;
  layout.stealth.size = [0, 0];
  const art = createMinionArt({
    layout, imageUrl: URLS, artImage: () => new Node("img"), formatStat: String, markText: (k) => `word:${k}`,
    log() {}, document: doc, fx: { supported: () => true, attach: () => true, setActive() {}, destroy() {} }, preload: false,
  });
  const el = art.render(m(["windfury", "stealth"]));
  assert.equal(el.children.filter((c) => c.tag === "canvas").length, 0);
  const marks = el.children.find((c) => c.className.includes("tlm-marks"));
  assert.deepEqual(marks.children.map((c) => c.textContent), ["word:stealth", "word:windfury"]);
});

test("big and unknown numbers are written by the host's own rule", async () => {
  const { createMinionArt } = await load("minion-art.js");
  const art = createMinionArt({
    layout: LAYOUT, imageUrl: URLS, artImage: () => null, formatStat: (v) => (v === 1999 ? "1.9K" : v == null ? "?" : String(v)),
    markText: String, log() {}, document: doc, fx: null, preload: false,
  });
  const nums = art.render(m([], { atk: 1999, health: null })).children.filter((c) => c.className.includes("tlm-num"));
  assert.deepEqual(nums.map((n) => n.textContent), ["1.9K", "?"]);
  assert.match(nums[0].className, /tlm-num-long/);
});

test("a layout that cannot be read gives no drawing at all", async () => {
  const { createMinionArt } = await load("minion-art.js");
  assert.equal(createMinionArt({ layout: { nope: true }, imageUrl: URLS, document: doc, preload: false }), null);
});

test("a drawn minion is updated in place: new numbers keep every layer, new keywords keep the picture", async () => {
  const { art } = await make();
  const el = art.render(m(["taunt"], { atk: 3, health: 4 }));
  const before = [...el.children];
  const picture = el.children.find((c) => c.className.includes("tlm-art"));
  // Only the numbers changed: nothing is rebuilt.
  art.update(el, m(["taunt"], { atk: 1999, health: 0 }));
  assert.deepEqual(el.children, before);
  assert.deepEqual(el.children.filter((c) => c.className.includes("tlm-num")).map((n) => n.textContent), ["1999", "0"]);
  // A keyword came: the layers are made again, the art box stays.
  art.update(el, m(["taunt", "divine_shield"], { atk: 5, health: 6 }));
  assert.ok(el.children.some((c) => c.src === "tex://divine_shield.webp"));
  assert.ok(el.children.find((c) => c.className.includes("tlm-art")) === picture, "same art box");
  assert.deepEqual(el.children.filter((c) => c.className.includes("tlm-num")).map((n) => n.textContent), ["5", "6"]);
  // Golden changes the frame.
  art.update(el, m(["taunt", "divine_shield"], { golden: true }));
  assert.match(el.className, /tlm-golden/);
  assert.ok(el.children.some((c) => c.src === "tex://frame_golden.webp"));
  assert.ok(!el.children.some((c) => c.src === "tex://frame_normal.webp"));
  // A keyword left.
  art.update(el, m([], { golden: true }));
  assert.ok(!el.children.some((c) => c.src === "tex://divine_shield.webp" || c.src === "tex://taunt_golden.webp"));
});

test("an effect added by an update that later loses WebGL leaves its word on the shown minion", async () => {
  let lost;
  const { art } = await make({ fx: { supported: () => true, attach: (kind, canvas, onLost) => { lost = onLost; return true; }, setActive() {}, destroy() {} } });
  const el = art.render(m([]));
  art.update(el, m(["windfury"]));
  assert.equal(el.children.filter((c) => c.tag === "canvas").length, 1);
  lost();
  const marks = el.children.find((c) => c.className.includes("tlm-marks"));
  assert.ok(marks, "the word is on the element the page shows");
  assert.deepEqual(marks.children.map((c) => c.textContent), ["word:windfury"]);
});

test("an element this drawing did not make is left alone", async () => {
  const { art } = await make();
  const stranger = new Node("div");
  art.update(stranger, m(["taunt"]));
  assert.deepEqual(stranger.children, []);
});

test("the module has no page-specific API inside: it takes everything from its options", () => {
  const fs = require("node:fs");
  for (const file of fs.readdirSync(dir).filter((f) => f.endsWith(".js"))) {
    const source = fs.readFileSync(path.join(dir, file), "utf8");
    assert.ok(!/__TAURI__|\bTLCards\b|\bTLI18n\b|\binvoke\(/.test(source.replace(/\/\/.*$/gm, "")), `${file} uses an app-only API`);
    assert.ok(/^\s*(import|export)\b/m.test(source), `${file} is an ES module`);
  }
});

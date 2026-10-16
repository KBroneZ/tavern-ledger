// Minions drawn like the game on the site (T-372, D-150, D-153): the texture
// set the site asks for, the attributes that carry a minion to the shared
// drawing, and the painter that puts the drawing in place or leaves the
// page's own look. The drawing itself (crates/desktop/ui/minion-art/) has its
// own tests; the look is checked in a real browser.
import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { FX_TEXTURES } from "../../../crates/desktop/ui/minion-art/fx.js";
import { planLayers, readLayout } from "../../../crates/desktop/ui/minion-art/layout.js";
import { drawnAttrs, readDrawn, type DrawnMinion } from "../../src/lib/art/drawn.ts";
import { paintHost, type ArtLike } from "../../src/lib/art/paint.ts";
import type { Tile } from "../../src/lib/game/replay.ts";
import { boardBlock } from "../../src/lib/game/boards.ts";
import { tileDrawn, tileView } from "../../src/lib/game/replayView.ts";
import type { VNode } from "../../src/lib/view/node.ts";
import { TEXTURE_BASE, TEXTURE_LAYOUT, TEXTURE_ORIGIN, TEXTURE_ROLES, TEXTURE_VERSION, textureUrl } from "../../src/lib/art/textureSet.ts";

const ROOT = join(import.meta.dirname, "..", "..", "..");

test("the site asks for the same texture set as the app: one version, tested against the app's", () => {
  const rust = readFileSync(join(ROOT, "crates", "cards", "src", "textures.rs"), "utf8");
  const app = /macro_rules! set_version \{\s*\(\) => \{\s*"([^"]+)"/.exec(rust)?.[1];
  assert.ok(app, "the app's version constant was found");
  assert.equal(TEXTURE_VERSION, app);
  const host = /BASE_URL: &str = concat!\("([^"]+)"/.exec(rust)?.[1];
  assert.equal(TEXTURE_BASE, `${host}${app}/`, "the same host and folder as the app");
});

test("textures come from the one host over https, with the layout's file names", () => {
  assert.match(TEXTURE_ORIGIN, /^https:\/\/[a-z.]+$/);
  assert.equal(textureUrl("frame_normal"), `${TEXTURE_BASE}frame_normal.webp`);
  assert.equal(textureUrl("StealthDome_old"), `${TEXTURE_BASE}StealthDome_old.webp`);
  for (const role of TEXTURE_ROLES) {
    assert.match(textureUrl(role) ?? "", /^https:\/\/assets\.tavernledger\.net\/v\d+\/[A-Za-z0-9_]+\.webp$/, role);
  }
});

test("a role the set does not carry has no address, and nothing else can be built into one", () => {
  for (const role of ["rally", "quest_icon", "../frame_normal", "frame_normal.webp", "", "constructor", "__proto__", "toString"]) {
    assert.equal(textureUrl(role), null, role);
  }
});

test("the bundled layout is one the shared drawing accepts, with every part the site draws", () => {
  const layout = readLayout(TEXTURE_LAYOUT);
  assert.ok(layout);
  assert.deepEqual([layout.width, layout.height], [400, 524]);
  for (const part of ["frame", "taunt_normal", "taunt_golden", "gem_attack", "gem_health", "skull", "trigger", "venomous", "poisonous", "divine_shield", "reborn_glow"]) {
    assert.ok(layout.boxes[part], part);
  }
  assert.ok(layout.windfury && layout.stealth, "both effects");
});

test("every texture the drawing can ask for is in the set the site names", () => {
  const layout = readLayout(TEXTURE_LAYOUT);
  const has = () => true;
  const keywords = ["taunt", "divine_shield", "reborn", "venomous", "poisonous", "windfury", "stealth", "deathrattle", "trigger"];
  const asked = new Set<string>(["oval_mask", "frame_normal", "frame_golden", "gem_attack", "gem_health"]);
  for (const golden of [false, true]) {
    for (const op of planLayers({ card_id: "BG_X", atk: 1, health: 1, golden, keywords }, layout, has)) {
      if (op.kind === "img") asked.add(op.name);
    }
  }
  for (const names of Object.values(FX_TEXTURES) as string[][]) for (const name of names) asked.add(name);
  for (const role of asked) assert.ok((TEXTURE_ROLES as readonly string[]).includes(role), role);
});

const minion: DrawnMinion = { cardId: "BG25_011", atk: 4, health: 5, golden: true, keywords: ["taunt", "divine_shield"] };

test("a minion goes to the page as attributes and comes back the same", () => {
  const attrs = drawnAttrs(minion);
  assert.deepEqual(attrs, {
    "data-tlm-card": "BG25_011",
    "data-tlm-atk": "4",
    "data-tlm-hp": "5",
    "data-tlm-gold": "1",
    "data-tlm-kw": "taunt divine_shield",
  });
  assert.deepEqual(readDrawn(attrs), minion);
});

test("a normal minion has no golden mark, and an unknown number stays unknown", () => {
  const attrs = drawnAttrs({ cardId: "BG25_011", atk: null, health: 0, golden: false, keywords: [] });
  assert.equal(attrs["data-tlm-gold"], undefined);
  assert.equal(attrs["data-tlm-atk"], "");
  assert.deepEqual(readDrawn(attrs), { cardId: "BG25_011", atk: null, health: 0, golden: false, keywords: [] });
});

test("what is not a card, a number or a keyword is refused, not drawn", () => {
  const base = { "data-tlm-card": "BG25_011", "data-tlm-atk": "1", "data-tlm-hp": "1" };
  assert.equal(readDrawn({}), null);
  for (const card of ["", "a b", "../x", "x".repeat(65), '"><img>']) assert.equal(readDrawn({ ...base, "data-tlm-card": card }), null, card);
  assert.equal(readDrawn({ ...base, "data-tlm-atk": "1e9999" })?.atk, null);
  assert.equal(readDrawn({ ...base, "data-tlm-atk": "abc" })?.atk, null);
  assert.deepEqual(readDrawn({ ...base, "data-tlm-kw": "taunt <b> stealth A" })?.keywords, ["taunt", "stealth"]);
});

// ---- the painter ----

class Drawn {
  minion: unknown;
  constructor(minion: unknown) {
    this.minion = minion;
  }
}
class Slot {
  made: Drawn[] = [];
  querySelector(selector: string): Drawn | null {
    return selector === ".tlm" ? (this.made.at(-1) ?? null) : null;
  }
  append(node: Drawn): void {
    this.made.push(node);
  }
}
/** A host as far as the painter sees it: its data attributes and its slot. */
function host(attrs: Record<string, string | undefined>, slot: Slot | null = new Slot()) {
  return {
    slot,
    getAttribute: (name: string) => attrs[name] ?? null,
    setAttribute: (name: string, value: string) => void (attrs[name] = value),
    querySelector: (selector: string) => (selector === ".tlm-slot" ? slot : null),
  };
}
function fakeArt() {
  const calls: string[] = [];
  const art: ArtLike = {
    render: (m) => {
      calls.push(`render ${m.card_id} ${m.atk}/${m.health}${m.golden ? " golden" : ""} ${m.keywords.join(",")}`);
      return new Drawn(m) as unknown as HTMLElement;
    },
    update: (el, m) => {
      calls.push(`update ${m.card_id} ${m.atk}/${m.health} ${m.keywords.join(",")}`);
      (el as unknown as Drawn).minion = m;
    },
  };
  return { art, calls };
}
const asHost = (h: ReturnType<typeof host>) => h as unknown as HTMLElement;

test("painting a host draws its minion in the slot, with the keywords the page gave", () => {
  const { art, calls } = fakeArt();
  const h = host(drawnAttrs(minion));
  assert.equal(paintHost(asHost(h), art), true);
  assert.deepEqual(calls, ["render BG25_011 4/5 golden taunt,divine_shield"]);
  assert.equal(h.slot?.made.length, 1);
});

test("painting again updates the drawing it made, never a second one", () => {
  const { art, calls } = fakeArt();
  const attrs: Record<string, string | undefined> = { ...drawnAttrs(minion) };
  const h = host(attrs);
  paintHost(asHost(h), art);
  attrs["data-tlm-atk"] = "7";
  attrs["data-tlm-kw"] = "taunt";
  paintHost(asHost(h), art);
  assert.deepEqual(calls, ["render BG25_011 4/5 golden taunt,divine_shield", "update BG25_011 7/5 taunt"]);
  assert.equal(h.slot?.made.length, 1);
});

test("a host the painter cannot read or that has no slot keeps the page's own look", () => {
  const { art, calls } = fakeArt();
  assert.equal(paintHost(asHost(host({})), art), false);
  assert.equal(paintHost(asHost(host(drawnAttrs(minion), null)), art), false);
  assert.deepEqual(calls, []);
});

// ---- the views carry minions to the drawing ----

const tile = (extra: Partial<Tile> = {}): Tile => ({
  ref: 3,
  cardId: "BG25_011",
  atk: 6,
  health: -2,
  golden: true,
  shield: true,
  keywords: ["taunt", "windfury"],
  gifts: [],
  giftUnknown: false,
  buffs: null,
  marks: {},
  ...extra,
});

const find = (n: VNode, pick: (n: VNode) => boolean): VNode[] => [n, ...n.children.flatMap((c) => (typeof c === "string" ? [] : find(c, pick)))].filter(pick);
const attr = (n: VNode, name: string) => n.attrs.find(([k]) => k === name)?.[1];

test("a replay token carries its minion and a slot for the drawing; overkill is drawn as 0 and the shield as a keyword", () => {
  assert.deepEqual(tileDrawn(tile()), { cardId: "BG25_011", atk: 6, health: 0, golden: true, keywords: ["taunt", "windfury", "divine_shield"] });
  const li = tileView(tile());
  assert.equal(attr(li, "data-tlm-card"), "BG25_011");
  assert.equal(attr(li, "data-tlm-hp"), "0");
  assert.equal(attr(li, "data-tlm-kw"), "taunt windfury divine_shield");
  const slots = find(li, (n) => attr(n, "class") === "tlm-slot");
  assert.equal(slots.length, 1);
  assert.ok(find(li, (n) => attr(n, "class") === "rp-art")[0].children.includes(slots[0]), "inside the art box, which carries the motion");
});

test("a shield that popped is no keyword, and a token with no card id has nothing to draw", () => {
  assert.ok(!tileDrawn(tile({ shield: false }))?.keywords.includes("divine_shield"));
  assert.equal(tileDrawn(tile({ cardId: null })), null);
  assert.equal(attr(tileView(tile({ cardId: null })), "data-tlm-card"), undefined);
});

test("a board's minion on the game page carries its minion and a slot for the drawing", () => {
  const m = { cardId: "BG25_011", atk: 3, health: 4, golden: false, keywords: ["taunt"] as Tile["keywords"], gifts: [], giftUnknown: false, buffs: null };
  const board = boardBlock("b", "Theirs", null, { minions: [m, { ...m, cardId: null }], attack: 6, health: 8 } as never);
  const lis = find(board, (n) => n.tag === "li");
  assert.equal(lis.length, 2);
  assert.equal(attr(lis[0], "data-tlm-card"), "BG25_011");
  assert.equal(attr(lis[0], "data-tlm-kw"), "taunt");
  assert.equal(find(lis[0], (n) => attr(n, "class") === "tlm-slot").length, 1);
  assert.equal(attr(lis[1], "data-tlm-card"), undefined, "an unknown card keeps the page's own look");
});

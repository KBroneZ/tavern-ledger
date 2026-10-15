// What a minion is made of and where every part sits (T-372, D-150). Plain
// data in, plain data out: no page, no app API. The positions come from the
// texture set's layout.json, which is read as untrusted input: a part whose
// box is not a box is left out, never guessed.

// A canvas nobody drew is refused: the frame canvas is 400 wide, effects a few hundred.
const MAX_SIZE = 1024;
const MAX_EFFECT_SIZE = 512;
// A part may reach past the frame (a shield, a bubble) but not across the screen.
const REACH = 2;
const isNum = (v) => typeof v === "number" && Number.isFinite(v);

/** [x0, y0, x1, y1] in canvas pixels, or null. */
const readBox = (v, canvas) =>
  Array.isArray(v) &&
  v.length === 4 &&
  v.every(isNum) &&
  v[2] > v[0] &&
  v[3] > v[1] &&
  v[0] >= -REACH * canvas.width &&
  v[2] <= (1 + REACH) * canvas.width &&
  v[1] >= -REACH * canvas.height &&
  v[3] <= (1 + REACH) * canvas.height
    ? v.slice()
    : null;

const readSize = (v) =>
  Array.isArray(v) && v.length === 2 && v.every((n) => Number.isInteger(n) && n > 0 && n <= MAX_EFFECT_SIZE) ? v.slice() : null;

const readPair = (v) => (Array.isArray(v) && v.length === 2 && v.every(isNum) ? v.slice() : null);

function readWindfury(raw, canvas) {
  if (!raw || typeof raw !== "object") return null;
  const box = readBox(raw.box_canvas_px, canvas);
  const size = readSize(raw.size);
  if (!box || !size || !isNum(raw.umin) || !isNum(raw.umax) || raw.umax <= raw.umin) return null;
  return { box, size, umin: raw.umin, umax: raw.umax };
}

function readStealth(raw, pad, canvas) {
  if (!raw || typeof raw !== "object" || !isNum(pad) || pad < 0 || pad > MAX_SIZE) return null;
  const box = readBox(raw.box_canvas_px, canvas);
  const size = readSize(raw.size);
  const u = readPair(raw.u);
  const v = readPair(raw.v);
  if (!box || !size || !u || !v || !isNum(pad)) return null;
  return { box, size, u, v, pad };
}

/**
 * The layout of the texture set, checked. Returns null when it cannot be
 * used at all (no canvas size), else { width, height, boxes, windfury,
 * stealth } where `boxes` has an entry only for each part with a valid box.
 */
export function readLayout(raw) {
  if (!raw || typeof raw !== "object" || Array.isArray(raw)) return null;
  const canvas = raw.canvas;
  if (!canvas || !isNum(canvas.width) || !isNum(canvas.height)) return null;
  if (canvas.width <= 0 || canvas.height <= 0 || canvas.width > MAX_SIZE || canvas.height > MAX_SIZE) return null;
  const boxes = {};
  const keep = (name, value) => {
    const box = readBox(value, canvas);
    if (box) boxes[name] = box;
  };
  const parts = raw.parts && typeof raw.parts === "object" ? raw.parts : {};
  for (const name of Object.keys(parts)) keep(name, parts[name] && parts[name].box);
  keep("frame", raw.frame && raw.frame.box);
  const taunt = raw.taunt && raw.taunt.box;
  keep("taunt_normal", taunt);
  keep("taunt_golden", taunt);
  return {
    width: canvas.width,
    height: canvas.height,
    boxes,
    windfury: readWindfury(raw.windfury, canvas),
    stealth: readStealth(raw.stealth, raw.fx_canvas_pad_px, canvas),
  };
}

/** A position as a share of the canvas, for CSS: "-22.5%". */
export const percent = (value, whole) => `${Math.round((value / whole) * 1e5) / 1e3}%`;

const FX_FOR = { stealth: "stealth", windfury: "windfury", mega_windfury: "windfury" };

/** The keywords of a minion as a set; anything but a list of strings is none. */
function keywordSet(minion) {
  const list = Array.isArray(minion.keywords) ? minion.keywords : [];
  return new Set(list.filter((k) => typeof k === "string"));
}

/**
 * The layers of one minion, back to front, in the order the game stacks
 * them (layout.json) with Venomous above the other effects (the user's
 * choice): Reborn glow, Taunt, the art under its mask, the frame, Stealth,
 * Windfury, the gems and their numbers, one bottom icon (skull, else
 * trigger, else Rally), Venomous, Poisonous, Divine Shield.
 *
 * `has(name)` says whether a texture can be drawn (it exists and has not
 * failed). Each op is { kind: "img", name, box } | { kind: "art" } |
 * { kind: "fx", fx } | { kind: "number", which, value }. Effects are always
 * planned: whether they run or show a word is up to the drawing.
 */
export function planLayers(minion, layout, has) {
  const kws = keywordSet(minion);
  const golden = minion.golden === true;
  const ops = [];
  const img = (name, boxName = name) => {
    const box = layout.boxes[boxName];
    if (box && has(name)) ops.push({ kind: "img", name, box });
  };
  if (kws.has("reborn")) img("reborn_glow");
  if (kws.has("taunt")) img(golden ? "taunt_golden" : "taunt_normal");
  ops.push({ kind: "art" });
  img(golden ? "frame_golden" : "frame_normal", "frame");
  const effects = new Set([...kws].map((k) => FX_FOR[k]).filter(Boolean));
  for (const fx of ["stealth", "windfury"]) if (effects.has(fx)) ops.push({ kind: "fx", fx });
  img("gem_attack");
  img("gem_health");
  ops.push({ kind: "number", which: "attack", value: minion.atk });
  ops.push({ kind: "number", which: "health", value: minion.health });
  const icon = kws.has("deathrattle") ? "skull" : kws.has("trigger") ? "trigger" : kws.has("rally") ? "rally" : null;
  if (icon) img(icon);
  if (kws.has("venomous")) img("venomous");
  if (kws.has("poisonous")) img("poisonous");
  if (kws.has("divine_shield")) img("divine_shield");
  return ops;
}

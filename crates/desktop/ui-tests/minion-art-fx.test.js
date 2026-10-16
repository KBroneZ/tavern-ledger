"use strict";
// The live effects runtime (ui/minion-art/fx.js) on a fake WebGL: how many
// drawings a frame costs, when the loop runs and stops, and what happens when
// WebGL is missing, fails or loses its context. The look is checked in a
// real browser (the workbench); here only the behaviour.
const test = require("node:test");
const assert = require("node:assert/strict");
const path = require("node:path");
const { pathToFileURL } = require("node:url");

const load = () => import(pathToFileURL(path.join(__dirname, "..", "ui", "minion-art", "fx.js")).href);

const LAYOUT = {
  windfury: { box: [0, 0, 100, 100], size: [238, 162], umin: -0.3, umax: 1.8 },
  stealth: { box: [0, 0, 100, 100], size: [177, 241], u: [0.002, -0.08], v: [-0.002, 1.2], pad: 100 },
};

// A GL whose every method does nothing but count; its constants are numbers.
function fakeGl(counts) {
  return new Proxy({}, {
    get(target, name) {
      if (name === "getShaderParameter" || name === "getProgramParameter") return () => true;
      if (name === "getError") return () => 0;
      if (name === "getExtension") return () => ({ loseContext: () => { counts.loseContext = (counts.loseContext || 0) + 1; } });
      if (typeof name !== "string" || name === "then") return undefined;
      if (name === "NO_ERROR") return 0;
      if (/^[A-Z0-9_]+$/.test(name)) return 1;
      return (...args) => {
        counts[name] = (counts[name] || 0) + 1;
        return {};
      };
    },
  });
}

function fakeDoc({ webgl = true } = {}) {
  const counts = {};
  const glCanvas = {
    width: 0, height: 0, listeners: {},
    getContext: () => (webgl ? fakeGl(counts) : null),
    addEventListener(type, fn) { this.listeners[type] = fn; },
  };
  const doc = {
    hidden: false, listeners: {}, createElement: () => glCanvas,
    addEventListener(type, fn) { this.listeners[type] = fn; },
    removeEventListener(type) { delete this.listeners[type]; },
  };
  return { doc, counts, glCanvas };
}

function fakeCanvas({ connected = true, shown = true } = {}) {
  const drawn = [];
  return {
    isConnected: connected,
    getClientRects: () => (shown ? [{}] : []),
    getContext: () => ({ clearRect() {}, drawImage: (...a) => drawn.push(a) }),
    drawn,
  };
}

// A loop that runs when the test says so.
function clock() {
  const state = { tasks: [], time: 0, cancelled: 0 };
  return {
    state,
    schedule: (fn, delay) => { const t = { fn, delay }; state.tasks.push(t); return t; },
    unschedule: (t) => { state.cancelled += 1; state.tasks = state.tasks.filter((x) => x !== t); },
    now: () => state.time,
    run() { const t = state.tasks.shift(); assert.ok(t, "something was scheduled"); t.fn(); return t; },
  };
}

async function make(options = {}) {
  const { createFx } = await load();
  const env = fakeDoc(options);
  const c = clock();
  const logs = [];
  const loadImage = options.loadImage || (async (name) => ({ name }));
  const fx = createFx({ document: env.doc, layout: LAYOUT, loadImage, log: (m) => logs.push(m), schedule: c.schedule, unschedule: c.unschedule, now: c.now, fps: 20, observe: options.observe });
  return { fx, env, c, logs };
}
const settle = () => new Promise((r) => setImmediate(r));

test("without WebGL nothing attaches and one line says why", async () => {
  const { fx, logs } = await make({ webgl: false });
  assert.equal(fx.supported(), false);
  assert.equal(fx.attach("windfury", fakeCanvas(), () => {}), false);
  assert.equal(fx.attach("stealth", fakeCanvas(), () => {}), false);
  assert.equal(logs.length, 1);
});

test("minions with the same effect share each drawing: a frame costs one per phase, not one per minion", async () => {
  const { fx, env, c } = await make();
  const canvases = Array.from({ length: 7 }, () => fakeCanvas());
  for (const canvas of canvases) assert.equal(fx.attach("windfury", canvas, () => {}), true);
  await settle();
  c.state.tasks.length && c.run();
  // Three phases, four passes each; every minion still gets its copy.
  assert.equal(env.counts.drawArrays, 3 * 4);
  assert.ok(canvases.every((canvas) => canvas.drawn.length === 1));
});

test("stealth draws one pass per phase", async () => {
  const { fx, env, c } = await make();
  const canvases = Array.from({ length: 6 }, () => fakeCanvas());
  for (const canvas of canvases) fx.attach("stealth", canvas, () => {});
  await settle();
  c.run();
  assert.equal(env.counts.drawArrays, 3);
  assert.ok(canvases.every((canvas) => canvas.drawn.length === 1));
});

test("the loop waits out the frame interval and runs only while something shows", async () => {
  const { fx, c } = await make();
  const canvas = fakeCanvas();
  fx.attach("windfury", canvas, () => {});
  await settle();
  const first = c.run();
  assert.equal(first.delay, 0);
  assert.equal(c.state.tasks.length, 1, "the next frame is scheduled");
  assert.ok(Math.abs(c.state.tasks[0].delay - 50) < 1, "20 frames a second: 50 ms apart");
  // The minion leaves the page: after a second the loop ends.
  canvas.isConnected = false;
  c.state.time = 0.5;
  c.run();
  assert.equal(c.state.tasks.length, 1, "kept for a moment in case it is put back");
  c.state.time = 2;
  c.run();
  assert.equal(c.state.tasks.length, 0, "nothing left to draw: the loop stopped");
});

test("a canvas that is not shown is not drawn", async () => {
  const { fx, env, c } = await make();
  const hidden = fakeCanvas({ shown: false });
  fx.attach("windfury", hidden, () => {});
  await settle();
  c.run();
  assert.equal(env.counts.drawArrays || 0, 0);
  assert.equal(hidden.drawn.length, 0);
});

test("pausing stops the loop and resuming starts it again", async () => {
  const { fx, c } = await make();
  fx.attach("windfury", fakeCanvas(), () => {});
  await settle();
  assert.equal(c.state.tasks.length, 1);
  fx.setActive(false);
  assert.equal(c.state.tasks.length, 0);
  assert.equal(c.state.cancelled, 1);
  fx.setActive(true);
  assert.equal(c.state.tasks.length, 1);
});

test("a hidden window pauses it, a shown one resumes it", async () => {
  const { fx, env, c } = await make();
  fx.attach("windfury", fakeCanvas(), () => {});
  await settle();
  env.doc.hidden = true;
  env.doc.listeners.visibilitychange();
  assert.equal(c.state.tasks.length, 0);
  env.doc.hidden = false;
  env.doc.listeners.visibilitychange();
  assert.equal(c.state.tasks.length, 1);
});

test("textures that cannot be loaded send the minions of that effect back to words, once", async () => {
  const { fx, logs } = await make({ loadImage: async (name) => { if (name === "wind1") throw new Error("no wind1"); return {}; } });
  const lost = [];
  fx.attach("windfury", fakeCanvas(), () => lost.push(1));
  fx.attach("windfury", fakeCanvas(), () => lost.push(2));
  await settle();
  await settle();
  assert.deepEqual(lost.sort(), [1, 2]);
  assert.equal(logs.filter((l) => /windfury/.test(l)).length, 1);
  assert.equal(fx.attach("windfury", fakeCanvas(), () => {}), false, "later minions go straight to words");
  assert.equal(fx.attach("stealth", fakeCanvas(), () => {}), true, "the other effect is unaffected");
});

test("losing the WebGL context sends every effect back to words and a restored one works again", async () => {
  const { fx, env, logs } = await make();
  const lost = [];
  fx.attach("windfury", fakeCanvas(), () => lost.push("w"));
  fx.attach("stealth", fakeCanvas(), () => lost.push("s"));
  await settle();
  let prevented = false;
  env.glCanvas.listeners.webglcontextlost({ preventDefault: () => { prevented = true; } });
  assert.ok(prevented, "the browser is asked to restore it");
  assert.deepEqual(lost.sort(), ["s", "w"]);
  assert.equal(fx.supported(), false);
  assert.equal(fx.attach("windfury", fakeCanvas(), () => {}), false);
  assert.equal(logs.length, 1);
  env.glCanvas.listeners.webglcontextrestored();
  assert.equal(fx.supported(), true);
  assert.equal(fx.attach("windfury", fakeCanvas(), () => {}), true);
});

test("destroy clears the loop, lets go of the page's listener and gives the GPU its memory back", async () => {
  const { fx, env, c, logs } = await make();
  fx.attach("stealth", fakeCanvas(), () => {});
  await settle();
  fx.destroy();
  assert.equal(c.state.tasks.length, 0);
  assert.equal(env.doc.listeners.visibilitychange, undefined);
  assert.equal(env.counts.loseContext, 1);
  assert.equal(fx.supported(), false);
  assert.equal(fx.attach("stealth", fakeCanvas(), () => {}), false);
  // The context the runtime itself let go of is not a loss to report.
  env.glCanvas.listeners.webglcontextlost({ preventDefault() {} });
  assert.deepEqual(logs, []);
});

test("effects in the page but hidden are looked at less often", async () => {
  const { fx, c } = await make();
  fx.attach("windfury", fakeCanvas({ shown: false }), () => {});
  await settle();
  c.run();
  assert.equal(c.state.tasks[0].delay, 500);
});

// A stand-in for the page's visibility observer: tells the test which canvases
// are watched and lets it say when each comes into view or leaves it.
function watcher() {
  const watched = new Map();
  return {
    watched,
    observe: (canvas, onChange) => {
      watched.set(canvas, onChange);
      return () => watched.delete(canvas);
    },
    say: (canvas, on) => watched.get(canvas)(on),
  };
}

test("an effect scrolled out of view is not drawn, and the loop stops once nothing is in view", async () => {
  const w = watcher();
  const { fx, env, c } = await make({ observe: w.observe });
  const away = fakeCanvas();
  fx.attach("windfury", away, () => {});
  assert.ok(w.watched.has(away), "the canvas is watched");
  await settle();
  w.say(away, false);
  c.run();
  assert.equal(env.counts.drawArrays || 0, 0, "nothing drawn while out of view");
  assert.equal(away.drawn.length, 0);
  assert.equal(c.state.tasks.length, 1, "no drawing: only a slow look for canvases that left the page");
  assert.equal(c.state.tasks[0].delay, 3000);
});

test("an effect that comes into view is drawn again, and only it", async () => {
  const w = watcher();
  const { fx, c } = await make({ observe: w.observe });
  const a = fakeCanvas();
  const b = fakeCanvas();
  fx.attach("windfury", a, () => {});
  fx.attach("windfury", b, () => {});
  await settle();
  w.say(a, false);
  w.say(b, false);
  c.run();
  w.say(b, true);
  assert.equal(c.state.tasks.length, 1, "coming into view wakes the loop at once, in place of the slow look");
  assert.equal(c.state.tasks[0].delay, 0);
  c.run();
  assert.equal(a.drawn.length, 0);
  assert.equal(b.drawn.length, 1);
});

test("a canvas is watched from the moment it attaches and let go when the effects end", async () => {
  const w = watcher();
  const { fx } = await make({ observe: w.observe });
  const canvas = fakeCanvas();
  fx.attach("stealth", canvas, () => {});
  await settle();
  fx.destroy();
  assert.equal(w.watched.size, 0, "destroy stops every watch");
});

test("a canvas dropped by the loop is no longer watched", async () => {
  const w = watcher();
  const { fx, c } = await make({ observe: w.observe });
  const canvas = fakeCanvas({ connected: false });
  fx.attach("windfury", canvas, () => {});
  await settle();
  c.run();
  c.state.time = 2;
  c.run();
  assert.equal(w.watched.size, 0);
});

test("without a visibility observer the effect is drawn whenever it is shown, as before", async () => {
  const { fx, env, c } = await make({ observe: null });
  fx.attach("windfury", fakeCanvas(), () => {});
  await settle();
  c.run();
  assert.ok(env.counts.drawArrays > 0);
});

test("attaching the same canvas again is a no-op while it runs, and starts it again once the loop dropped it", async () => {
  const { fx, env, c } = await make();
  const canvas = fakeCanvas();
  assert.equal(fx.attach("windfury", canvas, () => {}), true);
  assert.equal(fx.attach("windfury", canvas, () => {}), true);
  await settle();
  c.run();
  const once = env.counts.drawArrays;
  assert.equal(once, 4, "one instance: four passes, not eight");
  // Off the page for more than a second: the loop drops it.
  canvas.isConnected = false;
  c.state.time = 0.1;
  c.run();
  c.state.time = 2;
  c.run();
  assert.equal(c.state.tasks.length, 0);
  canvas.isConnected = true;
  assert.equal(fx.attach("windfury", canvas, () => {}), true, "a new effect for the same canvas");
  await settle();
  c.run();
  assert.equal(env.counts.drawArrays, once + 4);
});

test("by default the page's own IntersectionObserver watches the canvases, one for all, and lets go on destroy", async () => {
  const seen = { made: 0, observed: [], unobserved: [] };
  let notify;
  globalThis.IntersectionObserver = class {
    constructor(fn) { seen.made += 1; notify = fn; }
    observe(el) { seen.observed.push(el); }
    unobserve(el) { seen.unobserved.push(el); }
  };
  try {
    const { createFx } = await load();
    const env = fakeDoc();
    const c = clock();
    const fx = createFx({ document: env.doc, layout: LAYOUT, loadImage: async (name) => ({ name }), schedule: c.schedule, unschedule: c.unschedule, now: c.now });
    const a = fakeCanvas();
    const b = fakeCanvas();
    fx.attach("windfury", a, () => {});
    fx.attach("stealth", b, () => {});
    assert.equal(seen.made, 1, "one observer");
    assert.deepEqual(seen.observed, [a, b]);
    await settle();
    notify([{ target: a, isIntersecting: true }]);
    c.run();
    assert.equal(a.drawn.length, 1, "in view: drawn");
    assert.equal(b.drawn.length, 0, "not reported in view yet: not drawn");
    notify([{ target: a, isIntersecting: false }, { target: {}, isIntersecting: true }]);
    fx.destroy();
    assert.deepEqual(seen.unobserved, [a, b]);
  } finally {
    delete globalThis.IntersectionObserver;
  }
});

test("effects out of view whose canvases are then removed are swept out and the loop ends", async () => {
  const w = watcher();
  const { fx, c } = await make({ observe: w.observe });
  const canvas = fakeCanvas();
  fx.attach("windfury", canvas, () => {});
  await settle();
  w.say(canvas, false);
  c.run();
  assert.equal(c.state.tasks.length, 1);
  canvas.isConnected = false;
  c.state.time = 5;
  c.run();
  assert.equal(c.state.tasks.length, 1, "first seen gone");
  c.state.time = 10;
  c.run();
  assert.equal(c.state.tasks.length, 0, "dropped; nothing left to look at");
  assert.equal(w.watched.size, 0);
});

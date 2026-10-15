// Windfury and Stealth, drawn live from the game's own meshes and scrolling
// textures (D-153, T-372). One WebGL context for the whole page draws every
// effect in turn into its own small canvas, one loop for all of them, which
// stops when nothing shows. The meshes were baked offline into lookup images
// (see the texture set); the blend is our reading of the game's shaders.
//
// No page or app API here: the host gives the document, a way to load an
// image and the layout. If WebGL is missing, fails, or loses its context,
// `attach` says no (or calls `onLost`) and the host draws a word instead.

const WIND_CYCLE = 2.6; // seconds; both clips turn whole times in it
// Minions with the same effect share one drawing per frame, in this many
// phases (so they do not all move together); each minion copies its phase's.
const PHASES = 3;
// Nothing to draw but effects still in the page: look again this often (ms).
const IDLE_CHECK_MS = 500;
const STEALTH_TIME_SCALE = 0.05; // the game's scroll speeds are per 20 s
const UNITS = { wfMap: [0, 1, 2, 3], wind: 4, dome: 5, mask: 6, shadow: 7, cover: 8 };

const VERTEX = "attribute vec2 p;varying vec2 t;void main(){t=vec2(p.x*.5+.5,.5-p.y*.5);gl_Position=vec4(p,0.,1.);}";

const WINDFURY_FRAGMENT = `varying vec2 t;uniform sampler2D map,wind;uniform float us,col,inten,umin,urange;
void main(){vec4 m=texture2D(map,t);float w=m.b;if(w<=0.0){gl_FragColor=vec4(0.);return;}
 float u=m.r*urange+umin,v=m.g;float g=texture2D(wind,vec2(u+us,1.-v*.5)).r;float a=g*col*inten*w;
 gl_FragColor=vec4(vec3(.86,.95,1.)*a,a);}`;

const STEALTH_FRAGMENT = `varying vec2 t;uniform sampler2D dome,mask,shadow,cover;
uniform float time;uniform vec2 uA,vA,scrMod,scrMask,full,org;uniform float modI;
float lum(vec3 c){return dot(c,vec3(.299,.587,.114));}
void main(){float cv=texture2D(cover,t).r;if(cv<=0.003){gl_FragColor=vec4(0.);return;}
 vec2 px=org+t*full;float u=uA.x*px.x+uA.y,v=vA.x*px.y+vA.y;
 vec3 mk=texture2D(mask,vec2(u+scrMask.x*time*${STEALTH_TIME_SCALE},1.-(v+scrMask.y*time*${STEALTH_TIME_SCALE}))).rgb;
 vec2 d=(mk.rg-.5)*.05*modI;
 vec3 sm=texture2D(dome,vec2(u+scrMod.x*time*${STEALTH_TIME_SCALE}+d.x,1.-(v+scrMod.y*time*${STEALTH_TIME_SCALE}+d.y))).rgb;
 float a=clamp((lum(sm)-.30)*2.4,0.,1.)*(.4+.6*mk.b)*.85*cv;vec3 rgb=clamp(sm*1.5,0.,1.);
 float sh=texture2D(shadow,vec2(u,1.-v)).r;float as=clamp((1.-sh)*.9,0.,1.)*cv;
 vec4 o=vec4(rgb*a,a);gl_FragColor=vec4(o.rgb*(1.-as),as+o.a*(1.-as));}`;

/** The textures each effect samples, by the texture set's role names. */
export const FX_TEXTURES = {
  windfury: ["wf_map_A0", "wf_map_A1", "wf_map_B0", "wf_map_B1", "wind1"],
  stealth: ["StealthDome_old", "StealthDomeMask", "StealthDomeShadow_D", "st_cover"],
};

function compile(gl, type, source) {
  const shader = gl.createShader(type);
  gl.shaderSource(shader, source);
  gl.compileShader(shader);
  if (!gl.getShaderParameter(shader, gl.COMPILE_STATUS)) throw new Error(`shader: ${gl.getShaderInfoLog(shader)}`);
  return shader;
}

function program(gl, fragment) {
  const pr = gl.createProgram();
  gl.attachShader(pr, compile(gl, gl.VERTEX_SHADER, VERTEX));
  gl.attachShader(pr, compile(gl, gl.FRAGMENT_SHADER, `precision highp float;${fragment}`));
  gl.linkProgram(pr);
  if (!gl.getProgramParameter(pr, gl.LINK_STATUS)) throw new Error(`program: ${gl.getProgramInfoLog(pr)}`);
  gl.useProgram(pr);
  const buffer = gl.createBuffer();
  gl.bindBuffer(gl.ARRAY_BUFFER, buffer);
  gl.bufferData(gl.ARRAY_BUFFER, new Float32Array([-1, -1, 1, -1, -1, 1, 1, 1]), gl.STATIC_DRAW);
  const at = gl.getAttribLocation(pr, "p");
  gl.enableVertexAttribArray(at);
  gl.vertexAttribPointer(at, 2, gl.FLOAT, false, 0, 0);
  return { pr, at, buffer, loc: (name) => gl.getUniformLocation(pr, name) };
}

function texture(gl, image, unit, { repeat = false, nearest = false } = {}) {
  const tex = gl.createTexture();
  gl.activeTexture(gl.TEXTURE0 + unit);
  gl.bindTexture(gl.TEXTURE_2D, tex);
  gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA, gl.RGBA, gl.UNSIGNED_BYTE, image);
  const filter = nearest ? gl.NEAREST : gl.LINEAR;
  gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, filter);
  gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, filter);
  const wrap = repeat ? gl.REPEAT : gl.CLAMP_TO_EDGE;
  gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, wrap);
  gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, wrap);
  return tex;
}

/** One shared IntersectionObserver for every effect canvas, or null where the page has none. */
function pageObserver(win) {
  if (!win || typeof win.IntersectionObserver !== "function") return null;
  const callbacks = new Map();
  const io = new win.IntersectionObserver((entries) => {
    for (const entry of entries) {
      const onChange = callbacks.get(entry.target);
      if (onChange) onChange(entry.isIntersecting);
    }
  });
  return (canvas, onChange) => {
    callbacks.set(canvas, onChange);
    io.observe(canvas);
    return () => {
      callbacks.delete(canvas);
      io.unobserve(canvas);
    };
  };
}

/**
 * The effects runtime for one page.
 *
 * options: { document, layout (from readLayout), loadImage(name) -> Promise
 * of a CORS-clean image, log(message), fps, now() (seconds, for tests),
 * schedule(fn, delayMs) -> handle and unschedule(handle) (for tests),
 * observe(canvas, onChange(inView)) -> stop (default: the page's
 * IntersectionObserver when it has one, null for none) }.
 * An effect whose canvas is out of view is not drawn, and the loop sleeps
 * until one comes into view.
 * The loop waits out the frame interval in a timer and draws on the next
 * animation frame, so a page that draws 20 times a second is not woken 60.
 *
 * Returns { supported(), attach(kind, canvas, onLost) -> boolean,
 * setActive(on), destroy() }.
 */
export function createFx(options) {
  const { document: doc, layout, loadImage, log } = options;
  const fps = options.fps > 0 ? options.fps : 20;
  const interval = 1000 / fps;
  const now = options.now || (() => performance.now() / 1000);
  const schedule =
    options.schedule ||
    ((fn, delay) => {
      const handle = { frame: 0, wait: setTimeout(() => (handle.frame = requestAnimationFrame(fn)), Math.max(0, delay)) };
      return handle;
    });
  const unschedule =
    options.unschedule ||
    ((handle) => {
      clearTimeout(handle.wait);
      if (handle.frame) cancelAnimationFrame(handle.frame);
    });

  const observe = options.observe === undefined ? pageObserver(globalThis) : options.observe;
  const instances = new Set();
  const failed = new Set(); // kinds whose textures could not be sampled
  const loading = new Map(); // kind -> Promise
  let gl = null;
  let glCanvas = null;
  let programs = null;
  let ready = new Set(); // kinds with their textures on the GPU
  let lost = false;
  let active = true;
  let timer = null;
  let nextPhase = 0;
  let destroyed = false;
  const told = new Set();

  // One line per kind of trouble, however many minions meet it.
  const tell = (key, message) => {
    if (told.has(key)) return;
    told.add(key);
    if (log) log(message);
  };

  function fail(kind, message) {
    failed.add(kind);
    tell(kind, message);
    for (const inst of [...instances]) if (inst.kind === kind) drop(inst, true);
  }

  /** Takes an effect out of the loop and stops watching its canvas. */
  function remove(inst) {
    instances.delete(inst);
    if (inst.unwatch) inst.unwatch();
    inst.unwatch = null;
  }

  function drop(inst, lostIt) {
    remove(inst);
    if (lostIt && inst.onLost) inst.onLost();
  }

  function init() {
    const sizes = [layout.windfury, layout.stealth].filter(Boolean).map((e) => e.size);
    if (!sizes.length) throw new Error("the layout has no effect");
    glCanvas = doc.createElement("canvas");
    glCanvas.width = Math.max(...sizes.map((s) => s[0]));
    glCanvas.height = Math.max(...sizes.map((s) => s[1]));
    gl = glCanvas.getContext("webgl", { premultipliedAlpha: true, alpha: true, antialias: false });
    if (!gl) throw new Error("WebGL is not available");
    glCanvas.addEventListener("webglcontextlost", (event) => {
      event.preventDefault();
      if (destroyed) return;
      lost = true;
      tell("lost", "WebGL lost its context: Windfury and Stealth show as words until it comes back");
      for (const inst of [...instances]) drop(inst, true);
    });
    glCanvas.addEventListener("webglcontextrestored", () => {
      try {
        build();
        lost = false;
      } catch (error) {
        tell("restore", `WebGL could not restart: ${error.message}`);
      }
    });
    build();
  }

  /** The programs and state of the context; the textures load again after it. */
  function build() {
    gl.pixelStorei(gl.UNPACK_COLORSPACE_CONVERSION_WEBGL, gl.NONE);
    gl.enable(gl.SCISSOR_TEST);
    gl.enable(gl.BLEND);
    programs = {};
    if (layout.windfury) programs.windfury = windfuryProgram();
    if (layout.stealth) programs.stealth = stealthProgram();
    ready = new Set();
    loading.clear();
  }

  function windfuryProgram() {
    const p = program(gl, WINDFURY_FRAGMENT);
    const { umin, umax } = layout.windfury;
    gl.uniform1i(p.loc("wind"), UNITS.wind);
    gl.uniform1f(p.loc("umin"), umin);
    gl.uniform1f(p.loc("urange"), umax - umin);
    gl.uniform1f(p.loc("inten"), 2);
    return { ...p, map: p.loc("map"), us: p.loc("us"), col: p.loc("col") };
  }

  function stealthProgram() {
    const p = program(gl, STEALTH_FRAGMENT);
    const s = layout.stealth;
    const [x0, y0, x1, y1] = s.box;
    ["dome", "mask", "shadow", "cover"].forEach((name, i) => gl.uniform1i(p.loc(name), UNITS.dome + i));
    gl.uniform2f(p.loc("uA"), ...s.u);
    gl.uniform2f(p.loc("vA"), ...s.v);
    gl.uniform2f(p.loc("full"), x1 - x0, y1 - y0);
    gl.uniform2f(p.loc("org"), x0 + s.pad, y0 + s.pad);
    gl.uniform1f(p.loc("modI"), 2);
    gl.uniform2f(p.loc("scrMod"), 3.06, 1.63);
    gl.uniform2f(p.loc("scrMask"), -1.03, -2.08);
    return { ...p, time: p.loc("time") };
  }

  /** Loads and uploads an effect's textures once; later attaches wait for it. */
  function prepare(kind) {
    if (!loading.has(kind)) {
      const names = FX_TEXTURES[kind];
      loading.set(
        kind,
        Promise.all(names.map((name) => loadImage(name)))
          .then((images) => {
            if (lost || destroyed || !gl) return;
            const byName = Object.fromEntries(names.map((n, i) => [n, images[i]]));
            if (kind === "windfury") {
              ["wf_map_A0", "wf_map_A1", "wf_map_B0", "wf_map_B1"].forEach((n, i) => texture(gl, byName[n], UNITS.wfMap[i], { nearest: true }));
              texture(gl, byName.wind1, UNITS.wind, { repeat: true });
            } else {
              texture(gl, byName.StealthDome_old, UNITS.dome, { repeat: true });
              texture(gl, byName.StealthDomeMask, UNITS.mask, { repeat: true });
              texture(gl, byName.StealthDomeShadow_D, UNITS.shadow, { repeat: true });
              texture(gl, byName.st_cover, UNITS.cover, {});
            }
            if (gl.getError() !== gl.NO_ERROR) throw new Error("the GPU refused the textures");
            ready.add(kind);
          })
          .catch((error) => fail(kind, `${kind} textures are not usable (${error && error.message ? error.message : error})`)),
      );
    }
    return loading.get(kind);
  }

  function drawWindfury(offset, t) {
    const p = programs.windfury;
    gl.useProgram(p.pr);
    gl.blendFunc(gl.ONE, gl.ONE);
    const phase = (((t + offset) % WIND_CYCLE) + WIND_CYCLE) % WIND_CYCLE / WIND_CYCLE;
    const passes = [
      [0, -1.7 + 3 * phase, 0.7],
      [1, -1.7 + 3 * phase, 0.7],
      [2, 1.26 - 4 * phase, 0.45],
      [3, 1.26 - 4 * phase, 0.45],
    ];
    for (const [map, us, col] of passes) {
      gl.uniform1i(p.map, map);
      gl.uniform1f(p.us, us);
      gl.uniform1f(p.col, col);
      gl.drawArrays(gl.TRIANGLE_STRIP, 0, 4);
    }
  }

  function drawStealth(offset, t) {
    const p = programs.stealth;
    gl.useProgram(p.pr);
    gl.blendFunc(gl.ONE, gl.ONE_MINUS_SRC_ALPHA);
    gl.uniform1f(p.time, t + offset);
    gl.drawArrays(gl.TRIANGLE_STRIP, 0, 4);
  }

  /** One drawing of an effect, in the GL canvas, for a phase of its cycle. */
  function drawPhase(kind, size, phase, t) {
    const [w, h] = size;
    gl.viewport(0, 0, w, h);
    gl.scissor(0, 0, w, h);
    gl.clearColor(0, 0, 0, 0);
    gl.clear(gl.COLOR_BUFFER_BIT);
    if (kind === "windfury") drawWindfury((phase * WIND_CYCLE) / PHASES, t);
    else drawStealth(phase * 2.5, t);
  }

  /** Copies the GL canvas (read before the page is composed, in this task). */
  function copyTo(inst) {
    const [w, h] = inst.size;
    inst.ctx.clearRect(0, 0, w, h);
    inst.ctx.drawImage(glCanvas, 0, glCanvas.height - h, w, h, 0, 0, w, h);
  }

  /** Draws every visible effect once; true when something was drawn. */
  function drawFrame(t) {
    const groups = new Map();
    for (const inst of instances) {
      if (!ready.has(inst.kind) || !inst.inView || !visible(inst)) continue;
      const key = `${inst.kind}:${inst.phase}`;
      if (!groups.has(key)) groups.set(key, []);
      groups.get(key).push(inst);
    }
    for (const group of groups.values()) {
      const first = group[0];
      drawPhase(first.kind, first.size, first.phase, t);
      for (const inst of group) copyTo(inst);
    }
    return groups.size > 0;
  }

  function visible(inst) {
    return inst.canvas.isConnected && (!inst.canvas.getClientRects || inst.canvas.getClientRects().length > 0);
  }

  function tick() {
    timer = null;
    // A canvas not in the page yet may be about to be put there: dropped after a second.
    const stamp = now();
    for (const inst of [...instances]) {
      if (inst.canvas.isConnected) inst.gone = null;
      else if (inst.gone === null || inst.gone === undefined) inst.gone = stamp;
      else if (stamp - inst.gone > 1) remove(inst);
    }
    if (!instances.size || !active || lost) return;
    const started = now();
    let drew = false;
    try {
      drew = drawFrame(started);
    } catch (error) {
      fail("windfury", `effects stopped (${error && error.message ? error.message : error})`);
      fail("stealth", "effects stopped");
    }
    // With a visibility observer and nothing in view, sleep: coming into view
    // wakes the loop. A canvas that left the page is still swept out first.
    if (observe && [...instances].every((inst) => !inst.inView && inst.canvas.isConnected)) return;
    // Next frame one interval after this one started, less what drawing took;
    // when nothing showed (a hidden board still in the page) look again less often.
    timer = schedule(tick, drew ? interval - (now() - started) * 1000 : IDLE_CHECK_MS);
  }

  function wake() {
    if (timer === null && active && instances.size && !lost) timer = schedule(tick, 0);
  }

  let initError = null;
  try {
    init();
  } catch (error) {
    initError = error;
    tell("init", `Windfury and Stealth show as words: ${error.message}`);
  }

  const api = {
    supported: () => !initError && !lost && !destroyed,
    attach(kind, canvas, onLost) {
      if (initError || lost || destroyed || failed.has(kind) || !programs || !programs[kind] || !canvas.getContext) return false;
      const size = layout[kind].size;
      const ctx = canvas.getContext("2d");
      if (!ctx) return false;
      // With an observer an effect waits to be told it is in view.
      const inst = { kind, canvas, ctx, size, onLost, phase: nextPhase++ % PHASES, inView: !observe, unwatch: null };
      if (observe) {
        inst.unwatch = observe(canvas, (inView) => {
          inst.inView = inView === true;
          if (inst.inView) wake();
        });
      }
      instances.add(inst);
      prepare(kind).then(wake);
      return true;
    },
    setActive(on) {
      active = on !== false;
      if (!active && timer !== null) {
        unschedule(timer);
        timer = null;
      }
      if (active) wake();
    },
    destroy() {
      destroyed = true;
      for (const inst of [...instances]) remove(inst);
      if (timer !== null) unschedule(timer);
      timer = null;
      if (doc.removeEventListener) doc.removeEventListener("visibilitychange", onVisibility);
      // Give the GPU its memory back now, not when the page is collected.
      const lose = gl && gl.getExtension && gl.getExtension("WEBGL_lose_context");
      if (lose) lose.loseContext();
      programs = null;
    },
  };
  // Nothing draws while the window is hidden; it picks up when it shows.
  function onVisibility() {
    api.setActive(!doc.hidden);
  }
  if (doc.addEventListener) doc.addEventListener("visibilitychange", onVisibility);
  return api;
}

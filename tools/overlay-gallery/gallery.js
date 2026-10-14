// The gallery page: a frame per scenario, each frame the real overlay page
// (stored once, as one encoded block) at a fixed game-window size, with its
// caption under it. ?scenario=<id> shows one; ?lang=es the Spanish text.
"use strict";

(() => {
  const FRAME_W = 1920;
  const FRAME_H = 1080;
  // Phones start zoomed in on the part of the frame that matters.
  const PHONE_MAX = 820;
  const MAX_ZOOM = 2.4;
  const UI = {
    en: {
      title: "Overlay gallery",
      intro: "The overlay page of Tavern Ledger in every state it can show, with made-up example data and a short note on what each part is and where it comes from. Nothing here comes from a real game.",
      language: "Language:",
      index: "Frames",
      example: "Example",
      backdrop: "Game window · example",
      sees: "What you see",
      source: "Where it comes from",
      rule: "Only what the game shows you",
      all: "All frames",
      zoomIn: "Zoom in on the overlay",
      zoomOut: "Show the whole game window",
      foot: "Names and numbers are invented. Card names and pictures in a real game come from HearthstoneJSON, © Blizzard Entertainment.",
    },
    es: {
      title: "Galería de la superposición",
      intro: "La superposición de Tavern Ledger en cada estado que puede mostrar, con datos de ejemplo inventados y una nota breve sobre qué es cada parte y de dónde sale. Nada de esto viene de una partida real.",
      language: "Idioma:",
      index: "Cuadros",
      example: "Ejemplo",
      backdrop: "Ventana del juego · ejemplo",
      sees: "Lo que ves",
      source: "De dónde sale",
      rule: "Solo lo que el juego te enseña",
      all: "Todos los cuadros",
      zoomIn: "Acercar a la superposición",
      zoomOut: "Ver toda la ventana del juego",
      foot: "Los nombres y los números son inventados. En una partida real los nombres y las imágenes de las cartas vienen de HearthstoneJSON, © Blizzard Entertainment.",
    },
  };

  const params = new URLSearchParams(window.location.search);
  const lang = params.get("lang") === "es" ? "es" : "en";
  const wanted = params.get("scenario");
  const words = UI[lang];
  const data = JSON.parse(document.getElementById("gallery-data").textContent);
  const scenarios = data.scenarios;
  const shown = wanted && scenarios.some((s) => s.id === wanted) ? scenarios.filter((s) => s.id === wanted) : scenarios;

  function el(tag, className, text) {
    const node = document.createElement(tag);
    if (className) node.className = className;
    if (text !== undefined) node.textContent = text;
    return node;
  }

  function link(label, query, hash) {
    const a = el("a", null, label);
    const q = new URLSearchParams(query).toString();
    a.href = `${window.location.pathname}${q ? `?${q}` : ""}${hash || ""}`;
    return a;
  }

  // The overlay page, decoded once.
  let overlayDoc = null;
  function docText() {
    if (overlayDoc === null) {
      const bytes = Uint8Array.from(atob(document.getElementById("overlay-doc").textContent.trim()), (c) => c.charCodeAt(0));
      overlayDoc = new TextDecoder().decode(bytes);
    }
    return overlayDoc;
  }

  // The frame shows the whole game window, or only its focus region, larger.
  function fit(wrap) {
    const frame = wrap.querySelector("iframe");
    const focus = JSON.parse(wrap.dataset.focus);
    const zoomed = wrap.dataset.zoomed === "true";
    wrap.style.aspectRatio = zoomed ? `${focus.w} / ${focus.h}` : `${FRAME_W} / ${FRAME_H}`;
    wrap.style.maxWidth = zoomed ? `${Math.round(focus.w * MAX_ZOOM)}px` : "none";
    if (!frame) return;
    const region = zoomed ? focus : { x: 0, y: 0, w: FRAME_W };
    frame.style.transform = `scale(${wrap.clientWidth / region.w}) translate(${-region.x}px, ${-region.y}px)`;
  }

  // A frame is made when it comes near the screen: every page runs its own timers.
  function load(wrap, id) {
    if (wrap.querySelector("iframe")) return;
    const config = `window.__GALLERY__ = ${JSON.stringify({ scenario: id, lang })};`;
    const frame = document.createElement("iframe");
    frame.title = id;
    frame.setAttribute("scrolling", "no");
    frame.srcdoc = docText().replace("/*__GALLERY_CONFIG__*/", () => config);
    wrap.append(frame);
    wrap.dataset.loaded = "true";
    fit(wrap);
  }

  function shot(scenario) {
    const caption = scenario.caption[lang];
    const section = el("section", "shot");
    section.id = scenario.id;
    const bar = el("div", "bar");
    bar.append(el("h2", null, caption.title), el("span", "badge", words.example));
    section.append(bar);
    const wrap = el("div", "frame-wrap");
    wrap.dataset.backdrop = words.backdrop;
    wrap.dataset.scenario = scenario.id;
    wrap.dataset.focus = JSON.stringify(scenario.focus);
    wrap.dataset.zoomed = String(window.innerWidth < PHONE_MAX);
    const zoom = el("button", "zoom");
    zoom.type = "button";
    const label = () => { zoom.textContent = wrap.dataset.zoomed === "true" ? words.zoomOut : words.zoomIn; };
    label();
    zoom.addEventListener("click", () => {
      wrap.dataset.zoomed = String(wrap.dataset.zoomed !== "true");
      label();
      fit(wrap);
    });
    section.append(wrap, zoom);
    const list = el("dl", "caption");
    for (const key of ["sees", "source", "rule"]) {
      const group = el("div");
      group.append(el("dt", null, words[key]), el("dd", null, caption[key]));
      list.append(group);
    }
    section.append(list);
    return section;
  }

  document.documentElement.lang = lang;
  document.title = words.title;
  document.getElementById("title").textContent = words.title;
  document.getElementById("intro").textContent = words.intro;
  document.getElementById("foot").textContent = words.foot;
  document.getElementById("index-label").textContent = words.index;
  document.getElementById("lang-label").textContent = words.language;
  const langQuery = lang === "en" ? {} : { lang };
  const langLinks = document.getElementById("lang-links");
  for (const [code, label] of [["en", "English"], ["es", "Español"]]) {
    if (code === lang) langLinks.append(el("strong", null, label));
    else langLinks.append(link(label, { ...(wanted ? { scenario: wanted } : {}), lang: code }));
  }
  const index = document.getElementById("index");
  if (shown.length !== scenarios.length) index.append(link(words.all, langQuery));
  for (const s of scenarios) index.append(link(s.caption[lang].title, { scenario: s.id, ...langQuery }));
  const main = document.getElementById("shots");
  for (const s of shown) main.append(shot(s));

  const wraps = [...document.querySelectorAll(".frame-wrap")];
  const resize = new ResizeObserver(() => wraps.forEach(fit));
  wraps.forEach((wrap) => resize.observe(wrap));
  if ("IntersectionObserver" in window && wraps.length > 1) {
    const seen = new IntersectionObserver(
      (entries) => entries.forEach((e) => {
        if (e.isIntersecting) {
          load(e.target, e.target.dataset.scenario);
          seen.unobserve(e.target);
        }
      }),
      { rootMargin: "400px 0px" },
    );
    wraps.forEach((wrap) => seen.observe(wrap));
  } else {
    wraps.forEach((wrap) => load(wrap, wrap.dataset.scenario));
  }
})();

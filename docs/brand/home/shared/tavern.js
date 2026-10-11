// Tavern Ledger home page concepts (T-525): language toggle, hero portraits
// and sparks. No dependencies.
//
// Portraits load at view time from HearthstoneJSON's image host (D-038,
// D-074, Fan Content Policy) and are never committed. With
// <html data-portraits="silhouette"> (the published review page, whose CSP
// blocks that host) or when a picture fails, our own drawn silhouette shows
// instead.
(() => {
  "use strict";

  const ART_BASE = "https://art.hearthstonejson.com/v1/256x/";
  const CARD_ID = /^[A-Za-z0-9_]{1,64}$/;

  // Names from web/src/lib/heroes/names.data.ts (English, esES).
  const HEROES = {
    BG23_HERO_306: ["Sylvanas Windrunner", "Sylvanas Brisaveloz"],
    TB_BaconShop_HERO_22: ["The Lich King", "El Rey Exánime"],
    BG22_HERO_305: ["Onyxia", "Onyxia"],
    TB_BaconShop_HERO_41: ["Reno Jackson", "Reno Jackson"],
    TB_BaconShop_HERO_11: ["Ragnaros the Firelord", "Ragnaros, Señor del Fuego"],
    TB_BaconShop_HERO_52: ["Deathwing", "Alamuerte"],
    BG22_HERO_007: ["Queen Azshara", "Reina Azshara"],
    TB_BaconShop_HERO_18: ["Patches the Pirate", "Parches el Pirata"],
    TB_BaconShop_HERO_49: ["Millhouse Manastorm", "Molino Tormenta de Maná"],
    TB_BaconShop_HERO_08: ["Illidan Stormrage", "Illidan Tempestira"],
    BG20_HERO_100: ["Rokara", "Rokara"],
    BG25_HERO_100: ["Professor Putricide", "Profesor Putricidio"],
    TB_BaconShop_HERO_35: ["Yogg-Saron, Hope's End", "Yogg-Saron, Fin de la Esperanza"],
    TB_BaconShop_HERO_56: ["Alexstrasza", "Alexstrasza"],
    TB_BaconShop_HERO_KelThuzad: ["Kel'Thuzad", "Kel'Thuzad"],
    BG24_HERO_100: ["Sire Denathrius", "Sire Denathrius"],
  };

  const root = document.documentElement;
  const lang = () => (root.lang === "en" ? "en" : "es");

  // ---------- Silhouettes: our own drawing, tinted per hero ----------

  const TINTS = [
    ["#3a2414", "#ffb24a"],
    ["#14243a", "#8fb4ff"],
    ["#2a1834", "#d39bff"],
    ["#123024", "#8fe0a8"],
    ["#341414", "#ff9a7a"],
  ];

  function bucket(id, size) {
    let h = 0;
    for (const ch of id) h = (h * 31 + ch.charCodeAt(0)) >>> 0;
    return h % size;
  }

  function silhouette(id) {
    const [bg, light] = TINTS[bucket(id, TINTS.length)];
    const svg =
      `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 256 256">` +
      `<defs><radialGradient id="l" cx="50%" cy="38%" r="62%">` +
      `<stop offset="0" stop-color="${light}" stop-opacity=".75"/>` +
      `<stop offset=".55" stop-color="${light}" stop-opacity=".18"/>` +
      `<stop offset="1" stop-color="${bg}" stop-opacity="0"/></radialGradient></defs>` +
      `<rect width="256" height="256" fill="${bg}"/><rect width="256" height="256" fill="url(#l)"/>` +
      // hooded head and shoulders
      `<path fill="#0c0807" d="M128 52c-30 0-50 24-50 56 0 20 8 36 20 46-34 10-60 36-68 102h196c-8-66-34-92-68-102 12-10 20-26 20-46 0-32-20-56-50-56z"/>` +
      `<path fill="none" stroke="${light}" stroke-opacity=".5" stroke-width="2" d="M128 52c-30 0-50 24-50 56 0 20 8 36 20 46-34 10-60 36-68 102M128 52c30 0 50 24 50 56 0 20-8 36-20 46 34 10 60 36 68 102"/>` +
      `</svg>`;
    return "data:image/svg+xml," + encodeURIComponent(svg);
  }

  function heroName(id) {
    const names = HEROES[id];
    if (!names) return lang() === "es" ? "Héroe desconocido" : "Unknown hero";
    return names[lang() === "es" ? 1 : 0];
  }

  function loadPortraits() {
    const useSilhouettes = root.dataset.portraits === "silhouette";
    for (const img of document.querySelectorAll("img[data-hero]")) {
      const id = img.dataset.hero;
      if (!CARD_ID.test(id)) continue;
      img.decoding = "async";
      img.alt = heroName(id);
      if (useSilhouettes) {
        img.src = silhouette(id);
        continue;
      }
      img.addEventListener(
        "error",
        () => {
          img.src = silhouette(id);
        },
        { once: true },
      );
      img.src = ART_BASE + id + ".jpg";
    }
    for (const el of document.querySelectorAll("[data-hero-name]")) {
      el.textContent = heroName(el.dataset.heroName);
    }
  }

  // ---------- Language toggle ----------

  function applyLang(next) {
    root.lang = next;
    const title = root.dataset["title" + (next === "es" ? "Es" : "En")];
    if (title) document.title = title;
    for (const img of document.querySelectorAll("img[data-hero]")) img.alt = heroName(img.dataset.hero);
    for (const el of document.querySelectorAll("[data-hero-name]")) el.textContent = heroName(el.dataset.heroName);
    for (const btn of document.querySelectorAll(".lang-toggle")) {
      btn.setAttribute("aria-label", next === "es" ? "Read this page in English" : "Leer esta página en español");
    }
  }

  function initLang() {
    const wanted = new URLSearchParams(location.search).get("lang") || location.hash.slice(1);
    applyLang(wanted === "en" ? "en" : "es");
    for (const btn of document.querySelectorAll(".lang-toggle")) {
      btn.addEventListener("click", () => applyLang(lang() === "es" ? "en" : "es"));
    }
    // The review page shows every concept in a frame and switches them all.
    window.addEventListener("message", (event) => {
      if (event.source !== window.parent || window.parent === window) return;
      const next = event.data && event.data.tavernLang;
      if (next === "es" || next === "en") applyLang(next);
    });
  }

  // ---------- Sparks: embers drifting up from one side ----------

  const reduceMotion = window.matchMedia("(prefers-reduced-motion: reduce)");

  function sparks(canvas) {
    const ctx = canvas.getContext("2d");
    if (!ctx) return;
    const side = canvas.dataset.side || "left"; // left | right | bottom | all
    const hue = canvas.dataset.hue === "blue" ? "blue" : "gold";
    const count = Number(canvas.dataset.count) || 46;
    let w = 0;
    let h = 0;
    let dpr = 1;
    let parts = [];
    let raf = 0;

    function spawnX() {
      if (side === "left") return Math.random() * w * 0.32;
      if (side === "right") return w - Math.random() * w * 0.32;
      return Math.random() * w;
    }
    function make(initial) {
      return {
        x: spawnX(),
        y: initial ? Math.random() * h : h + Math.random() * 40,
        r: 0.6 + Math.random() * 1.9,
        vy: 0.25 + Math.random() * 0.75,
        drift: Math.random() * Math.PI * 2,
        life: 0.4 + Math.random() * 0.6,
      };
    }
    function resize() {
      dpr = Math.min(window.devicePixelRatio || 1, 2);
      w = canvas.clientWidth;
      h = canvas.clientHeight;
      canvas.width = Math.round(w * dpr);
      canvas.height = Math.round(h * dpr);
      ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
      parts = Array.from({ length: count }, () => make(true));
    }
    function draw() {
      ctx.clearRect(0, 0, w, h);
      ctx.globalCompositeOperation = "lighter";
      for (const p of parts) {
        const fade = Math.max(0, Math.min(1, p.y / h)) * p.life;
        const g = ctx.createRadialGradient(p.x, p.y, 0, p.x, p.y, p.r * 5);
        if (hue === "blue") {
          g.addColorStop(0, `rgba(220,240,255,${fade})`);
          g.addColorStop(0.3, `rgba(110,180,255,${fade * 0.7})`);
        } else {
          g.addColorStop(0, `rgba(255,245,210,${fade})`);
          g.addColorStop(0.3, `rgba(255,170,60,${fade * 0.75})`);
        }
        g.addColorStop(1, "rgba(0,0,0,0)");
        ctx.fillStyle = g;
        ctx.beginPath();
        ctx.arc(p.x, p.y, p.r * 5, 0, Math.PI * 2);
        ctx.fill();
      }
    }
    function step() {
      for (const p of parts) {
        p.y -= p.vy;
        p.drift += 0.02;
        p.x += Math.sin(p.drift) * 0.35;
        if (p.y < -10) Object.assign(p, make(false));
      }
      draw();
      raf = requestAnimationFrame(step);
    }
    function start() {
      cancelAnimationFrame(raf);
      resize();
      if (reduceMotion.matches || document.hidden) draw();
      else raf = requestAnimationFrame(step);
    }
    new ResizeObserver(start).observe(canvas);
    reduceMotion.addEventListener("change", start);
    document.addEventListener("visibilitychange", start);
  }

  initLang();
  loadPortraits();
  for (const c of document.querySelectorAll("canvas.sparks")) sparks(c);
})();

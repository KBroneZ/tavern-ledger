// The layout round's frames: the same example games under each layout
// variant (variants.js), so they compare fairly, in Solo and Duos, in a
// Recruit phase and in combat, at 1920x1080 and 2560x1440, over the plain
// game window of backdrop.js. Built with `build.js --layouts`.
"use strict";

const data = require("../scenarios/data.js");

const { soloGame, duosGame, SOLO_ODDS, DUOS_BOARDS } = data;

// A typical mid-game set of the player's own totals (four of the kinds).
const COUNTERS = [
  { kind: "blood_gem", atk: 3, health: 2 },
  { kind: "tavern_spell", atk: 2, health: 0 },
  { kind: "undead_attack", atk: 4 },
  { kind: "free_rolls", count: 2 },
];

const DUOS_ODDS = { ...SOLO_ODDS, seat: 1, round: 6, boards: DUOS_BOARDS([6, 6, 6]), won: 0.47, tied: 0.12, lost: 0.41 };

// Odds show in a Recruit phase only, as in the app.
function game(mode, phase) {
  const base = mode === "duos" ? duosGame() : soloGame();
  const odds = phase === "recruit" ? (mode === "duos" ? DUOS_ODDS : SOLO_ODDS) : null;
  return { ...base, counters: COUNTERS, odds };
}

const SCREENS = { 1080: [1920, 1080], 1440: [2560, 1440] };

// Every variant gets the same four frames.
const FRAMES = [
  { mode: "solo", phase: "recruit", screen: 1080 },
  { mode: "duos", phase: "combat", screen: 1080 },
  { mode: "solo", phase: "combat", screen: 1440 },
  { mode: "duos", phase: "recruit", screen: 1440 },
];

const VARIANTS = [
  {
    id: "a",
    layout: { place: "above" },
    caption: {
      en: {
        title: "A · Counters above the opponents",
        change: "Counters first, then the opponents list with the lobby's tribes as its header.",
        why: "In the Recruit phase the eye starts on your own buffs, which decide what you buy; the opponents list starts lower and moves down when a new counter appears. In combat the counters are not needed but stay on top.",
      },
      es: {
        title: "A · Contadores encima de los rivales",
        change: "Primero los contadores, luego la lista de rivales con las tribus de la partida como cabecera.",
        why: "En la fase de reclutamiento la vista empieza en tus mejoras, que deciden qué compras; la lista de rivales empieza más abajo y baja cuando aparece un contador nuevo. En combate los contadores no hacen falta pero siguen arriba.",
      },
    },
  },
  {
    id: "b",
    layout: { place: "below" },
    caption: {
      en: {
        title: "B · Counters below the opponents",
        change: "The opponents list first, with the tribes as its header; the counters under it.",
        why: "The opponents list never moves, so each row is always in the same place; the counters are lower, read when you buy. In combat the eye stays on the list and the counters are out of the way.",
      },
      es: {
        title: "B · Contadores debajo de los rivales",
        change: "Primero la lista de rivales, con las tribus como cabecera; los contadores debajo.",
        why: "La lista de rivales no se mueve nunca, así que cada fila está siempre en el mismo sitio; los contadores quedan más abajo y se leen al comprar. En combate la vista se queda en la lista y los contadores no estorban.",
      },
    },
  },
  {
    id: "c",
    layout: { place: "beside" },
    caption: {
      en: {
        title: "C · Counters beside the opponents",
        change: "Counters in a column of their own to the left of the opponents list, both from the top.",
        why: "Neither panel moves the other and both are at eye height in the Recruit phase; the overlay is twice as wide, closer to the board and the end-turn button, and that width stays during combat.",
      },
      es: {
        title: "C · Contadores al lado de los rivales",
        change: "Los contadores en su propia columna, a la izquierda de la lista de rivales, los dos desde arriba.",
        why: "Ningún panel mueve al otro y los dos están a la altura de la vista en el reclutamiento; la superposición ocupa el doble de ancho, más cerca del tablero y del botón de fin de turno, y ese ancho sigue en combate.",
      },
    },
  },
];

const TODAY = {
  id: "today",
  layout: null,
  caption: {
    en: {
      title: "Today, for comparison",
      change: "The overlay as it ships now: the tribes in a box of their own and no counters.",
      why: "The starting point every variant is measured against.",
    },
    es: {
      title: "Hoy, para comparar",
      change: "La superposición tal como sale ahora: las tribus en su propia caja y sin contadores.",
      why: "El punto de partida con el que se compara cada variante.",
    },
  },
};

const HIDE_ALL = {
  id: "hide-all",
  layout: { place: "above", hideAll: true },
  caption: {
    en: {
      title: "Hide-all key (every variant)",
      change: "One key hides everything the overlay draws, and the same key brings it back; a short note says which key.",
      why: "A clean screen for a tense turn or a stream without opening the app. The key, Ctrl+Shift+H, is a proposal.",
    },
    es: {
      title: "Tecla para ocultarlo todo (todas las variantes)",
      change: "Una tecla oculta todo lo que dibuja la superposición y la misma tecla lo vuelve a mostrar; una nota breve dice cuál es.",
      why: "Pantalla limpia en un turno tenso o en un directo sin abrir la aplicación. La tecla, Ctrl+Mayús+H, es una propuesta.",
    },
  },
};

const FRAME_WORDS = {
  en: { solo: "Solo", duos: "Duos", recruit: "Recruit phase", combat: "Combat" },
  es: { solo: "Solo", duos: "Dúos", recruit: "Reclutamiento", combat: "Combate" },
};

// The right-hand part of the window, where the stack is: what a phone zooms in on.
function focus(screen, wide) {
  const [w, h] = SCREENS[screen];
  const share = wide ? 0.24 : 0.13;
  return { x: Math.round(w * (1 - share)), y: Math.round(h * 0.06), w: Math.round(w * share), h: Math.round(h * 0.56) };
}

function frameCaption(variant, frame) {
  const out = {};
  for (const lang of ["en", "es"]) {
    const c = variant.caption[lang];
    const words = FRAME_WORDS[lang];
    const [w, h] = SCREENS[frame.screen];
    out[lang] = { ...c, title: `${c.title} — ${words[frame.mode]} · ${words[frame.phase]} · ${w}×${h}` };
  }
  return out;
}

function scenario(variant, frame) {
  return {
    id: `${variant.id}-${frame.mode}-${frame.phase}-${frame.screen}`,
    screen: SCREENS[frame.screen],
    state: {
      game: game(frame.mode, frame.phase),
      scene: null,
      lobby: null,
      hover: null,
      settings: { theme: "glass" },
      names: data.NAMES,
      layout: variant.layout,
      backdrop: { phase: frame.phase, duos: frame.mode === "duos" },
    },
    focus: focus(frame.screen, variant.layout && variant.layout.place === "beside"),
    caption: frameCaption(variant, frame),
  };
}

const SCENARIOS = [
  ...VARIANTS.flatMap((v) => FRAMES.map((f) => scenario(v, f))),
  scenario(TODAY, FRAMES[0]),
  scenario(HIDE_ALL, FRAMES[0]),
];

const PAGE = {
  en: {
    title: "Overlay layouts: pick one",
    intro:
      "Three ways to lay out the one overlay, all on the real overlay page with the same made-up game. In every variant the counters panel is back, the lobby's tribes are the header of the opponents list, the legend is not shown, and one key hides everything. They differ only in where the counters sit. The game window behind is plain shapes, not the game.",
    foot: "Names and numbers are invented. The shapes behind the overlay stand in for the game window; only the leaderboard is where the app measures it.",
  },
  es: {
    title: "Diseños de la superposición: elige uno",
    intro:
      "Tres maneras de colocar la única superposición, todas con la página real de la superposición y la misma partida inventada. En todas vuelve el panel de contadores, las tribus de la partida son la cabecera de la lista de rivales, la leyenda no se muestra y una tecla lo oculta todo. Solo cambia dónde van los contadores. La ventana del juego del fondo son formas simples, no el juego.",
    foot: "Los nombres y los números son inventados. Las formas detrás de la superposición sustituyen a la ventana del juego; solo la clasificación está donde la mide la aplicación.",
  },
  fields: [
    { key: "change", label: { en: "What changes", es: "Qué cambia" } },
    { key: "why", label: { en: "Why", es: "Por qué" } },
  ],
};

module.exports = { SCENARIOS, PAGE, VARIANTS, FRAMES, COUNTERS };

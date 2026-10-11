// Spanish (Spain) for lib/accuracy/strings.ts (T-607): the same keys and placeholders.
import type { AccKey } from "./strings.ts";

export const ES: Record<AccKey, string> = {
  "page.kicker": "Simulador de combates",
  "page.title": "Cuántas veces aciertan las probabilidades",
  "page.intro":
    "El simulador que da las probabilidades del overlay, comprobado con combates reales: para cada combate da sus probabilidades de victoria, empate y derrota, y el registro del juego dice qué pasó de verdad.",
  "page.version": "Motor {version} · medido el {date} · {trials} pruebas por combate",

  "mode.solo": "Solo",
  "mode.duos": "Dúos",
  "mode.combats": "{count} combates con el resultado del juego",

  "tile.right": "El resultado más probable acertó",
  "tile.meanP": "Probabilidad media dada a lo que pasó",
  "tile.meanP.back": "en {count} combates puntuados",
  "tile.blocked": "Sin probabilidades: falta una carta",
  "tile.brier": "Puntuación de Brier",
  "tile.brier.back": "0 es lo mejor, 2 lo peor",
  "tile.refused": "Rechazados por la comprobación del tablero",

  "right.backing": "{right} de {scored} combates",
  "blocked.some": "Sin probabilidades en el {share} de los combates de {mode} ({blocked} de {total}): una carta no está modelada.",
  "blocked.none": "Probabilidades en todos los combates de {mode} con resultado ({total}): no faltaba ninguna carta.",
  "blocked.empty": "Aún no hay combates de {mode} con resultado.",

  "cal.title": "Lo que dijo y lo que pasó",
  "cal.desc":
    "Cada probabilidad que dio el simulador en {mode} (victoria, empate y derrota por combate), agrupada según lo que dijo. Al lado de cada grupo, cuántas veces pasó ese resultado de verdad. Si las barras se parecen, las probabilidades son fiables.",
  "cal.key.said": "Dijo",
  "cal.key.happened": "Pasó",
  "cal.none": "sin probabilidades",
  "cal.mark": "Dijo {range}: {predictions} probabilidades, de media dijo {said}, pasó {happened}",
  "cal.table": "Ver como tabla",
  "cal.col.range": "Dijo",
  "cal.col.predictions": "Probabilidades",
  "cal.col.said": "Dijo de media",
  "cal.col.happened": "Pasó",

  "cards.title": "Cartas que más bloquean las probabilidades",
  "cards.desc":
    "Cuando una de estas está en un tablero, el simulador no da probabilidades en vez de dar un número equivocado. Contadas en Solo y Dúos juntos.",
  "cards.col.card": "Carta",
  "cards.col.id": "Id de carta",
  "cards.col.combats": "Combates bloqueados",
  "cards.none": "Ninguna carta bloqueó un combate.",
  "card.golden": "{name} (versión dorada)",

  "about.title": "Qué se mide y sus límites",
  "about.body":
    "Solo los combates de quien hace Tavern Ledger, de sus propias partidas, con los tableros tal como entraron en cada combate (el primer ataque). Los efectos de inicio de combate están desactivados, como en el overlay. Un combate cuenta como puntuado cuando el simulador pudo jugarlo; al lado siempre se muestra la parte de combates sin probabilidades. Pocos números y un solo jugador: una guía, no una promesa.",
  "about.refresh": "Estas cifras se actualizan después de cada ronda de cartas nuevas.",
};

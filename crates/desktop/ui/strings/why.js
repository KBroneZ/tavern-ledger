// Words of "why the result differed" in the recap: why.js (T-346, D-127),
// the same as the website's game page. English is the source; Spanish
// (Spain) is complete (ui-tests/i18n.test.js). Each row is [key, English,
// Spanish]. Prefix: why.
"use strict";

(() => {
  const rows = [
    ["why.title", "Odds and why results differed", "Probabilidades y por qué cambió el resultado"],
    ["why.about", "For each combat: the odds the simulator gives on the boards as they fought, the result and, when the likeliest result did not happen, why. The website's accuracy page lists every cause.",
      "Para cada combate: las probabilidades que da el simulador con los tableros tal como combatieron, el resultado y, si no pasó lo más probable, por qué. La página de precisión de la web explica cada causa."],
    ["why.loading", "Working out the odds of each combat…", "Calculando las probabilidades de cada combate…"],
    ["why.failed", "The odds of this game could not be worked out.", "No se pudieron calcular las probabilidades de esta partida."],
    ["why.none", "No combat of this game has a result.", "Ningún combate de esta partida tiene resultado."],
    ["why.count", "{missed} of {total} combats did not go the likeliest way.", "{missed} de {total} combates no acabaron como era más probable."],
    ["why.turn", "Turn {turn}", "Turno {turn}"],
    ["why.odds", "{won} win · {tied} tie · {lost} loss", "{won} victoria · {tied} empate · {lost} derrota"],
    ["why.noOddsShort", "no odds", "sin probabilidades"],
    ["why.more", "Details", "Detalles"],
    ["why.matched", "As expected.", "Lo esperado."],
    ["why.chance", "Chance: the replay matched until the dice.", "Azar: la repetición coincidió hasta los dados."],
    ["why.missing", "Not seen: {what}.", "No visto: {what}."],
    ["why.approximate", "Approximate: {cards}.", "Aproximado: {cards}."],
    ["why.rule", "Rule differs at attack {attack}: {card}.", "Una regla cambia en el ataque {attack}: {card}."],
    ["why.ruleNoCard", "Rule differs at attack {attack}.", "Una regla cambia en el ataque {attack}."],
    ["why.replay.not_saved", "No replay saved, so no cause is given.", "No se guardó repetición, así que no se da causa."],
    ["why.replay.unreadable", "The saved replay could not be read, so no cause is given.", "No se pudo leer la repetición guardada, así que no se da causa."],
    ["why.replay.stops", "The replay stops early, so no cause is given.", "La repetición se corta antes del final, así que no se da causa."],
    ["why.noOdds.no_boards", "No odds: a board of this combat was not seen.", "Sin probabilidades: no se vio un tablero de este combate."],
    ["why.noOdds.no_card_data", "No odds: no card data for {cards}.", "Sin probabilidades: no hay datos de {cards}."],
    ["why.noOdds.not_modelled", "No odds: the simulator does not know {cards}.", "Sin probabilidades: el simulador no conoce {cards}."],
    ["why.noOdds.bad_board", "No odds: the simulator refused these boards.", "Sin probabilidades: el simulador rechazó estos tableros."],
    ["why.unseen.blood_gems", "Blood Gem upgrades", "las mejoras de las Gemas de sangre"],
    ["why.unseen.spells", "Tavern spell upgrades", "las mejoras de los hechizos de taberna"],
    ["why.unseen.knights", "Eternal Knights that died", "los Caballeros eternos muertos"],
    ["why.unseen.beetles", "Beetle buffs", "las mejoras de los Escarabajos"],
    ["why.unseen.hand", "a minion from the hand ({card})", "un esbirro desde la mano ({card})"],
    ["why.unseen.earlier", "what {card} gained earlier in the game", "lo que {card} ganó antes en la partida"],
    ["why.unseen.gift", "a Dark Gift on {card}", "un Don siniestro en {card}"],
    ["why.ev.chance", "The simulator played all {attacks} attacks as the game did; the less likely result happened.",
      "El simulador jugó los {attacks} ataques como la partida; pasó el resultado menos probable."],
    ["why.ev.parts", "The replay parts from the game at attack {attack} ({rule}){where}.", "La repetición se aparta de la partida en el ataque {attack} ({rule}){where}."],
    ["why.ev.partsAtCard", ", on {card}", ", en {card}"],
    ["why.ev.notSaved", "The game did not play this combat back on your screen, or the game was saved before replays were kept.",
      "La partida no mostró este combate en tu pantalla, o se guardó antes de que se guardaran las repeticiones."],
    ["why.ev.missing", "A board brings what the log does not show, so the simulator could not play this combat exactly.",
      "Un tablero trae algo que el registro no muestra, así que el simulador no pudo jugar este combate con exactitud."],
    ["why.ev.approximate", "The simulator does not know these cards, or plays them with a known gap.",
      "El simulador no conoce estas cartas o las juega con un fallo conocido."],
    ["why.rule.replay_first_attacker", "who attacks first", "quién ataca primero"],
    ["why.rule.replay_after_teammate", "who attacks after a teammate joins", "quién ataca tras entrar un compañero"],
    ["why.rule.replay_attack_order", "attack order", "orden de ataque"],
    ["why.rule.replay_target", "target choice", "elección de objetivo"],
    ["why.rule.replay_minions", "deaths and summons", "muertes e invocaciones"],
    ["why.rule.replay_stats", "stats and keywords", "estadísticas y palabras clave"],
    ["why.rule.replay_end", "end of combat", "fin del combate"],
    ["why.rule.replay_no_defender", "defender not in the log", "defensor ausente del registro"],
  ];

  const table = { en: {}, es: {} };
  rows.forEach(([key, en, es]) => {
    table.en[key] = en;
    table.es[key] = es;
  });

  const i18n = typeof module !== "undefined" && module.exports ? require("../i18n.js") : window.TLI18n;
  i18n.define(table);
  if (typeof module !== "undefined" && module.exports) module.exports = table;
})();

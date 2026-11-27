// Words of "why the result differed" in the recap: why.js (T-346, D-127),
// the same as the website's game page. English is the source; Spanish
// (Spain) and Russian are complete (ui-tests/i18n.test.js). Each row is
// [key, English, Spanish, Russian]. Prefix: why.
"use strict";

(() => {
  const rows = [
    ["why.title", "Odds and why results differed", "Probabilidades y por qué cambió el resultado", "Шансы и причины расхождения результатов"],
    ["why.about",
      "For each combat: the odds the simulator gives on the boards as they fought, the result and, when the likeliest result did not happen, why. The website's accuracy page lists every cause.",
      "Para cada combate: las probabilidades que da el simulador con los tableros tal como combatieron, el resultado y, si no pasó lo más probable, por qué. La página de precisión de la web explica cada causa.",
      "Для каждого боя: шансы от симулятора для сражавшихся столов, фактический исход и причина расхождения, если победил менее вероятный исход. Все причины описаны на странице точности на сайте."],
    ["why.loading", "Working out the odds of each combat…", "Calculando las probabilidades de cada combate…", "Расчёт шансов для каждого боя…"],
    ["why.failed", "The odds of this game could not be worked out.", "No se pudieron calcular las probabilidades de esta partida.", "Не удалось рассчитать шансы для этой игры."],
    ["why.none", "No combat of this game has a result.", "Ningún combate de esta partida tiene resultado.", "В этой игре ни у одного боя нет результата."],
    ["why.count", "{missed} of {total} combats did not go the likeliest way.", "{missed} de {total} combates no acabaron como era más probable.", "{missed} из {total} боёв закончились не самым вероятным исходом."],
    ["why.turn", "Turn {turn}", "Turno {turn}", "Ход {turn}"],
    ["why.odds", "{won} win · {tied} tie · {lost} loss", "{won} victoria · {tied} empate · {lost} derrota", "{won} победа · {tied} ничья · {lost} поражение"],
    ["why.noOddsShort", "no odds", "sin probabilidades", "нет шансов"],
    ["why.more", "Details", "Detalles", "Подробнее"],
    ["why.matched", "As expected.", "Lo esperado.", "Как ожидалось."],
    ["why.chance", "Dice luck: the replay matched the game until the dice rolled.", "Azar: la repetición coincidió hasta los dados.", "Удача: повтор совпадал с игрой до случайного события."],
    ["why.missing", "Not seen: {what}.", "No visto: {what}.", "Неизвестно: {what}."],
    ["why.approximate", "Approximate: {cards}.", "Aproximado: {cards}.", "Приблизительно: {cards}."],
    ["why.rule", "Rule differs at attack {attack}: {card}.", "Una regla cambia en el ataque {attack}: {card}.", "Расхождение правил на атаке {attack}: {card}."],
    ["why.ruleNoCard", "Rule differs at attack {attack}.", "Una regla cambia en el ataque {attack}.", "Расхождение правил на атаке {attack}."],
    ["why.replay.not_saved", "No replay saved, so no cause is given.", "No se guardó repetición, así que no se da causa.", "Повтор не сохранён, поэтому причина не указана."],
    ["why.replay.unreadable", "The saved replay could not be read, so no cause is given.", "No se pudo leer la repetición guardada, así que no se da causa.", "Не удалось прочитать сохранённый повтор, поэтому причина не указана."],
    ["why.replay.stops", "The replay stops early, so no cause is given.", "La repetición se corta antes del final, así que no se da causa.", "Повтор обрывается раньше конца, поэтому причина не указана."],
    ["why.noOdds.no_boards", "No odds: a board of this combat was not seen.", "Sin probabilidades: no se vio un tablero de este combate.", "Нет шансов: один из столов в этом бою не был виден."],
    ["why.noOdds.no_card_data", "No odds: no card data for {cards}.", "Sin probabilidades: no hay datos de {cards}.", "Нет шансов: нет данных карт для {cards}."],
    ["why.noOdds.not_modelled", "No odds: the simulator does not know {cards}.", "Sin probabilidades: el simulador no conoce {cards}.", "Нет шансов: симулятор не поддерживает {cards}."],
    ["why.noOdds.bad_board", "No odds: the simulator refused these boards.", "Sin probabilidades: el simulador rechazó estos tableros.", "Нет шансов: симулятор отклонил эти столы."],
    ["why.unseen.blood_gems", "Blood Gem upgrades", "las mejoras de las Gemas de sangre", "улучшения Кровавых самоцветов"],
    ["why.unseen.spells", "Tavern spell upgrades", "las mejoras de los hechizos de taberna", "улучшения заклинаний таверны"],
    ["why.unseen.knights", "Eternal Knights that died", "los Caballeros eternos muertos", "погибшие Вечные рыцари"],
    ["why.unseen.beetles", "Beetle buffs", "las mejoras de los Escarabajos", "усиления жуков"],
    ["why.unseen.hand", "a minion from the hand ({card})", "un esbirro desde la mano ({card})", "существо из руки ({card})"],
    ["why.unseen.earlier", "what {card} gained earlier in the game", "lo que {card} ganó antes en la partida", "усиления, которые {card} получил ранее в игре"],
    ["why.unseen.gift", "a Dark Gift on {card}", "un Don siniestro en {card}", "Тёмный дар на карте {card}"],
    ["why.ev.chance",
      "The simulator played all {attacks} attacks as the game did; the less likely result happened.",
      "El simulador jugó los {attacks} ataques como la partida; pasó el resultado menos probable.",
      "Симулятор воспроизвёл все атаки ({attacks}) точно как в игре; выпал менее вероятный исход."],
    ["why.ev.parts",
      "The replay parts from the game at attack {attack} ({rule}){where}.",
      "La repetición se aparta de la partida en el ataque {attack} ({rule}){where}.",
      "Повтор расходится с игрой на атаке {attack} ({rule}){where}."],
    ["why.ev.partsAtCard", ", on {card}", ", en {card}", ", на карте {card}"],
    ["why.ev.notSaved",
      "The game did not play this combat back on your screen, or the game was saved before replays were kept.",
      "La partida no mostró este combate en tu pantalla, o se guardó antes de que se guardaran las repeticiones.",
      "Игра не показывала этот бой на вашем экране, либо матч был сохранён до появления записи повторов."],
    ["why.ev.missing",
      "A board brings what the log does not show, so the simulator could not play this combat exactly.",
      "Un tablero trae algo que el registro no muestra, así que el simulador no pudo jugar este combate con exactitud.",
      "На столе есть данные, которых нет в журнале, поэтому симулятор не смог воспроизвести этот бой точно."],
    ["why.ev.approximate",
      "The simulator does not know these cards, or plays them with a known gap.",
      "El simulador no conoce estas cartas o las juega con un fallo conocido.",
      "Симулятор не знает эти карты или разыгрывает их с известным ограничением."],
    ["why.rule.replay_first_attacker", "who attacks first", "quién ataca primero", "кто атакует первым"],
    ["why.rule.replay_after_teammate", "who attacks after a teammate joins", "quién ataca tras entrar un compañero", "кто атакует после выхода напарника"],
    ["why.rule.replay_attack_order", "attack order", "orden de ataque", "порядок атак"],
    ["why.rule.replay_target", "target choice", "elección de objetivo", "выбор цели"],
    ["why.rule.replay_minions", "deaths and summons", "muertes e invocaciones", "гибель и призыв существ"],
    ["why.rule.replay_stats", "stats and keywords", "estadísticas y palabras clave", "характеристики и свойства"],
    ["why.rule.replay_end", "end of combat", "fin del combate", "конец боя"],
    ["why.rule.replay_no_defender", "defender not in the log", "defensor ausente del registro", "защищающегося нет в журнале"],
  ];

  const table = { en: {}, es: {}, ru: {} };
  rows.forEach(([key, en, es, ru]) => {
    table.en[key] = en;
    table.es[key] = es;
    table.ru[key] = ru;
  });

  const i18n = typeof module !== "undefined" && module.exports ? require("../i18n.js") : window.TLI18n;
  i18n.define(table);
  if (typeof module !== "undefined" && module.exports) module.exports = table;
})();

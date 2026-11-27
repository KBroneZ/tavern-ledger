// Words of the combat replays in the recap: replay.js (T-215, D-099). English
// is the source; Spanish (Spain) and Russian are complete (ui-tests/i18n.test.js).
// Each row is [key, English, Spanish, Russian]. Prefix: replay.
"use strict";

(() => {
  const rows = [
    ["replay.title", "Combat replays", "Repeticiones de combate", "Повторы боёв"],
    ["replay.pick", "Combat to replay", "Combate que repetir", "Выбор боя для повтора"],
    ["replay.chip", "Turn {turn}", "Turno {turn}", "Ход {turn}"],
    ["replay.chip_tip", "Turn {turn}: {outcome}", "Turno {turn}: {outcome}", "Ход {turn}: {outcome}"],
    ["replay.combat_of", "Turn {turn} against {who}", "Turno {turn} contra {who}", "Ход {turn} против {who}"],
    ["replay.step_of", "Step {n} of {total}", "Paso {n} de {total}", "Шаг {n} из {total}"],
    ["replay.play", "Play", "Reproducir", "Воспроизвести"],
    ["replay.pause", "Pause", "Pausa", "Пауза"],
    ["replay.back", "Step back", "Paso atrás", "Шаг назад"],
    ["replay.forward", "Step forward", "Paso adelante", "Шаг вперёд"],
    ["replay.restart", "Back to the start", "Volver al inicio", "В начало"],
    ["replay.speed", "Speed", "Velocidad", "Скорость"],
    ["replay.speed_x", "{n}×", "{n}×", "{n}×"],
    ["replay.steps", "Every step ({n})", "Todos los pasos ({n})", "Все шаги ({n})"],
    ["replay.start", "Both boards as the combat starts.", "Los dos tableros al empezar el combate.", "Оба стола в начале боя."],
    ["replay.your", "your {name}", "tu {name}", "ваш {name}"],
    ["replay.their", "their {name}", "{name} rival", "{name} соперника"],
    ["replay.ev.attack", "{a} attacks {b}", "{a} ataca a {b}", "{a} атакует {b}"],
    ["replay.ev.attack_alone", "{a} attacks", "{a} ataca", "{a} атакует"],
    ["replay.ev.damage", "{m} takes {n} damage ({left} health left)", "{m} recibe {n} de daño (vida restante: {left})", "{m} получает {n} урона (осталось здоровья: {left})"],
    ["replay.ev.damage_lethal", "{m} takes {n} damage", "{m} recibe {n} de daño", "{m} получает {n} урона"],
    ["replay.ev.damage_by", "{m} takes {n} damage from {src} ({how}; {left} health left)", "{m} recibe {n} de daño de {src} ({how}; vida restante: {left})", "{m} получает {n} урона от {src} ({how}; осталось здоровья: {left})"],
    ["replay.ev.damage_by_lethal", "{m} takes {n} damage from {src} ({how})", "{m} recibe {n} de daño de {src} ({how})", "{m} получает {n} урона от {src} ({how})"],
    ["replay.ev.damage_unknown", "{m} takes {n} damage from an unknown source ({left} health left)", "{m} recibe {n} de daño de origen desconocido (vida restante: {left})", "{m} получает {n} урона из неизвестного источника (осталось здоровья: {left})"],
    ["replay.ev.damage_unknown_lethal", "{m} takes {n} damage from an unknown source", "{m} recibe {n} de daño de origen desconocido", "{m} получает {n} урона из неизвестного источника"],
    ["replay.ev.keyword_on", "{m} gets {k}", "{m} obtiene {k}", "{m} получает «{k}»"],
    ["replay.ev.keyword_off", "{m} loses {k}", "{m} pierde {k}", "{m} теряет «{k}»"],
    ["replay.how.attack", "attack", "ataque", "атака"],
    ["replay.how.start_of_combat", "start of combat", "inicio del combate", "начало боя"],
    ["replay.how.deathrattle", "Deathrattle", "Último aliento", "Предсмертный хрип"],
    ["replay.how.avenge", "Avenge", "Venganza", "Месть"],
    ["replay.how.rally", "Rally", "Alentar", "Сплочение"],
    ["replay.how.hero_power", "hero power", "poder de héroe", "сила героя"],
    ["replay.how.trinket", "trinket", "abalorio", "аксессуар"],
    ["replay.how.effect", "effect", "efecto", "эффект"],
    ["replay.ev.shield_off", "{m} loses Divine Shield", "{m} pierde el Escudo divino", "{m} теряет Божественный щит"],
    ["replay.ev.shield_on", "{m} gets Divine Shield", "{m} obtiene Escudo divino", "{m} получает Божественный щит"],
    ["replay.ev.stats", "{m} is now {atk}/{health}", "{m} pasa a {atk}/{health}", "{m} теперь {atk}/{health}"],
    // The tooltip of a minion (T-619): exact numbers and its buffs.
    ["replay.tip.attack", "Attack {value}", "Ataque {value}", "Атака: {value}"],
    ["replay.tip.health", "Health {value}", "Vida {value}", "Здоровье: {value}"],
    ["replay.tip.buffs", "Buffs when the combat began", "Mejoras al empezar el combate", "Усиления в начале боя"],
    ["replay.tip.none", "No buffs", "Sin mejoras", "Без усилений"],
    ["replay.tip.unknown", "Buffs are not recorded for this game.", "Las mejoras no se guardaron en esta partida.", "Усиления для этой игры не записаны."],
    ["replay.tip.buff", "{name} ×{count}", "{name} ×{count}", "{name} ×{count}"],
    ["replay.ev.death", "{m} dies", "{m} muere", "{m} погибает"],
    ["replay.ev.summon", "{m} is summoned", "se invoca {m}", "призывается {m}"],
    ["replay.ev.card_own", "your {name}", "tu {name}", "ваш {name}"],
    ["replay.ev.card_opponent", "their {name}", "{name} rival", "{name} соперника"],
    ["replay.ev.trigger.start_of_combat", "Start of combat: {m}", "Inicio del combate: {m}", "Начало боя: {m}"],
    ["replay.ev.trigger.deathrattle", "Deathrattle: {m}", "Último aliento: {m}", "Предсмертный хрип: {m}"],
    ["replay.ev.trigger.avenge", "Avenge: {m}", "Venganza: {m}", "Месть: {m}"],
    ["replay.ev.trigger.rally", "Rally: {m}", "Alentar: {m}", "Сплочение: {m}"],
    ["replay.ev.hero_own", "Your hero takes {n} damage", "Tu héroe recibe {n} de daño", "Ваш герой получает {n} урона"],
    ["replay.ev.hero_opponent", "Their hero takes {n} damage", "El héroe rival recibe {n} de daño", "Герой соперника получает {n} урона"],
    ["replay.ev.board_in_own", "Your team's next board comes in: {hero}", "Entra el siguiente tablero de tu equipo: {hero}", "Выходит следующий стол вашей команды: {hero}"],
    ["replay.ev.board_in_opponent", "Their team's next board comes in: {hero}", "Entra el siguiente tablero del equipo rival: {hero}", "Выходит следующий стол команды соперника: {hero}"],
    ["replay.ev.unknown", "A step the app cannot read", "Un paso que la app no puede leer", "Шаг, который приложение не может прочитать"],
    ["replay.cut",
      "This replay stops before the combat ends: the log was cut short, or the combat was longer than the app keeps.",
      "Esta repetición se corta antes de que acabe el combate: el registro se cortó o el combate era más largo de lo que guarda la app.",
      "Этот повтор обрывается до конца боя: журнал был оборван, либо бой оказался длиннее лимита записи."],
    ["replay.skipped.one", "{n} step could not be read and is left out.", "{n} paso no se ha podido leer y se ha dejado fuera.", "Не удалось прочитать {n} шаг — он пропущен."],
    ["replay.skipped.other", "{n} steps could not be read and are left out.", "{n} pasos no se han podido leer y se han dejado fuera.", "Не удалось прочитать {n} шагов — они пропущены."],
    ["replay.not_recorded",
      "Not recorded: this game was saved by an older version of the app. Run tavern-watch --reparse while its log is still on disk to add the replays.",
      "No registrado: esta partida la guardó una versión anterior de la app. Ejecuta tavern-watch --reparse mientras su registro siga en el disco para añadir las repeticiones.",
      "Не записано: эта игра сохранена старой версией приложения. Запустите tavern-watch --reparse, пока журнал ещё на диске, чтобы добавить повторы."],
    ["replay.none",
      "No replay: the game did not play back any combat of this game on screen.",
      "Sin repetición: el juego no mostró en pantalla ningún combate de esta partida.",
      "Нет повтора: игра не показывала на экране ни одного боя этой партии."],
    ["replay.not_shown.one", "Not shown in game, so no replay: turn {turns}.", "No se vio en la partida, así que no hay repetición: turno {turns}.", "Не показан в игре, повтора нет: ход {turns}."],
    ["replay.not_shown.other", "Not shown in game, so no replay: turns {turns}.", "No se vieron en la partida, así que no hay repetición: turnos {turns}.", "Не показаны в игре, повтора нет: ходы {turns}."],
    ["replay.about",
      "From the game's log: each combat as the game played it on your screen, step by step. Never a simulation. Combats the game does not show you (some of a Duos teammate's) have no replay.",
      "Del registro del juego: cada combate tal y como el juego lo mostró en tu pantalla, paso a paso. Nunca es una simulación. Los combates que el juego no te muestra (algunos de tu compañero en Dúos) no tienen repetición.",
      "Из журнала игры: каждый бой шаг за шагом так, как игра показывала его на вашем экране. Это не симуляция. Для боёв, которые игра вам не показывала (например, часть боёв напарника в Дуэтах), повтора нет."],
  ];

  const table = { en: {}, es: {}, ru: {} };
  rows.forEach(([key, en, es, ru]) => {
    table.en[key] = en;
    table.es[key] = es;
    table.ru[key] = ru;
  });
  table.ru["replay.skipped.few"] = "Не удалось прочитать {n} шага — они пропущены.";
  table.ru["replay.skipped.many"] = "Не удалось прочитать {n} шагов — они пропущены.";

  const i18n = typeof module !== "undefined" && module.exports ? require("../i18n.js") : window.TLI18n;
  i18n.define(table);
  if (typeof module !== "undefined" && module.exports) module.exports = table;
})();

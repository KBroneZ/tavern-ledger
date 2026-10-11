// Words of the combat replays in the recap: replay.js (T-215, D-099). English
// is the source; Spanish (Spain) is complete (ui-tests/i18n.test.js). Each row
// is [key, English, Spanish]. Prefix: replay.
"use strict";

(() => {
  const rows = [
    ["replay.title", "Combat replays", "Repeticiones de combate"],
    ["replay.pick", "Combat to replay", "Combate que repetir"],
    ["replay.chip", "Turn {turn}", "Turno {turn}"],
    ["replay.chip_tip", "Turn {turn}: {outcome}", "Turno {turn}: {outcome}"],
    ["replay.combat_of", "Turn {turn} against {who}", "Turno {turn} contra {who}"],
    ["replay.step_of", "Step {n} of {total}", "Paso {n} de {total}"],
    ["replay.play", "Play", "Reproducir"],
    ["replay.pause", "Pause", "Pausa"],
    ["replay.back", "Step back", "Paso atrás"],
    ["replay.forward", "Step forward", "Paso adelante"],
    ["replay.restart", "Back to the start", "Volver al inicio"],
    ["replay.speed", "Speed", "Velocidad"],
    ["replay.speed_x", "{n}×", "{n}×"],
    ["replay.steps", "Every step ({n})", "Todos los pasos ({n})"],
    ["replay.start", "Both boards as the combat starts.", "Los dos tableros al empezar el combate."],
    ["replay.your", "your {name}", "tu {name}"],
    ["replay.their", "their {name}", "{name} rival"],
    ["replay.ev.attack", "{a} attacks {b}", "{a} ataca a {b}"],
    ["replay.ev.attack_alone", "{a} attacks", "{a} ataca"],
    ["replay.ev.damage", "{m} takes {n} damage ({left} health left)", "{m} recibe {n} de daño (vida restante: {left})"],
    ["replay.ev.damage_lethal", "{m} takes {n} damage", "{m} recibe {n} de daño"],
    ["replay.ev.shield_off", "{m} loses Divine Shield", "{m} pierde el Escudo divino"],
    ["replay.ev.shield_on", "{m} gets Divine Shield", "{m} obtiene Escudo divino"],
    ["replay.ev.stats", "{m} is now {atk}/{health}", "{m} pasa a {atk}/{health}"],
    ["replay.ev.death", "{m} dies", "{m} muere"],
    ["replay.ev.summon", "{m} is summoned", "se invoca {m}"],
    ["replay.ev.card_own", "your {name}", "tu {name}"],
    ["replay.ev.card_opponent", "their {name}", "{name} rival"],
    ["replay.ev.trigger.start_of_combat", "Start of combat: {m}", "Inicio del combate: {m}"],
    ["replay.ev.trigger.deathrattle", "Deathrattle: {m}", "Último aliento: {m}"],
    ["replay.ev.trigger.avenge", "Avenge: {m}", "Venganza: {m}"],
    ["replay.ev.trigger.rally", "Rally: {m}", "Alentar: {m}"],
    ["replay.ev.hero_own", "Your hero takes {n} damage", "Tu héroe recibe {n} de daño"],
    ["replay.ev.hero_opponent", "Their hero takes {n} damage", "El héroe rival recibe {n} de daño"],
    ["replay.ev.board_in_own", "Your team's next board comes in: {hero}", "Entra el siguiente tablero de tu equipo: {hero}"],
    ["replay.ev.board_in_opponent", "Their team's next board comes in: {hero}", "Entra el siguiente tablero del equipo rival: {hero}"],
    ["replay.ev.unknown", "A step the app cannot read", "Un paso que la app no puede leer"],
    ["replay.cut", "This replay stops before the combat ends: the log was cut short, or the combat was longer than the app keeps.",
      "Esta repetición se corta antes de que acabe el combate: el registro se cortó o el combate era más largo de lo que guarda la app."],
    ["replay.skipped.one", "{n} step could not be read and is left out.", "{n} paso no se ha podido leer y se ha dejado fuera."],
    ["replay.skipped.other", "{n} steps could not be read and are left out.", "{n} pasos no se han podido leer y se han dejado fuera."],
    ["replay.not_recorded", "Not recorded: this game was saved by an older version of the app. Run tavern-watch --reparse while its log is still on disk to add the replays.",
      "No registrado: esta partida la guardó una versión anterior de la app. Ejecuta tavern-watch --reparse mientras su registro siga en el disco para añadir las repeticiones."],
    ["replay.none", "No replay: the game did not play back any combat of this game on screen.",
      "Sin repetición: el juego no mostró en pantalla ningún combate de esta partida."],
    ["replay.not_shown.one", "Not shown in game, so no replay: turn {turns}.", "No se vio en la partida, así que no hay repetición: turno {turns}."],
    ["replay.not_shown.other", "Not shown in game, so no replay: turns {turns}.", "No se vieron en la partida, así que no hay repetición: turnos {turns}."],
    ["replay.about", "From the game's log: each combat as the game played it on your screen, step by step. Never a simulation. Combats the game does not show you (some of a Duos teammate's) have no replay.",
      "Del registro del juego: cada combate tal y como el juego lo mostró en tu pantalla, paso a paso. Nunca es una simulación. Los combates que el juego no te muestra (algunos de tu compañero en Dúos) no tienen repetición."],
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

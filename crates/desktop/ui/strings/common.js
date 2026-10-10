// Words shared by the window and the overlay (T-325): languages, the Settings
// page, where values come from, tribes, keywords, modes and results, plus the
// fixed sentences the app's Rust side writes. English is the source; Spanish
// (Spain) is complete (ui-tests/i18n.test.js). Card and hero names are not
// here: they stay as the card data and the game's log give them.
"use strict";

(() => {
  const table = {
    en: {
      // Each language by its own name, the same in every table.
      "lang.en": "English",
      "lang.es": "Español",

      "settings.tab": "Settings",
      "settings.title": "Settings",
      "settings.intro": "Changes apply at once and are kept on this PC.",
      "settings.language.title": "Language",
      "settings.language.label": "Language of the app",
      "settings.language.system": "Windows language ({name})",
      "settings.language.note": "Card and hero names stay as the card data and the game's log give them.",
      "settings.language.failed": "The language could not be changed ({error}).",
      "settings.overlay.title": "Overlay",
      "settings.overlay.theme": "Overlay theme",
      "settings.windows.title": "Windows",
      "settings.autostart": "Start with Windows",
      "settings.autostart.unavailable": "Not available: the app's folder has spaces in its path.",
      "settings.autostart.failed": "The Windows start-up setting could not be read or changed ({error}).",
      "settings.memory.title": "Lobby tribes and rating from the game",
      "settings.memory.later": "Not in this version yet. When it comes, it is off until you turn it on here, with what it reads and its risk said first.",

      "theme.glass": "Glass (dark)",
      "theme.contrast": "High contrast",

      // Where a value comes from (T-109); the keys are tracker::provenance's.
      "source.log.label": "from the log",
      "source.log.description": "The game's own log says so. Counts and averages are worked out from such values.",
      "source.inferred.label": "inferred",
      "source.inferred.description": "Worked out by Tavern Ledger from the log (for example tribes seen in the tavern, or skins grouped under one hero). It can be wrong.",
      "source.entered.label": "entered by you",
      "source.entered.description": "You typed it in.",
      "source.leaderboard.label": "from the leaderboard",
      "source.leaderboard.description": "Blizzard's public leaderboard says so.",
      "source.card_data.label": "card database",
      "source.card_data.description": "Card name or art looked up by the card id in the log, in HearthstoneJSON's copy of Blizzard's card data (English). It can lag behind a game patch.",
      "source.possible.label": "possible",
      "source.possible.description": "A minion this opponent could have at their tavern tier, from the lobby's tribes and the card data's minion pool. Not seen: they may have none of them.",
      "source.unknown.label": "unknown",
      "source.unknown.description": "Nothing in the log says. It is not zero.",

      // Tribes as the log names them (CARDRACE).
      "tribe.ABERRATION": "Aberration",
      "tribe.BEAST": "Beast",
      "tribe.DEMON": "Demon",
      "tribe.DRAGON": "Dragon",
      "tribe.ELEMENTAL": "Elemental",
      "tribe.MECHANICAL": "Mechanical",
      "tribe.MURLOC": "Murloc",
      "tribe.NAGA": "Naga",
      "tribe.PIRATE": "Pirate",
      "tribe.QUILBOAR": "Quilboar",
      "tribe.UNDEAD": "Undead",
      "tribe.NEUTRAL": "No tribe",
      "tribe.ALL": "All tribes",

      // Minion keywords: the short mark and the full word.
      "keyword.taunt.short": "T",
      "keyword.taunt": "Taunt",
      "keyword.divine_shield.short": "DS",
      "keyword.divine_shield": "Divine Shield",
      "keyword.reborn.short": "R",
      "keyword.reborn": "Reborn",
      "keyword.venomous.short": "V",
      "keyword.venomous": "Venomous",
      "keyword.windfury.short": "W",
      "keyword.windfury": "Windfury",
      "keyword.stealth.short": "S",
      "keyword.stealth": "Stealth",
      "keyword.deathrattle.short": "DR",
      "keyword.deathrattle": "Deathrattle",
      "keyword.rally.short": "Ra",
      "keyword.rally": "Rally",
      "keyword.golden": "Golden",

      "mode.GT_BATTLEGROUNDS": "Solo",
      "mode.GT_BATTLEGROUNDS_DUO": "Duos",
      "mode.OTHER": "Other",

      "outcome.won": "Won",
      "outcome.lost": "Lost",
      "outcome.tie": "Tie",
      "outcome.unknown": "Unknown",
    },

    es: {
      "lang.en": "English",
      "lang.es": "Español",

      "settings.tab": "Ajustes",
      "settings.title": "Ajustes",
      "settings.intro": "Los cambios se aplican al momento y se guardan en este PC.",
      "settings.language.title": "Idioma",
      "settings.language.label": "Idioma de la app",
      "settings.language.system": "Idioma de Windows ({name})",
      "settings.language.note": "Los nombres de cartas y héroes se quedan como los dan los datos de cartas y el registro del juego.",
      "settings.language.failed": "No se ha podido cambiar el idioma ({error}).",
      "settings.overlay.title": "Overlay",
      "settings.overlay.theme": "Tema del overlay",
      "settings.windows.title": "Windows",
      "settings.autostart": "Iniciar con Windows",
      "settings.autostart.unavailable": "No disponible: la ruta de la carpeta de la app tiene espacios.",
      "settings.autostart.failed": "No se ha podido leer o cambiar el inicio con Windows ({error}).",
      "settings.memory.title": "Tribus de la partida y puntuación desde el juego",
      "settings.memory.later": "Aún no está en esta versión. Cuando llegue, estará desactivado hasta que lo actives aquí, y antes te diremos qué lee y qué riesgo tiene.",

      "theme.glass": "Cristal (oscuro)",
      "theme.contrast": "Alto contraste",

      "source.log.label": "del registro",
      "source.log.description": "Lo dice el propio registro del juego. Los recuentos y las medias se calculan con estos valores.",
      "source.inferred.label": "deducido",
      "source.inferred.description": "Lo deduce Tavern Ledger a partir del registro (por ejemplo, las tribus vistas en la taberna o los aspectos agrupados bajo un héroe). Puede estar mal.",
      "source.entered.label": "introducido por ti",
      "source.entered.description": "Lo has escrito tú.",
      "source.leaderboard.label": "de la clasificación",
      "source.leaderboard.description": "Lo dice la clasificación pública de Blizzard.",
      "source.card_data.label": "base de cartas",
      "source.card_data.description": "Nombre o imagen de la carta buscados por su id en el registro, en la copia de HearthstoneJSON de los datos de cartas de Blizzard (en inglés). Puede ir por detrás de un parche.",
      "source.possible.label": "posible",
      "source.possible.description": "Un esbirro que este rival podría tener en su nivel de taberna, según las tribus de la partida y los esbirros de los datos de cartas. No visto: puede que no tenga ninguno.",
      "source.unknown.label": "desconocido",
      "source.unknown.description": "El registro no lo dice. No es cero.",

      "tribe.ABERRATION": "Aberración",
      "tribe.BEAST": "Bestia",
      "tribe.DEMON": "Demonio",
      "tribe.DRAGON": "Dragón",
      "tribe.ELEMENTAL": "Elemental",
      "tribe.MECHANICAL": "Robot",
      "tribe.MURLOC": "Múrloc",
      "tribe.NAGA": "Naga",
      "tribe.PIRATE": "Pirata",
      "tribe.QUILBOAR": "Jabaespín",
      "tribe.UNDEAD": "No-muerto",
      "tribe.NEUTRAL": "Sin tribu",
      "tribe.ALL": "Todas las tribus",

      "keyword.taunt.short": "P",
      "keyword.taunt": "Provocar",
      "keyword.divine_shield.short": "ED",
      "keyword.divine_shield": "Escudo divino",
      "keyword.reborn.short": "Re",
      "keyword.reborn": "Reencarnación",
      "keyword.venomous.short": "V",
      "keyword.venomous": "Venenoso",
      "keyword.windfury.short": "VF",
      "keyword.windfury": "Viento furioso",
      "keyword.stealth.short": "S",
      "keyword.stealth": "Sigilo",
      "keyword.deathrattle.short": "UA",
      "keyword.deathrattle": "Último aliento",
      "keyword.rally.short": "Ar",
      "keyword.rally": "Arenga",
      "keyword.golden": "Dorado",

      "mode.GT_BATTLEGROUNDS": "Individual",
      "mode.GT_BATTLEGROUNDS_DUO": "Dúos",
      "mode.OTHER": "Otros",

      "outcome.won": "Ganado",
      "outcome.lost": "Perdido",
      "outcome.tie": "Empate",
      "outcome.unknown": "Desconocido",
    },

    // Fixed sentences from the Rust side, by their exact English text. Any
    // sentence not here (one with a file name or an error code in it) is
    // shown as written.
    backend: {
      es: {
        // tracker::scene
        "In the main menu": "En el menú principal",
        "In the Battlegrounds lobby": "En la sala de Battlegrounds",
        "Battlegrounds game loading": "Cargando una partida de Battlegrounds",
        "In a game": "En una partida",
        "Elsewhere in the game": "En otra parte del juego",
        // tracker::lobby_tribes
        "Pick exactly 5 tribes.": "Elige exactamente 5 tribus.",
        "That is not a Battlegrounds tribe.": "Esa no es una tribu de Battlegrounds.",
        "Each tribe can be picked once.": "Cada tribu solo se puede elegir una vez.",
        "That game is not in the history.": "Esa partida no está en el historial.",
        "No tribes were entered for that game.": "No se introdujeron tribus para esa partida.",
        "That game already has tribes entered. Clear them first.": "Esa partida ya tiene tribus introducidas. Quítalas primero.",
        // desktop (main.rs, overlay.rs, settings.rs, card_data.rs, upload.rs)
        "Entering tribes is not available: this window does not own the history.": "No se pueden introducir tribus: esta ventana no gestiona el historial.",
        "The Downloads folder was not found, so nothing was saved.": "No se ha encontrado la carpeta Descargas, así que no se ha guardado nada.",
        "Tavern Ledger is already running (or tavern-watch is). This window only shows the history; use the other one to follow your games.": "Tavern Ledger ya está abierto (o tavern-watch). Esta ventana solo muestra el historial; usa la otra para seguir tus partidas.",
        "Hearthstone's Logs folder was not found. Is the game installed?": "No se ha encontrado la carpeta Logs de Hearthstone. ¿Está instalado el juego?",
        "The tracker stopped after an internal error. Restart the app; your history is safe.": "El seguimiento se ha detenido por un error interno. Reinicia la app; tu historial está a salvo.",
        "The overlay stopped after an internal error. Restart the app; your history is safe.": "El overlay se ha detenido por un error interno. Reinicia la app; tu historial está a salvo.",
        "The overlay could not change between locked and unlocked; it is off until you restart the app.": "El overlay no ha podido bloquearse o desbloquearse; está desactivado hasta que reinicies la app.",
        "The leaderboard box was not kept: bring Hearthstone to the front once, then try again.": "No se ha guardado el recuadro de la clasificación: pon Hearthstone en primer plano una vez y vuelve a intentarlo.",
        "The overlay settings have no folder to be saved in; they will be lost after a restart.": "Los ajustes del overlay no tienen carpeta donde guardarse; se perderán al reiniciar.",
        "Could not change the Windows start-up setting.": "No se ha podido cambiar el inicio con Windows.",
        "The language could not be saved; it will be lost after a restart.": "No se ha podido guardar el idioma; se perderá al reiniciar.",
        "Card names and art are starting.": "Los nombres e imágenes de cartas se están preparando.",
        "Upload is not running in this window: it starts once the app follows your games (if another Tavern Ledger is open, use that one).": "La subida no está activa en esta ventana: empieza cuando la app sigue tus partidas (si hay otro Tavern Ledger abierto, usa ese).",
        "The upload worker stopped; restart the app.": "La subida se ha detenido; reinicia la app.",
        "That email is too long.": "Ese correo es demasiado largo.",
        "No upload server is set up yet.": "Aún no hay ningún servidor de subida configurado.",
        "The sign-in stopped after an internal error.": "El inicio de sesión se ha detenido por un error interno.",
        "Signed out on this PC, but the server session could not be ended (the server could not be reached or refused).": "Sesión cerrada en este PC, pero no se ha podido cerrar la sesión en el servidor (no responde o lo ha rechazado).",
        "Signed out: the sign-in on this PC is gone. Sign in again.": "Sesión cerrada: el inicio de sesión de este PC ya no está. Vuelve a iniciar sesión.",
        "Signed out: your sign-in expired or was ended. Sign in again.": "Sesión cerrada: tu sesión ha caducado o se ha cerrado. Vuelve a iniciar sesión.",
        "Upload paused: too many sign-in refreshes.": "Subida en pausa: demasiadas renovaciones de sesión.",
      },
    },
  };

  const i18n = typeof module !== "undefined" && module.exports ? require("../i18n.js") : window.TLI18n;
  i18n.define(table);
  if (typeof module !== "undefined" && module.exports) module.exports = table;
})();

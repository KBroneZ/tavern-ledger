// What each gallery frame says about itself, in English and Spanish: what the
// player sees, where the values come from, and how that keeps to "only what
// the game shows you". Public wording: no internal ids, no notes about how
// the app is made.
"use strict";

const LOG = {
  en: "Hearthstone's own log files on this PC, read while the game runs.",
  es: "Los archivos de registro del propio Hearthstone en este PC, leídos mientras el juego está abierto.",
};

const CAPTIONS = {
  waiting: {
    en: {
      title: "Waiting for a game",
      sees: "A small status box saying the app is waiting. The overlay stays out of the way until a Battlegrounds game starts.",
      source: "Nothing yet: no game is running, so there is nothing to read.",
      rule: "It shows no numbers at all. Without a game there is nothing the game could have shown you.",
    },
    es: {
      title: "Esperando una partida",
      sees: "Una cajita de estado que dice que la app espera. La superposición no estorba hasta que empieza una partida de Campos de batalla.",
      source: "Todavía nada: no hay partida en marcha, así que no hay nada que leer.",
      rule: "No muestra ningún número. Sin partida no hay nada que el juego te haya podido enseñar.",
    },
  },
  lobby: {
    en: {
      title: "The lobby panel",
      sees: "On the Battlegrounds lobby screen, a small panel in the top left: your rating in each mode, how it changed after your last game, and a line per mode with today's games, average place and games in the top half.",
      source: "The rating is the number the game shows on your own screen, read from the running game (read-only) only if you turn that on. Today's games come from your saved games on this PC.",
      rule: "Only your own rating and your own games. Nobody else's rating is read or shown.",
    },
    es: {
      title: "El panel del vestíbulo",
      sees: "En la pantalla del vestíbulo de Campos de batalla, un panel pequeño arriba a la izquierda: tu puntuación en cada modo, cómo cambió tras tu última partida y una línea por modo con las partidas de hoy, la posición media y las partidas en la mitad superior.",
      source: "La puntuación es el número que el juego muestra en tu pantalla, leído del juego en marcha (solo lectura) y solo si lo activas. Las partidas de hoy salen de tus partidas guardadas en este PC.",
      rule: "Solo tu propia puntuación y tus propias partidas. No se lee ni se muestra la de nadie más.",
    },
  },
  "solo-simple": {
    en: {
      title: "Solo shop turn, Simple",
      sees: "The default view during your shop turn: the lobby's tribes as short chips, then one row per opponent in the game's leaderboard order with their portrait, tavern tier, health and triples. The opponent the game marks as next is lit, with the combat odds under it.",
      source: LOG.en + " Tribes marked \"inferred\" were worked out from what the tavern offered you.",
      rule: "Every number is on your own screen already: the game's leaderboard shows tiers, health, triples and who is next. The \"inferred\" mark says when a value is a deduction and not something the game printed.",
    },
    es: {
      title: "Turno de tienda en Solo, Simple",
      sees: "La vista por defecto en tu turno de tienda: las tribus de la partida como etiquetas cortas y una fila por rival, en el orden de la clasificación del juego, con su retrato, nivel de taberna, vida y tripletes. El rival que el juego marca como siguiente aparece resaltado, con las probabilidades de combate debajo.",
      source: LOG.es + " Las tribus marcadas «deducido» se calcularon a partir de lo que la taberna te ofreció.",
      rule: "Todos los números ya están en tu pantalla: la clasificación del juego enseña niveles, vida, tripletes y quién es el siguiente. La marca «deducido» avisa cuando un valor es una deducción y no algo que el juego mostró.",
    },
  },
  "solo-advanced": {
    en: {
      title: "The same turn, Advanced",
      sees: "Everything Simple shows plus: your hero, the mode and your last combat, the tribes seen in the tavern, a block for the next opponent with the last board you saw of theirs, and a head line for every opponent with their name and your record against them. Small marks say where each value comes from.",
      source: LOG.en + " Names and pictures come from the public card database (HearthstoneJSON), in the app's language.",
      rule: "Boards are only ones that were on your screen in a past combat, shown as of the turn you saw them, never a live view of their shop. Source marks keep a guess from passing as something seen.",
    },
    es: {
      title: "El mismo turno, Avanzado",
      sees: "Todo lo de Simple y además: tu héroe, el modo y tu último combate, las tribus vistas en la taberna, un bloque del siguiente rival con el último tablero que viste de él, y una línea de cabecera por rival con su nombre y tu historial contra él. Pequeñas marcas dicen de dónde sale cada valor.",
      source: LOG.es + " Los nombres y las imágenes salen de la base pública de cartas (HearthstoneJSON), en el idioma de la app.",
      rule: "Los tableros son solo los que estuvieron en tu pantalla en un combate pasado, con el turno en que los viste; nunca una vista en directo de su tienda. Las marcas de origen evitan que una suposición pase por algo visto.",
    },
  },
  duos: {
    en: {
      title: "Duos",
      sees: "In Duos each row is a team of two: two portraits with their tiers, the team's shared health and triples. Your own team is never listed as an opponent.",
      source: LOG.en,
      rule: "Only the opposing teams' values the game's leaderboard shows. Your teammate's board and hand are not read.",
    },
    es: {
      title: "Dúos",
      sees: "En Dúos cada fila es un equipo de dos: dos retratos con sus niveles, la vida y los tripletes del equipo. Tu propio equipo nunca aparece como rival.",
      source: LOG.es,
      rule: "Solo los valores de los equipos rivales que enseña la clasificación del juego. No se lee el tablero ni la mano de tu compañero.",
    },
  },
  odds: {
    en: {
      title: "Combat odds",
      sees: "Under the next opponent: your chances to win, tie or lose against the last board you saw of theirs, and how old that board is. Advanced adds the likely damage each way.",
      source: "Worked out on your PC by simulating a few thousand fights between your board and their last seen board, only during your shop turn.",
      rule: "A calculation from boards you have already seen, not a peek at their current board. It says when the board is old, and a \"≈\" marks a result that is approximate.",
    },
    es: {
      title: "Probabilidades de combate",
      sees: "Bajo el siguiente rival: tus posibilidades de ganar, empatar o perder contra el último tablero que viste de él, y qué antigüedad tiene. Avanzado añade el daño probable en cada caso.",
      source: "Calculadas en tu PC simulando unos miles de combates entre tu tablero y el último tablero visto del rival, solo durante tu turno de tienda.",
      rule: "Es un cálculo a partir de tableros que ya has visto, no un vistazo a su tablero actual. Avisa cuando el tablero es antiguo, y un «≈» marca un resultado aproximado.",
    },
  },
  "gap-no-board-seen": {
    en: {
      title: "Odds: no board seen yet",
      sees: "Instead of numbers, one line says you have not seen a board of the next opponent yet.",
      source: "There is nothing to simulate: the game has not shown you their board in a combat.",
      rule: "No number is made up. When the game has not shown you something, the overlay says so.",
    },
    es: {
      title: "Probabilidades: aún sin tablero visto",
      sees: "En lugar de números, una línea dice que todavía no has visto un tablero del siguiente rival.",
      source: "No hay nada que simular: el juego aún no te ha enseñado su tablero en un combate.",
      rule: "No se inventa ningún número. Cuando el juego no te ha enseñado algo, la superposición lo dice.",
    },
  },
  "gap-no-card-data": {
    en: {
      title: "Odds: card data not loaded",
      sees: "One line says the card data is not loaded, so the fight cannot be simulated.",
      source: "The simulation needs the public card database, which is downloaded once and kept on this PC.",
      rule: "Without the card data the odds are left out instead of guessed.",
    },
    es: {
      title: "Probabilidades: datos de cartas sin cargar",
      sees: "Una línea dice que faltan los datos de cartas, así que no se puede simular el combate.",
      source: "La simulación necesita la base pública de cartas, que se descarga una vez y se guarda en este PC.",
      rule: "Sin los datos de cartas, las probabilidades se omiten en vez de adivinarse.",
    },
  },
  "gap-not-modelled": {
    en: {
      title: "Odds: a card can't be simulated",
      sees: "One line says a card on one of the boards is not something the simulator can play yet.",
      source: "The simulator knows most but not all cards. This frame shows what happens with one it does not know.",
      rule: "A result that would be wrong is never shown; the overlay says why there is none.",
    },
    es: {
      title: "Probabilidades: una carta no se puede simular",
      sees: "Una línea dice que una carta de uno de los tableros todavía no la sabe jugar el simulador.",
      source: "El simulador conoce la mayoría de cartas, pero no todas. Este cuadro enseña qué pasa con una que no conoce.",
      rule: "Nunca se enseña un resultado que sería incorrecto; la superposición dice por qué no hay ninguno.",
    },
  },
  "gap-bad-board": {
    en: {
      title: "Odds: boards could not be read",
      sees: "One line says the boards could not be read well enough to simulate.",
      source: "The boards come from the log; here something about them did not add up.",
      rule: "When the input is doubtful the overlay shows no odds, and says so.",
    },
    es: {
      title: "Probabilidades: no se pudieron leer los tableros",
      sees: "Una línea dice que los tableros no se pudieron leer lo bastante bien como para simular.",
      source: "Los tableros salen del registro; aquí algo en ellos no cuadraba.",
      rule: "Cuando los datos son dudosos la superposición no muestra probabilidades, y lo dice.",
    },
  },
  "gap-order": {
    en: {
      title: "Duos odds: order not known yet",
      sees: "In Duos, one line says it is not known yet who fights first or who is on which team.",
      source: "Duos odds need both teams' boards and the fight order, which the game shows a little later in the turn.",
      rule: "Until the game has shown the order, no odds are shown.",
    },
    es: {
      title: "Probabilidades en Dúos: orden aún desconocido",
      sees: "En Dúos, una línea dice que todavía no se sabe quién lucha primero ni quién va con quién.",
      source: "Las probabilidades de Dúos necesitan los tableros de ambos equipos y el orden de lucha, que el juego enseña algo más tarde en el turno.",
      rule: "Hasta que el juego haya enseñado el orden, no se muestran probabilidades.",
    },
  },
  "gap-missing-board": {
    en: {
      title: "Duos odds: a board not seen yet",
      sees: "In Duos, one line says which board you have not seen yet: your teammate's, or one of the opposing team's two.",
      source: "Duos odds use four boards. Any board the game has not shown you leaves a gap.",
      rule: "The overlay never fills a board it has not seen. It names the missing one instead.",
    },
    es: {
      title: "Probabilidades en Dúos: falta un tablero",
      sees: "En Dúos, una línea dice qué tablero aún no has visto: el de tu compañero o uno de los dos del equipo rival.",
      source: "Las probabilidades de Dúos usan cuatro tableros. Cualquier tablero que el juego no te haya enseñado deja un hueco.",
      rule: "La superposición nunca rellena un tablero que no ha visto. Nombra el que falta.",
    },
  },
  "hover-solo": {
    en: {
      title: "Hover card, Solo",
      sees: "Point at a hero on the game's own leaderboard and a card appears across the top of the game: that opponent's last board you saw, laid out like the game's, with their tier, health, triples and trinkets.",
      source: LOG.en,
      rule: "It is a memory of what you were shown: the board is from a past combat and the card says which turn. It sits over the top of the screen, away from your own board, hand and the game's tooltips.",
    },
    es: {
      title: "Tarjeta al pasar el ratón, Solo",
      sees: "Apunta a un héroe de la clasificación del propio juego y aparece una tarjeta arriba en el juego: el último tablero que viste de ese rival, dispuesto como en el juego, con su nivel, vida, tripletes y baratijas.",
      source: LOG.es,
      rule: "Es un recuerdo de lo que se te enseñó: el tablero es de un combate pasado y la tarjeta dice de qué turno. Queda arriba en la pantalla, lejos de tu tablero, tu mano y las ayudas del juego.",
    },
  },
  "hover-duos": {
    en: {
      title: "Hover card, Duos",
      sees: "In Duos the card shows the one hero of the team under the pointer, with the team's shared health.",
      source: LOG.en,
      rule: "Same as Solo: only boards you were shown, one hero at a time, matched to the right seat.",
    },
    es: {
      title: "Tarjeta al pasar el ratón, Dúos",
      sees: "En Dúos la tarjeta muestra al héroe del equipo que está bajo el puntero, con la vida compartida del equipo.",
      source: LOG.es,
      rule: "Igual que en Solo: solo tableros que se te enseñaron, un héroe cada vez, asignado a su asiento.",
    },
  },
  "hover-advanced": {
    en: {
      title: "Hover card, Advanced",
      sees: "Advanced adds trinket names, your record against them and, for a hero you have not met yet, a clearly labelled list of minions they could have.",
      source: LOG.en + " The possible minions come from the public card database and the lobby's tribes.",
      rule: "The list of possible minions is marked \"possible\" and never looks like something seen: it is what the game's rules allow at their tavern tier, not what they have.",
    },
    es: {
      title: "Tarjeta al pasar el ratón, Avanzado",
      sees: "Avanzado añade los nombres de las baratijas, tu historial contra el rival y, para un héroe que aún no has enfrentado, una lista claramente marcada de esbirros que podría tener.",
      source: LOG.es + " Los esbirros posibles salen de la base pública de cartas y de las tribus de la partida.",
      rule: "La lista de esbirros posibles lleva la marca «posible» y nunca parece algo visto: es lo que permiten las reglas del juego en su nivel de taberna, no lo que tiene.",
    },
  },
  "damage-cap": {
    en: {
      title: "Damage cap",
      sees: "A line above the opponents repeats the game's \"Max damage\" number: the most a lost combat can hurt you now.",
      source: "The game shows this number on your own screen; the overlay copies it from the log.",
      rule: "It is the game's own figure, in the game's words, and it is left out when no cap applies.",
    },
    es: {
      title: "Límite de daño",
      sees: "Una línea sobre los rivales repite el número de «Daño máximo» del juego: lo máximo que puede dolerte ahora perder un combate.",
      source: "El juego muestra este número en tu pantalla; la superposición lo copia del registro.",
      rule: "Es la cifra del propio juego, con sus palabras, y se omite cuando no hay límite.",
    },
  },
  unreadable: {
    en: {
      title: "A build the app can't read",
      sees: "A red box saying this version of the app does not understand the game's log format. Nothing else is shown.",
      source: "After a game update the log can change shape. The app notices when it no longer fits what it knows.",
      rule: "It shows nothing instead of guessing. A wrong number would be worse than none.",
    },
    es: {
      title: "Una versión que la app no sabe leer",
      sees: "Un cuadro rojo dice que esta versión de la app no entiende el formato del registro del juego. No se muestra nada más.",
      source: "Tras una actualización del juego el registro puede cambiar de forma. La app se da cuenta cuando ya no encaja con lo que conoce.",
      rule: "No muestra nada en lugar de adivinar. Un número equivocado sería peor que ninguno.",
    },
  },
  "game-over": {
    en: {
      title: "Game over",
      sees: "When the game ends the status box says so, and the last values stay on screen until you leave.",
      source: LOG.en,
      rule: "Same values as during the game; the odds are gone because there is no next fight.",
    },
    es: {
      title: "Partida terminada",
      sees: "Cuando acaba la partida el cuadro de estado lo dice y los últimos valores se quedan en pantalla hasta que sales.",
      source: LOG.es,
      rule: "Los mismos valores que durante la partida; las probabilidades desaparecen porque ya no hay otro combate.",
    },
  },
  "high-contrast": {
    en: {
      title: "High contrast theme",
      sees: "The same Solo view in black and white with strong colours, for players who find the default theme hard to read.",
      source: "A theme choice in the app, nothing to do with the game's data.",
      rule: "Only the colours change. Meaning never depends on colour alone: every mark also has a word or a letter.",
    },
    es: {
      title: "Tema de alto contraste",
      sees: "La misma vista de Solo en blanco y negro con colores intensos, para quien encuentra difícil de leer el tema por defecto.",
      source: "Una elección de tema en la app, sin relación con los datos del juego.",
      rule: "Solo cambian los colores. El significado nunca depende solo del color: cada marca lleva también una palabra o una letra.",
    },
  },
  counters: {
    en: {
      title: "Counters",
      sees: "A small box of running totals for your own game: for example Blood Gem size, extra Tavern spell power, Tavern minion bonuses and free rolls. A line shows only while its number is above zero.",
      source: LOG.en,
      rule: "Only your own totals, built from things that happened on your own screen. They are a memory aid: nothing about opponents, your teammate or anything the game did not show you.",
    },
    es: {
      title: "Contadores",
      sees: "Una cajita de totales acumulados de tu propia partida: por ejemplo el tamaño de las Gemas de sangre, el poder extra de los hechizos de taberna, las mejoras de los esbirros de taberna y las tiradas gratis. Una línea solo aparece mientras su número es mayor que cero.",
      source: LOG.es,
      rule: "Solo tus propios totales, hechos con cosas que pasaron en tu propia pantalla. Son una ayuda de memoria: nada de rivales, de tu compañero ni de lo que el juego no te enseñó.",
    },
  },
};

module.exports = { CAPTIONS };

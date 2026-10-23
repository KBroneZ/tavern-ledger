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
      title: "The lobby panel: the day at a glance",
      sees: "On the Battlegrounds lobby screen, a small panel in the top left: your rating in each mode now and the rating you started the day with, with the difference. Under it the day's last eight games as tiles, the newest first: your hero's portrait behind the place you finished and the rating you won (green, up) or lost (red, down) in that game. A line per mode closes it with today's games, average place and games in the top half.",
      source: "The ratings are the numbers the game shows on your own screen, read from the running game (read-only) while that option is on, and each game's rating is kept with it on this PC only. Heroes, places and the day's games come from your saved games on this PC.",
      rule: "Only your own rating and your own games; nobody else's rating is read or shown, and a rating is never uploaded. A change appears only when the app read your rating right after this game and right after the one before it.",
    },
    es: {
      title: "El panel del vestíbulo: el día de un vistazo",
      sees: "En la pantalla del vestíbulo de Campos de batalla, un panel pequeño arriba a la izquierda: tu puntuación en cada modo ahora y la puntuación con la que empezaste el día, con la diferencia. Debajo, las últimas ocho partidas del día como fichas, la más reciente primero: el retrato de tu héroe detrás del puesto en que acabaste y la puntuación que ganaste (verde, hacia arriba) o perdiste (rojo, hacia abajo) en esa partida. Una línea por modo lo cierra con las partidas de hoy, la posición media y las partidas en la mitad superior.",
      source: "Las puntuaciones son los números que el juego muestra en tu pantalla, leídos del juego en marcha (solo lectura) mientras esa opción esté activada, y la puntuación de cada partida se guarda con ella solo en este PC. Los héroes, los puestos y las partidas del día salen de tus partidas guardadas en este PC.",
      rule: "Solo tu propia puntuación y tus propias partidas; no se lee ni se muestra la de nadie más, y la puntuación nunca se sube a internet. Un cambio solo aparece cuando la app leyó tu puntuación justo después de esta partida y justo después de la anterior.",
    },
  },
  "lobby-no-rating": {
    en: {
      title: "The lobby panel with the game read off",
      sees: "The same last eight games with their heroes and places, and no rating anywhere: no rating lines, no changes on the tiles.",
      source: "Without reading from the game there is no rating to show. Heroes, places and the day's games come from your saved games on this PC.",
      rule: "A rating is never guessed. With the option off, nothing about your rating is read or kept.",
    },
    es: {
      title: "El panel del vestíbulo con la lectura del juego desactivada",
      sees: "Las mismas ocho últimas partidas con sus héroes y puestos, y ninguna puntuación: ni líneas de puntuación ni cambios en las fichas.",
      source: "Sin leer del juego no hay puntuación que mostrar. Los héroes, los puestos y las partidas del día salen de tus partidas guardadas en este PC.",
      rule: "Una puntuación nunca se inventa. Con la opción desactivada no se lee ni se guarda nada de tu puntuación.",
    },
  },
  "lobby-few-games": {
    en: {
      title: "The lobby panel early in the day",
      sees: "With fewer than eight games only those tiles show. A game that did not finish in the log has a dash instead of a place, and a game the app has no rating reading for has no change on it.",
      source: "The same sources as the full panel. A missing number is left out rather than shown as zero.",
      rule: "Unknown is not zero: no place and no change are shown when the app does not have them.",
    },
    es: {
      title: "El panel del vestíbulo al empezar el día",
      sees: "Con menos de ocho partidas solo salen esas fichas. Una partida que no terminó en el registro lleva un guion en vez del puesto, y una partida de la que la app no tiene lectura de puntuación no lleva cambio.",
      source: "Las mismas fuentes que el panel completo. Un número que falta se omite en vez de mostrarse como cero.",
      rule: "Desconocido no es cero: no se muestra puesto ni cambio cuando la app no los tiene.",
    },
  },
  "lobby-duos": {
    en: {
      title: "The lobby panel with Duos games",
      sees: "Solo and Duos games share the tiles; a Duos tile is marked with its mode and its change is in the Duos rating. Each mode has its own rating line and its own line of today's games.",
      source: "The same sources as the full panel, with the Duos rating the game shows next to the Solo one.",
      rule: "A change is always in the mode of the game it sits on; a Duos game never moves the Solo number shown.",
    },
    es: {
      title: "El panel del vestíbulo con partidas de Dúos",
      sees: "Las partidas de Solo y de Dúos comparten las fichas; una ficha de Dúos lleva su modo marcado y su cambio es en la puntuación de Dúos. Cada modo tiene su línea de puntuación y su línea de partidas de hoy.",
      source: "Las mismas fuentes que el panel completo, con la puntuación de Dúos que el juego muestra junto a la de Solo.",
      rule: "Un cambio es siempre en el modo de la partida en la que está; una partida de Dúos nunca mueve el número de Solo que se muestra.",
    },
  },
  solo: {
    en: {
      title: "Solo",
      sees: "The default view during a Solo game: the lobby's tribes as short chips, then one row per opponent in the game's leaderboard order with their portrait, tavern tier, health and triples, with the next opponent lit. During a combat the combat odds are one bar at the top centre of the window, and each opponent's health and triples also sit beside their hero in the game's own leaderboard.",
      source: LOG.en + " The tribes are the ones the game itself holds for this lobby, read from the running game only if you turn that on. The combat odds are worked out on your PC by simulating a few thousand fights between the two boards that entered play in that combat, only while it is on.",
      rule: "Every number is on your own screen already: the game's leaderboard shows tiers, health, triples and who is next. The tribes appear only when the game gave them, never as a guess. The odds are for the combat you are watching, from the boards you are seeing, and are gone when it ends.",
    },
    es: {
      title: "Solo",
      sees: "La vista por defecto en una partida de Solo: las tribus de la partida como etiquetas cortas y una fila por rival, en el orden de la clasificación del juego, con su retrato, nivel de taberna, vida y tripletes. El rival que el juego marca como siguiente aparece resaltado. Durante un combate, las probabilidades son una barra arriba en el centro de la ventana.",
      source: LOG.es + " Las tribus marcadas «deducido» se calcularon a partir de lo que la taberna te ofreció. Las probabilidades de combate se calculan en tu PC simulando unos miles de combates entre los dos tableros que entraron en juego en ese combate, solo mientras dura.",
      rule: "Todos los números ya están en tu pantalla: la clasificación del juego enseña niveles, vida, tripletes y quién es el siguiente. La marca «deducido» avisa cuando un valor es una deducción y no algo que el juego mostró. Las probabilidades son del combate que estás viendo, con los tableros que ves, y desaparecen cuando termina.",
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
  "gap-no-board-seen": {
    en: {
      title: "Odds: the boards are not in yet",
      sees: "Instead of numbers, one line says the boards of this combat have not entered play yet.",
      source: "There is nothing to simulate until the game puts both boards into play.",
      rule: "No number is made up. When the game has not shown you something, the overlay says so.",
    },
    es: {
      title: "Probabilidades: los tableros aún no han entrado",
      sees: "En lugar de números, una línea dice que los tableros de este combate todavía no han entrado en juego.",
      source: "No hay nada que simular hasta que el juego pone los dos tableros en juego.",
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
  "gap-missing-board": {
    en: {
      title: "Duos odds: a board is not in play yet",
      sees: "In Duos, one line says which board has not entered the combat yet: a team's second board comes in only when its first one is gone.",
      source: "Duos odds use the four boards of the combat. Any board the game has not put into play leaves a gap, and the odds appear once the last one has entered.",
      rule: "The overlay never fills a board it has not seen. It names the missing one instead.",
    },
    es: {
      title: "Probabilidades en Dúos: un tablero aún no ha entrado",
      sees: "En Dúos, una línea dice qué tablero todavía no ha entrado en el combate: el segundo tablero de un equipo entra solo cuando se acaba el primero.",
      source: "Las probabilidades de Dúos usan los cuatro tableros del combate. Cualquier tablero que el juego no haya puesto en juego deja un hueco, y las probabilidades salen cuando entra el último.",
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
  "crowded": {
    en: {
      title: "Crowded screen",
      sees: "Everything at once in a Solo game: the odds bar with its warning and its approximate note, the hover card under it, the lobby's tribes and the eight opponents in the list. Nothing covers anything else.",
      source: LOG.en,
      rule: "No overlaps: the default places keep the odds bar, the hover card, the side panels and the marks apart, whatever each holds.",
    },
    es: {
      title: "Pantalla llena",
      sees: "Todo a la vez en una partida Solo: la barra de probabilidades con su aviso y su nota de aproximado, la tarjeta bajo ella, las razas de la sala y los ocho rivales en la lista. Nada tapa a nada.",
      source: LOG.es,
      rule: "Sin solapes: los sitios por defecto mantienen separados la barra, la tarjeta, los paneles laterales y las marcas, lleve lo que lleve cada uno.",
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
  "hover-keywords": {
    en: {
      title: "Hover card, every keyword",
      sees: "The board drawn like the game: its frame, the picture under it, attack and health on the gems, and a layer for each keyword the minion had when you saw it: Taunt, Divine Shield, Reborn, Venomous, Poisonous, Deathrattle, Windfury and Stealth.",
      source: LOG.en,
      rule: "Keywords are only those you were shown on that board; the images are the game's own, from the app's texture set.",
    },
    es: {
      title: "Tarjeta con todas las palabras clave",
      sees: "El tablero dibujado como en el juego: su marco, la imagen debajo, ataque y vida en las gemas, y una capa por cada palabra clave que el esbirro tenía cuando lo viste: Provocar, Escudo divino, Renacer, Ponzoña, Veneno, Último aliento, Viento furioso y Sigilo.",
      source: LOG.es,
      rule: "Las palabras clave son solo las que se te mostraron en ese tablero; las imágenes son las del propio juego, del conjunto de texturas de la app.",
    },
  },
  "hover-golden": {
    en: {
      title: "Hover card, golden minions",
      sees: "The same board with golden minions: the golden frame and the golden Taunt shield.",
      source: LOG.en,
      rule: "Golden is what the log says; nothing is guessed.",
    },
    es: {
      title: "Tarjeta con esbirros dorados",
      sees: "El mismo tablero con esbirros dorados: el marco dorado y el escudo de Provocar dorado.",
      source: LOG.es,
      rule: "Dorado es lo que dice el registro; no se adivina nada.",
    },
  },
  "hover-effects-seven": {
    en: {
      title: "Hover card, seven moving effects",
      sees: "Seven minions that all have Windfury and Stealth: the worst case for the live effects.",
      source: LOG.en,
      rule: "The effects share one WebGL canvas and stop when the card is hidden.",
    },
    es: {
      title: "Tarjeta con siete efectos en movimiento",
      sees: "Siete esbirros con Viento furioso y Sigilo: el peor caso para los efectos en vivo.",
      source: LOG.es,
      rule: "Los efectos comparten un solo lienzo WebGL y se detienen cuando la tarjeta se oculta.",
    },
  },
  "hover-plain-seven": {
    en: {
      title: "Hover card, seven plain minions",
      sees: "The same seven minions without keywords: the baseline for the effects' cost.",
      source: LOG.en,
      rule: "Only used to measure what the effects cost.",
    },
    es: {
      title: "Tarjeta con siete esbirros sin palabras clave",
      sees: "Los mismos siete esbirros sin palabras clave: la base para medir lo que cuestan los efectos.",
      source: LOG.es,
      rule: "Solo sirve para medir lo que cuestan los efectos.",
    },
  },
  "hover-textures-none": {
    en: {
      title: "Hover card, game images not downloaded",
      sees: "When the game's images are not on this PC yet (first start, no network), the minions are plain card art with their numbers, as before.",
      source: LOG.en,
      rule: "Never a broken frame: no images means the plain look.",
    },
    es: {
      title: "Tarjeta sin las imágenes del juego",
      sees: "Si las imágenes del juego aún no están en este PC (primer inicio, sin red), los esbirros son solo la imagen de la carta con sus números, como antes.",
      source: LOG.es,
      rule: "Nunca un marco roto: sin imágenes, el aspecto sencillo.",
    },
  },
  "hover-texture-bad": {
    en: {
      title: "Hover card, some images missing",
      sees: "Three images do not load (Deathrattle skull, Taunt shield, Stealth mask): only those layers are left out and the rest is drawn.",
      source: LOG.en,
      rule: "One bad image never spoils the minion.",
    },
    es: {
      title: "Tarjeta con algunas imágenes ausentes",
      sees: "Tres imágenes no cargan (calavera de Último aliento, escudo de Provocar, máscara de Sigilo): solo esas capas faltan y el resto se dibuja.",
      source: LOG.es,
      rule: "Una imagen mala nunca estropea el esbirro.",
    },
  },
  "hover-no-webgl": {
    en: {
      title: "Hover card, no WebGL",
      sees: "Without WebGL the frame, gems and the other keywords are drawn, and Windfury and Stealth show as words instead of the moving effects.",
      source: LOG.en,
      rule: "The effects are a nicety: their word keeps the keyword visible.",
    },
    es: {
      title: "Tarjeta sin WebGL",
      sees: "Sin WebGL se dibujan el marco, las gemas y las demás palabras clave, y Viento furioso y Sigilo salen como palabras en vez de los efectos en movimiento.",
      source: LOG.es,
      rule: "Los efectos son un extra: la palabra mantiene visible la palabra clave.",
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
  "odds-bar": {
    en: {
      title: "The combat odds bar",
      sees: "One bar at the top centre of the game window: your chance to win in green, to tie in grey and to lose in red. To the left of the green, the range of health you would take from your opponent if you win; to the right of the red, the range of damage you would take if you lose. It shows only while a combat is on and goes when the combat ends.",
      source: "Worked out on your PC by simulating a few thousand fights between the two boards that entered play in this combat. The two ranges are the damage of those fights after the game's damage cap.",
      rule: "A range the app cannot work out is left out, never guessed. The odds are for the combat you are watching only: never from an older board, never in the shop.",
    },
    es: {
      title: "La barra de probabilidades del combate",
      sees: "Una barra arriba en el centro de la ventana del juego: tu probabilidad de ganar en verde, de empatar en gris y de perder en rojo. A la izquierda del verde, el margen de vida que le quitarías a tu rival si ganas; a la derecha del rojo, el margen de daño que recibirías si pierdes. Solo aparece mientras hay un combate y desaparece cuando termina.",
      source: "Se calcula en tu PC simulando unos miles de combates entre los dos tableros que entraron en juego en este combate. Los dos márgenes son el daño de esos combates tras el tope de daño del juego.",
      rule: "Un margen que la app no puede calcular se omite, nunca se adivina. Las probabilidades son solo del combate que estás viendo: nunca de un tablero antiguo, nunca en la tienda.",
    },
  },
  "odds-bar-no-ranges": {
    en: {
      title: "Odds bar without damage ranges",
      sees: "The same bar when the app cannot say how much damage a win or a loss would do: only the three shares, with nothing at either side.",
      source: "The shares are simulated on your PC as before; the damage is not known for this fight.",
      rule: "Empty is not zero: with no known damage the bar shows no range instead of an invented one.",
    },
    es: {
      title: "Barra sin márgenes de daño",
      sees: "La misma barra cuando la app no puede decir cuánto daño haría una victoria o una derrota: solo los tres porcentajes, sin nada a los lados.",
      source: "Los porcentajes se simulan en tu PC como siempre; el daño de este combate no se conoce.",
      rule: "Vacío no es cero: sin daño conocido la barra no muestra margen en lugar de inventarlo.",
    },
  },
  "leaderboard-marks": {
    en: {
      title: "Health and triples beside the leaderboard",
      sees: "To the left of each opponent's hero in the game's own leaderboard: their remaining health in big numbers and a star for each triple they have made.",
      source: LOG.en,
      rule: "The same health and triples the game's leaderboard already shows you; nothing is added that your screen does not have. Rows with unknown values show nothing.",
    },
    es: {
      title: "Vida y tripletes junto a la clasificación",
      sees: "A la izquierda del héroe de cada rival en la clasificación del propio juego: su vida restante en números grandes y una estrella por cada triplete que ha hecho.",
      source: LOG.es,
      rule: "La misma vida y los mismos tripletes que la clasificación del juego ya te enseña; no se añade nada que tu pantalla no tenga. Las filas con valores desconocidos no muestran nada.",
    },
  },
  "tribes-game": {
    en: {
      title: "Tribes from the game",
      sees: "The five tribes of this lobby as short chips at the top of the opponents panel, above a thin line, exactly as the game holds them.",
      source: "Read from the running game (read-only), and only if you leave that option on. Nothing is uploaded.",
      rule: "These are the tribes the game shows you at the start of the match.",
    },
    es: {
      title: "Tribus leídas del juego",
      sees: "Las cinco tribus de esta partida como etiquetas cortas arriba del panel de rivales, sobre una línea fina, tal como las tiene el juego.",
      source: "Se leen del juego en marcha (solo lectura), y solo si dejas esa opción activada. No se sube nada.",
      rule: "Son las tribus que el juego te muestra al empezar la partida.",
    },
  },
  "tribes-off": {
    en: {
      title: "Tribes with the game read off",
      sees: "No tribes: the opponents panel starts with one short line saying the game read is off.",
      source: "Nothing: the app does not look at the game while that option is off.",
      rule: "The overlay never guesses the tribes. Without the game's own data the panel stays empty and says why.",
    },
    es: {
      title: "Tribus con la lectura del juego desactivada",
      sees: "Ninguna tribu: el panel de rivales empieza con una línea corta que dice que la lectura del juego está desactivada.",
      source: "Nada: la app no mira el juego mientras esa opción esté desactivada.",
      rule: "La superposición nunca adivina las tribus. Sin los datos del propio juego el panel queda vacío y dice por qué.",
    },
  },
  "tribes-unsupported": {
    en: {
      title: "Tribes on a game version not supported yet",
      sees: "No tribes: the opponents panel starts with one short line saying this game version is not supported yet.",
      source: "After a game update the app has to be checked against the new version before it reads anything from it.",
      rule: "A wrong tribe would be worse than none, so the panel stays empty until the version is checked.",
    },
    es: {
      title: "Tribus en una versión del juego aún no compatible",
      sees: "Ninguna tribu: el panel de rivales empieza con una línea corta que dice que esta versión del juego aún no es compatible.",
      source: "Tras una actualización del juego hay que comprobar la app con la versión nueva antes de que lea nada de ella.",
      rule: "Una tribu equivocada sería peor que ninguna, así que el panel queda vacío hasta que se comprueba la versión.",
    },
  },
  "tribes-no-lobby": {
    en: {
      title: "Tribes in a game the client reopened",
      sees: "No tribes: the opponents panel starts with one short line saying no lobby was seen for this game.",
      source: "When the game restarts inside a game, the app never saw the lobby that holds the tribes, so it does not read them.",
      rule: "A tribe the app did not see being dealt would be a guess, so the panel stays empty and says why.",
    },
    es: {
      title: "Tribus en una partida que el cliente reabrió",
      sees: "Ninguna tribu: el panel de rivales empieza con una línea corta que dice que no se vio el lobby de esta partida.",
      source: "Cuando el juego se reinicia dentro de una partida, la app nunca vio el lobby que guarda las tribus, así que no las lee.",
      rule: "Una tribu que la app no vio repartir sería una suposición, así que el panel queda vacío y dice por qué.",
    },
  },
  "hero-select": {
    en: {
      title: "Hero select",
      sees: "While you choose a hero, a card for each hero on offer: its portrait, its name and how you have done with it so far: games played, average place and the share of those games you finished in the top half. The lobby's tribes sit above the cards. A hero you have never played says there are no games yet.",
      source: "The heroes on offer are the ones the game lists for you in its own log. The numbers come from your saved games on this PC. The tribes are the ones the game holds for this lobby, read from the running game only if you leave that on.",
      rule: "Only the heroes the game put on your own screen, and only your own results with them. Nobody else's offer is known to the log, and an empty spot stays empty rather than showing a made-up number.",
    },
    es: {
      title: "Elección de héroe",
      sees: "Mientras eliges héroe, una tarjeta por cada héroe a elegir: su retrato, su nombre y cómo te ha ido con él hasta ahora: partidas jugadas, puesto medio y la parte de esas partidas que acabaste en la mitad superior. Las tribus de la partida van encima de las tarjetas. Un héroe que nunca has jugado dice que aún no hay partidas.",
      source: "Los héroes a elegir son los que el juego te lista en su propio registro. Los números salen de tus partidas guardadas en este PC. Las tribus son las que el juego tiene para esta partida, leídas del juego en marcha solo si lo dejas activado.",
      rule: "Solo los héroes que el juego puso en tu propia pantalla y solo tus resultados con ellos. El registro no conoce la oferta de nadie más, y un hueco vacío queda vacío en vez de mostrar un número inventado.",
    },
  },
  "hero-select-reroll": {
    en: {
      title: "Hero select after a reroll",
      sees: "The same cards after you swap one hero for another: the new hero takes the old one's place, and the small count of rerolls goes up. The new hero here has no games yet.",
      source: "The new hero is read from the game's log the moment the game swaps it in. Numbers are from your saved games on this PC.",
      rule: "It follows what the game shows you and nothing else: a hero you swapped away is gone from the panel.",
    },
    es: {
      title: "Elección de héroe tras un cambio",
      sees: "Las mismas tarjetas después de cambiar un héroe por otro: el héroe nuevo ocupa el lugar del anterior y sube el pequeño contador de cambios. El héroe nuevo de este ejemplo aún no tiene partidas.",
      source: "El héroe nuevo se lee del registro del juego en cuanto el juego lo pone. Los números salen de tus partidas guardadas en este PC.",
      rule: "Sigue solo lo que el juego te enseña: un héroe que cambiaste ya no está en el panel.",
    },
  },
  "hero-select-duos": {
    en: {
      title: "Hero select in Duos",
      sees: "In Duos the game offers two heroes, so there are two cards. The numbers are your own Duos history, and the top half means the top two teams.",
      source: "The heroes come from the game's log; the numbers from your saved Duos games on this PC.",
      rule: "Same rule as Solo: your own offer and your own results only, nothing about your teammate's choice.",
    },
    es: {
      title: "Elección de héroe en Dúos",
      sees: "En Dúos el juego ofrece dos héroes, así que hay dos tarjetas. Los números son tu propio historial de Dúos y la mitad superior son los dos primeros equipos.",
      source: "Los héroes salen del registro del juego; los números, de tus partidas de Dúos guardadas en este PC.",
      rule: "La misma regla que en Solo: solo tu oferta y tus resultados, nada de la elección de tu compañero.",
    },
  },
  "hero-select-new": {
    en: {
      title: "Hero select with no history",
      sees: "For a new player, or after clearing the history, every card says there are no games yet: no zeros, no placeholder numbers.",
      source: "There is nothing saved on this PC for these heroes, so there is nothing to show.",
      rule: "Unknown is not zero: the panel says so instead of inventing a record.",
    },
    es: {
      title: "Elección de héroe sin historial",
      sees: "Para un jugador nuevo, o tras borrar el historial, cada tarjeta dice que aún no hay partidas: sin ceros ni números de relleno.",
      source: "No hay nada guardado en este PC sobre estos héroes, así que no hay nada que mostrar.",
      rule: "Desconocido no es cero: el panel lo dice en vez de inventar un historial.",
    },
  },
  counters: {
    en: {
      title: "Counters",
      sees: "A small box of running totals for your own game, under the opponents list: for example Blood Gem size, extra Tavern spell power, Tavern minion bonuses and free rolls. A line shows only while its number is above zero, and the box only while it has a line, so the opponents list never moves.",
      source: LOG.en,
      rule: "Only your own totals, built from things that happened on your own screen. They are a memory aid: nothing about opponents, your teammate or anything the game did not show you.",
    },
    es: {
      title: "Contadores",
      sees: "Una cajita de totales acumulados de tu propia partida, debajo de la lista de rivales: por ejemplo el tamaño de las Gemas de sangre, el poder extra de los hechizos de taberna, las mejoras de los esbirros de taberna y los refrescos gratis. Una línea solo aparece mientras su número es mayor que cero, y la caja solo mientras tiene alguna línea, así que la lista de rivales no se mueve nunca.",
      source: LOG.es,
      rule: "Solo tus propios totales, hechos con cosas que pasaron en tu propia pantalla. Son una ayuda de memoria: nada de rivales, de tu compañero ni de lo que el juego no te enseñó.",
    },
  },
  "legend-open": {
    en: {
      title: "The legend, from the layout editor",
      sees: "While you arrange the overlay, a \"?\" next to the editor's title shows what the marks mean and where card names and pictures come from. A second click or Esc hides it again; it is never shown during a game.",
      source: "Fixed text in the app; nothing is read.",
      rule: "It explains the overlay itself and shows nothing about the game.",
    },
    es: {
      title: "La leyenda, desde el editor del diseño",
      sees: "Mientras colocas el overlay, un «?» junto al título del editor muestra qué significan las marcas y de dónde salen los nombres e imágenes de las cartas. Otro clic o Esc la vuelve a ocultar; nunca aparece durante la partida.",
      source: "Texto fijo de la app; no se lee nada.",
      rule: "Explica el propio overlay y no muestra nada de la partida.",
    },
  },
  "hide-all": {
    en: {
      title: "Hide everything with one key",
      sees: "Ctrl+Shift+H hides everything the overlay draws; a short note says which key brings it back and goes after about 3 seconds. The same key, or \"Hide overlay\" in the tray menu, shows it again as it was; the next game starts shown. This frame hides and shows it over and over.",
      source: "Your own key press, taken by the app only while Hearthstone runs. Every start begins with the overlay shown.",
      rule: "Nothing is sent to the game and nothing is read from it: while Hearthstone runs, only that one key combination goes to the app instead.",
    },
    es: {
      title: "Ocultarlo todo con una tecla",
      sees: "Ctrl+Mayús+H oculta todo lo que dibuja el overlay; una nota breve dice qué tecla lo vuelve a mostrar y se va a los 3 segundos más o menos. La misma tecla, o «Ocultar overlay» en el menú de la bandeja, lo vuelve a mostrar como estaba; la siguiente partida empieza con él a la vista. Este cuadro lo oculta y lo muestra una y otra vez.",
      source: "Tu propia pulsación, que la app toma solo mientras Hearthstone está abierto. Cada inicio empieza con el overlay a la vista.",
      rule: "No se envía nada al juego ni se lee nada de él: mientras Hearthstone está abierto, solo esa combinación de teclas va a la app.",
    },
  },
  preview: {
    en: {
      title: "Preview from the tray",
      sees: "\"Preview overlay with example data\" in the tray menu shows the overlay over your desktop with made-up numbers, so you can see it without a game. Clicks go through it. It closes from the same item, and by itself when Hearthstone starts, a game begins or the game's window comes to the front.",
      source: "Example data built into the app; nothing is read.",
      rule: "It never sits over the game and shows nothing from it.",
    },
    es: {
      title: "Vista previa desde la bandeja",
      sees: "«Vista previa del overlay con datos de ejemplo» en el menú de la bandeja muestra el overlay sobre tu escritorio con números inventados, para verlo sin partida. Los clics pasan a través. Se cierra desde la misma opción, y sola cuando se abre Hearthstone, empieza una partida o la ventana del juego pasa al frente.",
      source: "Datos de ejemplo incluidos en la app; no se lee nada.",
      rule: "Nunca queda encima del juego y no muestra nada de él.",
    },
  },
};

module.exports = { CAPTIONS };

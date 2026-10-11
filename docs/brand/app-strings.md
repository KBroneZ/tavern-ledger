# Desktop app strings: proposed wording (T-533 for T-531)

These were wording proposals for the app session T-531; **session 093 applied them** (D-106), keys unchanged, with these differences:

- `tribe.MECHANICAL` and `ov.tribeShort.MECHANICAL` (ES) stay **Robot**: the esES game's own string for the Mech tribe (`GLOBAL_RACE_MECHANICAL`) is "Robot", so "Mecánico" was not applied.
- The esES game writes "Campos de batalla" (lowercase b) in its own strings; the app keeps "Campos de Batalla", as the voice guide and the website do. Changing it is one decision for both.
- Checked against the esES game strings: Venomous is "Ponzoña" and Rally is "Alentar", as the app already had them.
- `lobby.placed` (EN) is now "{n} with a place": it counts the day's games with a known final place (lobby-words.js).
- Not in the table but done for the same reasons: the tray's Spanish tooltip says "Campos de Batalla", the tray's Spanish overlay item matches the Settings switch ("Mostrar overlay durante la partida"), and the Settings switch has a one-line note on what "off" means.

Only strings worth changing were listed; where one row covers a `.one` and a `.other` key, the two proposals are separated by ` // `.

Rows marked "Rust" are sentences the Rust side writes: the Spanish table is keyed by the English text, so changing the English there means changing the key too. Rows whose key is an English sentence are Rust-side.

## `strings/common.js`

| Key | Lang | Current | Proposed | Why |
|-----|------|---------|----------|-----|
| settings.intro | EN | Changes apply at once and are kept on this PC. | Changes apply right away and are saved on this PC. | stiff |
| settings.language.note | EN | Card and hero names follow this language too (from the card data; the game's own name when the card data does not have the card). | Card and hero names use this language too, taken from the card data. If a card isn't in the card data, you get the game's own name. | nested parentheses |
| settings.language.note | ES | Los nombres de cartas y héroes también siguen este idioma (de los datos de cartas; el nombre del propio juego cuando los datos de cartas no tienen la carta). | Los nombres de cartas y héroes también usan este idioma, tomados de los datos de cartas. Si una carta no está en esos datos, se usa el nombre del propio juego. | literal, nested parentheses |
| settings.overlay.theme | EN | Overlay theme | Theme | repeats heading |
| settings.overlay.theme | ES | Tema del overlay | Tema | repeats heading |
| settings.detail.failed | EN | The choice could not be saved ({error}); it holds until the app closes. | Couldn't save your choice ({error}). It stays until you close the app. | passive, stiff |
| settings.detail.failed | ES | No se ha podido guardar la elección ({error}); vale hasta que cierres la app. | No se ha podido guardar la elección ({error}). Se mantiene hasta que cierres la app. | "vale" odd |
| settings.windows.title | EN | Windows | Startup | repeats the switch below |
| settings.windows.title | ES | Windows | Inicio | repeats the switch below |
| settings.autostart.failed | EN | The Windows start-up setting could not be read or changed ({error}). | Couldn't read or change the Windows startup setting ({error}). | passive |
| settings.overlay.lobby | ES | Panel de la sala: tu puntuación y las partidas de hoy en la pantalla de Battlegrounds | Panel de la sala: tu puntuación y las partidas de hoy en la pantalla de Campos de Batalla | game term |
| settings.overlay.odds | EN | Combat odds: win, tie and lose against your next opponent, from the boards you have seen | Combat odds: your chance to win, tie or lose against your next opponent, based on the boards you've seen | unclear grammar |
| settings.overlay.odds | ES | Probabilidades del combate: ganar, empatar y perder contra tu siguiente rival, con los tableros que has visto | Probabilidades de combate: ganar, empatar o perder contra tu siguiente rival, según los tableros que has visto | natural order |
| settings.memory.title | EN | Lobby tribes and rating from the game | Read from the game | repeats the switch below |
| settings.memory.title | ES | Tribus de la partida y puntuación desde el juego | Lectura del juego | repeats the switch below |
| settings.memory.what | EN | It reads two things the game already shows you, the lobby's five tribes at hero select and your own Battlegrounds rating, straight from the game while it runs. It only reads: it never changes the game, adds code to it or plays for you. The values stay on this PC and are never uploaded. | It reads two things the game already shows you: the lobby's five tribes at hero select and your own Battlegrounds rating. It reads them from the game while it runs, and only reads: it never changes the game, adds code to it or plays for you. The values stay on this PC and are never uploaded. | run-on list |
| settings.memory.what | ES | Lee dos cosas que el juego ya te muestra, las cinco tribus de la partida al elegir héroe y tu propia puntuación de Battlegrounds, directamente del juego mientras está abierto. Solo lee: nunca cambia el juego, no le añade código ni juega por ti. Los valores se quedan en este PC y nunca se suben. | Lee dos cosas que el juego ya te muestra: las cinco tribus de la partida al elegir héroe y tu propia puntuación de Campos de Batalla. Las lee del juego mientras está abierto y solo las lee: nunca cambia el juego, no le añade código ni juega por ti. Los valores se quedan en este PC y nunca se suben. | run-on list, game term |
| settings.memory.status.waiting | ES | Activado. Esperando una partida de Battlegrounds o la pantalla de Battlegrounds. | Activado. Esperando una partida de Campos de Batalla o su pantalla. | game term, repeats |
| settings.memory.status.no_access | EN | On, but Windows refused to let the app read the game (is the game running as administrator?). | On, but Windows won't let the app read the game. Is the game running as administrator? | stiff |
| settings.memory.status.no_access | ES | Activado, pero Windows no deja a la app leer el juego (¿se está ejecutando como administrador?). | Activado, pero Windows no deja que la app lea el juego. ¿El juego se está ejecutando como administrador? | unclear subject |
| settings.memory.failed | EN | The setting could not be read or changed ({error}). | Couldn't read or change the setting ({error}). | passive |
| source.log.description | EN | The game's own log says so. Counts and averages are worked out from such values. | The game's own log says so. Counts and averages are calculated from values like this. | stiff |
| source.inferred.description | EN | Worked out by Tavern Ledger from the log (for example tribes seen in the tavern, or skins grouped under one hero). It can be wrong. | Tavern Ledger worked this out from the log (for example, tribes seen in the tavern, or skins grouped under one hero). It can be wrong. | passive |
| source.card_data.description | EN | Card name or art looked up by the card id in the log, in HearthstoneJSON's copy of Blizzard's card data, in the app's language. It can lag behind a game patch. | Card name or art, looked up by the card id the log gives, in HearthstoneJSON's copy of Blizzard's card data, in the app's language. It can lag behind a game patch. | unclear "in the log" |
| source.card_data.description | ES | Nombre o imagen de la carta buscados por su id en el registro, en la copia de HearthstoneJSON de los datos de cartas de Blizzard, en el idioma de la app. Puede ir por detrás de un parche. | Nombre o imagen de la carta, buscados por el id que da el registro, en la copia de HearthstoneJSON de los datos de cartas de Blizzard, en el idioma de la app. Puede ir por detrás de un parche. | unclear "en el registro" |
| source.unknown.description | EN | Nothing in the log says. It is not zero. | The log doesn't say. It is not zero. | awkward |
| tribe.MECHANICAL | ES | Robot | Mecánico | game term (check the esES client first) |
| mode.GT_BATTLEGROUNDS | ES | Individual | Solo | game term |
| outcome.won | ES | Ganado | Victoria | natural ("Victoria contra X") |
| outcome.lost | ES | Perdido | Derrota | natural ("Derrota contra X") |
| In the Battlegrounds lobby (Rust) | ES | En la sala de Battlegrounds | En la sala de Campos de Batalla | game term |
| Battlegrounds game loading (Rust) | ES | Cargando una partida de Battlegrounds | Cargando una partida de Campos de Batalla | game term |
| That is not a Battlegrounds tribe. (Rust) | ES | Esa no es una tribu de Battlegrounds. | Esa no es una tribu de Campos de Batalla. | game term |
| The upload worker stopped; restart the app. (Rust) | EN | The upload worker stopped; restart the app. | Uploading stopped. Restart the app. | jargon ("worker") |
| The upload worker stopped; restart the app. (Rust) | ES | La subida se ha detenido; reinicia la app. | La subida se ha detenido. Reinicia la app. | split sentence |
| Optional: ... can have these lines ... (Rust pattern) | ES | ... muestra cuándo estás en la sala de Battlegrounds o cargando una partida ... | ... muestra cuándo estás en la sala de Campos de Batalla o cargando una partida ... | game term |

## `strings/details.js`

| Key | Lang | Current | Proposed | Why |
|-----|------|---------|----------|-----|
| extras.results.inferred | ES | {n} inferidos por la vida | {n} deducidos de la vida | term ("deducido" is the source label) |
| extras.gold.extra.tip | EN | Gold beyond each turn's own and beyond sells: cards, trinkets, refunds. | Gold that is not the turn's own gold or from selling: cards, trinkets, refunds. | unclear |
| extras.gold.extra.tip | ES | Oro más allá del propio de cada turno y de las ventas: cartas, abalorios, reembolsos. | Oro que no es el de cada turno ni el de vender: cartas, abalorios, reembolsos. | unclear |
| extras.gold.beyond | EN | Gold beyond the fixed income | Gold beyond base income | jargon ("fixed income") |
| extras.gold.beyond | ES | Oro más allá de los ingresos fijos | Oro más allá del oro base del turno | jargon |
| extras.avg.beyond | EN | Gold beyond income / game | Gold beyond base income / game | match label |
| extras.avg.beyond | ES | Oro más allá de los ingresos / partida | Oro más allá del oro base / partida | match label |
| Gold beyond the fixed income (Rust) | EN / ES | Gold beyond the fixed income / Oro más allá de los ingresos fijos | Gold beyond base income / Oro más allá del oro base del turno | match label; change the Rust label and its ES key together |
| extras.free.given.tip | ES | Refrescos que usaron una refresco gratis mostrada en el botón de refrescar. Las demás que no gastaron oro vinieron de cartas o efectos. | Refrescos que usaron un refresco gratis de los que muestra el botón de refrescar. Los demás refrescos que no gastaron oro vinieron de cartas o efectos. | gender |
| extras.avg.free | ES | Refrescos gratis dadas / partida | Refrescos gratis dados / partida | gender |
| Rolls paid with gold (Rust) | ES | Refrescos pagadas con oro | Refrescos pagados con oro | gender |
| extras.avg_over.one / .other | ES | Media sobre {n} partida con este registro. // Media sobre {n} partidas con este registro. | Media de {n} partida con este registro. // Media de {n} partidas con este registro. | literal ("over") |
| extras.place_tip.one / .other | ES | Sobre {n} partida con un puesto que cuenta. // Sobre {n} partidas con un puesto que cuenta. | En {n} partida con un puesto que cuenta. // En {n} partidas con un puesto que cuenta. | literal ("over") |
| extras.picks.note.one / .other | ES | Sobre {n} partida cuya elección de héroe está en el registro. // Sobre {n} partidas cuya elección de héroe está en el registro. | En {n} partida cuya elección de héroe está en el registro. // En {n} partidas cuya elección de héroe está en el registro. | literal ("over") |
| r2.place_tip.one / .other | ES | Sobre {n} partida con un puesto que cuenta. // Sobre {n} partidas con un puesto que cuenta. | En {n} partida con un puesto que cuenta. // En {n} partidas con un puesto que cuenta. | literal ("over") |
| r2.stats.note.one / .other | ES | Sobre {n} partida cuyas elecciones están en el registro. // Sobre {n} partidas cuyas elecciones están en el registro. | En {n} partida cuyas elecciones están en el registro. // En {n} partidas cuyas elecciones están en el registro. | literal ("over") |
| r2.left.no_data | EN | {n} the log could not read | {n} whose data could not be read | broken fragment after "Not counted:" |
| r2.left.no_data | ES | {n} que el registro no pudo leer | {n} cuyos datos no se pudieron leer | broken fragment |
| turns.key | EN | Open a turn for both boards, its shop and what you bought. Health change over each combat | Open a turn to see both boards, its shop and what you bought. Health change over each combat: | fragment; a mark and a full stop follow it |
| turns.key | ES | Abre un turno para ver ambos tableros, su tienda y lo que compraste. Cambio de vida en cada combate | Abre un turno para ver ambos tableros, su tienda y lo que compraste. Cambio de vida en cada combate: | fragment; a mark and a full stop follow it |
| curious.in_none.one | EN | In none of your {n} game. | Not in your only game ({n}). | awkward, matches ES |
| curious.in_all.one | EN | In all {n} game. | In your only game ({n}). | awkward, matches ES |
| curious.unavailable | EN | Curious stats are not available yet. | Records aren't available yet. | rename D-102 |
| curious.unavailable | ES | Las estadísticas curiosas aún no están disponibles. | Los récords aún no están disponibles. | rename D-102 |
| compact.about | ES | Sobre esto | Más información | inconsistent with win.about |
| Most combats won in a row inside one game... (Rust) | ES | ... Es inferido si algún resultado se dedujo de la vida. | ... Es un dato deducido si algún resultado salió de la vida. | term ("deducido") |
| Your health before a combat minus your health after it... (Rust) | ES | ... Inferido: también cuenta la vida pagada o curada ... | ... Deducido: también cuenta la vida pagada o curada ... | term ("deducido") |
| Every game with a shop record sold a golden minion the card data does not know, so it cannot be told. (Rust) | EN | ... so it cannot be told. | ... so Tavern Ledger can't tell. | awkward; change Rust text and ES key together |

## `strings/window.js`

| Key | Lang | Current | Proposed | Why |
|-----|------|---------|----------|-----|
| win.tagline | ES | Tus partidas de Battlegrounds, leídas del registro del propio juego. | Tus partidas de Campos de Batalla, leídas del registro del juego. | game term |
| win.tab.curious | EN | Curious stats | Records | rename D-102 |
| win.tab.curious | ES | Curiosidades | Récords | rename D-102 |
| win.curious.aboutTitle | EN | About these totals | About these records | rename D-102 |
| win.curious.aboutTitle | ES | Sobre estos totales | Sobre estos récords | rename D-102 |
| win.curious.about | EN | Totals over all your games, Solo and Duos together. Each one says where it comes from and how many games back it; a game saved before a value was kept is left out of that count, never added as zero. | Records and totals across all your games, Solo and Duos together. Each one says where it comes from and how many games back it. A game saved before a value was kept is left out of that count, never added as zero. | rename D-102, long sentence |
| win.curious.about | ES | Totales de todas tus partidas, Individual y Dúos juntos. Cada uno dice de dónde viene y cuántas partidas lo respaldan; una partida guardada antes de que se registrara un valor queda fuera de ese recuento, nunca se suma como cero. | Récords y totales de todas tus partidas, Solo y Dúos juntos. Cada uno dice de dónde sale y cuántas partidas lo respaldan. Una partida guardada antes de que se registrara un valor queda fuera de ese recuento y nunca se suma como cero. | rename D-102, game term |
| win.tally.dev | EN | {n} played with the reconnect dev tool left out. | {n} left out: played with the reconnect dev tool. | garbled order |
| win.tally.dev | ES | {n} jugadas con la herramienta de reconexión de desarrollo quedan fuera. | {n} quedan fuera: jugadas con la herramienta de reconexión para desarrolladores. | literal, garbled order |
| win.dev.mark | ES | Reconexión de desarrollo | Reconexión dev | literal, long |
| win.dev.tip | ES | Jugada con la herramienta de reconexión de desarrollo: no cuenta en las estadísticas y nunca se sube. | Jugada con la herramienta de reconexión para desarrolladores: no cuenta en las estadísticas y nunca se sube. | literal |
| win.upload.devReconnect | ES | {n} se quedan en este PC (jugadas con la herramienta de reconexión de desarrollo) | {n} se quedan en este PC (jugadas con la herramienta de reconexión para desarrolladores) | literal |
| win.upload.heldDev | ES | {n} retenidas: no se puede leer el archivo de la herramienta de reconexión de desarrollo | {n} retenidas: no se puede leer el archivo de la herramienta de reconexión para desarrolladores | literal |
| win.remove | EN | Remove | Remove tribes | button does not say what it removes |
| win.remove | ES | Quitar | Quitar tribus | button does not say what it removes |
| win.lobby.change | EN | Change | Change tribes | button does not say what it changes |
| win.lobby.change | ES | Cambiar | Cambiar tribus | button does not say what it changes |
| win.lobby.about | EN | ... they are saved on your computer and always marked as entered by you. They are never mixed with the tribes seen in the tavern. | ... they are saved on this PC and always marked as entered by you. They are never mixed with the tribes seen in the tavern. | inconsistent ("this PC") |
| win.lobby.about | ES | ... se guardan en tu ordenador y siempre se marcan como introducidas por ti. ... | ... se guardan en este PC y siempre se marcan como introducidas por ti. ... | inconsistent ("este PC") |
| win.ledger.title | ES | Registro | Historial | clashes with "registro" (the log) |
| win.ledger.emptyInitial | ES | Aún no hay partidas. Juega una partida de Battlegrounds y aparecerá aquí cuando termine. | Aún no hay partidas. Juega una de Campos de Batalla y aparecerá aquí cuando termine. | game term |
| win.prov.parserUnknown | EN | parser unknown version | parser version unknown | word order |
| win.entered.about | EN | Typed by you, one pick of five per game. Separate from the tribes seen in the tavern. | Entered by you: five tribes per game. Kept apart from the tribes seen in the tavern. | unclear |
| win.entered.about | ES | Escritas por ti, una elección de cinco por partida. Aparte de las tribus vistas en la taberna. | Introducidas por ti: cinco tribus por partida. Se guardan aparte de las tribus vistas en la taberna. | unclear, literal |
| win.entered.none | ES | No has introducido las tribus de la partida para esta partida. | No has introducido las tribus de esta partida. | repeats "partida" |
| win.shop.about | EN | Averages per game over the games whose shop the log has from start to end. Only your own shop is in the log. | Averages per game, from the games where the log has your shop from start to finish. Only your own shop is in the log. | stiff |
| win.shop.about | ES | Medias por partida sobre las partidas cuya tienda tiene el registro de principio a fin. En el registro solo está tu propia tienda. | Medias por partida, de las partidas en las que el registro tiene tu tienda completa. En el registro solo está tu propia tienda. | literal, twisted order |
| win.picks.title | ES | Héroes elegidos | Elecciones de héroe | inconsistent with other hero pick strings |
| win.choices.about | ES | Cuántas veces te ofrecieron cada uno, cuántas veces lo cogiste y tu puesto medio cuando lo hiciste. ... | Cuántas veces te ofrecieron cada uno, cuántas veces lo elegiste y tu puesto medio cuando lo hiciste. ... | inconsistent ("elegidos") |
| win.overlay.during | EN | Overlay during a game | Show overlay during a game | switch label should say the action |
| win.overlay.during | ES | Overlay durante una partida | Mostrar overlay durante la partida | switch label should say the action |
| win.overlay.how | EN | Also in the tray menu. It shows over Hearthstone only while a game is on, and its small lobby panel on the Battlegrounds lobby screen; hover a hero on the game's leaderboard to see the last board you met. "Arrange overlay" lets you move the panels and fit the leaderboard box (the whole screen takes clicks until you press "Lock overlay"). Exclusive fullscreen hides any overlay: run Hearthstone windowed or borderless. | Also in the tray menu. The overlay shows over Hearthstone only during a game, plus a small lobby panel on the Battlegrounds lobby screen. Hover a hero on the game's leaderboard to see the last board you saw of theirs. "Arrange overlay" lets you move the panels and fit the leaderboard box; until you press "Lock overlay", clicks don't reach the game. Exclusive fullscreen hides any overlay, so run Hearthstone windowed or borderless. | long, unclear, "board you met" |
| win.overlay.how | ES | También está en el menú de la bandeja. Se muestra sobre Hearthstone solo mientras hay una partida en curso, y su pequeño panel de la sala en la pantalla de Battlegrounds; pasa el ratón por un héroe de la clasificación del juego para ver el último tablero que te encontraste. «Organizar overlay» te deja mover los paneles y ajustar el recuadro de la clasificación (toda la pantalla recibe los clics hasta que pulses «Bloquear overlay»). El pantalla completa exclusivo oculta cualquier overlay: usa Hearthstone en ventana o sin bordes. | También está en el menú de la bandeja. El overlay se muestra sobre Hearthstone solo durante una partida, y su pequeño panel de la sala en la pantalla de Campos de Batalla. Pasa el ratón por un héroe de la clasificación del juego para ver el último tablero suyo que viste. «Organizar overlay» te deja mover los paneles y ajustar el recuadro de la clasificación; hasta que pulses «Bloquear overlay», los clics no llegan al juego. El modo de pantalla completa exclusivo oculta cualquier overlay, así que usa Hearthstone en ventana o sin bordes. | grammar ("El pantalla"), unclear, game term |
| win.overlay.failed | EN | The overlay setting could not be read or changed ({error}). | Couldn't read or change the overlay setting ({error}). | passive |
| win.upload.offNote | EN | Off until you turn it on. Each finished game's report goes up: heroes, card ids, places and boards. Never player names, BattleTags or ratings. | Off until you turn it on. When it's on, each finished game's report is uploaded: heroes, card ids, places and boards. Never player names, BattleTags or ratings. | clearer |
| win.recap.tribes | EN | Tribes | Enter tribes | button does not say what it does |
| win.recap.tribes | ES | Tribus | Introducir tribus | button does not say what it does |
| win.recap.tribesDone | EN | Tribes ✓ | Tribes entered ✓ | unclear |
| win.recap.tribesDone | ES | Tribus ✓ | Tribus introducidas ✓ | unclear |
| win.recap.report | EN | Report | Report a problem | unclear button |
| win.recap.report | ES | Informar | Informar de un problema | unclear button |
| win.report.note | EN | This is the file that will be saved (only its time stamp is set again when you save). It holds no player names: the app refuses to make it if it finds one. Nothing is sent anywhere. Saving puts the file in your Downloads folder, and you can send it yourself. | This is the file that will be saved; only its timestamp is set again when you save. It has no player names: the app won't create it if it finds one. Nothing is sent anywhere. Saving puts the file in your Downloads folder, and you can send it yourself. | stiff |
| win.report.note | ES | Este es el archivo que se guardará (solo se vuelve a fijar su marca de tiempo al guardar). No contiene nombres de jugadores: la app se niega a crearlo si encuentra alguno. No se envía nada a ninguna parte. Al guardar, el archivo se pone en tu carpeta Descargas y puedes enviarlo tú. | Este es el archivo que se guardará; al guardar solo cambia su marca de hora. No contiene nombres de jugadores: la app no lo crea si encuentra alguno. No se envía nada a ninguna parte. Al guardar, el archivo queda en tu carpeta Descargas y puedes enviarlo tú. | literal |

## `strings/overlay.js`

| Key | Lang | Current | Proposed | Why |
|-----|------|---------|----------|-----|
| ov.toolbar.note | EN | Clicks do not reach the game while unlocked. Drag a panel to move it, its corner to resize it; or click a panel and use the buttons next to it; or Tab to a panel and use the arrow keys (Shift for bigger steps, Alt to resize). Example data is shown when no game is on. Drag the dashed "Leaderboard" box over the game's column of heroes if it is not there already; it is kept for this game window size. | While the overlay is unlocked, clicks don't reach the game. Drag a panel to move it, or its corner to resize it. You can also click a panel and use the buttons next to it, or Tab to a panel and use the arrow keys (Shift for bigger steps, Alt to resize). Example data shows when no game is on. Drag the dashed "Leaderboard" box over the game's column of heroes if it isn't there already; it's saved for this game window size. | too long, run-on |
| ov.toolbar.note | ES | Los clics no llegan al juego mientras está desbloqueado. Arrastra un panel para moverlo y su esquina para cambiar el tamaño; o haz clic en un panel y usa los botones de al lado; o ve con Tab a un panel y usa las flechas (Mayús para pasos grandes, Alt para cambiar el tamaño). Se muestran datos de ejemplo cuando no hay partida. Arrastra el recuadro discontinuo «Clasificación» sobre la columna de héroes del juego si no está ya ahí; se guarda para este tamaño de ventana del juego. | Mientras el overlay está desbloqueado, los clics no llegan al juego. Arrastra un panel para moverlo, o su esquina para cambiar el tamaño. También puedes hacer clic en un panel y usar los botones de al lado, o ir con Tab a un panel y usar las flechas (Mayús para pasos grandes, Alt para cambiar el tamaño). Se ven datos de ejemplo cuando no hay partida. Arrastra el recuadro discontinuo «Clasificación» sobre la columna de héroes del juego si no está ya ahí; se guarda para este tamaño de ventana del juego. | too long, run-on |
| ov.warn.updateFailed | EN | The overlay settings could not be updated ({error}). | Couldn't update the overlay settings ({error}). | passive |
| ov.box.default | EN | Leaderboard (measured default) | Leaderboard (default position) | jargon |
| ov.box.default | ES | Clasificación (medida por defecto) | Clasificación (posición por defecto) | jargon |
| ov.status.loading | ES | Cargando una partida de Battlegrounds… | Cargando una partida de Campos de Batalla… | game term |
| ov.status.unreadable.title | EN | Cannot read this game | Can't read this game | stiff |
| ov.status.unreadable.body | EN | The game's log is not in a form this version understands, so nothing is shown rather than a guess. | This version of the app doesn't understand the game's log format, so it shows nothing instead of guessing. | stiff |
| ov.status.unreadable.body | ES | El registro del juego no tiene un formato que esta versión entienda, así que no se muestra nada antes que un dato inventado. | Esta versión de la app no entiende el formato del registro del juego, así que no muestra nada en vez de inventar datos. | literal |
| ov.error.update | EN | The overlay could not update ({error}). | Couldn't update the overlay ({error}). | passive |
| ov.tribes.lobbyAll | ES | Sala (las 5 confirmadas) | Partida (las 5 confirmadas) | match "tribus de la partida" |
| ov.tribes.lobbyPart | ES | Sala ({count} de 5 confirmadas) | Partida ({count} de 5 confirmadas) | match "tribus de la partida" |
| ov.tribes.lobbyGame | ES | Sala | Partida | match "tribus de la partida" |
| ov.tribeShort.MECHANICAL | ES | Robot | Mecán | game term |
| ov.opponents.hint | EN | Hover a hero on the game's leaderboard to see the last board you met. | Hover a hero on the game's leaderboard to see the last board you saw of theirs. | "board you met" |
| ov.opponents.hint | ES | Pasa el ratón por un héroe de la clasificación del juego para ver el último tablero que enfrentaste. | Pasa el ratón por un héroe de la clasificación del juego para ver el último tablero suyo que viste. | unclear |
| ov.record.won | ES | {value}G | {value}V | matches "Victoria" |
| ov.record.lost | ES | {value}P | {value}D | matches "Derrota" |
| ov.now.heading | ES | Rival ahora | Rival actual | natural |
| ov.now.headingTeam | ES | Equipo rival ahora | Equipo rival actual | natural |
| ov.odds.tie | ES | Empate {value} | Empatar {value} | consistent verbs |
| ov.odds.tieLabel | ES | Empate | Empatar | consistent verbs |
| ov.odds.missing | EN | Not modelled: {cards} | Not simulated: {cards} | jargon |
| ov.odds.missing | ES | Sin modelar: {cards} | Sin simular: {cards} | jargon |
| ov.odds.gap.not_modelled | EN | Odds: a card is not modelled | Odds: a card can't be simulated | jargon |
| ov.odds.gap.not_modelled | ES | Probabilidades: hay una carta sin modelar | Probabilidades: hay una carta que no se puede simular | jargon |
| ov.possible.none | EN | No pool minion matches. | No minion in the pool matches. | unclear |
| ov.possible.noTribes | ES | Tribus de la sala desconocidas: introdúcelas en la app para ver esto. No se adivina nada. | Tribus de la partida desconocidas: introdúcelas en la app para ver esto. No se adivina nada. | match "tribus de la partida" |
| ov.possible.fromLobbyAll | EN | From the lobby tribes {list} (all 5 confirmed in the tavern) | From the lobby tribes: {list} (all 5 confirmed in the tavern) | missing colon before a list |
| ov.possible.fromLobbyAll | ES | De las tribus de la sala {list} (las 5 confirmadas en la taberna) | De las tribus de la partida: {list} (las 5 confirmadas en la taberna) | missing colon, term |
| ov.possible.fromLobbyPart | EN | From the {count} lobby tribes confirmed in the tavern so far {list}; the others are missing until offered or entered | From the {count} lobby tribes confirmed in the tavern so far: {list}; the others are missing until offered or entered | missing colon before a list |
| ov.possible.fromLobbyPart | ES | De las {count} tribus de la sala confirmadas hasta ahora en la taberna {list}; faltan las demás hasta que salgan o las introduzcas | De las {count} tribus de la partida confirmadas hasta ahora en la taberna: {list}; faltan las demás hasta que salgan o las introduzcas | missing colon, term |
| ov.possible.fromSeen | EN | From the tribes seen so far {list}; other lobby tribes are missing until you enter them | From the tribes seen so far: {list}; other lobby tribes are missing until you enter them | missing colon before a list |
| ov.possible.fromSeen | ES | De las tribus vistas hasta ahora {list}; faltan las demás tribus de la sala hasta que las introduzcas | De las tribus vistas hasta ahora: {list}; faltan las demás tribus de la partida hasta que las introduzcas | missing colon, term |
| ov.possible.fromTribes | EN | From the tribes {list} | From the tribes: {list} | missing colon before a list |
| ov.possible.fromTribes | ES | De las tribus {list} | De las tribus: {list} | missing colon before a list |
| ov.detail.note | ES | Sencillo: un panel estrecho con las tribus de la sala y, de cada rival, retratos, nivel, vida y triples; ... | Sencillo: un panel estrecho con las tribus de la partida y, de cada rival, retratos, nivel, vida y triples; ... | match "tribus de la partida" |

## Not changed on purpose

- Fan notice (`win.footer.fan`, both languages) and the legend lines with the Blizzard copyright: kept exactly.
- `win.ledger.title` in English ("Ledger"): it is the product's own word and does not repeat the tab above it. Only the Spanish "Registro" clashes with "registro" meaning the log.
- Spanish rating word "puntuación": no game term was given for it, so it stays.
- Spanish keywords (Provocar, Escudo divino, Renacer, Ponzoña, Viento furioso, Sigilo, Último aliento, Alentar) and their short marks: left as the game has them; Ponzoña and Alentar should be checked against the esES client in T-531.
- Other tribe names (Aberración, Bestia, Demonio, Dragón, Elemental, Múrloc, Naga, Pirata, Jabaespín, No-muerto): fine.
- `ov.hp.unit` ("hp"): kept for space in the narrow overlay; the Spanish "vida" is fine.
- Game terms already correct: refresco/refrescar, abalorio, Don siniestro, Dúos, nivel de taberna, mejora de taberna, esbirro, dorado, triple, héroe, poder de héroe, puesto, rival, compañero, vida, oro.
- Errors using "does not say" / "the log has no ...": repeated on purpose; they keep "unknown is not zero" explicit.
- `settings.detail.*` and `win.detail.*` ("Detail", "Simple", "Advanced"): clear enough and shared with the overlay.
- Remaining Rust-side English sentences (setup lines, sign-in messages) read fine; only the ones listed above change.
- `lobby.placed` ("{n} placed"): unclear, but its exact meaning is not visible in the strings files, so no proposal; T-531 should check it in `lobby-words.js` before touching it.

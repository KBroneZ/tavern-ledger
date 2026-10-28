// Words of the game review in the recap: review-words.js and review.js
// (T-658, D-176). It measures the next fight only: no word here may call a
// difference the cost in final place. English is the source; Spanish
// (Spain) is complete (ui-tests/i18n.test.js). Each row is [key, English,
// Spanish]. Prefix: review.
"use strict";

(() => {
  const rows = [
    ["review.title", "Game review", "Revisión de la partida"],
    ["review.measures", "What it measures: the next fight only, simulated against the board your opponent had. Later turns and your final place are not measured.",
      "Qué mide: solo el siguiente combate, simulado contra el tablero que tenía tu rival. Los turnos siguientes y tu posición final no se miden."],
    ["review.start", "Review my choices", "Revisar mis decisiones"],
    ["review.startNote", "Simulates the next fight of each choice again with the other cards you were offered. Takes up to a minute, and only runs between games.",
      "Vuelve a simular el siguiente combate de cada decisión con las otras cartas que te ofrecieron. Tarda hasta un minuto y solo funciona entre partidas."],
    ["review.reviewing", "Reviewing… simulating the next fight of each choice again. This takes up to a minute.",
      "Revisando… simulando de nuevo el siguiente combate de cada decisión. Tarda hasta un minuto."],
    ["review.gameOn", "Reviews run only between games. Finish the game you are playing, then review this one.",
      "Las revisiones solo se hacen entre partidas. Acaba la partida que estás jugando y revisa esta después."],
    ["review.busy", "Another game is being reviewed. Try again when it is done.", "Ya se está revisando otra partida. Vuelve a intentarlo cuando acabe."],
    ["review.failed", "This game could not be reviewed.", "No se pudo revisar esta partida."],
    ["review.again", "Try again", "Reintentar"],
    ["review.noReview.not_solo", "Reviews are for Solo games for now.", "De momento, las revisiones son solo para partidas en Solo."],
    ["review.noReview.no_shop", "This game's record has no shop to read your choices from.", "El registro de esta partida no tiene tienda de la que leer tus decisiones."],
    ["review.outOfTime", "Time ran out partway through: the choices after that say “not reached in time”, and only the choices reviewed count below.",
      "Se acabó el tiempo a medias: las decisiones posteriores dicen «no se llegó a tiempo» y abajo solo cuentan las decisiones revisadas."],
    ["review.stoppedForGame", "A game started, so the review stopped: the choices after that say “not reached in time”, and only the choices reviewed count below.",
      "Empezó una partida, así que la revisión se detuvo: las decisiones posteriores dicen «no se llegó a tiempo» y abajo solo cuentan las decisiones revisadas."],
    ["review.count", "{rebuilt} of {total} choices simulated again.", "{rebuilt} de {total} decisiones simuladas de nuevo."],
    ["review.topTitle", "The choices that cost you the most in the next fight", "Las decisiones que más te costaron en el siguiente combate"],
    ["review.none", "No choice clearly cost you the next fight.", "Ninguna decisión te costó claramente el siguiente combate."],
    ["review.nonePartial", "No choice reviewed in time clearly cost you the next fight.", "Ninguna de las decisiones revisadas a tiempo te costó claramente el siguiente combate."],
    ["review.noneRebuilt", "No choice of this game could be simulated again; “Show every turn” says why.",
      "No se pudo volver a simular ninguna decisión de esta partida; «Ver todos los turnos» explica por qué."],
    ["review.all", "Show every turn", "Ver todos los turnos"],
    ["review.turn", "Turn {turn}", "Turno {turn}"],
    ["review.unnamed", "a card the log does not name", "una carta que el registro no nombra"],

    // What the player did, as in "Turn 5: you bought X".
    ["review.did.buy", "you bought {card}", "compraste {card}"],
    ["review.did.choose", "you picked {card}", "elegiste {card}"],
    ["review.did.sell", "you sold {card}", "vendiste {card}"],
    ["review.did.buy_spell", "you bought the spell {card}", "compraste el hechizo {card}"],
    ["review.did.roll", "you rolled", "refrescaste"],
    ["review.did.freeze", "you froze the tavern", "congelaste la taberna"],
    ["review.did.unfreeze", "you unfroze the tavern", "descongelaste la taberna"],
    ["review.did.tier_up", "you upgraded the tavern to tier {tier}", "mejoraste la taberna a nivel {tier}"],
    ["review.did.tier_upNoTier", "you upgraded the tavern", "mejoraste la taberna"],
    ["review.did.fallback", "a choice", "una decisión"],
    ["review.line", "{turn}: {did}.", "{turn}: {did}."],

    // The plain reason of a costly choice. {result}: review.result.won or .lost, whichever moved more.
    ["review.why.buy", "{turn}: you bought {taken}. Had you bought {alt} instead, {result}.",
      "{turn}: compraste {taken}. Si hubieras comprado {alt}, en la simulación {result}."],
    ["review.why.choose", "{turn}: you picked {taken}. Had you picked {alt} instead, {result}.",
      "{turn}: elegiste {taken}. Si hubieras elegido {alt}, en la simulación {result}."],
    ["review.why.sell", "{turn}: you sold {taken}. Had you kept it, {result}.",
      "{turn}: vendiste {taken}. Si te lo hubieras quedado, en la simulación {result}."],
    // A sale followed by a buy the same turn: the gold of the sale paid for it.
    ["review.why.sellBought", "{turn}: you sold {taken} and bought {bought}. Had you kept {taken} instead of buying {bought}, {result}.",
      "{turn}: vendiste {taken} y compraste {bought}. Si te hubieras quedado {taken} en vez de comprar {bought}, en la simulación {result}."],
    ["review.result.won", "you win the next fight {alt} of the time in the simulation, instead of {actual}",
      "ganas el siguiente combate el {alt} de las veces en vez del {actual}"],
    ["review.result.lost", "you lose the next fight {alt} of the time in the simulation, instead of {actual}",
      "pierdes el siguiente combate el {alt} de las veces en vez del {actual}"],
    ["review.ties", "Ties: {alt} instead of {actual}.", "Empates: {alt} en vez de {actual}."],
    ["review.place", "The expected-place model puts you at {alt} instead of {actual} going into that fight.",
      "El modelo de posición esperada te sitúa en {alt} en vez de {actual} al entrar en ese combate."],
    ["review.before", "Your board", "Tu tablero"],
    ["review.after", "With {card}", "Con {card}"],
    ["review.afterKept", "Keeping {card}", "Quedándote {card}"],
    ["review.afterKeptInstead", "Keeping {card} instead of buying {bought}", "Quedándote {card} en vez de comprar {bought}"],
    ["review.trinket", "Trinket: {alt} instead of {taken}; the board stays the same.", "Abalorio: {alt} en vez de {taken}; el tablero no cambia."],

    // What a simulated alternative takes as given (Approx).
    ["review.approx.base_stats", "{alt} is simulated at its base stats, without what {taken} gained after you took it.",
      "{alt} se simula con sus estadísticas base, sin lo que {taken} ganó después de cogerlo."],
    ["review.approx.battlecry_not_played", "Neither card's Battlecry, nor anything else done as it was played, is simulated again.",
      "No se vuelve a simular el Grito de batalla de ninguna de las dos cartas, ni nada de lo que hicieron al jugarse."],
    ["review.approx.gold_ignored", "The gold each card cost, and what was left for the rest of the turn, is not counted.",
      "No se cuenta el oro que costaba cada carta ni el que quedaba para el resto del turno."],
    ["review.stalled.one", "{n} simulated fight hit the turn limit and counts as a tie.", "{n} combate simulado llegó al límite de turnos y cuenta como empate."],
    ["review.stalled.other", "{n} simulated fights hit the turn limit and count as ties.", "{n} combates simulados llegaron al límite de turnos y cuentan como empates."],
    ["review.progress", "Reviewing… {done} of {total} choices simulated so far.", "Revisando… {done} de {total} decisiones simuladas hasta ahora."],
    ["review.approx.earlier_stats", "{taken} is kept with its stats from the fight before you sold it.",
      "{taken} se queda con sus estadísticas del combate anterior a la venta."],
    ["review.approx.start_of_combat_on_board", "A card on the boards acts at the start of combat, which is not simulated; it might have changed this.",
      "Una carta de los tableros actúa al inicio del combate, que no se simula; podría haber cambiado esto."],
    ["review.approx.gold_maybe_short", "After this sale you rolled, upgraded the tavern, bought a spell or froze the shop that turn: the gold from the sale may have paid for that, so keeping the minion may have left you short. It is not ranked among the choices that cost the most.",
      "Después de esta venta refrescaste, mejoraste la taberna, compraste un hechizo o congelaste la tienda en ese mismo turno: puede que el oro de la venta pagara eso, así que quedarte el esbirro quizá te habría dejado sin oro suficiente. No cuenta entre las decisiones que más te costaron."],
    ["review.approx.fallback", "Simulated with a simplification this version of the app does not describe.",
      "Simulado con una simplificación que esta versión de la aplicación no describe."],
    ["review.approximate", "Simulated with a known gap: {cards}.", "Simulado con un fallo conocido: {cards}."],
    ["review.unknownOffers.one", "{n} card on offer is not named in the log and was not simulated.", "{n} carta ofrecida no aparece en el registro y no se simuló."],
    ["review.unknownOffers.other", "{n} cards on offer are not named in the log and were not simulated.", "{n} cartas ofrecidas no aparecen en el registro y no se simularon."],

    // Each alternative in the full list.
    ["review.keep", "Keeping {card}", "Quedarte {card}"],
    ["review.keepInstead", "Keeping {card} instead of buying {bought}", "Quedarte {card} en vez de comprar {bought}"],
    ["review.alt.result.won", "{alt} won instead of {actual}", "{alt} de victorias en vez del {actual}"],
    ["review.alt.result.lost", "{alt} lost instead of {actual}", "{alt} de derrotas en vez del {actual}"],
    ["review.alt.better", "{card}: clearly better in the next fight, {result}.",
      "{card}: claramente mejor en el siguiente combate, {result}."],
    ["review.alt.worse", "{card}: clearly worse in the next fight, {result}.",
      "{card}: claramente peor en el siguiente combate, {result}."],
    ["review.alt.close", "{card}: about the same in the next fight ({result}).",
      "{card}: más o menos igual en el siguiente combate ({result})."],
    ["review.alt.unclear", "{card}: no clear difference after {trials} simulated fights ({result}).",
      "{card}: sin diferencia clara tras {trials} combates simulados ({result})."],
    ["review.alt.gap", "{card}: not simulated, {why}.", "{card}: no simulado, {why}."],
    ["review.gap.no_card_data", "the card data is not on this PC or lacks a card", "faltan los datos de las cartas en este PC o falta una carta"],
    ["review.gap.not_a_minion", "it is not a minion", "no es un esbirro"],
    ["review.gap.start_of_combat", "it acts at the start of combat, which is not simulated", "actúa al inicio del combate, que no se simula"],
    ["review.gap.not_modelled", "a card on the boards is not in the simulator", "una carta de los tableros no está en el simulador"],
    ["review.gap.bad_board", "the simulator refused the boards", "el simulador rechazó los tableros"],
    ["review.gap.out_of_time", "not reached in time", "no se llegó a tiempo"],
    ["review.gap.fallback", "for a reason this version of the app does not describe", "por un motivo que esta versión de la aplicación no describe"],

    // Why a choice was not simulated again (NotRebuilt).
    ["review.not.needs_shop_playout", "Not simulated: it would need shops and gold the log never showed.", "No simulada: harían falta tiendas y oro que el registro nunca mostró."],
    ["review.not.no_card_taken", "Not simulated: the log does not say which card it was.", "No simulada: el registro no dice qué carta fue."],
    ["review.not.no_alternative", "Nothing else was on offer.", "No había otra opción."],
    ["review.not.no_next_combat", "Not simulated: there was no next fight, or it was not shown.", "No simulada: no hubo siguiente combate, o no se mostró."],
    ["review.not.no_board_before", "Not simulated: your board in the fight before was not seen, so what was new on it is not known.",
      "No simulada: no se vio tu tablero en el combate anterior, así que no se sabe qué había de nuevo en él."],
    ["review.not.taken_not_on_board", "Not simulated: the card was not on your board in the next fight (in hand, sold, made golden…).",
      "No simulada: la carta no estaba en tu tablero en el siguiente combate (en la mano, vendida, dorada…)."],
    ["review.not.sold_not_on_board", "Not simulated: the minion sold was not on your board in the fight before.",
      "No simulada: el esbirro vendido no estaba en tu tablero en el combate anterior."],
    ["review.not.ambiguous", "Not simulated: with copies of the card on your board, it is not known which one it was.",
      "No simulada: con varias copias de la carta en tu tablero, no se sabe cuál era."],
    ["review.not.next_buy_not_on_board", "Not simulated: the minion you bought after this sale was not on your board in the next fight (in hand, sold, made golden…), or the log does not name it.",
      "No simulada: el esbirro que compraste después de esta venta no estaba en tu tablero en el siguiente combate (en la mano, vendido, dorado…), o el registro no lo nombra."],
    ["review.not.order_unknown", "Not simulated: the log does not show whether you bought a minion with the gold from this sale.",
      "No simulada: el registro no muestra si compraste un esbirro con el oro de esta venta."],
    ["review.not.board_full", "Not simulated: keeping it would need an eighth space on the board.", "No simulada: quedártelo necesitaría un octavo hueco en el tablero."],
    ["review.not.start_of_combat", "Not simulated: the card acts at the start of combat, which is not simulated.",
      "No simulada: la carta actúa al inicio del combate, que no se simula."],
    ["review.not.actual", "Not simulated: your own board could not be simulated ({why}).", "No simulada: no se pudo simular tu propio tablero ({why})."],
    ["review.not.out_of_time", "Not reached in time.", "No se llegó a tiempo."],
    ["review.not.fallback", "Not simulated.", "No simulada."],
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

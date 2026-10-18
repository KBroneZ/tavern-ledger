// Why a card is "approximate" (T-606): the engine gives each reason in
// English (crates/combat-sim, its card lists, gifts.rs, kit.rs and trials.rs). Spanish pages
// show them in Spanish from this table; a test fails when the engine has a
// reason the table lacks. A reason the table does not know (a newer engine)
// is shown as the engine wrote it, marked as English.
import { lang } from "../i18n.ts";

export const REASONS_ES: Readonly<Record<string, string>> = {
  "summons tokens that may have game-long buffs the log does not show":
    "invoca fichas que pueden tener mejoras de toda la partida que el registro no muestra",
  "its numbers grow during the game; the base values are used": "sus números crecen durante la partida; se usan los de base",
  "buffs the Deity, which is not on the board": "mejora a la Deidad, que no está en el tablero",
  "Blood Gems are played as +1/+1; upgrades are not in the log": "las Gemas de sangre se juegan como +1/+1; sus mejoras no están en el registro",
  "which minions it picks is not checked against the game yet": "aún no se ha comprobado con la partida a qué esbirros elige",
  "grows when another Eternal Knight dies; not played": "crece cuando muere otro Caballero eterno; no se juega",
  "the spell's values can grow during the game; the printed ones are used": "los valores del hechizo pueden crecer durante la partida; se usan los impresos",
  "counts damage from attacks only, not from effects": "solo cuenta el daño de los ataques, no el de los efectos",
  "which Deathrattle it triggers and when is not checked against the game yet":
    "aún no se ha comprobado con la partida qué Último aliento activa ni cuándo",
  "Blood Gems played on it in combat are not passed on": "las Gemas de sangre que recibe en combate no se transmiten",
  "Eternal Knights grow with the Knights that died this game, which the log does not show":
    "los Caballeros eternos crecen con los Caballeros muertos en la partida, que el registro no muestra",
  "Undead summoned after it died do not get the Attack": "los no muertos invocados después de su muerte no reciben el Ataque",
  "its bonus grows during the game; the printed one is used": "su bonificación crece durante la partida; se usa la impresa",
  "its order with other summon effects is not checked yet": "aún no se ha comprobado su orden con otros efectos de invocación",
  "its Golem's card id is the one the log shows for Jailbird Juggernaut":
    "el id de carta de su Gólem es el que el registro muestra para Jailbird Juggernaut",
  "its Golem's card id is from the log; what the card does when its Golem kills the target is not checked":
    "el id de carta de su Gólem sale del registro; no se ha comprobado qué hace la carta cuando su Gólem mata al objetivo",
  "Deathrattles it gained in earlier combats are not on the board": "los Últimos alientos que ganó en combates anteriores no están en el tablero",
  "Beetles' game-long buffs are not in the log, and Beetles summoned later miss this combat's":
    "las mejoras de toda la partida de los Escarabajos no están en el registro, y los invocados después no reciben las de este combate",
  "which neighbour it picks, and neighbours that die with it, are not checked":
    "no se ha comprobado qué vecino elige ni los vecinos que mueren a la vez",
  "neighbours that die with it are not checked": "no se han comprobado los vecinos que mueren a la vez",
  "Scribes that die in this combat do not buff it yet": "los Escribas que mueren en este combate aún no lo mejoran",
  "it improves during the game; the card's printed numbers are used": "mejora durante la partida; se usan los números impresos de la carta",
  "picks only minions that miss one of the keywords": "solo elige esbirros a los que les falta alguna de las palabras clave",
  "may come back with Reborn; card stats unknown": "puede volver con Renacer; no se conocen las estadísticas de la carta",
  "summons a minion from the hand, which is not seen: the summon is left out":
    "invoca un esbirro de la mano, que no se ve: la invocación se deja fuera",
  "on a board taken after the start of combat, the Deathrattles it copied are not on the board":
    "en un tablero tomado después del inicio del combate, los Últimos alientos que copió no están en el tablero",
  "comes back whole with its keywords; what it keeps is not checked against the logs":
    "vuelve entero con sus palabras clave; no se ha comprobado con los registros qué conserva",
  "gilded, but its golden card is not modelled: it plays the plain card":
    "dorado, pero su carta dorada no está modelada: se juega como la carta normal",
  "its Deathrattle copies the minion it destroys at the start of combat; the board is taken after that, so the copied minion is gone and not known":
    "su Último aliento copia al esbirro que destruye al inicio del combate; el tablero se toma después, así que el esbirro copiado ya no está y no se conoce",
  "its Deathrattle summons random Undead Creations and the list of them is not known":
    "su Último aliento invoca Creaciones no muertas al azar y no se conoce la lista de ellas",
  "the simulator has not reviewed this card yet": "el simulador aún no ha revisado esta carta",
  "the simulator has not reviewed this Dark Gift yet": "el simulador aún no ha revisado este Don siniestro",
  "its Start of Combat effect is not played, and this board was taken before it":
    "su efecto de Inicio del combate no se juega, y este tablero se tomó antes de él",
  "a spell it casts can trigger cards that follow spells (Torrential Ruiner, Fleeing Fugitive, Abyssal Bruiser): not played":
    "un hechizo que lanza puede activar cartas que siguen a los hechizos (Torrential Ruiner, Fleeing Fugitive, Abyssal Bruiser): no se juega",
  "what its maximum stats are is not checked: its Attack and full Health are used":
    "no se ha comprobado cuáles son sus estadísticas máximas: se usan su Ataque y su Salud completa",
  "its bonus improves with the Tavern spells cast this game; the printed one is used":
    "su bonificación mejora con los hechizos de la Taberna lanzados en la partida; se usa la impresa",
  "it picks from every Deathrattle minion, not only the minion types in this game":
    "elige entre todos los esbirros con Último aliento, no solo los de los tipos de esta partida",
  "a trinket that acts at the start of combat; trinkets are not part of the boards the simulator is given":
    "un abalorio que actúa al inicio del combate; los abalorios no forman parte de los tableros que recibe el simulador",
  "a trinket that gives your Undead Attack; trinkets are not part of the boards the simulator is given":
    "un abalorio que da Ataque a tus no muertos; los abalorios no forman parte de los tableros que recibe el simulador",
  "a spell that awakens an Old God after Aberrations die in combat; it is not a minion on the board and is not played":
    "un hechizo que despierta a un Dios Antiguo cuando mueren Aberraciones en combate; no es un esbirro del tablero y no se juega",
  "it gains stats until next turn whenever another friendly minion gains stats in combat; that is not played":
    "gana estadísticas hasta el próximo turno cuando otro esbirro aliado gana estadísticas en combate; eso no se juega",  "the order of start-of-combat effects between the two sides is not proven":
    "no está comprobado el orden de los efectos de Inicio del combate entre los dos lados",
  "the simulator has not reviewed this hero power or trinket yet": "el simulador aún no ha revisado este poder de héroe o abalorio",
  "its effect during combat is not played yet": "su efecto durante el combate aún no se juega",
  "it makes a hero power that acts in combat trigger twice": "hace que un poder de héroe que actúa en combate se active dos veces",
};

/** The reason in the page's language, and whether it had to stay in English. */
export function reasonText(reason: string): { text: string; english: boolean } {
  if (lang() !== "es") return { text: reason, english: false };
  const es = REASONS_ES[reason];
  return es ? { text: es, english: false } : { text: reason, english: true };
}

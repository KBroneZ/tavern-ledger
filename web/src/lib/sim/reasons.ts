// Why a card is "approximate" (T-606): the engine gives each reason in
// English (crates/combat-sim, its card lists and trials.rs). Spanish pages
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
};

/** The reason in the page's language, and whether it had to stay in English. */
export function reasonText(reason: string): { text: string; english: boolean } {
  if (lang() !== "es") return { text: reason, english: false };
  const es = REASONS_ES[reason];
  return es ? { text: es, english: false } : { text: reason, english: true };
}

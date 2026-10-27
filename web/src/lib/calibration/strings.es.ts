// Spanish (Spain) for lib/calibration/strings.ts (T-655): the same keys and placeholders.
import type { CalKey } from "./strings.ts";

export const ES: Record<CalKey, string> = {
  "page.kicker": "Puesto esperado",
  "page.title": "Cuánto se acerca el puesto esperado",
  "page.intro":
    "En cada turno de una partida, el puesto esperado es el puesto hacia el que iba la partida, según lo que el jugador veía en ese momento. Esta página lo comprueba con partidas que el modelo nunca vio: por ejemplo, en los turnos en los que dijo entre 3 y 4, en qué puesto acabaron de verdad esas partidas.",
  "page.date": "Comprobado el {date}",
  "page.accuracy": "Las probabilidades de combate tienen su propia comprobación:",
  "page.accuracy.link": "Precisión del simulador",

  "empty.label": "Aún no hay ningún modelo publicado",
  "empty.body":
    "El puesto esperado necesita un modelo entrenado con al menos {min} partidas por modo, y todavía no se ha publicado ninguno. Mientras tanto, ni la app ni la web muestran el puesto esperado, y esta página no tiene cifras que enseñar: mejor ninguna que unas inventadas.",
  "empty.next.title": "Qué mostrará esta página",
  "empty.next.groups":
    "Para cada modo y versión del juego, los turnos agrupados según el puesto esperado que se dio (de 1 a 2, de 2 a 3, etc.), con el puesto previsto medio, el puesto final medio de esas partidas y cuántas partidas y combates hay detrás de cada grupo. Si las dos medias se parecen, el puesto esperado es fiable.",
  "empty.next.thin":
    "Un grupo con menos de {thin} partidas muestra un guion en lugar de sus medias: son muy pocas para sacar conclusiones, y muy pocas para que no se pueda deducir el puesto de una partida concreta.",
  "empty.next.distance":
    "Al lado, a qué distancia quedó de media el puesto esperado del puesto real, junto a dos estimaciones que no saben nada del tablero.",

  "mode.solo": "Solo",
  "mode.duos": "Dúos",
  "mode.none": "Aún no hay ningún modelo de {mode} publicado.",
  "mode.summary":
    "Entrenado con {games} partidas ({rows} combates) y comprobado en {folds} partes: cada partida la predijo un modelo entrenado sin ella.",
  "mode.notReady": "Todavía no se muestra a los jugadores: se entrenó con menos de {min} partidas de este modo.",
  "mode.ready": "Los jugadores ya pueden verlo: se entrenó con al menos {min} partidas de este modo.",

  "distance.label": "Distancia media al puesto real",
  "distance.back": "En puestos; cuanto menos, mejor. Estimando solo por el turno: {perTurn}. Estimando por la frecuencia de cada puesto: {overall}.",
  "distance.more":
    "Las dos estimaciones salen de las partidas con las que se entrenó cada modelo, igual que el propio modelo, y se comprueban con las mismas partidas que no vio.",

  "build.caption": "Versión {build}",
  "build.summary": "{games} partidas, {rows} combates",
  "build.col.range": "Puesto esperado dado",
  "build.col.said": "Puesto previsto medio",
  "build.col.real": "Puesto final medio",
  "build.col.games": "Partidas",
  "build.col.rows": "Combates",
  "build.range": "De {from} a {to}",
  "build.thin": "menos de {thin} partidas",
  "build.empty": "ninguno",
  "build.note":
    "El puesto esperado de cada combate cuenta una vez. Cada grupo va desde su primer número hasta el siguiente, sin incluirlo; el último incluye el {last}. Un guion: ningún combate en ese grupo, o menos de {thin} partidas.",
  "builds.other.one":
    "Hay {count} partida más, de una versión del juego con menos de {thin} partidas o desconocida, que cuenta en el total pero no se muestra por versión.",
  "builds.other.other":
    "Hay {count} partidas más, de versiones del juego con menos de {thin} partidas o desconocidas, que cuentan en el total pero no se muestran por versión.",
  "builds.none": "Ninguna versión del juego tiene aún {thin} partidas, así que no se puede mostrar ningún grupo.",

  "about.title": "Cómo se comprueba y sus límites",
  "about.body":
    "Validación cruzada por partida: las partidas se reparten en partes, y cada parte la predice un modelo entrenado con las demás, así que los turnos de una partida nunca sirven a la vez para aprender y para comprobar. Las cifras dicen cómo funciona el entrenamiento con partidas que no ha visto; después, el modelo publicado se entrena con todas.",
  "about.private":
    "Solo se publican totales, nunca una partida, un jugador ni un héroe. Una media necesita al menos {thin} partidas detrás, y cada versión del juego se cuenta por separado, sin un total de todas las versiones al lado.",
};

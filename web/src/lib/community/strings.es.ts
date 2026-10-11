// Spanish (Spain) for lib/community/strings.ts (T-508): the same keys and placeholders.
import type { ChKey } from "./strings.ts";

export const ES: Record<ChKey, string> = {
  "page.kicker": "Comunidad",
  "page.title": "Estadísticas de héroes",
  "page.intro":
    "Cómo le va a cada héroe en las partidas de los jugadores que han decidido compartirlas. Cifras que envía la aplicación Tavern Ledger de cada jugador, sin puntuaciones y sin mostrar a ningún jugador.",
  "page.updated": "Actualizado {time}, con partidas hasta el {day}.",
  "page.updated.never": "Aún no se ha calculado.",
  "page.loading": "Cargando las estadísticas de héroes",
  "page.loaded": "Estadísticas de héroes cargadas",
  "page.error": "No se han podido cargar las estadísticas de héroes",
  "page.error.body": "Comprueba tu conexión e inténtalo de nuevo.",
  "page.retry": "Reintentar",

  "period.label": "Periodo",
  "period.30d": "30 días",
  "period.all": "Histórico",
  "period.30d.in": "en los últimos 30 días",
  "period.all.in": "desde siempre",

  "sort.label": "Ordenar por",
  "sort.games": "Partidas",
  "sort.avg": "Puesto medio",
  "sort.top": "Top 4 / Top 2",
  "sort.pick": "Porcentaje de elección",
  "sorted.games": "Ordenado por partidas",
  "sorted.avg": "Ordenado por puesto medio",
  "sorted.top": "Ordenado por porcentaje de top 4 o top 2",
  "sorted.pick": "Ordenado por porcentaje de elección",

  "col.hero": "Héroe",
  "col.games": "Partidas",
  "col.players": "Jugadores",
  "col.pick": "Elegido",
  "col.avg": "Puesto medio",
  "col.top.solo": "Top 4",
  "col.top.duos": "Top 2",
  "col.win": "1.º",
  "col.offered": "Salió",
  "col.offeredPlayers": "Jugadores a los que salió",

  "caption": "Héroes, {mode}, {period}, ordenados por {sort}",
  "sort.name.games": "partidas",
  "sort.name.avg": "puesto medio",
  "sort.name.top": "porcentaje de top 4 o top 2",
  "sort.name.pick": "porcentaje de elección",

  "count.atLeast": "{n}+",
  "pick.few": "Aún no hay suficientes elecciones de héroe",
  "rounding.note":
    "50+ significa de 50 a 59 partidas, y 5+, de 5 a 9 jugadores: las partidas se cuentan de diez en diez y los jugadores de cinco en cinco, el puesto medio va con un decimal y los porcentajes, sin decimales.",
  "simple.more": "Avanzado añade el porcentaje de victorias y cuántas veces salió cada héroe.",
  "pick.note":
    "Elegido: de las partidas con la elección de héroe registrada por la aplicación en las que salió el héroe, el porcentaje en que el jugador lo eligió. Un guion significa que esas partidas aún no llegan al mismo mínimo.",

  "empty.title": "Aún no hay suficientes partidas",
  "empty.body":
    "Un héroe aparece aquí cuando hay al menos {min} partidas con él, {period}, de al menos {players} jugadores que comparten sus partidas, y ningún jugador, ni dos, reúnen la mayoría de ellas.",
  "empty.join": "Comparte las tuyas desde la página de tu cuenta:",
  "empty.join.link": "Incluir mis partidas en las estadísticas de héroes de la comunidad",

  "unavailable.title": "Actualizándose",
  "unavailable.body":
    "Las estadísticas de héroes se actualizan una vez al día y la última actualización tiene más de un día, así que no se muestra nada hasta que la siguiente funcione. Vuelve a intentarlo más tarde.",

  "how.summary": "Cómo se cuenta",
  "how.who":
    "Solo cuentan las partidas de los jugadores que han activado «Incluir mis partidas en las estadísticas de héroes de la comunidad» en la página de su cuenta. Está desactivado para todos hasta que lo activen, y no depende de tener un perfil público: un jugador con perfil privado puede compartir, y uno con perfil público no tiene por qué hacerlo.",
  "how.min":
    "Un héroe solo aparece con al menos {min} partidas de al menos {players} jugadores distintos, sin que ninguno tenga más de la mitad de ellas ni dos tengan más de tres cuartas partes. El porcentaje de elección exige lo mismo sobre las partidas en las que salió el héroe. «Histórico» muestra un héroe solo cuando sus partidas anteriores a los últimos 30 días cumplen la misma regla por sí solas, o no hay ninguna. Por debajo de eso, la página lo indica en lugar de mostrar una cifra de unas pocas partidas.",
  "how.what":
    "Solo partidas terminadas, de los últimos 30 días o de todo el histórico, hasta el día anterior a la actualización. El puesto medio, el porcentaje de top 4 en Solo (top 2 en Dúos, por equipos) y el porcentaje de victorias se calculan sobre esas partidas; un aspecto cuenta como su héroe.",
  "how.rounding":
    "Las partidas se cuentan de diez en diez y los jugadores de cinco en cinco, el puesto medio va con un decimal y los porcentajes, sin decimales. Con estos volúmenes, cifras más finas serían sobre todo ruido, y permitirían deducir partidas concretas comparando dos días.",
  "how.source":
    "Los datos los envía cada jugador: cada partida sale de su propia aplicación, y el servidor comprueba cada informe antes de guardarlo. Una partida con un valor que ninguna partida real puede tener se deja fuera, como en las clasificaciones.",
  "how.refresh":
    "Se actualiza una vez al día con las partidas compartidas. Solo se guardan los totales: no se guarda con ellos ni se muestra aquí ningún jugador, nombre ni partida. Un jugador que deja de compartir sale en la siguiente actualización; si una actualización falla, pasadas 26 horas la página no muestra nada en lugar de cifras antiguas.",
  "how.privacy": "Política de privacidad",
  "how.privacy.lead": "Qué significa esto para tus datos:",
};

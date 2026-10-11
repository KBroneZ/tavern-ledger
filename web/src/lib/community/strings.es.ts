// Spanish (Spain) for lib/community/strings.ts (T-508): the same keys and placeholders.
import type { ChKey } from "./strings.ts";

export const ES: Record<ChKey, string> = {
  "page.kicker": "Comunidad",
  "page.title": "Estadísticas de héroes",
  "page.intro":
    "Cómo le va a cada héroe en las partidas de los jugadores que han decidido compartirlas. Cifras autodeclaradas desde la aplicación Tavern Ledger, sin puntuaciones y sin mostrar a ningún jugador.",
  "page.updated": "Actualizado {time}, con partidas hasta el {day}.",
  "page.updated.never": "Aún no se ha generado.",
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
  "sort.pick": "Tasa de elección",
  "sorted.games": "Ordenado por partidas",
  "sorted.avg": "Ordenado por puesto medio",
  "sorted.top": "Ordenado por tasa de top 4 o top 2",
  "sorted.pick": "Ordenado por tasa de elección",

  "col.hero": "Héroe",
  "col.games": "Partidas",
  "col.players": "Jugadores",
  "col.pick": "Elegido",
  "col.avg": "Puesto medio",
  "col.top.solo": "Top 4",
  "col.top.duos": "Top 2",
  "col.win": "1.º",
  "col.offered": "Ofrecido",
  "col.offeredPlayers": "Jugadores a los que se ofreció",

  "caption": "Héroes, {mode}, {period}, ordenados por {sort}",
  "sort.name.games": "partidas",
  "sort.name.avg": "puesto medio",
  "sort.name.top": "tasa de top 4 o top 2",
  "sort.name.pick": "tasa de elección",

  "count.atLeast": "{n}+",
  "pick.few": "Aún no hay suficientes elecciones de héroe",
  "rounding.note":
    "50+ significa de 50 a 59 partidas, y 5+ significa de 5 a 9 jugadores: los recuentos se muestran en decenas y de cinco en cinco, el puesto medio con un decimal y las tasas en porcentaje entero.",
  "simple.more": "Avanzado añade la tasa de victorias y cuántas veces se ofreció cada héroe.",
  "pick.note":
    "Elegido: de las partidas cuya elección de héroe registró la aplicación y en las que se ofreció el héroe, la proporción en que el jugador lo eligió. Un guion significa que esas partidas aún no llegan al mismo mínimo.",

  "empty.title": "Aún no hay suficientes partidas",
  "empty.body":
    "Un héroe aparece aquí cuando hay al menos {min} partidas con él, {period}, de al menos {players} jugadores que comparten sus partidas, y ningún jugador, ni dos, reúnen la mayoría de ellas.",
  "empty.join": "Comparte las tuyas desde la página de tu cuenta:",
  "empty.join.link": "Incluir mis partidas en las estadísticas de héroes de la comunidad",

  "unavailable.title": "En regeneración",
  "unavailable.body":
    "Las estadísticas de héroes se regeneran una vez al día y la última regeneración tiene más de un día, así que no se muestra nada hasta que la siguiente funcione. Inténtalo de nuevo más tarde.",

  "how.summary": "Cómo se cuenta",
  "how.who":
    "Solo cuentan las partidas de los jugadores que han activado «Incluir mis partidas en las estadísticas de héroes de la comunidad» en la página de su cuenta. Está desactivado para todos hasta que lo activen, y no depende de tener un perfil público: un jugador con perfil privado puede compartir, y uno con perfil público no tiene por qué hacerlo.",
  "how.min":
    "Un héroe aparece solo con al menos {min} partidas de al menos {players} jugadores distintos, sin que ninguno tenga más de la mitad de ellas ni dos tengan más de tres cuartas partes. La tasa de elección exige lo mismo sobre las partidas en las que se ofreció el héroe. «Desde siempre» muestra un héroe solo cuando sus partidas anteriores a los últimos 30 días cumplen la misma regla por sí solas, o no hay ninguna. Por debajo de eso, la página lo indica en lugar de mostrar una cifra de unas pocas partidas.",
  "how.what":
    "Solo partidas terminadas, de los últimos 30 días o desde siempre, hasta el día anterior a la regeneración. El puesto medio, la tasa de top 4 en Solo (top 2 en Dúos, por equipos) y la tasa de victorias se calculan sobre esas partidas; un aspecto cuenta como su héroe. Los héroes se identifican por id de carta hasta que los nombres de las cartas lleguen al sitio.",
  "how.rounding":
    "Los recuentos se muestran en decenas de partidas y de cinco en cinco jugadores, el puesto medio con un decimal y las tasas en porcentaje entero. Cifras más precisas serían sobre todo ruido con estos volúmenes, y permitirían deducir partidas concretas comparando dos días.",
  "how.source":
    "Autodeclarado: cada partida procede de la propia aplicación de un jugador, y el servidor comprueba cada informe antes de guardarlo. Una partida con un valor que ninguna partida real puede tener queda fuera, como en las clasificaciones.",
  "how.refresh":
    "Se regenera una vez al día a partir de las partidas compartidas. Solo se guardan los totales: no se almacena ni se muestra aquí ningún jugador, nombre ni partida. Un jugador que deja de compartir queda fuera en la siguiente regeneración; si una regeneración falla, la página no muestra nada pasadas 26 horas en lugar de cifras antiguas.",
  "how.privacy": "Política de privacidad",
  "how.privacy.lead": "Qué significa esto para tus datos:",
};

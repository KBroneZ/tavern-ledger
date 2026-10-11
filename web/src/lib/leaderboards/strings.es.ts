// Spanish (Spain) for lib/leaderboards/strings.ts (T-508): the same keys and placeholders.
import type { LbKey } from "./strings.ts";

export const ES: Record<LbKey, string> = {
  "page.kicker": "Público",
  "page.title": "Clasificaciones",
  "page.intro":
    "Jugadores que han hecho público su perfil, ordenados según las partidas que han subido desde la aplicación. Cifras autodeclaradas, sin puntuaciones.",
  "page.updated": "Actualizado {time}.",
  "page.updated.never": "Aún no se ha generado.",
  "page.loading": "Cargando las clasificaciones",
  "page.loaded": "Clasificaciones cargadas",
  "page.error": "No se han podido cargar las clasificaciones",
  "page.error.body": "Comprueba tu conexión e inténtalo de nuevo.",
  "page.retry": "Reintentar",

  "period.label": "Periodo",
  "period.7d": "7 días",
  "period.30d": "30 días",
  "period.all": "Histórico",
  "period.7d.long": "los últimos 7 días",
  "period.30d.long": "los últimos 30 días",
  "period.all.long": "desde siempre",

  "section.simple": "Clasificaciones principales",
  "section.advanced": "Economía y curiosidades",

  "col.standing": "#",
  "col.player": "Jugador",
  "col.hero": "Héroe",
  "col.games": "Partidas",

  "board.avg_place": "Puesto medio",
  "board.avg_place.col": "Puesto medio",
  "board.avg_place.how":
    "Puesto final medio en las partidas terminadas del jugador. Cuanto más bajo, mejor. En Dúos el puesto es el del equipo (del 1.º al 4.º).",
  "board.top_half_rate": "Tasa de top 4",
  "board.top_half_rate.duos": "Tasa de top 2",
  "board.top_half_rate.col": "Top 4",
  "board.top_half_rate.col.duos": "Top 2",
  "board.top_half_rate.how": "Proporción de partidas terminadas que acabaron en el top 4 (Solo) o en el top 2 (Dúos).",
  "board.win_rate": "Tasa de victorias",
  "board.win_rate.col": "1.º",
  "board.win_rate.how": "Proporción de partidas terminadas que acabaron en el 1.er puesto.",
  "board.games": "Partidas jugadas",
  "board.games.col": "Partidas",
  "board.games.how": "Partidas terminadas subidas para este modo y periodo.",
  "board.top_half_streak": "Racha de top 4 más larga",
  "board.top_half_streak.duos": "Racha de top 2 más larga",
  "board.top_half_streak.col": "Seguidas",
  "board.top_half_streak.col.duos": "Seguidas",
  "board.top_half_streak.how":
    "Más finales seguidos en el top 4 (Solo) o en el top 2 (Dúos), según el orden en que se jugaron las partidas y contando solo las partidas del periodo.",
  "board.hero_avg_place": "Mejor puesto medio por héroe",
  "board.hero_avg_place.col": "Puesto medio",
  "board.hero_avg_place.how":
    "Puesto medio con un héroe (un aspecto cuenta como su héroe), para los héroes con los que el jugador ha terminado al menos {hero} partidas en el periodo. Un jugador puede aparecer una vez por héroe.",
  "board.apm": "Acciones por minuto",
  "board.apm.col": "APM",
  "board.apm.how":
    "Acciones que el registro del juego anota como enviadas por el cliente del jugador (compras, ventas, refrescos, congelaciones, cartas y poderes de héroe jugados, esbirros movidos, elecciones) por minuto, en promedio sobre las partidas que las registraron. No se lee ninguna entrada de teclado ni de ratón.",
  "board.gold_beyond_income": "Oro más allá de los ingresos",
  "board.gold_beyond_income.col": "Por partida",
  "board.gold_beyond_income.how":
    "Oro por encima de los ingresos propios de cada turno (oro extra y oro de las ventas), por partida, sobre las partidas que lo registraron.",
  "board.rolls": "Refrescos",
  "board.rolls.col": "Por partida",
  "board.rolls.how": "Refrescos de la taberna por partida, incluidos los gratis, sobre las partidas que los registraron.",
  "board.triples": "Triples",
  "board.triples.col": "Por partida",
  "board.triples.how": "Triples por partida, tal como los cuenta la propia clasificación del juego al final de la partida.",
  "board.biggest_board": "Tablero más grande",
  "board.biggest_board.col": "Ataque + vida",
  "board.biggest_board.how":
    "El mayor total de ataque más vida del propio tablero del jugador al entrar en un combate, en una sola partida del periodo.",
  "board.most_golden": "Más esbirros dorados",
  "board.most_golden.col": "Dorados",
  "board.most_golden.how": "El mayor número de esbirros dorados en el propio tablero del jugador al entrar en un combate, en una sola partida del periodo.",

  "board.how": "Cómo se cuenta",
  "board.how.min":
    "Cuenta las partidas terminadas de {period}. Un jugador necesita al menos {min} de ellas, con este dato registrado, para aparecer.",
  "board.how.min.hero":
    "Cuenta las partidas terminadas de {period}. Un jugador necesita al menos {min} de ellas, y al menos {hero} con el héroe, para aparecer.",
  "board.how.source":
    "Autodeclarado: cada partida procede de la propia aplicación del jugador, y el servidor comprueba cada informe antes de guardarlo. Una partida con un valor que ninguna partida real puede tener queda fuera de todas las clasificaciones.",
  "board.how.ties": "Los valores iguales comparten puesto; después van primero los que tienen más partidas.",
  "board.empty": "Aún no hay nadie",
  "board.empty.body": "Ningún jugador público tiene {min} partidas terminadas en {period} para esta clasificación.",
  "board.empty.body.hero": "Ningún jugador público tiene {min} partidas terminadas en {period} con {hero} de ellas con un mismo héroe.",
  "board.caption": "{board}, {mode}, {period}",

  "about.summary": "Acerca de estas clasificaciones",
  "about.who":
    "Solo aparecen los jugadores que han activado «Perfil público» en la página de su cuenta, con el nombre visible que han elegido y un enlace a su perfil público. Si vuelven a poner el perfil en privado, desaparecen de todas las clasificaciones en cuanto guardan.",
  "about.min": "Una clasificación incluye a un jugador cuando tiene {min} partidas terminadas en el modo y el periodo ({hero} con un héroe en la clasificación de héroes). Cada clasificación muestra sus 50 primeros.",
  "about.refresh": "Las clasificaciones se regeneran cada 10 minutos a partir de las partidas subidas. Un perfil público nuevo o partidas nuevas aparecen en la siguiente regeneración.",
  "about.data":
    "Nada de esto procede de Blizzard: ni puntuaciones, ni BattleTags, ni nombres de otros jugadores. Las cifras son autodeclaradas por la aplicación de cada jugador; consulta la política de privacidad para saber qué se hace público.",
  "about.privacy": "Política de privacidad",

  "hero.unknown": "Héroe desconocido",
};

# Prototipo de parser con hslog (T-003)

Fecha: 2026-10-08. Prototipo: [`tools/parse_bg.py`](../../tools/parse_bg.py). Tests: [`tests/test_parse_bg.py`](../../tests/test_parse_bg.py) (logs sintéticos).

## Resumen

- **Se puede reconstruir una partida de Battlegrounds solo con `Power.log`.** Héroe propio y del compañero, lobby con héroes y equipos, vida por ronda, rivales de cada combate, tableros al empezar cada combate y puesto final salen del log sin lectura de memoria.
- Probado con las **23 partidas reales** del usuario (6 sesiones, build 253216): 22 de Duos (`GT_BATTLEGROUNDS_DUO`) y 1 de Solo (`GT_BATTLEGROUNDS`). 23 de 23 en estado `ok`, lobby de 8 jugadores en todas, puestos sin repetir (en Duos, 1–4 por parejas), 0 rivales sin identificar.
- Al ser eliminado, el juego **copia el héroe del jugador local** con un puesto viejo y la vida reiniciada. Hasta la sesión 005 el prototipo leía la copia: vida final 30 en las 13 partidas perdidas (12 de Duos y la de Solo) y puesto mal en 5 (ver "Eliminación del jugador local", abajo).
- **No sale del log:** el MMR y la lista exacta de tribus del lobby. Las tribus solo se pueden inferir de lo que ofrece la taberna, y la inferencia no es fiable (ver abajo).
- En Duos, **una parte de los combates del compañero no se ve** en el log local: 141 de 1145 entradas de combate (12 %). El prototipo las marca "no visible" en vez de mostrarlas vacías.
- `hslog` tiene un fallo con logs de varias partidas de Duos (`InconsistentPlayerIdError`). Se evita con un parser nuevo por partida.
- Rendimiento: 15 s para el log más grande (563 MB, 8 partidas), procesando una partida cada vez.

## Dependencias

| Paquete | Versión | Licencia | Último release | Notas |
|---------|---------|----------|----------------|-------|
| `hslog` | 1.20.0 | MIT | 2026-08-15 | Parser de `Power.log` de HearthSim. |
| `hearthstone` | 9.21.1 | MIT | 2026-09-23 | Enums (`GameTag`, `Race`, `GameType`). Ya conoce `GT_BATTLEGROUNDS_DUO` y los tags `BACON_*` de Duos; unos cuantos tags nuevos salen solo como número (p. ej. `4901`), sin efecto en el prototipo. |

Arrastran `aniso8601`, `requests`, `urllib3`, `certifi`, `idna` y `charset-normalizer` (licencias en [`PROVENANCE.md`](../../PROVENANCE.md)). El prototipo no hace peticiones de red; `requests` solo lo usa `hearthstone` para descargar la base de cartas, que aquí no se usa.

Las versiones van fijadas con hash en [`requirements.txt`](../../requirements.txt) (solo ruedas puras de Python, `--require-hashes`).

## Qué sale del log

Evidencia: partida del 2026-10-02 (`Hearthstone_2026_10_02_15_44_55/Power_old.log`, 1 partida). Las líneas son aproximadas y se refieren a ese archivo.

| Dato | ¿Sale? | Cómo | Evidencia |
|------|--------|------|-----------|
| Tipo de partida y build | Sí | `GameState.DebugPrintGame()`: `GameType=`, `BuildNumber=` | líneas 250–251 |
| Jugador local y jugador "tienda" | Sí | En `CREATE_GAME` hay dos `Player`: el local y uno ficticio (`BACON_DUMMY_PLAYER=1`, `lo=0`) que controla a Bob, la tienda y a los rivales en combate | líneas 2–97 |
| Héroe propio | Sí | Héroe del leaderboard (`CARDTYPE=HERO` con `PLAYER_LEADERBOARD_PLACE`) cuyo `PLAYER_ID` es el del jugador local | línea 871 en adelante |
| Compañero en Duos | Sí | `BACON_DUO_TEAMMATE_PLAYER_ID` en la entidad del jugador local → héroe del leaderboard con ese `PLAYER_ID` | línea 72 |
| Lobby: héroes, equipo, nivel de taberna | Sí | Los 8 héroes del leaderboard: `PLAYER_ID`, `BACON_DUO_TEAM_ID`, `PLAYER_TECH_LEVEL`. El héroe propio no lleva `BACON_DUO_TEAM_ID`; está en la entidad del jugador | líneas 75, 871 |
| Vida por ronda (propia y de cada rival) | Sí | `HEALTH + ARMOR − DAMAGE` de cada héroe del leaderboard al cerrar el combate, con mínimo 0 (el golpe final puede pasarse: `DAMAGE` 33 con `HEALTH` 30). Puede subir (curación, armadura) | 281 rondas, ninguna sin dato |
| Puesto final | Sí | `PLAYER_LEADERBOARD_PLACE` del héroe propio con la partida en `STATE=COMPLETE`. En Duos es el puesto del equipo (1–4). Si el jugador fue eliminado, hay que ignorar la copia de su héroe (abajo) | línea 910647 |
| Rondas | Sí | `TURN` de `GameEntity`: impar = taberna, par = combate; ronda = `TURN // 2`. `TURN` 0 es la elección de héroe | — |
| Rival de cada combate | Sí | En combate, el jugador ficticio cambia su `HERO_ENTITY` a una copia del héroe rival; se identifica por el `card_id` del héroe del lobby y, si no casa (fantasmas de jugadores eliminados, p. ej. `TB_BaconShop_HERO_KelThuzad`), por `BACON_CURRENT_COMBAT_PLAYER_ID` | línea 7880 |
| Quién pelea de tu lado (tú o tu compañero) | Sí | El jugador local cambia su `HERO_ENTITY` a la copia del héroe del compañero y vuelve al suyo | — |
| Tablero propio y rival al empezar el combate | Sí, salvo combates ocultos | Esbirros en `ZONE=PLAY` de cada lado en el primer bloque `BLOCK_START BlockType=ATTACK` tras el cambio de héroe: `card_id`, `ATK`, `HEALTH − DAMAGE`, `ZONE_POSITION` | línea 9044 (primer ataque; 257 en la partida) |
| Esbirro dorado | Sí | Por `card_id` (`…_G` o `TB_BaconUps_…`). **El tag `PREMIUM` no sirve**: es cosmético (aparece en esbirros de nivel 1 recién comprados) | 895 esbirros dorados vistos en las 23 partidas |
| Tribus del lobby | Solo inferidas | `CARDRACE` de los esbirros de la tienda (`IS_BACON_POOL_MINION=1`, controlados por el jugador ficticio en turno de taberna) | líneas 2573, 2580 |
| MMR | **No** | No aparece en `Power.log` (búsqueda de `MMR`, `Rating`: sin resultados). Ver T-004 | — |

### Combates que no se ven (Duos)

Antes de cada combate el juego crea en `SETASIDE` una copia de todos los participantes (héroe, baratijas, esbirros), con su `card_id` visible. Después, algunos combates del compañero no se reproducen: los esbirros rivales se ocultan (`HIDE_ENTITY`, línea 5309 en adelante) y pasan a `ZONE=HAND`, y no hay ningún `BLOCK_START BlockType=ATTACK`.

- El prototipo **solo usa tableros que entran en juego** y los toma al primer ataque. No usa las copias de `SETASIDE`, aunque tengan los `card_id`, para no mostrar algo que el jugador no ve en pantalla (regla 3 de `CLAUDE.md`).
- Un combate sin ningún ataque queda con `board = null` ("not visible in the log"), nunca como tablero vacío.
- En las 23 partidas: 1145 entradas de combate, 141 no visibles.

### Eliminación del jugador local

Sesión 005 (2026-10-09). Evidencia: partida de Solo `Hearthstone_2026_10_09_00_26_56/Power_old.log` (22 MB, 1 partida; líneas aproximadas) y las 22 de Duos. Sin nombres: entidades por número.

Secuencia cuando el jugador local muere (igual en Solo y en Duos):

1. El golpe final deja el héroe del leaderboard propio con más daño que vida: `DAMAGE=33` con `HEALTH=30`, `ARMOR=0` (línea 159936). En Duos, entre 31 y 59 de daño.
2. El juego crea **una copia del héroe** (`FULL_ENTITY` nueva, línea 159967) y le pone `PLAYER_LEADERBOARD_PLACE` con el puesto que tenía en ese momento (7), el mismo `PLAYER_ID`, `DAMAGE=33`, `COPIED_FROM_ENTITY_ID` = el héroe original y después `DAMAGE=0` (líneas 159988–160005). Queda en `SETASIDE`.
3. El original pasa a `ZONE=GRAVEYARD` y el jugador a `PLAYSTATE=LOSING` y luego `LOST` (líneas 160010–162403).
4. Los demás héroes reciben su puesto en ese momento (líneas 162417–162427) y, justo antes de `STATE=COMPLETE`, **el original recibe el puesto final**: 8 (líneas 164051–164059). La copia se queda con el viejo.

Los dos fallos de la primera partida de Solo salían de ahí: `leaderboard_heroes()` se quedaba con la última entidad por `PLAYER_ID`, que era la copia. De ahí la vida 30 (copia con `DAMAGE=0`) y el puesto 7 (viejo), repetido con el del jugador que de verdad quedó 7.º. El jugador local quedó **8.º** (el primero eliminado; los otros 7 seguían con vida > 0), no 7.º.

En Duos pasa lo mismo cuando cae el equipo local: 12 de 22 partidas tienen la copia (todas las no ganadas) y en 4 el puesto de la copia era distinto del final (2 en vez de 3 tres veces, 1 en vez de 2 una vez). Con la copia, el jugador local salía con un puesto distinto del de su compañero; con el original, los puestos salen por parejas en las 22.

Cómo distinguir la copia: `COPIED_FROM_ENTITY_ID` apunta a otro héroe del leaderboard. **No basta con que tenga `COPIED_FROM_ENTITY_ID`**: los héroes del leaderboard de los rivales también son copias (en Solo y en Duos), pero de entidades que no son héroes del leaderboard. El prototipo descarta solo las copias de otro héroe del leaderboard.

Los rivales eliminados no generan copia (0 en las 23 partidas). Su vida quedaba negativa (hasta −36); ahora sale 0.

### Tribus: por qué no basta con la tienda

Contando los esbirros ofrecidos por partida, solo en 17 de 23 partidas salen exactamente 5 tribus con 5 o más apariciones. En el resto aparecen tribus con 1–4 apariciones que no son del lobby o que podrían serlo (efectos que generan esbirros de otras tribus, esbirros de doble tribu con un solo `CARDRACE`). El prototipo da los recuentos tal cual, etiquetados como inferidos. Para el producto hace falta otra fuente (otro log, otro tag) o mostrarlo como "probable".

## Fallos y límites encontrados

| Hallazgo | Impacto | Qué hace el prototipo |
|----------|---------|-----------------------|
| `hslog` lanza `InconsistentPlayerIdError` en el 2.º juego de un log con varias partidas de Duos: guarda el estado de jugadores para todo el archivo y el mismo nombre vuelve con otro `PlayerID` | 4 de 6 logs no se podían leer | Parte el log en cada `CREATE_GAME` y usa un `LogParser` nuevo por partida. Además, una partida rota no tapa las siguientes |
| `hslog` solo trata como Battlegrounds `GT_BATTLEGROUNDS` en su heurística de nombres (`player.py`), no las variantes de Duos | Ninguno visto con un parser por partida | Nada; vigilar al subir de versión |
| Miles de avisos `Broken option nesting` | Ruido | Se silencian (nivel `ERROR`) |
| Tags nuevos sin nombre en `hearthstone` 9.21.1 | Ninguno para estos datos | Nada |
| Las excepciones de `hslog` citan la línea del log, que puede llevar BattleTags | Fuga de nombres en errores | Solo se informa el tipo de excepción |

## Garantías del prototipo

- **Nada inventado.** Si falta algo que el informe necesita (jugador local, héroe del leaderboard, tamaño de lobby razonable) o `hslog` falla, la partida sale como `unsupported` sin datos. Una partida sin `STATE=COMPLETE` sale como `incomplete` y sin puesto.
- **Build no probada** (≠ 253216) o **tipo de partida no probado** (cualquiera salvo `GT_BATTLEGROUNDS` y `GT_BATTLEGROUNDS_DUO`): se procesa, pero con aviso.
- **Privacidad:** la salida solo lleva `card_id`, números de jugador del lobby (1–8) y números. Comprobado sobre la salida JSON de las 23 partidas: ni BattleTags, ni `PlayerName`, ni `GameAccountId`.
- Solo lectura del archivo de log. Sin red.

## Qué implica para el producto

- **Historial y stats (F1):** viable ya. Héroe, compañero, puesto, vida por ronda, rivales.
- **Replays (F2):** viables con tableros por combate. En Duos, los combates ocultos del compañero se mostrarán como "no visible". Los nombres de cartas e imágenes necesitan la base de cartas (HearthstoneJSON u otra fuente con licencia compatible), pendiente.
- **Overlay (F3):** el último tablero visto de cada rival sale del log en tiempo real si se sigue el archivo mientras crece. Las tribus del lobby, no con seguridad.
- **MMR:** no está en `Power.log`; queda para T-004 (otros logs y leaderboard público).
- **Stack (P-002):** el formato del log se entiende bien y el prototipo cabe en un archivo de menos de 500 líneas. Portarla a C# o Rust es viable sin depender de `hslog`, escribiéndola desde este informe y no desde código de otros trackers. Python + `hslog` sirve para prototipos y tests.

## Pendiente

- Más partidas de Solo: solo hay una real, en la que el jugador local cae el primero. Falta ver un Solo ganado y uno con rivales eliminados antes que el jugador local (cubierto solo con logs sintéticos).
- **Puesto de los rivales que siguen vivos** cuando cae el jugador local: el log les pone su puesto en ese momento, no el final (la partida sigue sin el jugador). El prototipo lo da como `final_place` igual que el de los ya eliminados. Hay que distinguirlos (p. ej. vida > 0 al acabar) y marcar el suyo como desconocido o "puesto al salir".
- Fixtures recortados de logs propios, sin nombres de terceros, con OK del usuario.
- Fuente exacta de las tribus del lobby.
- Reconexiones a mitad de partida: si el juego escribe un `CREATE_GAME` nuevo para la misma partida, el prototipo la contaría como dos. No ha pasado en los 23 logs (el recuento coincide con `check_logs.py`), pero no está probado.
- Modo espectador: no probado.

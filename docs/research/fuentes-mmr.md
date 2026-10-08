# Fuentes de MMR (T-004)

Fecha: 2026-10-09. Logs del usuario (6 sesiones, 23 partidas, todas `GT_BATTLEGROUNDS_DUO`, build 253216; ninguna de Solo) y 7 peticiones manuales al leaderboard público. Las líneas citadas son aproximadas y se refieren a la sesión `Hearthstone_2026_10_08_00_05_26` salvo que se diga otra cosa. Ningún nombre de jugador sale de los logs ni de las respuestas a este informe.

## Resumen

- **Ningún log local trae el valor del MMR**, ni en Solo ni en Duos. Ni antes ni después de la partida.
- Sí hay una **señal**: `Net.log` registra la llegada de `NetCacheBaconRatingInfo` dos veces al iniciar sesión y, normalmente, dos veces entre 4 y 9 s después de cada fin de partida. Dice *cuándo* cambia el rating, no *cuánto*.
- Con la sección `[Net]` en `Verbose=true` **tampoco** aparece el valor: probado con una partida de Solo el 2026-10-09 (ver [Experimento: `[Net]` verbose](#experimento-net-verbose)).
- El **leaderboard público** funciona como se describió en [`viabilidad.md`](viabilidad.md): 25 filas por página, solo nombre visible (BattleTag sin número) y rating, corte en 8000. No tiene búsqueda por nombre.
- **Riesgo nuevo:** los términos de uso de las webs de Blizzard dan una licencia "personal use only" y excluyen el uso comercial y la descarga de partes del sitio salvo la caché de página. Afecta a cómo se usa el leaderboard (D-005). Queda como pendiente P-008.
- **Decisión (D-011, cierra P-007):** leaderboard como guía. Dentro, su rating; fuera, "por debajo del corte" (< 8000), nunca un número. **No inferir deltas.**
- Primera partida de Solo (2026-10-09): `GT_BATTLEGROUNDS`, misma build. Tampoco hay MMR en ningún log.

## Tabla: dato → fuente

| Dato | Fuente | Cómo | Evidencia | Limitaciones |
|------|--------|------|-----------|--------------|
| MMR actual (Solo o Duos) | **Ninguna local** | — | Búsqueda de `mmr`, `rating`, `leaderboard`, `rank`, `medal`, `elo`, `season` en todos los logs de las 6 sesiones: solo nombres de clases (abajo), ningún valor | — |
| MMR si está en el top | Leaderboard público | `rating` de la fila cuyo `accountid` coincide con el nombre local, en la región y el modo de la partida | 7 peticiones el 2026-10-09 (abajo) | Solo cubre ≥ 8000. Nombres repetidos. Hay que recorrer páginas. Términos de uso (P-008) |
| "El rating ha cambiado" | `Net.log` | Línea `OnNetCacheObjReceived SAVE --> NetCacheBaconRatingInfo` | Líneas 33 y 54 (inicio de sesión); 57–58, 62–63… (una pareja tras cada partida, 4–9 s después de `Reason: EndGameScreen` en `GameNetLogger.log`, líneas 14, 28, 77…). También tras la partida de Solo (sesión `Hearthstone_2026_10_09_00_26_56`, líneas 57–58) | Sin valor, ni con `Verbose=true`. La pareja llega igual en Solo y en Duos: no distingue el modo |
| Fin de partida | `GameNetLogger.log` | `Network.DisconnectFromGameServer() - Reason: EndGameScreen` | 8 en la sesión del 8-oct, igual que las 8 partidas de `Power.log` | `Power.log` ya da el fin de partida (`STATE=COMPLETE`) |
| Región de la cuenta | `Hearthstone.log` | Línea `Region: EU` justo después del login | Línea 48 en las 6 sesiones | Formato no documentado |
| Nombre del jugador local | `Power.log` | Nombre del `Player` local en `CREATE_GAME`. `tools/parse_bg.py` identifica al jugador local pero no saca su nombre (privacidad) | Ver [`parser-hslog.md`](parser-hslog.md) | Es un BattleTag con número: el leaderboard lo da sin número |
| Modo (Solo o Duos) | `Power.log` | `GameType=` (`GT_BATTLEGROUNDS` / `GT_BATTLEGROUNDS_DUO`) | Ver [`parser-hslog.md`](parser-hslog.md) | Solo sin probar con logs reales |
| Variación de MMR por partida | Solo si se tienen dos valores | Diferencia entre el rating antes y después | — | Fuera del leaderboard no hay valor base; dentro, la frecuencia de actualización es desconocida |

## Logs locales

### `log.config`

`%LOCALAPPDATA%\Blizzard\Hearthstone\log.config` tiene 7 secciones, todas con `LogLevel=1` y `FilePrinting=true`:

| Sección | `Verbose` | Archivo | Contenido útil para MMR |
|---------|-----------|---------|-------------------------|
| `[Power]` | true | `Power_old.log` (120–590 MB) | Nada (T-003) |
| `[Net]` | false (true desde el 2026-10-09, para el experimento) | `Net.log` (4–8 KB) | Señal `NetCacheBaconRatingInfo`, sin valor |
| `[Achievements]` | true | `Achievements_old.log` | Nada (solo `MERCENARIES_SEASON_ROLL`) |
| `[LoadingScreen]` | false | `LoadingScreen_old.log` | Nada (cambios de escena) |
| `[Decks]` | false | `Decks.log` | Nada |
| `[FullScreenFX]` | true | `FullScreenFX.log` | Nada |
| `[Arena]` | false | (no se crea: el usuario no ha jugado Arena) | — |

### Archivos de cada sesión

El juego escribe estos archivos aunque no tengan sección en `log.config`. Tamaños de la sesión del 8-oct.

| Archivo | Tamaño | Qué tiene | ¿MMR? |
|---------|--------|-----------|-------|
| `Hearthstone.log` | 5,7 MB | Log general del cliente: arranque, región, UI de la taberna (`TB_BaconShop_*`, `PlayerLeaderboardMainCardActor` es el marcador de vida de la partida, no el ranking) | No |
| `GameNetLogger.log` | 23 KB | Cola, conexión y desconexión del servidor de partida | No |
| `Net.log` | 8 KB | Objetos de `NetCache` que llegan del servidor, saldo de monedas | Solo la señal |
| `Gameplay.log`, `Spells.log`, `Asset.log` | ≤ 15 KB | Efectos visuales | No |
| `Login.log`, `LuckyDraw.log`, `Store.log`, `Downloader.log`, `ExceptionReporter.log`, `All.log`, `BattleNet.log` | ≤ 150 KB | Login, eventos, tienda, errores | No |

`options.txt` (junto a `log.config`) no tiene ninguna clave de Battlegrounds ni de rating.

### Datos sensibles en otros logs

`Net.log` incluye en la línea de conexión a Battle.net un **token de autenticación**. `Hearthstone.log` y `Power.log` llevan BattleTags. Consecuencias para el producto:

- La app nunca sube, adjunta ni copia `Net.log` o `Hearthstone.log` enteros (ni en informes de errores).
- Si lee algo de ellos, lo hace línea a línea con un patrón cerrado (p. ej. solo `Region: ([A-Z]+)`) y descarta el resto.
- Nada de estos archivos entra en fixtures sin anonimizar.

### Experimento: `[Net]` verbose

`NetCacheBaconRatingInfo` es un objeto de la sección `[Net]` y esa sección ya vuelca contenidos en otros casos (`Caching currency state: { … }` con los saldos). Se probó si con `Verbose=true` escribía también el rating.

- Cambio hecho por el usuario el 2026-10-09: línea `Verbose=false` → `Verbose=true` de `[Net]`, guardado 8 s antes de arrancar el juego. Para deshacerlo, volver a `Verbose=false`.
- Una partida de Solo (`GT_BATTLEGROUNDS`, build 253216) en la sesión `Hearthstone_2026_10_09_00_26_56`.
- Resultado: **negativo**. `Net.log` tiene el mismo tamaño (≈ 4,3 KB) y las mismas líneas que con `Verbose=false`; las cuatro de `NetCacheBaconRatingInfo` (líneas 33, 54, 57, 58) siguen sin contenido. Ningún otro log de la sesión menciona rating, MMR o leaderboard.

## Leaderboard público

### Endpoint

Lo usa la propia web (`hearthstone.blizzard.com/<locale>/community/leaderboards`). No está documentado: el script de la página arma la URL así y no hay otra API pública de leaderboards (la API oficial de desarrolladores de Hearthstone cubre cartas, mazos y metadatos; los enlaces `partner-*.api.blizzard.net` que aparecen en la respuesta son de una API de socios, no pública).

```
GET https://hearthstone.blizzard.com/en-gb/api/community/leaderboardsData?region=EU&leaderboardId=battlegrounds&page=1
```

| Parámetro | Valores | Notas |
|-----------|---------|-------|
| `region` | `EU`, `US`, `AP` | La respuesta trae el mapa `regions` (`US`=1, `EU`=2, `AP`=3) |
| `leaderboardId` | `battlegrounds` (Solo), `battlegroundsduo` (Duos) | También `standard`, `wild`, `arena`… |
| `page` | 1… | 25 filas por página. La web no lo manda si es 1 |
| `seasonId` | opcional | Sin él, temporada actual. BG Solo y Duos: temporada 19 el 2026-10-09 |

Petición de ejemplo, con User-Agent identificable y timeout (regla 4):

```
curl -sS -m 15 -A "TavernLedger-research/0.1 (+https://github.com/KBroneZ/tavern-ledger)" \
  "https://hearthstone.blizzard.com/en-gb/api/community/leaderboardsData?region=EU&leaderboardId=battlegroundsduo&page=1"
```

### Respuesta

JSON de unos 300 KB por página, casi todo metadatos de presentación (`displayMetaData`, `seasonMetaData` con las temporadas de cada modo y región). Lo útil (nombres inventados):

```json
{
  "seasonId": 19,
  "region": "EU",
  "leaderboard": {
    "columns": ["rank", "accountid", "rating"],
    "rows": [
      {"rank": 1, "accountid": "ExamplePlayer", "rating": 19118},
      {"rank": 2, "accountid": "AnotherName", "rating": 18688}
    ],
    "pagination": {"totalPages": 173, "totalSize": 4305}
  }
}
```

- `accountid` es el nombre visible: BattleTag **sin** `#número` (0 de 25 con `#` en la página 1). Puede tener caracteres no ASCII.
- No hay id estable de cuenta ni fecha de actualización. El script de la web lee un `metadata.last_updated_time` que hoy no viene en la respuesta.
- No se guarda ninguna respuesta real en el repo: los términos de la web no permiten descargar ni redistribuir partes del sitio (ver abajo). Para T-006 bastan respuestas sintéticas con esta forma.

### Cobertura (2026-10-09, temporada 19)

| Leaderboard | Filas | Páginas | Primero | Último |
|-------------|-------|---------|---------|--------|
| BG Solo EU | 4305 | 173 | 19118 | 8000 |
| BG Duos EU | 1482 | 60 | 22498 | 8000 |
| BG Solo EU, temporada 18 (cerrada) | 11450 | 458 | 19020 | — |

El corte en 8000 se confirma en Solo y Duos. La temporada 18 tiene muchas más filas; no se ha comprobado su corte.

### Comportamiento y límites

| Hallazgo | Implicación |
|----------|-------------|
| Página fuera de rango (`page=999`): HTTP 200 con `rows: []` y `totalPages: 0` | Una página vacía **no** significa "el jugador no está"; el cliente tiene que comparar con `totalPages` de la página 1 |
| Sin búsqueda por nombre | Encontrar un nombre exige recorrer páginas: hasta 173 (≈ 52 MB) en Solo EU |
| Respuestas en 0,7–1,8 s; sin cabeceras de rate limit ni `Cache-Control`; `ETag` débil | Límite real desconocido. Usar `If-None-Match` y caché propia |
| Pone cookies `session` y `locale` | El cliente no las guarda ni las reenvía |
| `robots.txt` no bloquea `/api/` | No es un permiso; los términos mandan |

### Términos

- **Términos de uso de las webs de Blizzard** (rev. 2018-09-26): licencia "for personal use only"; excluyen "any commercial use of the Site or the Materials therein" y "downloading (other than the page caching) of any portion of the Site", y transmitir los materiales a otra web. No hablan de scraping ni de peticiones automáticas. [Fuente](https://www.blizzard.com/en-us/legal/29232b30-6ae1-4d74-b1c5-8bd1df9e0b63/terms-of-use-for-blizzards-websites)
- **EULA** (2024-03-21): prohíbe procesos no autorizados que "intercepts, collects, reads, or 'mines' information" de la *Platform* (app, servicio y juegos; las webs no están en la definición). [Fuente](https://www.blizzard.com/en-us/legal/fba4d00f-c7e4-4883-b8b9-1b4500a402ea/blizzard-end-user-license-agreement)
- **Términos de la API de desarrolladores**: según foros (sin verificar en el texto oficial), prohíben scraping y consultas automáticas salvo por las APIs autorizadas. Solo obligan a quien se registra como desarrollador, pero indican la postura de Blizzard. [Foro](https://us.forums.blizzard.com/en/blizzard/t/questions-about-blizzard-developer-api-terms-of-use/57897)

Lectura (no es asesoría legal): consultar la propia fila desde la app del jugador, a petición suya y con caché, se parece a usar la web en persona. Rastrear el leaderboard entero desde un servidor, guardarlo y mostrarlo en otra web, o meterlo en un extra de pago, choca con "personal use only" y con la prohibición de redistribuir. Decisión pendiente: P-008.

## Cruce: usuario local ↔ fila del leaderboard

1. **Clave:** nombre local sin `#número` + región (`Hearthstone.log`) + modo (`GameType` de la partida) + temporada actual.
2. **Comparación exacta** de `accountid` con el nombre. Mayúsculas y normalización Unicode sin comprobar: hasta probarlo, exacta y sin normalizar.
3. **Resultado:**
   - Una fila → MMR con su `rank`, temporada y hora de consulta.
   - Ninguna fila en todas las páginas → "not in the public leaderboard (top ~8000)", sin valor.
   - Dos o más filas con el mismo nombre → "ambiguous", sin valor. No se elige por rating parecido: sería inventar.
   - Error de red, respuesta inválida o páginas incoherentes → "unknown", distinto de "no está".
4. **Coste:** si la app recuerda el último `rank` o rating conocido del usuario, puede ir directa a esa zona (las filas van por rating descendente) en 1–3 peticiones; si no, hay que recorrer desde la página 1. Con caché de al menos una hora por página, consultar tras cada partida y nunca recorrer el leaderboard entero más de una vez al día por usuario. Cifras a fijar en T-006.

## P-007: MMR fuera del leaderboard

| Opción | Qué es | A favor | En contra |
|--------|--------|---------|-----------|
| A. A mano | El usuario escribe el MMR que ve en el juego; la app lo muestra como "entered by you" con fecha | Dato real, lo ve en su pantalla (regla 3) | Se queda viejo tras cada partida; depende del usuario |
| B. Inferir deltas | Estimar la variación por puesto | Automático | Fuera del leaderboard no hay valor base, y la variación depende del MMR medio del lobby, que no está en el log. Sería un valor inventado (D-005) |
| C. No mostrarlo | "No data" | Honesto, cero mantenimiento | Peor experiencia para la mayoría (fuera del top) |
| D. `[Net]` verbose | Leer el rating si el juego lo escribe | — | **Descartada:** probado el 2026-10-09, el juego no lo escribe |

**Recomendación inicial:** C por defecto con A como opción. Descartar B y D.

**Decisión del usuario (D-011, 2026-10-09):** el leaderboard es la guía. Variante de C: en vez de "sin datos", quien no aparece se muestra como "por debajo del corte" (< 8000), porque juega poca gente y estar fuera del top ya es información. Condiciones para no inventar:

| Situación | Se muestra |
|-----------|-----------|
| Una fila con su nombre | Su rating, `rank`, temporada y hora de consulta |
| Ninguna fila, leaderboard entero revisado (todas las páginas de `totalPages`) | "Below the leaderboard cut-off (< 8000)", con el corte tomado de la última fila |
| Su nombre aparece dos o más veces | "Ambiguous" |
| Error de red, respuesta inválida o recorrido incompleto | "Unknown" |

Coste: afirmar que alguien **no** está obliga a recorrer todas las páginas (173 en Solo EU, ≈ 52 MB). Para T-006: hacerlo como mucho una vez al día por región y modo, con caché e `If-None-Match`, y depende de cómo se cierre P-008.

## Pendiente

- Volver a `Verbose=false` en `[Net]` (el experimento no aportó nada).
- P-008: uso del leaderboard frente a los términos de la web.
- Frecuencia de actualización del leaderboard (comparar dos consultas separadas por horas, en T-006).
- Mayúsculas y Unicode en `accountid` frente al nombre local.

# Sesión 004: Cliente del leaderboard (T-006)

Modelo: sonnet
Effort: medium
Subagentes: haiku

Trabaja en `C:\Users\andia\tavern-ledger` siguiendo su `CLAUDE.md`: caveman en el chat; documentos en español; README público y producto en inglés. **Tier R2** (red): TDD + `/code-review` + `/security-review`. Rama y PR; nunca push directo a `main`.

**Línea roja (D-004, D-006):** solo logs locales y el endpoint público que usa la propia web. Nada de memoria, inyección ni automatizar el juego.

## Effort y modelos por fase

| Fase | Effort | Subagentes (si hacen falta) |
|------|--------|-----------------------------|
| Lectura, D-012 y plan | low | Haiku (búsquedas) |
| Medidas previas (gzip, reparto de ratings) | medium | — |
| Tests y cliente | medium | Sonnet si hace falta delegar código |
| Reviews (`/code-review`, `/security-review`) | high | — |
| Docs y PR | low | — |

**Autonomía:** Claude decide el diseño interno, los nombres y los límites concretos dentro de lo de abajo. **El usuario aprueba** cualquier dependencia nueva y el merge del PR.

## Al empezar

1. Título **`#004 Cliente del leaderboard`**.
2. Comprueba que el PR KBroneZ/tavern-ledger#3 está mergeado y que `main` está al día. Si no, avisa.
3. Lee: `README.md`, `docs/plan/plan.md` (T-006), `docs/decisions/DECISIONS.md` (D-003, D-005, D-011, P-008) y `docs/research/fuentes-mmr.md` entero.
4. Ejecuta `python tools/check_logs.py` y anota en el plan si hay partidas nuevas.
5. Este archivo de prompt entra en el PR de esta sesión.

## Primero: registrar D-012 (decisión ya tomada)

El usuario cerró P-008 en la sesión 003, después del merge. Pásala de pendientes a tomadas, con fecha 2026-10-09, en el PR de esta sesión:

> **D-012.** El MMR del leaderboard es una función gratis (D-003). La consulta sale solo de la app de escritorio de cada usuario, para su propia fila, a petición o tras una partida, con caché y el mínimo de peticiones; sin rastreo desde un servidor, sin guardar filas de otros jugadores y nunca en extras de pago. El diseño concreto para gastar pocas peticiones lo decide Claude. Cierra P-008.
> Motivo: decisión del usuario; los términos de las webs de Blizzard dan licencia "personal use only" y excluyen el uso comercial y la descarga salvo caché de página.

## Contexto

- Endpoint, parámetros, forma de la respuesta y comportamiento: `docs/research/fuentes-mmr.md`. Ojo: una página fuera de rango devuelve **200 con `rows: []`**; no hay búsqueda por nombre; ~300 KB por página; sin cabeceras de rate limit; `ETag` débil; pone cookies que no hay que guardar.
- D-011 fija los cuatro estados de salida: rating (una fila), "below the cut-off" (ninguna fila tras revisar **todas** las páginas; corte = rating de la última fila), "ambiguous" (nombre repetido), "unknown" (error o recorrido incompleto).
- Datos locales de entrada: nombre (de `Power.log`, sin `#número`), región (`Region: XX` en `Hearthstone.log`, hacia la línea 48), modo (`GameType`). `Net.log` lleva un token de autenticación: **no se lee**. Nunca imprimir BattleTags ni `GameAccountId` en chat, docs, tests o logs de la herramienta.
- Regla 4 de `CLAUDE.md`: caché, pocas peticiones, timeout, User-Agent identificable (`TavernLedger/<versión> (+https://github.com/KBroneZ/tavern-ledger)`).
- El stack de escritorio (P-002) sigue sin decidir: el cliente es un **prototipo en Python** como `tools/parse_bg.py`, pensado para portarse. Lo valioso es el diseño, los estados y los tests.

## Tarea

1. **Medidas previas (pocas peticiones manuales, ≤ 10 en total):**
   - ¿Responde comprimido con `Accept-Encoding: gzip`? Bytes con y sin.
   - ¿Responde `304` con `If-None-Match`? ¿Cada cuánto cambia el `ETag` de una página?
   - Reparto de ratings por página cerca del corte (cuántas páginas hay entre 8000 y 9000 en Solo EU), para dimensionar la estrategia. Solo recuentos; no guardar respuestas reales.
2. **Estrategia de pocas peticiones** (decide Claude, documéntala con números). Puntos de partida:
   - Si se conoce el último `rank`/rating del usuario, pedir primero la página donde debería estar y las vecinas (las filas van por rating descendente).
   - Si el último estado era "below the cut-off", tras una partida basta mirar las páginas del final cercanas al corte (quien entra lo hace por abajo), salvo que haya pasado mucho tiempo.
   - Recorrido completo solo cuando no haya estado previo o la caché haya caducado, y como mucho una vez al día por región y modo.
   - Pausa entre peticiones, tope de peticiones por día, `timeout`, reintentos con espera y parada al primer error grave → "unknown".
3. **Cliente** `tools/leaderboard.py` (CLI de solo lectura + funciones testeables):
   - Transporte inyectable (en tests nunca hay red). Preferir la biblioteca estándar (`urllib`); `requests` ya está fijado en `requirements.txt` como dependencia transitiva, pero usarlo directo cuenta como dependencia nueva → preguntar.
   - Validación estricta de la respuesta (tipos, `rank` creciente, `rating` no creciente, `totalPages` coherente); cualquier cosa rara → "unknown", nunca un valor.
   - Caché local en `.local/` (ya en `.gitignore`) solo con lo necesario: estado propio, corte, `ETag` y hora. Nada de filas de otros jugadores.
   - Comparación de nombres exacta (D-011); documentar la duda de mayúsculas/Unicode.
   - Salida sin nombres: estado, rating, rank, corte, temporada, región, modo, hora y nº de peticiones hechas.
4. **Tests** (`tests/test_leaderboard.py`, unittest) con respuestas **sintéticas** (nombres inventados), escritos antes que el código: los cuatro estados, página vacía fuera de rango, nombre repetido, recorrido incompleto, respuesta malformada, timeout, `304`, caché caducada, topes de peticiones.
5. **Docs:** sección del cliente en `docs/research/fuentes-mmr.md` (o informe nuevo si crece), `PROVENANCE.md` si entra algo, plan (T-006), `DECISIONS.md` (D-012 y lo que surja) y README (comando de uso).

## Fuera de alcance

- Arreglar `tools/parse_bg.py` para Solo (vida final y puestos repetidos): va en otra sesión.
- Elegir stack (P-002), app, web u overlay.
- Cualquier servidor, cuenta, gasto o aceptación de términos.

## Criterios de aceptación

- [ ] D-012 registrada y P-008 fuera de pendientes.
- [ ] Medidas previas documentadas (gzip, `304`, reparto cerca del corte) con nº de peticiones usadas.
- [ ] Estrategia documentada con peticiones esperadas por caso (usuario en el top, por debajo del corte, sin estado previo).
- [ ] Cliente con los cuatro estados de D-011; nunca un número fuera del leaderboard.
- [ ] Tests sin red, en verde; `/code-review` y `/security-review` sin hallazgos CRITICAL ni HIGH abiertos.
- [ ] Ningún nombre de jugador ni respuesta real en el repo.
- [ ] Plan, decisiones y README al día en el mismo PR; CI en verde.

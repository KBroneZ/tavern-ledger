# Cliente del leaderboard (T-006)

Fecha: 2026-10-09. Prototipo en Python, [`tools/leaderboard.py`](../../tools/leaderboard.py), con tests sin red en [`tests/test_leaderboard.py`](../../tests/test_leaderboard.py). Endpoint, forma de la respuesta y términos: [`fuentes-mmr.md`](fuentes-mmr.md). Decisiones: D-011 (cuatro estados), D-012 (uso permitido) y D-013 (estrategia).

Ningún nombre de jugador sale de las medidas ni entra en el repo: los scripts de medida solo imprimieron recuentos, ratings y hashes, y no guardaron respuestas.

## Resumen

- **Gzip funciona:** una página baja de 308 KB a ~17 KB. Recorrer todo Solo EU cuesta ~3 MB, no 52 MB.
- **`If-None-Match` no sirve:** el `ETag` cambia en cada petición aunque el contenido sea el mismo. Nunca hay `304`. El cliente no lo usa.
- **Los ratings se amontonan cerca del corte:** de la página 125 a la 173 (≈ 1200 jugadores) solo hay 50 puntos de MMR. Una partida mueve a un jugador de esa zona decenas de páginas.
- **Consecuencia:** ir directo a la página conocida solo ahorra peticiones a quien está arriba. Cerca del corte, y para afirmar "por debajo del corte", hace falta el recorrido completo (174 peticiones en Solo EU), como mucho una vez al día.

## Medidas previas

10 peticiones manuales (presupuesto: ≤ 10), Solo EU, temporada 19, con el User-Agent del proyecto, timeout de 15 s y 2 s de pausa.

| # | Hora (local) | Petición | Resultado |
|---|--------------|----------|-----------|
| 1 | 00:49 | Página 1, sin gzip | 200, 308 464 bytes, `ETag` débil, sin `Cache-Control`, `Vary`, `Age` ni `Last-Modified`. `totalSize` 4310 (4305 en T-004, horas antes) |
| 2 | 00:49 | Página 1, `Accept-Encoding: gzip` | 200, `Content-Encoding: gzip`, **16 984 bytes** (18 veces menos), mismo cuerpo descomprimido. `ETag` distinto del #1 |
| 3 | 00:49 | Página 1, gzip, `If-None-Match` con el `ETag` del #1 | **200, no 304**, tercer `ETag` distinto con el mismo tamaño de cuerpo |
| 4–8 | 00:50 | Páginas 173, 150, 125, 100 y 70, gzip | Reparto de ratings (abajo). 0 nombres repetidos dentro de cada página; 0–2 nombres no ASCII por página |
| 9 | 00:53 | Página 150 otra vez | Filas idénticas (mismo hash) a las de 3 minutos antes |
| 10 | (ver "Frecuencia de actualización") | Página 150 otra vez | — |

### Reparto de ratings (Solo EU, 4310 filas, 173 páginas)

| Página | Ranks | Rating (primera → última fila) | Jugadores por punto de MMR (aprox.) |
|--------|-------|--------------------------------|-------------------------------------|
| 1 | 1–25 | 19 118 → 15 009 | < 0,01 |
| 70 | 1726–1750 | 8506 → 8493 | ≈ 2 (páginas 70–100) |
| 100 | 2476–2500 | 8157 → 8147 | ≈ 9 (páginas 100–125) |
| 125 | 3101–3125 | 8052 → 8050 | ≈ 19 (páginas 125–150) |
| 150 | 3726–3750 | 8019 → 8017 | ≈ 30 (páginas 150–173) |
| 173 | 4301–4310 | 8000 → 8000 | — (último, 10 filas) |

Entre 8000 y 9000 hay más de 120 de las 173 páginas (la página 70 ya está por debajo de 8510). Los `rank` son consecutivos también con empates, y el cliente lo exige: lo confirma la prueba real de abajo con las páginas 170–173, todas en 8000 o cerca.

### Prueba real del cliente

Aparte del presupuesto de medidas: 4 peticiones del propio cliente (01:04) con una caché sembrada a mano (último rank en la página 172, recorrido del día ya gastado) y un nombre que no es de ningún jugador. Pidió las páginas 172, 171, 173 y 170, todas pasaron la validación estricta (filas por página, `rank` consecutivos, orden de ratings, temporada y región) y, al no encontrar el nombre, devolvió el estado guardado con su hora. Sin errores ni reintentos.

### Frecuencia de actualización

La página 150 (zona densa, donde cualquier cambio se nota) devolvió las mismas filas a las 00:50 y a las 00:53: el leaderboard no cambia cada pocos segundos. Entre la consulta de T-004 y la de esta sesión, `totalSize` pasó de 4305 a 4310. Petición 10: pendiente al cerrar la sesión (ver abajo).

## Estrategia (D-013)

El cliente guarda, por región y modo, solo el estado propio: estado, rating, rank, corte, temporada, nº de páginas, hora de la consulta, hora del último recorrido completo y un hash del nombre (para saber si la caché es de otra cuenta sin guardar el nombre). Nada de filas de otros jugadores.

1. **Menos de 5 min desde el último intento:** devuelve lo guardado, 0 peticiones.
2. **Último estado "rating" con recorrido completo de hace ≤ 7 días:** pide la página del último rank y hasta dos a cada lado (máx. 5). Si el nombre aparece una vez, ese es el rating; si aparece dos veces en la misma página, "ambiguous". Que no haya otro igual en el resto lo garantiza el último recorrido completo.
3. **Si no se resuelve así** (no hay estado, era "below", "ambiguous" o "unknown", el jugador no está en la ventana o cambió la temporada): recorrido completo, **como mucho uno al día** por región y modo (día UTC). Página 1 para saber el tamaño, de la última a la 2 y otra vez la última para comprobar que el leaderboard no cambió durante el recorrido.
4. **Si el recorrido de hoy ya se gastó:** devuelve el último estado guardado **con su hora** ("as of"), o "unknown" si no hay ninguno. Nunca un valor nuevo inventado.

Límites (`Limits` en el código): 1,5 s entre peticiones, timeout de 15 s, 2 reintentos con espera de 5 y 15 s (solo red caída, timeout o 5xx), 60 peticiones dirigidas al día por región y modo, 600 páginas como máximo por recorrido. Un 403 o 429 corta en seco y pausa todas las consultas 1 hora. Cualquier otro código (incluido un `304` que nunca se pide) o una respuesta rara corta en seco con "unknown".

### Peticiones esperadas (Solo EU, 173 páginas)

| Caso | Peticiones | Tiempo aprox. |
|------|-----------|---------------|
| Sin estado previo (primera vez, caché borrada, temporada nueva) | 174 | ~7 min (1,5 s de pausa + ~1 s de respuesta) |
| En el top, tras una partida | 1 (hasta 5 si cambió de página) | 1–12 s |
| Cerca del corte (≈ 8000–8150), tras una partida | 5 y, si no está en la ventana, 174 la primera vez del día; después, el valor guardado con su hora | — |
| Por debajo del corte | 174 la primera consulta del día; después, el estado guardado con su hora | ~7 min |
| Tope diario por región y modo | 174 + 60 dirigidas (+ reintentos) | — |

Un jugador en ~8100 (≈ 10–20 jugadores por punto) se mueve ±50 ranks con solo ±3–5 puntos de MMR: la ventana de ±2 páginas casi nunca lo encuentra tras una partida. En el top (página 1: 160 puntos por puesto) la primera petición basta.

### Opciones descartadas

| Opción | Por qué no |
|--------|-----------|
| `If-None-Match` / `304` | El `ETag` cambia en cada respuesta (medida 1–3) |
| Mirar solo las últimas páginas para seguir diciendo "below" tras una partida | Quien entra lo hace por abajo, pero con 20–30 jugadores por punto, +50 de MMR son 40–60 páginas por encima del final: casi un recorrido entero, y además apoyaría "below" en un supuesto sobre cuánto se gana por partida (D-005). D-011 pide revisar todas las páginas |
| Búsqueda binaria por rating | Exige conocer el rating nuevo; estimarlo sería inferir deltas (descartado en D-011) |
| Parar el recorrido al encontrar el nombre | No se sabría si el nombre está repetido ("ambiguous") |

## Validación de cada respuesta

Todo lo raro da "unknown", nunca un valor:

- JSON objeto, `region` igual a la pedida, `seasonId` entero ≥ 1, `rows` lista, `totalPages` y `totalSize` enteros (sin aceptar `true` como 1) con `totalPages = ⌈totalSize / 25⌉`.
- 25 filas en cada página salvo la última, que tiene `totalSize − 25·(totalPages − 1)`.
- `rank` consecutivos desde `25·(página − 1) + 1`; `rating` no creciente dentro de la página y entre páginas; nombre texto no vacío de ≤ 64 caracteres.
- Página vacía dentro del rango → error. Fuera de rango (200 con `rows: []` y `totalPages: 0`) → en la búsqueda dirigida significa que el leaderboard encogió y se pasa al recorrido completo; en el recorrido completo, error.
- Durante un recorrido: misma temporada, `totalPages` y `totalSize` en todas las páginas, y la última página idéntica al principio y al final.
- Transporte: sin cookies (no se guardan ni se reenvían), sin seguir redirecciones, máximo 1 MB comprimido y 2 MB descomprimido, gzip truncado o codificación desconocida → error. Los mensajes de error no repiten texto del servidor.

## Riesgos y dudas abiertas

- **Filas que cruzan de página durante un recorrido.** Si alguien sube o baja junto al jugador entre dos peticiones, este puede salir dos veces ("ambiguous" falso) o ninguna ("below" falso). La comprobación de la última página lo detecta si el leaderboard se actualiza durante el recorrido; la medida 9 indica que no cambia en minutos. Riesgo residual bajo; repasarlo cuando se conozca la frecuencia real.
- **Mayúsculas y Unicode.** La comparación es exacta, sin normalizar (D-011). El 0–8 % de los nombres de cada página tiene caracteres no ASCII. Si el log local y la web escribieran un mismo nombre con otra normalización Unicode, el cliente diría "below" en vez del rating. Comprobar con la fila propia de un usuario que esté en el top antes de portarlo.
- **Nombre repetido entre recorridos.** Si otro jugador con el mismo nombre entra después del último recorrido completo y fuera de la ventana, la búsqueda dirigida daría el rating propio sin ver al otro. El recorrido diario lo corrige; la confianza caduca a los 7 días.
- **Un recorrido que falla gasta el del día.** A propósito: un día con la red inestable no se convierte en varios recorridos.
- **US y AP sin medir.** Si tienen más de 600 páginas, el cliente responde "unknown" (límite de cordura).

## Uso

```
python tools/leaderboard.py --name "<your BattleTag>" --mode solo --region EU
python tools/leaderboard.py --name "<your BattleTag>" --mode duos --hearthstone-log "<Logs>\Hearthstone_<date>\Hearthstone.log" --json
```

Lee la región de `Hearthstone.log` solo con el patrón cerrado `Region: XX` en las primeras 500 líneas. No lee `Net.log`. La caché va en `.local/leaderboard-cache.json` (fuera de git).

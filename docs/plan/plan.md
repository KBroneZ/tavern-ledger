# Plan

Estados: `pendiente` · `en curso` · `hecho` · `bloqueado`.

## F0 — Prueba de viabilidad (40–80 h estimadas)

Objetivo: demostrar que se puede reconstruir una partida de Battlegrounds solo con logs locales.

| ID | Tarea | Estado | Criterio de aceptación |
|----|-------|--------|------------------------|
| T-001 | Elegir nombre (P-001) y crear el repo en GitHub (con OK del usuario) | hecho | Repo remoto con README, LICENSE MIT y `.gitignore`. |
| T-002 | Activar logs (`log.config`) y recoger 10+ partidas propias como fixtures (Solo y Duos) | en curso | Fixtures en `fixtures/` sin datos de terceros que no sean públicos; anotados en `PROVENANCE.md`. |
| T-003 | Prototipo de parser con `hslog` (Python, MIT) para ver qué expone `Power.log` en BG | hecho | Informe: héroe, tribus del lobby, tableros por ronda, rivales, puesto final, Duos. Qué falta. |
| T-004 | Investigar fuentes de MMR (P-007): logs locales + leaderboard público | hecho | Tabla de qué dato sale de dónde, con evidencia. |
| T-005 | Decidir stack de escritorio (P-002) | pendiente | Decisión en `DECISIONS.md` con motivo. |
| T-006 | Cliente del leaderboard con caché y límites | hecho | Tests con respuestas sintéticas (no se guardan respuestas reales, D-012); timeout, errores y los cuatro estados de D-011 explícitos. |

**Notas T-002 (2026-10-08).** `tools/check_logs.py` confirma que `log.config` ya activa `Power` (LogLevel=1, FilePrinting, Verbose). Los logs están en `C:\Battle.net\Battle.net\Hearthstone\Logs\Hearthstone_<fecha>\`. Inventario: 6 sesiones (2 a 8 de octubre), 23 inicios de partida, todos `GT_BATTLEGROUNDS_DUO`, build 253216. Ninguna de Solo. El juego deja `Power_old.log` (120–563 MB por sesión). Falta: jugar partidas de Solo y recortar cada partida a un fixture pequeño (necesita el parser de T-003) y quitar nombres de jugadores que no sean públicos.

**Notas T-002 (2026-10-08, sesión 002).** Sin partidas de Solo nuevas. Los tests de T-003 usan logs sintéticos; los fixtures recortados de logs propios quedan pendientes (necesitan OK del usuario antes de commitearlos).

**Notas T-002 (2026-10-09, sesión 003).** Primera partida de Solo (`GT_BATTLEGROUNDS`, build 253216, sesión `Hearthstone_2026_10_09_00_26_56`). `parse_bg.py` la lee en `ok` pero con dos fallos a investigar: la vida final sale 30 aunque el jugador quedó 7.º (eliminado), y la vida sube de 12 a 30 en la última ronda; además, otro jugador del lobby sale también con puesto 7. Sin tocar el parser en esta sesión.

**Notas T-002 (2026-10-09, sesión 004).** `check_logs.py`: sin partidas nuevas (6 sesiones, 22 de Duos y 1 de Solo).

**Notas T-002 (2026-10-09, sesión 005).** `check_logs.py`: sin partidas nuevas (6 sesiones, 22 de Duos y 1 de Solo). Causa de los dos fallos de Solo: al ser eliminado, el juego copia el héroe del jugador local (`COPIED_FROM_ENTITY_ID`, puesto viejo, `DAMAGE=0`) y el parser leía la copia. El jugador quedó 8.º, no 7.º; el otro "7" era correcto. Lo mismo pasaba en las 12 partidas de Duos perdidas (vida 30; puesto mal en 4). Arreglado en `parse_bg.py` con tests sintéticos; vida mínima 0. Solo (`GT_BATTLEGROUNDS`) pasa a tipo probado. Detalle en [`parser-hslog.md`](../research/parser-hslog.md). Queda: más partidas de Solo y el puesto de los rivales que siguen vivos cuando cae el jugador (es el del momento, no el final).

**Notas T-003 (2026-10-08).** `tools/parse_bg.py` + informe [`docs/research/parser-hslog.md`](../research/parser-hslog.md). 23 de 23 partidas reales (Duos, build 253216) en `ok`. Sale todo salvo el MMR y la lista exacta de tribus (solo inferida de la tienda); el 12 % de los combates del compañero en Duos no se ve en el log local. Solo sin probar con logs reales.

**Notas T-004 (2026-10-09).** Informe [`docs/research/fuentes-mmr.md`](../research/fuentes-mmr.md). Ningún log local trae el valor del MMR; `Net.log` solo marca cuándo llega `NetCacheBaconRatingInfo` (tras cada partida). Leaderboard: endpoint, parámetros, corte en 8000 y cruce por nombre documentados. `[Net]` con `Verbose=true` probado con OK del usuario (cambio hecho por él): no expone el rating. Nuevo pendiente P-008 (términos de la web). P-007 cerrado con D-011: rating del leaderboard si aparece; si no, "por debajo del corte" (< 8000).

**Notas T-006 (2026-10-09).** `tools/leaderboard.py` + informe [`docs/research/cliente-leaderboard.md`](../research/cliente-leaderboard.md). P-008 cerrado con D-012 (solo la fila propia, desde la app del usuario); estrategia en D-013. Medidas con 10 peticiones: gzip (308 → 17 KB por página), `ETag` inútil (cambia en cada respuesta), 10–30 jugadores por punto de MMR cerca del corte. Recorrido completo de Solo EU: 174 peticiones, ~3 MB, ~7 min, como mucho uno al día. Tests sin red (respuestas sintéticas y un servidor local para el transporte). Queda: medir la frecuencia de actualización en horas y comprobar mayúsculas y Unicode con la fila de un usuario real del top.

## F1 — Historial local y web mínima

| ID | Tarea | Estado |
|----|-------|--------|
| T-101 | App de escritorio: vigilar `Power.log` y guardar partidas en local | pendiente |
| T-102 | Stats personales: héroes, puestos medios, tribus | pendiente |
| T-103 | Decidir stack web y backend (P-003) | pendiente |
| T-104 | Cuentas, subida de partidas, perfil público; privacidad y borrado (RGPD) | pendiente |
| T-105 | Firma de código (P-005) e instalador con autoactualización | pendiente |

## F2 — Replays y recaps

| ID | Tarea | Estado |
|----|-------|--------|
| T-201 | Visor de partida turno a turno en la web (tableros vistos de los rivales) | pendiente |
| T-202 | Recap tras cada partida | pendiente |
| T-203 | Récord contra cada rival del lobby | pendiente |

## F3 — Overlay mínimo

| ID | Tarea | Estado |
|----|-------|--------|
| T-301 | Overlay con tribus del lobby, último tablero visto de cada rival y récord | pendiente |

## F4 — Comunidad y extras

| ID | Tarea | Estado |
|----|-------|--------|
| T-401 | Stats agregadas de comunidad (solo con consentimiento y volumen suficiente) | pendiente |
| T-402 | Extras de pago (P-006) | pendiente |

## Fuera de alcance por ahora

- Simulador de combate (coste muy alto; solo si el proyecto sigue vivo tras F4).
- Lectura de memoria (D-004).
- Inyección o modificación del cliente, incluido el "minion dance": descartado para siempre (D-006).

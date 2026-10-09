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
| T-005 | Decidir stack de escritorio (P-002) | hecho | Decisión en `DECISIONS.md` con motivo. |
| T-006 | Cliente del leaderboard con caché y límites | hecho | Tests con respuestas sintéticas (no se guardan respuestas reales, D-012); timeout, errores y los cuatro estados de D-011 explícitos. |

**Notas T-002 (2026-10-08).** `tools/check_logs.py` confirma que `log.config` ya activa `Power` (LogLevel=1, FilePrinting, Verbose). Los logs están en `C:\Battle.net\Battle.net\Hearthstone\Logs\Hearthstone_<fecha>\`. Inventario: 6 sesiones (2 a 8 de octubre), 23 inicios de partida, todos `GT_BATTLEGROUNDS_DUO`, build 253216. Ninguna de Solo. El juego deja `Power_old.log` (120–563 MB por sesión). Falta: jugar partidas de Solo y recortar cada partida a un fixture pequeño (necesita el parser de T-003) y quitar nombres de jugadores que no sean públicos.

**Notas T-002 (2026-10-08, sesión 002).** Sin partidas de Solo nuevas. Los tests de T-003 usan logs sintéticos; los fixtures recortados de logs propios quedan pendientes (necesitan OK del usuario antes de commitearlos).

**Notas T-002 (2026-10-09, sesión 003).** Primera partida de Solo (`GT_BATTLEGROUNDS`, build 253216, sesión `Hearthstone_2026_10_09_00_26_56`). `parse_bg.py` la lee en `ok` pero con dos fallos a investigar: la vida final sale 30 aunque el jugador quedó 7.º (eliminado), y la vida sube de 12 a 30 en la última ronda; además, otro jugador del lobby sale también con puesto 7. Sin tocar el parser en esta sesión.

**Notas T-002 (2026-10-09, sesión 004).** `check_logs.py`: sin partidas nuevas (6 sesiones, 22 de Duos y 1 de Solo).

**Notas T-002 (2026-10-09, sesión 005).** `check_logs.py`: sin partidas nuevas (6 sesiones, 22 de Duos y 1 de Solo). Causa de los dos fallos de Solo: al ser eliminado, el juego copia el héroe del jugador local (`COPIED_FROM_ENTITY_ID`, puesto viejo, `DAMAGE=0`) y el parser leía la copia. El jugador quedó 8.º, no 7.º; el otro "7" era correcto. Lo mismo pasaba en las 12 partidas de Duos perdidas (vida 30; puesto mal en 4). Arreglado en `parse_bg.py` con tests sintéticos; vida mínima 0. Solo (`GT_BATTLEGROUNDS`) pasa a tipo probado. Detalle en [`parser-hslog.md`](../research/parser-hslog.md). Queda: más partidas de Solo y el puesto de los rivales que siguen vivos cuando cae el jugador (es el del momento, no el final).

**Notas T-005 (2026-10-09, sesión 006).** Informe [`stack-escritorio.md`](../research/stack-escritorio.md): Tauri 2, .NET (WPF/Avalonia) y Electron comparados en instalador, overlay, interfaz compartida con la web, autoactualización y licencias para SignPath. Recomendación: Tauri 2 (Rust + TS); plan B WPF. Prueba del overlay en `spikes/overlay-tauri/`: transparente, siempre encima y los clics pasan a la ventana de debajo (comprobado con un clic real, debug y release; `.exe` de 8,5 MB). Probado también encima de Hearthstone en pantalla completa sin bordes, sin mandar entrada al juego: el overlay queda encima y los clics van al juego. El usuario elige Tauri (D-014).

**Notas T-003 (2026-10-08).** `tools/parse_bg.py` + informe [`docs/research/parser-hslog.md`](../research/parser-hslog.md). 23 de 23 partidas reales (Duos, build 253216) en `ok`. Sale todo salvo el MMR y la lista exacta de tribus (solo inferida de la tienda); el 12 % de los combates del compañero en Duos no se ve en el log local. Solo sin probar con logs reales.

**Notas T-004 (2026-10-09).** Informe [`docs/research/fuentes-mmr.md`](../research/fuentes-mmr.md). Ningún log local trae el valor del MMR; `Net.log` solo marca cuándo llega `NetCacheBaconRatingInfo` (tras cada partida). Leaderboard: endpoint, parámetros, corte en 8000 y cruce por nombre documentados. `[Net]` con `Verbose=true` probado con OK del usuario (cambio hecho por él): no expone el rating. Nuevo pendiente P-008 (términos de la web). P-007 cerrado con D-011: rating del leaderboard si aparece; si no, "por debajo del corte" (< 8000).

**Notas T-006 (2026-10-09).** `tools/leaderboard.py` + informe [`docs/research/cliente-leaderboard.md`](../research/cliente-leaderboard.md). P-008 cerrado con D-012 (solo la fila propia, desde la app del usuario); estrategia en D-013. Medidas con 10 peticiones: gzip (308 → 17 KB por página), `ETag` inútil (cambia en cada respuesta), 10–30 jugadores por punto de MMR cerca del corte. Recorrido completo de Solo EU: 174 peticiones, ~3 MB, ~7 min, como mucho uno al día. Tests sin red (respuestas sintéticas y un servidor local para el transporte). Medida 10 (01:49): el leaderboard cambia por tandas, entre minutos y menos de una hora. Queda: comprobar mayúsculas y Unicode con la fila de un usuario real del top.

## F1 — Historial local y web mínima

**Notas T-101 (2026-10-09, sesión 006/007).** Workspace de Rust con tres crates:
- `crates/bg-parser`: port del parser de Python sin `hslog` (misma salida JSON). Paridad comprobada con 13 casos sintéticos (en tests y CI) y con las 23 partidas reales (en local, `tools/compare_parsers.py`): idénticas. Log de 590 MB: 2,6 s frente a 19,8 s en Python. Da cada partida en cuanto llega `STATE=COMPLETE`.
- `crates/tracker` + CLI `tavern-watch`: encuentra la carpeta de logs (registro o `--logs-dir`), sigue `Power.log` de la sesión más reciente (solo líneas completas; rotación a `Power_old.log` y sesiones nuevas) y guarda en `%APPDATA%\TavernLedger\games.jsonl` (D-015). `--import` lee las sesiones antiguas: 23 partidas en 8 s; ninguno de los 149 nombres de jugador de los logs aparece en el historial.
- `crates/desktop`: app Tauri (sin npm todavía) que sigue el log en segundo plano y muestra el historial con stats por modo (Solo 1–8, Duos 1–4 por equipo; top 4 / top 2). Probada con el historial real.

Revisiones de código y de seguridad: sin CRITICAL ni HIGH; los MEDIUM, arreglados (líneas de más de 1 MiB marcan la partida como `unsupported`; el seguimiento lee cada byte una vez, detecta la rotación también por los primeros bytes y no pierde la última línea; las partidas pendientes sobreviven a un error; el historial aguanta una escritura cortada y UTF-8 inválido; la app no se congela, avisa si el hilo de seguimiento cae o si falta `Power.log`; `reg.exe` con ruta fija; CSP más estricta).

**Notas T-101 (2026-10-09, sesión 008).**
- Nombres de héroe (D-017): salen del propio `Power.log`, que escribe el nombre de cada carta en las referencias `[entityName=… cardId=…]`, en el idioma del cliente del jugador. No hay dataset externo: los JSON de HearthstoneJSON son "Copyright © Blizzard Entertainment - All Rights Reserved" (el envoltorio es CC0) y `hsdata` no tiene licencia. El informe gana el campo `card_names` (solo los héroes que menciona; solo en Rust, el prototipo de Python queda congelado y la paridad lo ignora). Con las 23 partidas reales: 184 de 184 héroes del lobby con nombre, ningún nombre de jugador en la salida (comprobado contra los `PlayerName` del log sin imprimirlos) y paridad con Python intacta. El historial del usuario se reimportó con `--import` (copia previa en `games.jsonl.bak-008`): 23 de 23 partidas con nombre.
- Una sola instancia (D-016): bloqueo del sistema sobre `games.lock` junto al historial (`File::try_lock` de la biblioteca estándar, sin dependencias). Lo toman la app y `tavern-watch`; el segundo proceso lo dice (la app muestra el historial en solo lectura). Probado con dos `tavern-watch`: el segundo sale con error y, al cerrar el primero, vuelve a funcionar.
- Revisiones de código y de seguridad: sin CRITICAL ni HIGH. Arreglado el MEDIUM que compartían: el `cardId` se leía fuera del corchete de la entidad, así que un formato raro podía emparejar un BattleTag con un héroe, y una línea larga de referencias rotas costaba 15 s (cuadrático). Ahora el `cardId` solo se lee dentro del mismo corchete y con caracteres de id, se descartan nombres con forma de BattleTag, solo se guardan ids de héroe y el escaneo está acotado (milisegundos). `rust-version = "1.89"` por `File::try_lock`.
- Prueba en vivo: pendiente; el usuario no ha jugado con la app abierta desde la sesión 007.
- Bandeja y arranque con Windows (D-018, con OK del usuario a las dependencias): icono en la bandeja con "Open Tavern Ledger", "Start with Windows" y "Quit"; cerrar la ventana la esconde y la app sigue leyendo el log. El arranque con Windows está desactivado por defecto y solo se activa desde ese menú; la entrada de inicio abre la app con `--minimized`, directamente en la bandeja. Revisión: sin CRITICAL ni HIGH; arreglados los MEDIUM (si falla el cambio, la ventana se abre con el aviso; con espacios en la ruta la opción se desactiva) y el parpadeo al arrancar minimizada (la ventana nace oculta). Para T-105: el instalador debe usar una carpeta sin espacios o escribir la entrada de inicio con comillas. Comprobado: la app arranca en release y no crea entrada en `HKCU\...\Run` hasta que el usuario la activa.

Queda para cerrar T-101: la prueba en vivo (jugar una partida con `cargo run --release -p desktop` abierto y ver que aparece sola al terminar). La build firmada es T-105. La CI no compila `desktop` (necesita librerías del sistema en Linux).

| ID | Tarea | Estado |
|----|-------|--------|
| T-101 | App de escritorio: vigilar `Power.log` y guardar partidas en local | en curso |
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
| T-302 | Overlay personalizable: botón de bloquear/desbloquear para mover los paneles donde cada uno quiera, elegir qué paneles se ven y varios temas | pendiente |

## F4 — Comunidad y extras

| ID | Tarea | Estado |
|----|-------|--------|
| T-401 | Stats agregadas de comunidad (solo con consentimiento y volumen suficiente) | pendiente |
| T-402 | Extras de pago (P-006) | pendiente |

## Fuera de alcance por ahora

- Simulador de combate (coste muy alto; solo si el proyecto sigue vivo tras F4).
- Lectura de memoria (D-004).
- Inyección o modificación del cliente, incluido el "minion dance": descartado para siempre (D-006).

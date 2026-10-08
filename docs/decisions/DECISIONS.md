# Decisiones

## Tomadas

| ID | Fecha | Decisión | Motivo |
|----|-------|----------|--------|
| D-001 | 2026-10-08 | Repo propio, separado de StartAICareer. | No mezclar con la CI, los PR ni los minutos de Actions del HQ; es un proyecto personal. |
| D-002 | 2026-10-08 | La app de escritorio es open source (MIT). El premium vive en el servidor. | Confianza de los usuarios y firma de código gratis o barata (SignPath Foundation, Certum Open Source). |
| D-003 | 2026-10-08 | Modelo: todo lo personal gratis (historial, stats por héroe, MMR, rivales, replays con límite). Extras de pago opcionales. Sin anuncios. | Objetivo de proyecto personal, no de lucro; diferenciarse de Firestone Premium y Tier7. |
| D-004 | 2026-10-08 | MVP solo con logs locales: sin lectura de memoria, sin inyección, sin automatizar. | EULA de Blizzard y riesgo de ban para los usuarios. Ver `docs/research/viabilidad.md`. |
| D-005 | 2026-10-08 | MMR desde el leaderboard público de Blizzard; fuera del leaderboard, "sin datos" (nunca un valor inventado). | El leaderboard solo cubre el top (~8000 de MMR, unas 4300 cuentas en EU); ver research. |
| D-006 | 2026-10-08 | **Nada de inyección ni de modificar el cliente, nunca** (BepInEx, DLLs, parches de archivos del juego). El "minion dance" de Nomi queda descartado. Cierra P-004. | Decisión del usuario: nada que vaya contra las normas de Blizzard. |
| D-007 | 2026-10-08 | Nombre: **Tavern Ledger** (repo `tavern-ledger`). Cierra P-001. | Elección del usuario. Sin tracker ni marca evidente con ese nombre; GitHub y dominios `.gg`/`.app` libres. Ver `docs/research/nombres.md`. |
| D-008 | 2026-10-08 | Repo público en GitHub bajo la cuenta KBroneZ. Cambios a `main` solo por PR con la CI en verde. | Elección del usuario; Actions gratis en repos públicos. Aun así, la CI va en un solo job, con `concurrency` (cancela runs viejos) y timeout de 5 min, para no gastar minutos si el repo pasara a privado. |
| D-009 | 2026-10-08 | Dependencias Python fijadas con versión y hash en `requirements.txt` (solo ruedas puras, `pip --require-hashes`). Primeras: `hslog` 1.20.0 y `hearthstone` 9.21.1 (MIT). | Aprobado por el usuario en la sesión 002. Builds reproducibles y protección frente a paquetes alterados; origen y licencia en `PROVENANCE.md`. |
| D-010 | 2026-10-08 | El parser lee cada partida con un parser nuevo y solo muestra tableros que entran en juego; los combates que el log no reproduce salen como "no visible". | `hslog` falla con varias partidas de Duos en un log; las copias en `SETASIDE` de los combates ocultos no se ven en pantalla (regla 3). Ver `docs/research/parser-hslog.md`. |

## Pendientes

| ID | Tema | Opciones | Recomendación |
|----|------|----------|---------------|
| P-002 | Stack de la app de escritorio | C#/.NET (WPF o Avalonia) · Rust + Tauri · Electron | Decidir en F0 (T-005). El prototipo (T-003) muestra que la lógica sobre `Power.log` es pequeña y se puede reescribir en cualquier lenguaje a partir de `docs/research/parser-hslog.md`. Firestone no tiene licencia: no se usa como base. Tauri da instaladores pequeños. |
| P-003 | Stack web y backend | p. ej. Astro/Next.js + Supabase/Postgres + almacenamiento S3/R2 | Decidir en F1, con costes reales de los planes gratuitos. |
| P-005 | Firma de código | SignPath Foundation (gratis, open source, build automatizada) · Certum Open Source in the Cloud (desde 49 €) · Certum Standard (desde 139–209 €) | SignPath primero; Certum OS como plan B. Azure Artifact Signing no admite particulares en España. |
| P-006 | Cómo cobrar los extras | Polar · Ko-fi/Patreon (supporters) | Decidir en F4; Polar ya se usa en StartAICareer. |
| P-007 | MMR fuera del leaderboard | Introducirlo a mano · inferir deltas · no mostrarlo | Ningún log local trae el valor (T-004), ni con `[Net]` en `Verbose=true` (probado). "Sin datos" por defecto, con opción de escribirlo a mano (etiquetado y con fecha). Descartar inferir deltas: no hay valor base ni MMR del lobby, sería inventar. Ver `docs/research/fuentes-mmr.md`. |
| P-008 | Uso del leaderboard frente a los términos de la web de Blizzard ("personal use only", sin uso comercial ni descarga salvo caché de página) | Consultar solo la fila propia desde la app del usuario, con caché · rastrear desde un servidor y mostrarlo en la web · no usarlo | Solo desde la app del usuario, a petición, con caché, sin guardar filas de otros jugadores y nunca en extras de pago. Afecta a D-005 y T-006. Ver `docs/research/fuentes-mmr.md`. |

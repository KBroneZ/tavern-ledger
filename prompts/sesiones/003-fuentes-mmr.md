# Sesión 003: Fuentes de MMR

Modelo: sonnet
Effort: medium
Subagentes: haiku

Trabaja en `C:\Users\andia\tavern-ledger` siguiendo su `CLAUDE.md`: caveman en el chat; documentos en español; README público y producto en inglés. **Tier R2 si toca red** (consultas al leaderboard público: `/code-review` + `/security-review`); R0 si solo hay research y docs. Rama y PR; nunca push directo a `main`.

**Línea roja (D-004, D-006):** solo lectura de archivos de log locales. Nada de lectura de memoria, inyección, BepInEx, DLLs, modificar archivos del juego ni automatizar acciones. Si una fuente de MMR lo necesitara, queda descartada y se anota.

## Effort y modelos por fase

| Fase | Effort | Subagentes (si hacen falta) |
|------|--------|-----------------------------|
| Lectura y plan | low | Haiku (búsquedas) |
| Logs locales: inventario de archivos y secciones de `log.config` | medium | Haiku para barrer archivos |
| Leaderboard público: endpoint, datos, límites | medium | Haiku para leer docs; ChatGPT (`ask-chatgpt`) como segunda opinión si hay dudas |
| Informe y docs | low | — |

**Autonomía:** Claude decide cómo buscar y la estructura del informe. **El usuario aprueba** cualquier cambio en `log.config` (aunque solo active secciones de log), cualquier dependencia nueva y el merge del PR.

## Al empezar

1. Título **`#003 Fuentes de MMR`**.
2. Comprueba que el PR KBroneZ/tavern-ledger#2 está mergeado y que `main` está al día. Si no, avisa.
3. Lee: `README.md`, `docs/plan/plan.md` (T-002, T-004, T-006), `docs/decisions/DECISIONS.md` (D-005, P-007), `docs/research/viabilidad.md` y `docs/research/parser-hslog.md`.
4. Ejecuta `python tools/check_logs.py` (puede haber partidas de Solo nuevas: el usuario iba a jugar alguna).
5. Este archivo de prompt entra en el PR de esta sesión.

## Contexto

- `Power.log` **no** expone el MMR (T-003: búsqueda de `MMR`/`Rating` sin resultados). Ver `docs/research/parser-hslog.md`.
- D-005: el MMR viene del leaderboard público de Blizzard; fuera del leaderboard, "sin datos", nunca un valor inventado. El leaderboard solo cubre el top (~8000 de MMR).
- Logs en `C:\Battle.net\Battle.net\Hearthstone\Logs\Hearthstone_<fecha>\`; `log.config` en `%LOCALAPPDATA%\Blizzard\Hearthstone\log.config`.
- Los logs pueden llevar BattleTags e ids de cuenta (`GameAccountId`): **nunca** volcarlos al chat ni a docs. Al hacer `grep`, imprimir solo números de línea, nombres de tag o recuentos.
- Regla 4 de `CLAUDE.md`: el leaderboard se consulta con caché, pocas peticiones, timeout y User-Agent identificable; solo endpoints públicos que la propia web usa.

## Tarea

1. **Logs locales:** inventario de los archivos de cada carpeta de sesión y de las secciones de `log.config`. Buscar rating/MMR y su variación (antes/después de partida) en Solo y Duos. Si alguna sección sin activar podría exponerlo, proponer al usuario activarla (con el texto exacto del cambio y cómo deshacerlo) y no tocar nada sin su OK.
2. **Leaderboard público:** identificar el endpoint que usa la web oficial, sus parámetros (región, modo Solo/Duos, temporada, página), el formato de respuesta, qué identifica a un jugador (nombre visible, sin BattleTag completo), cuántas filas cubre y qué límites o términos aplican. Como mucho unas pocas peticiones manuales con User-Agent identificable; guardar una respuesta de ejemplo solo si no lleva datos de jugadores fuera de lo público, y anotarla en `PROVENANCE.md`.
3. **Cruce:** cómo emparejar al usuario local con su fila del leaderboard (nombre, región, modo) sin inventar, y qué pasa si hay nombres repetidos o el jugador no aparece.
4. **Informe:** `docs/research/fuentes-mmr.md` con una tabla dato → fuente → cómo → evidencia → limitaciones, y una propuesta para cerrar P-007 (opciones: introducirlo a mano, inferir deltas, no mostrarlo) con recomendación.
5. Actualizar plan (T-004 hecho o en curso; T-002 si hay partidas de Solo), `DECISIONS.md` (cerrar P-007 solo si el usuario decide) y README, en el mismo PR.

## Fuera de alcance

- Cliente del leaderboard con caché y tests (T-006).
- Cambios en `tools/parse_bg.py` salvo que aparezca un dato de MMR en un log que ya lee (entonces, con test).
- Elegir stack (P-002), app, web u overlay.
- Cualquier gasto, cuenta nueva o aceptación de términos.

## Criterios de aceptación

- [ ] Inventario de logs locales con evidencia (archivo, sección, línea aproximada), sin nombres de jugadores.
- [ ] Endpoint del leaderboard documentado con ejemplo de petición y límites conocidos.
- [ ] Informe `docs/research/fuentes-mmr.md` con tabla y recomendación para P-007.
- [ ] Ningún cambio en `log.config` sin OK explícito del usuario.
- [ ] Plan, decisiones y README al día en el mismo PR; CI en verde.

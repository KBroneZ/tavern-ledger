# Sesión 005: Parser de Solo (T-002)

Modelo: sonnet
Effort: medium
Subagentes: haiku

Trabaja en `C:\Users\andia\tavern-ledger` siguiendo su `CLAUDE.md`: caveman en el chat; documentos en español; README público y producto en inglés. **Tier R1** (código normal): TDD + `/code-review`. Rama y PR; nunca push directo a `main`. El usuario quiere que avances solo: decide lo que puedas, pregunta solo lo que sea suyo (dependencias nuevas, commitear logs reales, merge).

**Línea roja (D-004, D-006):** solo logs locales. Nada de memoria, inyección ni automatizar el juego. Nunca imprimir BattleTags ni `GameAccountId` en chat, docs, tests o salida de la herramienta.

## Effort y modelos por fase

| Fase | Effort | Subagentes (si hacen falta) |
|------|--------|-----------------------------|
| Lectura y diagnóstico | medium | Haiku (búsquedas en logs grandes) |
| Tests y arreglo | medium | Sonnet si hace falta delegar código |
| Review y docs | medium | — |

## Al empezar

1. Título **`#005 Parser de Solo`**.
2. Comprueba que `main` está al día y que no hay PR abiertos de la sesión 004 sin mergear (el de la medida 10 puede seguir abierto; no lo toques).
3. Lee: `README.md`, `docs/plan/plan.md` (T-002, notas de la sesión 003), `docs/research/parser-hslog.md`, `tools/parse_bg.py` y `tests/test_parse_bg.py` + `tests/bg_log_builder.py`.
4. Ejecuta `python tools/check_logs.py` y anota en el plan si hay partidas nuevas (sobre todo de Solo, `GT_BATTLEGROUNDS`).
5. Este archivo de prompt entra en el PR de esta sesión si aún no está en `main`.

## Contexto

La primera partida de Solo (sesión `Hearthstone_2026_10_09_00_26_56`, build 253216) sale `ok` en `tools/parse_bg.py` pero con dos fallos (plan, notas T-002 del 2026-10-09):

1. **Vida final 30** aunque el jugador quedó 7.º (eliminado), y la vida sube de 12 a 30 en la última ronda. Sospecha: al morir en Solo el héroe del leaderboard cambia o se reinicia (fantasma, `HERO_ENTITY` nuevo, `DAMAGE` a 0…) y `hero_health()` / `rounds_of()` leen la entidad equivocada al final. En Duos no pasa porque el compañero sigue vivo.
2. **Otro jugador del lobby también con puesto 7.** Sospecha: `PLAYER_LEADERBOARD_PLACE` se lee al final de la partida, cuando ya no es el puesto final de los eliminados, o se lee de un héroe fantasma.

## Tarea

1. **Diagnóstico** con el log real (solo lectura, sin imprimir nombres): qué tags cambian en el héroe local y en los de los eliminados al morir en Solo (`HEALTH`, `DAMAGE`, `HERO_ENTITY`, `PLAYER_LEADERBOARD_PLACE`, zona…). Usa `hslog` o búsquedas acotadas; el log tiene 22 MB. Anota en `docs/research/parser-hslog.md` lo que encuentres, con número de línea aproximado y sin nombres.
2. **Tests primero** con logs sintéticos (`tests/bg_log_builder.py`): una partida de Solo donde el jugador local muere en la ronda N con la secuencia de tags real, y otra con dos eliminados en rondas distintas. Deben fallar con el código actual.
3. **Arreglo mínimo** en `tools/parse_bg.py`: vida final = la del momento de la eliminación (o 0 si el juego lo marca así; decide con evidencia), puestos sin repetir. Si un dato no se puede saber con certeza, sale como desconocido, nunca un valor inventado.
4. Comprobar que las 22 partidas de Duos siguen en `ok` con los mismos resultados (comparar salida `--json` antes y después, sin guardarla en el repo).
5. `GT_BATTLEGROUNDS` pasa a `TESTED_GAME_TYPES` solo si la partida real queda bien.

## Fuera de alcance

- Fixtures recortados de logs reales (necesitan OK del usuario antes de commitearlos).
- Elegir stack (P-002), app, web u overlay.
- El cliente del leaderboard (T-006, hecho).

## Criterios de aceptación

- [ ] Causa de los dos fallos documentada con evidencia del log (sin nombres).
- [ ] Tests sintéticos nuevos que fallan antes y pasan después; suite entera en verde.
- [ ] La partida real de Solo da vida final y puesto coherentes; las 22 de Duos, sin cambios.
- [ ] `/code-review` sin CRITICAL ni HIGH abiertos.
- [ ] Plan, `parser-hslog.md` y README al día en el mismo PR; CI en verde.

# Sesión 002: Prototipo de parser con hslog

Modelo: sonnet
Effort: medium
Subagentes: haiku

Trabaja en `C:\Users\andia\tavern-ledger` siguiendo su `CLAUDE.md`: caveman en el chat; documentos en español; README público y producto en inglés. **Tier R2** (entra la primera dependencia externa: `/code-review` + `/security-review`). Rama y PR; nunca push directo a `main`.

**Línea roja (D-004, D-006):** solo lectura de archivos de log locales. Nada de inyección, BepInEx, DLLs, lectura de memoria, modificar archivos del juego ni automatizar acciones. Si una tarea parece necesitarlo, se para y se avisa.

## Effort y modelos por fase

| Fase | Effort | Subagentes (si hacen falta) |
|------|--------|-----------------------------|
| Lectura y plan | low | Haiku (búsquedas) |
| Research de `hslog` y `hearthstone` (versión, licencia, soporte de BG y Duos) | medium | Haiku para leer docs y código MIT de HearthSim |
| Implementación del prototipo | medium | Sonnet |
| Review | medium | — (`/code-review` + `/security-review`) |
| Informe y docs | low | — |

**Autonomía:** Claude decide la estructura del prototipo y del informe. **El usuario aprueba** añadir dependencias (con versión fijada y fila en `PROVENANCE.md`) y el merge del PR.

## Al empezar

1. Título **`#002 Prototipo parser hslog`**.
2. Comprueba que el PR KBroneZ/tavern-ledger#1 está mergeado y que `main` está al día. Si no, avisa.
3. Lee: `README.md`, `docs/plan/plan.md` (T-002 y T-003, con sus notas), `docs/decisions/DECISIONS.md`, `docs/research/viabilidad.md` y `PROVENANCE.md`.
4. Ejecuta `python tools/check_logs.py` para ver qué logs hay (puede haber partidas de Solo nuevas).

## Contexto

- Logs en `C:\Battle.net\Battle.net\Hearthstone\Logs\Hearthstone_<fecha>\Power_old.log` (120–563 MB por sesión). A 2026-10-08: 23 partidas, todas `GT_BATTLEGROUNDS_DUO`, build 253216; ninguna de Solo.
- Los logs **no se copian al repo** en esta sesión. El prototipo los lee desde su ruta local y su salida de ejemplo no debe incluir nombres de otros jugadores (BattleTags) salvo que sean públicos.
- `python-hslog` y `python-hearthstone` (HearthSim) son MIT. HDT, Firestone, sus simuladores, HearthMirror, BobsBuddy y Nomi's Kitchen **no se leen ni se portan** (ver `CLAUDE.md`).

## Tarea

1. **Dependencias:** comprobar en PyPI y GitHub la última versión de `hslog` y `hearthstone`, su licencia y si mantienen el ritmo de parches (fecha del último release, enums de BG/Duos). Proponer al usuario añadirlas con versión fijada (`requirements.txt` o `pyproject.toml`) y anotarlas en `PROVENANCE.md`. Si `hslog` no soporta el build actual, decirlo con evidencia y proponer alternativa antes de seguir.
2. **Prototipo (T-003):** script en `tools/` que, para cada partida de un `Power*.log`, intente extraer: héroe propio (y del compañero en Duos), tribus del lobby, tablero propio y de cada rival por ronda, rivales enfrentados, vida y puesto final. Lo que no salga del log se marca "no disponible", nunca se inventa. Si el parser falla con un build nuevo, el script dice "versión no soportada" en vez de dar datos falsos.
3. **Tests:** con un fragmento sintético o recortado de log propio, sin BattleTags de terceros. Si se recorta un log real para fixture, anotarlo en `PROVENANCE.md` (origen: logs propios del usuario) y pedir OK antes de commitearlo.
4. **Informe:** `docs/research/parser-hslog.md` con una tabla de dato → ¿sale del log? → cómo (tag, entidad, evento) → evidencia (partida y línea aproximada), más lo que falta y qué implica para el producto (overlay, replays, MMR).
5. **CI:** si se añaden dependencias, instalar las versiones fijadas en el job existente sin crear jobs nuevos (cuidado con los minutos de Actions).
6. Actualizar el plan (T-003 hecho o en curso; T-002 según haya fixtures), `DECISIONS.md` si hay decisiones nuevas y el README, en el mismo PR.

## Fuera de alcance

- MMR y leaderboard (T-004, T-006).
- Elegir stack (P-002), aunque el informe puede dar datos para decidirlo.
- App de escritorio, web, overlay.
- Cualquier gasto, dominio o firma de código.

## Criterios de aceptación

- [ ] Dependencias con versión fijada, licencia verificada y fila en `PROVENANCE.md`, aprobadas por el usuario.
- [ ] Prototipo ejecutado contra al menos 3 partidas reales, con su salida (sin BattleTags de terceros) como evidencia.
- [ ] Informe `docs/research/parser-hslog.md` con lo que sale, lo que no y la evidencia.
- [ ] Tests en verde en local y en la CI (enlace al run).
- [ ] Plan, decisiones y README al día en el mismo PR.

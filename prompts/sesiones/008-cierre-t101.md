# Sesión 008: Cierre de T-101 (app de escritorio)

Modelo: sonnet
Effort: medium
Subagentes: haiku para búsquedas; sonnet para código

Trabaja en `C:\Users\andia\tavern-ledger` siguiendo su `CLAUDE.md`: caveman en el chat; documentos en español; README público, producto y UI en inglés. **Tier R2** (dependencias nuevas, archivos del usuario, posible red para datos de cartas): TDD + `/code-review` + `/security-review`. Rama y PR; nunca push directo a `main`. El usuario quiere que avances solo: decide lo que puedas y pregunta solo lo que sea suyo (dependencias nuevas, fuentes de datos con licencia dudosa, commitear logs reales, merge).

**Línea roja (D-004, D-006):** solo logs locales. Nada de memoria, inyección ni automatizar el juego; no mandes clics ni teclas a Hearthstone. Nunca imprimas BattleTags ni `GameAccountId` en chat, docs, tests, historial o salida de las herramientas.

## Effort y modelos por fase

| Fase | Effort | Subagentes (si hacen falta) |
|------|--------|-----------------------------|
| Lectura y estado | low | Haiku |
| Licencia de datos de cartas | medium | Haiku (búsqueda); decisión con el usuario si hay duda |
| Código (tests primero) | medium | Sonnet |
| Review | medium | `code-reviewer` y `security-reviewer` en paralelo |

## Al empezar

1. Título **`#008 Cierre de T-101`**.
2. Comprueba que el PR de las sesiones 006/007 (rama `006-007-tauri-app`: stack Tauri, parser en Rust, tracker y app) está mergeado en `main`. Si no, para y avisa.
3. Lee: `README.md`, `docs/plan/plan.md` (notas de T-101), `docs/decisions/DECISIONS.md` (D-014, D-015), `docs/research/stack-escritorio.md` y el código de `crates/` (`bg-parser`, `tracker`, `desktop`).
4. Ejecuta `cargo test --workspace --exclude desktop` y `python -m unittest discover -s tests` (con `.venv`). Todo debe estar en verde antes de tocar nada.
5. Pregunta al usuario si ha jugado partidas con la app abierta desde la sesión 007 y qué vio (¿apareció sola al acabar?). Si hay partidas nuevas, compáralas con `tools/compare_parsers.py` (solo resultados, sin nombres) y anota en el plan.

## Estado (2026-10-09)

- `bg-parser`: port del prototipo de Python, idéntico en las 23 partidas reales y en 13 casos sintéticos (CI). Líneas de más de 1 MiB se saltan y marcan la partida como `unsupported`.
- `tracker` / `tavern-watch`: sigue `Power.log` (rotación, sesiones nuevas, líneas a medias) y guarda en `%APPDATA%\TavernLedger\games.jsonl` (D-015). El historial real del usuario ya tiene sus 23 partidas importadas.
- `desktop`: app Tauri sin npm, stats por modo (Solo 1–8, Duos 1–4 por equipo). La CI no la compila (Linux necesita librerías del sistema).
- Revisiones de la sesión 007 sin CRITICAL ni HIGH; los MEDIUM están arreglados.

## Tarea (por orden; para si una pieza necesita decisión del usuario)

1. **Prueba en vivo.** Si el usuario ha jugado con la app abierta, confirma con su relato y con el historial que la partida apareció al terminar. Si no, déjalo como pendiente: no lances Hearthstone para jugar.
2. **Nombres de héroe** en vez de ids de carta. Busca una fuente de datos de cartas con licencia compatible (por ejemplo, los datos que usa la librería `hearthstone` de HearthSim, MIT, o HearthstoneJSON) y **verifica la licencia de los datos, no solo la del código**. Si no está clara, para y pregunta (regla 1 y Fan Content Policy, regla 5). Nada de imágenes de cartas en esta tarea. Los datos se empaquetan o se descargan con caché, timeout y versión fijada; anótalos en `PROVENANCE.md`. Un id sin nombre conocido sale como id, nunca inventado.
3. **Una sola instancia** de la app (la revisión lo marcó LOW: dos procesos escriben el mismo historial). Opciones: `tauri-plugin-single-instance` (dependencia nueva, pide OK) o un archivo de bloqueo propio. Recomienda y pregunta.
4. **Bandeja y arranque con Windows** (opcional, solo si sobra tiempo): la app sigue el log minimizada. El arranque automático es opcional y está desactivado por defecto.
5. Cierra T-101 en el plan si 1–3 están hechos; si no, deja claro qué falta.

## Fuera de alcance

- Stats avanzadas (T-102), web y cuentas (T-103/T-104), firma e instalador (T-105), overlay (T-301/T-302).
- npm/TypeScript en la UI, salvo que el usuario lo pida.
- Fixtures recortados de logs reales (necesitan OK del usuario antes de commitearlos).

## Criterios de aceptación

- [ ] Tests nuevos antes del código; `cargo test --workspace --exclude desktop`, `cargo clippy -- -D warnings`, `cargo fmt --check` y los tests de Python en verde.
- [ ] Nombres de héroe con fuente y licencia anotadas en `PROVENANCE.md` (o la decisión del usuario de aplazarlo, en `DECISIONS.md`).
- [ ] Ningún nombre de jugador en el historial, la UI ni los tests (comprobado contra los nombres del log sin imprimirlos, como en la sesión 007).
- [ ] `/code-review` y `/security-review` sin CRITICAL ni HIGH abiertos.
- [ ] Plan, `DECISIONS.md`, `PROVENANCE.md` y README al día en el mismo PR; CI en verde.

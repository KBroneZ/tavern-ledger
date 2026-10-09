# Sesión 009: Stats personales (T-102)

Modelo: sonnet
Effort: medium
Subagentes: haiku para búsquedas; sonnet para código y revisiones

Trabaja en `C:\Users\andia\tavern-ledger` siguiendo su `CLAUDE.md`: caveman en el chat; documentos en español; README público, producto y UI en inglés. **Tier R1** (código normal sobre datos locales): TDD + `/code-review`. Si añades una dependencia o tocas red, pasa a R2 (`/security-review` y fila en `PROVENANCE.md`). Rama y PR; nunca push directo a `main`.

El usuario quiere que avances solo: decide tú lo técnico y pregunta solo lo que sea suyo (dependencias nuevas, datos con licencia dudosa, commitear logs reales y merge). Lo que se quede corriendo mucho rato (la app, procesos vivos) lánzalo en una ventana aparte del ordenador principal (`Start-Process`), no en la conversación. `gh` no tiene sesión: para PRs, usa el token que Git guarda en el Credential Manager (`git credential fill` → `GH_TOKEN`), sin imprimirlo nunca.

**Línea roja (D-004, D-006):** solo logs locales. Nada de memoria, inyección ni automatizar el juego. Nunca imprimas BattleTags ni `GameAccountId` en el chat, los docs, los tests, el historial ni la salida de las herramientas.

## Effort y modelos por fase

| Fase | Effort | Subagentes (si hacen falta) |
|------|--------|-----------------------------|
| Lectura y estado | low | Haiku |
| Diseño de las stats | medium | — |
| Código (tests primero) | medium | Sonnet |
| Review | medium | `code-reviewer` (y `security-reviewer` si pasa a R2) |

## Al empezar

1. Título **`#009 Stats personales`**.
2. `git switch main && git pull`; rama `009-stats-personales`.
3. Lee `README.md`, `docs/plan/plan.md` (notas de T-101 y T-102), `docs/decisions/DECISIONS.md` (D-005, D-015 a D-018) y el código de `crates/` (`bg-parser/src/report.rs`, `tracker/src/store.rs`, `desktop/src/main.rs`, `desktop/ui/app.js`).
4. `cargo test --workspace --exclude desktop`, `cargo test -p desktop` y `python -m unittest discover -s tests` (con `.venv`) en verde antes de tocar nada.
5. **Cierre de T-101:** pregunta al usuario si ha jugado con la app abierta y si la partida apareció sola al terminar. Compruébalo en `%APPDATA%\TavernLedger\games.jsonl` contando partidas por sesión, sin nombres (en la sesión 008 había 23, de 5 sesiones). Si apareció, marca T-101 como hecha en el plan. Si hubo un fallo, arreglarlo pasa por delante de T-102.

## Estado (2026-10-09, tras la sesión 008)

- Todo está en `main` (PR #7): el parser en Rust, `tavern-watch`, la app Tauri con bandeja, el arranque con Windows opcional y una sola instancia escribiendo (`games.lock`).
- Cada informe del historial trae `hero`, `teammate_hero`, `final_place`, `final_health`, `lobby` (8 héroes con puesto y equipo de Duos), `shop_tribes` (tribus de las ofertas de la taberna, no las tribus exactas del lobby), `rounds` (vida y tableros) y `card_names` (nombre de cada héroe sacado del log, en el idioma del cliente; si falta, se usa el id).
- La app ya muestra, por modo, el número de partidas, el puesto medio, el % de top 4 (Solo) o top 2 (Duos) y las victorias. Esa lógica vive en `app.js`, sin tests.
- El historial real tiene 23 partidas: 22 de Duos y 1 de Solo.

## Tarea

1. **Stats por héroe** (por modo, Solo y Duos por separado): partidas, puesto medio, % de la mitad de arriba y victorias. Ordenadas por partidas jugadas. Nombre del héroe desde `card_names`, con el id si falta.
2. **Agrupar skins.** Los ids traen skins y variantes (`TB_BaconShop_HERO_102_SKIN_G`, `BG20_HERO_102pe`). Decide con datos si se agrupan por héroe base: compara ids y nombres del historial real **sin imprimir nombres de jugadores** (los nombres de héroe sí se pueden ver). Si no está claro, no los agrupes y anótalo. No inventes ningún mapeo.
3. **Tribus:** muestra `shop_tribes` como "tribus vistas en la taberna", nunca como "tribus del lobby" (el log no las da con exactitud; ver `docs/research/parser-hslog.md`).
4. **Lógica en Rust y con tests:** un módulo de stats (por ejemplo, `tracker::stats`) con tests de unidad, y la app pide el resultado por un comando de Tauri. `app.js` solo pinta, con `textContent`. Pasa a ese módulo también los totales que hoy calcula `app.js`, para que no haya dos cálculos.
5. **Vacío ≠ desconocido ≠ cero:** las partidas `incomplete` o `unsupported` no cuentan para los puestos, pero se dice cuántas hay. Con 0 partidas válidas se muestra "—", nunca 0.
6. Plan (T-102), README y, si hay decisiones, `DECISIONS.md`, en el mismo PR.

## Fuera de alcance

- La web, las cuentas y la subida de partidas (T-103/T-104); el MMR (D-011 a D-013, en otra tarea); el overlay; el instalador (T-105).
- npm/TypeScript en la UI, salvo que el usuario lo pida.
- Imágenes de cartas.

## Criterios de aceptación

- [ ] Tests nuevos antes del código. En verde: `cargo test --workspace --exclude desktop`, `cargo test -p desktop`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --check` y los tests de Python.
- [ ] Stats por héroe y totales calculados en Rust, con tests que cubren las partidas incompletas, el historial vacío y Solo frente a Duos.
- [ ] Ningún nombre de jugador en la UI, los tests ni la salida (comprobado contra los `PlayerName` del log sin imprimirlos).
- [ ] La app probada con el historial real, lanzada en una ventana aparte.
- [ ] `/code-review` sin CRITICAL ni HIGH abiertos; CI en verde.
- [ ] Plan, README (y `DECISIONS.md` si hace falta) al día en el mismo PR.

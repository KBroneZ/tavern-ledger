# Provenance

Origen y licencia de cada dependencia, dataset o fixture. Nada entra sin fila aquí.

| Componente | Origen | Licencia | Uso | Añadido |
|------------|--------|----------|-----|---------|
| Python (biblioteca estándar) | [python.org](https://www.python.org/) | PSF-2.0 | `tools/check_logs.py`, `tools/leaderboard.py` (`urllib`, `gzip`/`zlib`, `json`) y sus tests (sin dependencias externas) | 2026-10-08 |
| `actions/checkout` v7.0.1 (`3d3c42e`) | [GitHub](https://github.com/actions/checkout) | MIT | CI | 2026-10-08 |
| `actions/setup-python` v7.0.0 (`5fda3b9`) | [GitHub](https://github.com/actions/setup-python) | MIT | CI (tests) | 2026-10-08 |
| `lycheeverse/lychee-action` v2.9.0 (`e747777`) con lychee v0.24.2 | [GitHub](https://github.com/lycheeverse/lychee-action) | Apache-2.0 (acción); lychee: Apache-2.0 o MIT | CI: enlaces Markdown relativos (`--offline`) | 2026-10-08 |
| `hslog` 1.20.0 | [PyPI](https://pypi.org/project/hslog/) · [GitHub](https://github.com/HearthSim/python-hslog) | MIT | `tools/parse_bg.py`: parser de `Power.log` | 2026-10-08 |
| `hearthstone` 9.21.1 | [PyPI](https://pypi.org/project/hearthstone/) · [GitHub](https://github.com/HearthSim/python-hearthstone) | MIT | `tools/parse_bg.py`: enums del juego (dependencia de `hslog`) | 2026-10-08 |
| `aniso8601` 10.0.1 | [PyPI](https://pypi.org/project/aniso8601/) | BSD-3-Clause | Dependencia de `hslog` | 2026-10-08 |
| `requests` 2.34.2 | [PyPI](https://pypi.org/project/requests/) | Apache-2.0 | Dependencia de `hearthstone`; el prototipo no hace peticiones de red | 2026-10-08 |
| `urllib3` 2.8.0 | [PyPI](https://pypi.org/project/urllib3/) | MIT | Dependencia de `requests` | 2026-10-08 |
| `certifi` 2026.7.22 | [PyPI](https://pypi.org/project/certifi/) | MPL-2.0 | Dependencia de `requests`; se usa sin modificar | 2026-10-08 |
| `idna` 3.20 | [PyPI](https://pypi.org/project/idna/) | BSD-3-Clause | Dependencia de `requests` | 2026-10-08 |
| `charset-normalizer` 3.5.2 | [PyPI](https://pypi.org/project/charset-normalizer/) | MIT | Dependencia de `requests` | 2026-10-08 |
| Logs sintéticos de test (`tests/bg_log_builder.py`) | Escritos a mano imitando el formato de `Power.log` | MIT (este repo) | Tests de `tools/parse_bg.py`; sin datos de partidas reales | 2026-10-08 |
| Respuestas sintéticas del leaderboard (`tests/test_leaderboard.py`) | Escritas a mano con la forma documentada en `docs/research/fuentes-mmr.md`; nombres inventados | MIT (este repo) | Tests de `tools/leaderboard.py`; ninguna respuesta real (D-012) | 2026-10-09 |
| `tauri` 2.12.2 y `tauri-build` 2.7.1, con 407 crates en total fijadas en `spikes/overlay-tauri/Cargo.lock` (checksums de crates.io) | [crates.io](https://crates.io/crates/tauri) · [GitHub](https://github.com/tauri-apps/tauri) | MIT o Apache-2.0; las transitivas, licencias OSI permisivas y 5 con MPL-2.0, sin modificar | Prueba del overlay (T-005); no se distribuye | 2026-10-09 |
| `tauri` 2.12.2 y `tauri-build` 2.7.1 en `crates/desktop` (mismas versiones que la prueba, fijadas en el `Cargo.lock` raíz) | [crates.io](https://crates.io/crates/tauri) · [GitHub](https://github.com/tauri-apps/tauri) | MIT o Apache-2.0; transitivas con licencias OSI permisivas y MPL-2.0 sin modificar | App de escritorio (T-101, D-014) | 2026-10-09 |
| `serde` 1.0.229 y `serde_json` 1.0.151, con sus dependencias fijadas en `Cargo.lock` (`serde_core`, `serde_derive`, `proc-macro2`, `quote`, `syn`, `unicode-ident`, `itoa`, `memchr`, `zmij`) | [crates.io](https://crates.io/crates/serde) · [GitHub](https://github.com/serde-rs/serde) | MIT o Apache-2.0 (`memchr`: Unlicense o MIT; `zmij`: MIT; `unicode-ident`: también Unicode-3.0) | `crates/bg-parser`: salida JSON del parser | 2026-10-09 |
| Lectura de `Power.log` en `crates/bg-parser` | Escrita para este repo a partir de `docs/research/parser-hslog.md` y de la gramática de líneas y la resolución de nombres de `hslog` | MIT (este repo); `hslog` es MIT | Port del prototipo de Python sin `hslog` | 2026-10-09 |
| Logs sintéticos y salidas esperadas (`crates/bg-parser/tests/data/`) | Generados con `tools/gen_parser_fixtures.py` desde `tests/bg_log_builder.py` y `tools/parse_bg.py` | MIT (este repo) | Tests de paridad Rust/Python; sin datos de partidas reales | 2026-10-09 |
| Nombres de héroe (`card_names` en `crates/bg-parser`) | Leídos en tiempo de ejecución del `Power.log` del propio jugador (`[entityName=… cardId=…]`); nada se empaqueta ni se descarga | Texto del juego, © Blizzard Entertainment; no entra en el repo (D-017) | Mostrar el héroe por su nombre en la app | 2026-10-09 |
| `File::try_lock` (biblioteca estándar de Rust 1.89+) | [doc.rust-lang.org](https://doc.rust-lang.org/std/fs/struct.File.html#method.try_lock) | MIT o Apache-2.0 | Una sola instancia que escribe el historial (D-016); sin dependencias nuevas | 2026-10-09 |
| `tauri-plugin-autostart` 2.5.1 (fijada con `=`), con `auto-launch` 0.5.0, `winreg` 0.10.1, `dirs` 4.0.0, `dirs-sys` 0.3.7, `redox_users` 0.4.6, `getrandom` 0.2.17 y `tauri-plugin` 2.7.1 (`Cargo.lock`); feature `tray-icon` de `tauri` 2.12.2 (`tray-icon` 0.25.1 ya estaba en el lock) | [crates.io](https://crates.io/crates/tauri-plugin-autostart) · [GitHub](https://github.com/tauri-apps/plugins-workspace) | MIT o Apache-2.0 (`auto-launch`, `winreg`, `redox_users`: MIT) | Bandeja y arranque con Windows de la app (D-018) | 2026-10-09 |
| gitleaks v8.30.1 (binario linux_x64, SHA-256 verificado) | [GitHub releases](https://github.com/gitleaks/gitleaks/releases/tag/v8.30.1) | MIT | CI: búsqueda de secretos en todo el historial | 2026-10-08 |

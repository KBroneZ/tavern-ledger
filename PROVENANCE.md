# Provenance

Origen y licencia de cada dependencia, dataset o fixture. Nada entra sin fila aquí.

| Componente | Origen | Licencia | Uso | Añadido |
|------------|--------|----------|-----|---------|
| Python (biblioteca estándar) | [python.org](https://www.python.org/) | PSF-2.0 | `tools/check_logs.py` y sus tests (sin dependencias externas) | 2026-10-08 |
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
| gitleaks v8.30.1 (binario linux_x64, SHA-256 verificado) | [GitHub releases](https://github.com/gitleaks/gitleaks/releases/tag/v8.30.1) | MIT | CI: búsqueda de secretos en todo el historial | 2026-10-08 |

# Provenance

Origen y licencia de cada dependencia, dataset o fixture. Nada entra sin fila aquí.

| Componente | Origen | Licencia | Uso | Añadido |
|------------|--------|----------|-----|---------|
| Python (biblioteca estándar) | [python.org](https://www.python.org/) | PSF-2.0 | `tools/check_logs.py` y sus tests; sin dependencias externas | 2026-10-08 |
| `actions/checkout` v7.0.1 (`3d3c42e`) | [GitHub](https://github.com/actions/checkout) | MIT | CI | 2026-10-08 |
| `actions/setup-python` v7.0.0 (`5fda3b9`) | [GitHub](https://github.com/actions/setup-python) | MIT | CI (tests) | 2026-10-08 |
| `lycheeverse/lychee-action` v2.9.0 (`e747777`) con lychee v0.24.2 | [GitHub](https://github.com/lycheeverse/lychee-action) | Apache-2.0 (acción); lychee: Apache-2.0 o MIT | CI: enlaces Markdown relativos (`--offline`) | 2026-10-08 |
| gitleaks v8.30.1 (binario linux_x64, SHA-256 verificado) | [GitHub releases](https://github.com/gitleaks/gitleaks/releases/tag/v8.30.1) | MIT | CI: búsqueda de secretos en todo el historial | 2026-10-08 |

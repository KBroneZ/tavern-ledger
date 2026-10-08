# Tavern Ledger

Free, open-source Battlegrounds tracker for Windows, plus a website to follow your progress. It only reads the game's local log files: no memory reading, no injection, no game file changes.

> Unofficial fan project. Not affiliated with or endorsed by Blizzard Entertainment.

## Status

**Phase F0, feasibility proof.** Nothing to install yet. Plan in [docs/plan/plan.md](docs/plan/plan.md), decisions in [docs/decisions/DECISIONS.md](docs/decisions/DECISIONS.md), research in [docs/research/](docs/research/viabilidad.md) (project docs are in Spanish).

Next task: T-002 (collect own Solo and Duos games as test fixtures), then T-003 (parser prototype).

## Check your log setup

Read-only script (Python 3.10+, no dependencies). It reports whether `log.config` enables the Power log, where the game writes its logs, and which game types each log contains. It never writes anything and never prints player names.

```
python tools/check_logs.py
```

Options: `--config PATH` and `--logs-dir PATH` if your install is not found automatically.

## Development

```
python -m unittest discover -s tests
```

CI runs the tests, checks relative Markdown links and scans the history for secrets.

## License

MIT. See [LICENSE](LICENSE).

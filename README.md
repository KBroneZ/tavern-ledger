# Tavern Ledger

Free, open-source Battlegrounds tracker for Windows, plus a website to follow your progress. It only reads the game's local log files: no memory reading, no injection, no game file changes.

> Unofficial fan project. Not affiliated with or endorsed by Blizzard Entertainment.

## Status

**Phase F0, feasibility proof.** Nothing to install yet. Plan in [docs/plan/plan.md](docs/plan/plan.md), decisions in [docs/decisions/DECISIONS.md](docs/decisions/DECISIONS.md), research in [docs/research/](docs/research/viabilidad.md) (project docs are in Spanish).

A prototype parser rebuilds each Battlegrounds game from `Power.log`: hero, Duos teammate, lobby, health per round, opponents, boards at the start of each combat and final place. It was checked against 23 real Duos games. MMR and the exact lobby tribes are not in the log. Findings: [docs/research/parser-hslog.md](docs/research/parser-hslog.md).

No local log has the MMR value; the public leaderboard only covers ratings of 8000 and above. Findings: [docs/research/fuentes-mmr.md](docs/research/fuentes-mmr.md).

Next tasks: T-002 (Solo games and trimmed fixtures) and T-005 (desktop stack).

## Check your log setup

Read-only script (Python 3.10+, no dependencies). It reports whether `log.config` enables the Power log, where the game writes its logs, and which game types each log contains. It never writes anything and never prints player names.

```
python tools/check_logs.py
```

Options: `--config PATH` and `--logs-dir PATH` if your install is not found automatically.

## Parse a log (prototype)

Needs Python 3.10+ and the pinned dependencies. Read-only; the output has card ids and lobby player numbers, never player names.

```
python -m pip install --require-hashes --only-binary=:all: -r requirements.txt
python tools/parse_bg.py "<Hearthstone folder>\Logs\Hearthstone_<date>\Power_old.log"
```

Add `--json` for machine-readable output.

## Development

```
python -m pip install --require-hashes --only-binary=:all: -r requirements.txt
python -m unittest discover -s tests
```

CI runs the tests, checks relative Markdown links and scans the history for secrets.

## License

MIT. See [LICENSE](LICENSE).

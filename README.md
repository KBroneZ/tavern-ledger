# Tavern Ledger

Free, open-source Battlegrounds tracker for Windows, plus a website to follow your progress. It only reads the game's local log files: no memory reading, no injection, no game file changes.

> Unofficial fan project. Not affiliated with or endorsed by Blizzard Entertainment.

## Status

**Phase F0, feasibility proof.** Nothing to install yet. Plan in [docs/plan/plan.md](docs/plan/plan.md), decisions in [docs/decisions/DECISIONS.md](docs/decisions/DECISIONS.md), research in [docs/research/](docs/research/viabilidad.md) (project docs are in Spanish).

A prototype parser rebuilds each Battlegrounds game from `Power.log`: hero, Duos teammate, lobby, health per round, opponents, boards at the start of each combat and final place. It was checked against 23 real games (22 Duos, 1 Solo). MMR and the exact lobby tribes are not in the log. Findings: [docs/research/parser-hslog.md](docs/research/parser-hslog.md).

No local log has the MMR value. The public leaderboard only covers ratings of 8000 and above: players in it will see their rating, everyone else "below the leaderboard cut-off", never a made-up number. Findings: [docs/research/fuentes-mmr.md](docs/research/fuentes-mmr.md).

A prototype client looks up your own row in the public leaderboard with a local cache and a small, capped number of requests. It never stores other players' rows. Strategy and measurements: [docs/research/cliente-leaderboard.md](docs/research/cliente-leaderboard.md).

Desktop stack (T-005): the comparison recommends Tauri 2. A minimal Tauri overlay (transparent, always on top, clicks pass through) works on Windows 11, also over the game in borderless fullscreen. Decision: Tauri 2 (D-014). Findings: [docs/research/stack-escritorio.md](docs/research/stack-escritorio.md).

Next tasks: T-101 (desktop app: follow `Power.log`, store games locally) and T-002 (more Solo games and trimmed fixtures).

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

## Look up your leaderboard rating (prototype)

Python 3.10+, no dependencies. It asks the public leaderboard for your own row only and prints one of four states: your rating, below the leaderboard cut-off, ambiguous (your name appears more than once) or unknown. It never prints your name.

```
python tools/leaderboard.py --name "<your BattleTag>" --mode solo --region EU
```

Use `--mode duos` for Duos, `--hearthstone-log PATH` to read the region from the game's `Hearthstone.log` instead of `--region`, and `--json` for machine-readable output. To answer "below the cut-off" it must check every page (about 170 requests, a few minutes), so it does that at most once a day per region and mode and otherwise shows the last result with its time. The cache lives in `.local/leaderboard-cache.json`.

## Desktop app and tracker (prototype)

Rust workspace (Tauri 2, D-014). `tavern-watch` follows the game's `Power.log`
and saves each Battlegrounds game to `%APPDATA%\TavernLedger\games.jsonl` as
soon as it ends; the desktop app does the same in the background and shows
the history. Both only read the log files. The history holds card ids,
places and boards, never player names.

```
cargo run --release -p tracker -- --import    # follow live, after importing older sessions
cargo run --release -p desktop                # the app
cargo run --release -p bg-parser -- "<path>\Power_old.log" --json
```

## Development

```
python -m pip install --require-hashes --only-binary=:all: -r requirements.txt
python -m unittest discover -s tests
cargo test --workspace
```

CI runs the tests, checks relative Markdown links and scans the history for secrets.

## License

MIT. See [LICENSE](LICENSE).

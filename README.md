# Tavern Ledger

Free, open-source Battlegrounds tracker for Windows, plus a website to follow your progress. It only reads the game's local log files: no memory reading, no injection, no game file changes.

> Unofficial fan project. Not affiliated with or endorsed by Blizzard Entertainment.

## Status

**Phase F0, feasibility proof.** Nothing to install yet. Plan in [docs/plan/plan.md](docs/plan/plan.md), decisions in [docs/decisions/DECISIONS.md](docs/decisions/DECISIONS.md), research in [docs/research/](docs/research/feasibility.md).

A prototype parser rebuilds each Battlegrounds game from `Power.log`: hero, Duos teammate, lobby, health per round, opponents, boards at the start of each combat and final place. It was checked against 23 real games (22 Duos, 1 Solo). MMR and the exact lobby tribes are not in the log. Findings: [docs/research/parser-hslog.md](docs/research/parser-hslog.md).

No local log has the MMR value. The public leaderboard only covers ratings of 8000 and above: players in it will see their rating, everyone else "below the leaderboard cut-off", never a made-up number. Findings: [docs/research/mmr-sources.md](docs/research/mmr-sources.md).

A prototype client looks up your own row in the public leaderboard with a local cache and a small, capped number of requests. It never stores other players' rows. Strategy and measurements: [docs/research/leaderboard-client.md](docs/research/leaderboard-client.md).

Desktop stack (T-005): the comparison recommends Tauri 2. A minimal Tauri overlay (transparent, always on top, clicks pass through) works on Windows 11, also over the game in borderless fullscreen. Decision: Tauri 2 (D-014). Findings: [docs/research/desktop-stack.md](docs/research/desktop-stack.md).

Personal stats (T-102) are in the app: totals, per-hero numbers and tribes seen in the tavern, per mode.

Web stack (T-103): a static Astro site plus Supabase in the EU (Postgres, login, file storage); the desktop app will upload each finished game, about 2.5 KB gzipped, only after you sign in and turn it on. No Battle.net login: Blizzard's developer terms do not allow it in an app with paid extras. Decision D-020; findings: [docs/research/web-stack.md](docs/research/web-stack.md). Nothing is online yet.

Backend (T-104a): the database schema, row-level security, export, account deletion and backups exist and are tested on a local Supabase stack. Clients can only read their own data and public profiles; game uploads will go through one server function. The hosted project is not created yet.

Robustness (session 014): the app and `check_logs.py` check that the game is set up to write a complete log (`log.config` and `client.config`), every saved game records which parser version read it, `tavern-watch --reparse` re-reads old sessions after a parser fix, and a dev tool replays a saved log so the app can be tested without playing.

Website (T-104c): a static Astro site in `web/` with sign-in by email, an account page (display name, public profile switch, download all your data, delete your account) and a public profile page that only exists when you turn it on. It shows no ratings and no other players' names. It runs against the local stack; it is not online yet.

Where values come from (T-109): every number or label in the window says where it comes from. Values straight from the game's log carry no mark; inferred ones (tribes seen in the tavern, skins grouped under one hero) and unknown ones carry a mark, every value has a tooltip, and a legend explains them. "Report a problem" (T-110): a button on each game shows the whole report file (game report, parser version, game build, app version, log setup check, warnings) and saves it to your Downloads folder if you choose. The app refuses to make it if it finds anything that looks like a player name, and it never sends it anywhere; you send it yourself.


Privacy and terms (T-104b): a data inventory written from the code, and drafts of the privacy policy and terms in [docs/legal/](docs/legal/), shown on the site at `/privacy/` and `/terms/`. They are not in force: the controller's name and contact, the email sender and some provider details are still to be decided. No analytics, minimum age 16.

Next tasks: T-101 (only a live test during a real game is left), T-002 (more Solo games and trimmed fixtures), the hosted Supabase project and the site's hosting (need the user), T-104b (fill in the open items of the privacy policy) and T-104d (upload from the desktop app).

## Check your log setup

Read-only script (Python 3.10+, no dependencies). It reports whether `log.config` enables the Power log, where the game writes its logs, and which game types each log contains. It never writes anything and never prints player names.

```
python tools/check_logs.py
```

It also checks `client.config` next to `Hearthstone.exe`: without `[Log]` `FileSizeLimit.Int=-1` the game stops writing the log at about 10 MB. A missing file, an unreadable file and a missing setting are reported as three different problems, with the lines to add. The script never edits either file.

Options: `--config PATH`, `--client-config PATH` and `--logs-dir PATH` if your install is not found automatically.

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
places and boards, never player names. Hero names come from the log itself,
in your game's language; a hero the log does not name shows its card id.
Only one of them can write the history at a time: a second copy says so and
does not write. The app lives in the system tray: closing the window keeps it
following the log, and "Quit" in the tray menu exits. "Start with Windows" in
the same menu is off until you turn it on.

Per mode (Solo and Duos apart), the app shows your totals, your numbers per
hero (skins of a hero grouped together) and the tribes the tavern offered you.
Those are not the lobby's tribes: the log does not say which tribes were in
the lobby. Unfinished or unreadable games are counted apart, never as places.

```
cargo run --release -p tracker -- --import    # follow live, after importing older sessions
cargo run --release -p desktop                # the app
cargo run --release -p tracker -- --reparse   # re-read saved sessions after a parser fix
cargo run --release -p bg-parser -- "<path>\Power_old.log" --json
```

Each saved game records the parser version that read it (shown in the tooltip of its date in the app; older records say "unknown version"). `--reparse` saves only games whose report changed and lists the ones whose logs are gone; it does not run while the app is open.

### Replay a log (development only)

To try the app or `tavern-watch` without playing, replay a saved log into a temporary folder. Never shipped to users; it only writes under the folder you give it.

```
python tools/dev/replay_log.py "<path>\Power_old.log" --dest-dir "$env:TEMP\replay" --lines-per-second 500
cargo run -p tracker -- --logs-dir "$env:TEMP\replay\Logs" --data-dir "$env:TEMP\replay\data"
```

## Development

```
python -m pip install --require-hashes --only-binary=:all: -r requirements.txt
python -m unittest discover -s tests
cargo test --workspace
```

CI runs the tests, checks relative Markdown links and scans the history for secrets.

### Backend (local Supabase stack)

Needs Node.js and Docker Desktop. The CLI is pinned in `package.json`.

```
npm ci
npx supabase start                    # first run pulls the images (a few GB)
npx supabase test db                  # pgTAP: schema, row-level security, export
deno test supabase/functions/delete-account/
$env:TAVERN_SUPABASE_LOCAL = "1"; python -m unittest tests.test_supabase_local -v
pwsh tools/backup_supabase.ps1 -Local # backup into .local/backups/
```

These run on the developer's machine, not in CI. Without `-Local`, the backup script backs up the linked hosted project, reading `SUPABASE_URL` and `SUPABASE_SERVICE_ROLE_KEY` from `.local/supabase.env`.

### Website (`web/`)

Needs Node.js 22.12 or later. Copy `web/.env.example` to `web/.env` and fill in the local stack's API URL and anon key from `npx supabase status` (only the public key: the build refuses any other).

```
cd web
npm ci
npm test                    # unit tests (Node's built-in runner)
npm run build               # static site in web/dist/
npm run test:dist           # checks on the built files: CSP, nothing inlined, no secret key
npm run preview             # http://127.0.0.1:3000, the address the local stack's auth expects
$env:TAVERN_SUPABASE_LOCAL = "1"; npm run test:local   # end to end against the local stack
```

CI runs the unit tests, the build and the built-file checks.

## License

MIT. See [LICENSE](LICENSE).

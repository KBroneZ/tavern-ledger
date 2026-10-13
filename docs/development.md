# Development

How to run the tools, the desktop app, the backend and the website from a clone. Everything here is for development. To only use the app, take the installer from the [download page](https://tavernledger.net/download/); to build the same installer yourself, see below.

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
the history. Both read the log files; the app can also read two values from
the game itself if you turn that on (D-063, below). The history holds card ids,
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

Upload to the website: the "Upload to the website" panel at the top of the window signs you in by email link, shows how many games are waiting, the last result and why uploading stopped, if it did, and holds the "Upload games" switch (off until you turn it on). The app uploads to the hosted project; a `server.json` in the data folder (`{"url": "...", "anon_key": "..."}`, the public key only) points it at another server, such as the local stack. `--data-dir <folder>` makes the app use another folder for the history and the upload settings, e.g. to try a build:

```
cargo run -p desktop -- --data-dir "$env:TEMP\tl-try"
```

Each saved game records the parser version that read it (shown in the tooltip of its date in the app; older records say "unknown version"). `--reparse` saves only games whose report changed and lists the ones whose logs are gone; it does not run while the app is open.

### Replay a log (development only)

To try the app or `tavern-watch` without playing, replay a saved log into a temporary folder. Never shipped to users; it only writes under the folder you give it.

```
python tools/dev/replay_log.py "<path>\Power_old.log" --dest-dir "$env:TEMP\replay" --lines-per-second 500
cargo run -p tracker -- --logs-dir "$env:TEMP\replay\Logs" --data-dir "$env:TEMP\replay\data"
```

### Reconnect dev tool (development only)

To collect test games faster, `tools/dev/reconnect.py` drops Hearthstone's own connections on a hotkey so the client reconnects past combat animations. Never shipped to users; needs an elevated terminal; it can carry risk for your Battle.net account, so only use it by hand on your own account. Games played with it are marked in the history and left out of the stats, the upload and fixtures. Details, risk and how to undo: [tools/dev/README.md](../tools/dev/README.md).

To see the overlay on a replayed game, follow the replayed folder with the app and force the overlay on (`--overlay-dev` turns it on, shows it whatever window is in front and does not save the on/off choice (layout and theme are saved in the data folder you give; `--overlay-unlock` starts it unlocked); `--logs-dir` is for this and nothing else):

```
cargo run -p desktop -- --data-dir "$env:TEMP\replay\data" --logs-dir "$env:TEMP\replay\Logs" --overlay-dev
```

## Windows installer

Needs the same Rust toolchain as the app, Node.js 22.12 or later and an internet connection. From a clean checkout:

```
npm ci
npm run installer
```

`npm run installer` runs the pinned Tauri CLI (`@tauri-apps/cli` in `package.json`), which builds the app in release mode and makes `target\release\bundle\nsis\Tavern Ledger_<version>_x64-setup.exe` (about 6.5 MB). The version is `version` in `crates/desktop/tauri.conf.json`. To try an upgrade without changing it, build a second time from `crates\desktop` with `npx tauri build --bundles nsis --config '{"version":"0.1.1"}'`. The build also leaves `target\release\TavernLedger.exe`; `cargo build --release -p desktop` still makes `target\release\desktop.exe`.

On its first run the CLI downloads, into `%LOCALAPPDATA%\tauri`: NSIS 3.11 and the `nsis_tauri_utils` 0.5.3 plugin from Tauri's GitHub releases (checked against hashes built into the CLI), and Microsoft's WebView2 bootstrapper (not hash-checked; `Get-AuthenticodeSignature "$env:LOCALAPPDATA\tauri\MicrosoftEdgeWebview2Setup.exe"` should say `Valid` and `CN=Microsoft Corporation`).

What the installer does:

- **Per user, no administrator rights:** installs to `%LOCALAPPDATA%\Tavern Ledger` with a Start menu shortcut (and a desktop one if you tick it), and registers an uninstaller in Settings > Apps. Silent: `/S`; uninstall silently with `"%LOCALAPPDATA%\Tavern Ledger\uninstall.exe" /S`.
- **License page:** the fan-project notice, what the app reads and the privacy link, then `LICENSE` (`crates/desktop/installer/license.txt`; a test checks that it ends with `LICENSE` unchanged).
- **Compression:** zlib (NSIS's LZMA module is CPL-1.0, which does not mix with AGPL-3.0).
- **WebView2:** the bootstrapper is embedded (about 1.8 MB) and runs silently only when WebView2 is missing (Windows 11 and up-to-date Windows 10 have it); it still needs internet to fetch the runtime.
- **Upgrade:** a newer installer replaces the app in place. If the installed app is running, the installer closes that copy only (by its path), never a build run from source.
- **Your data:** install, upgrade and uninstall never touch `%APPDATA%\TavernLedger` (history, settings, upload settings). The uninstaller's "Delete the application data" box removes only the folders named after the app's identifier, `%APPDATA%\dev.tavernledger.app` (unused) and `%LOCALAPPDATA%\dev.tavernledger.app` (the window's WebView2 cache, shared with builds run from source), and the installer's own registry keys. Uninstalling also removes the "Start with Windows" entry, which a build run from source writes under the same name.
- **Not signed:** Windows SmartScreen warns before it runs, so a published build should list its SHA-256 next to the download (`Get-FileHash "<setup file>"` to compare). No automatic updates.

## Development

```
python -m pip install --require-hashes --only-binary=:all: -r requirements.txt
python -m unittest discover -s tests
cargo test --workspace
```

CI runs the tests (the desktop app's on Windows, the Edge Functions' with Deno, the database's with pgTAP on the local stack), rebuilds the website's engine and compares it with the committed one, checks relative Markdown links and that every workflow action is pinned by commit SHA, and scans the history for secrets.

### Real-game fixtures

Copy the raw log into `.local/` (never commit it), build the parser and make one fixture per game; the script writes nothing if the log has a game played with the reconnect dev tool (D-043), if a name or id would be left, or if the fixture would read differently from the raw game:

```
cargo build --release -p bg-parser
python tools/make_fixture.py .local/fixtures-raw/Power_old.log --game 3 --session Hearthstone_<date> --out crates/bg-parser/tests/data/real/b<build>_duos.log --parser target/release/bg-parse
```

Check the report it writes next to the fixture (`.json`) by hand, add the build to `TESTED_BUILDS` if it is new (then make the fixture again, so the report has no "not tested" warning), and get the user's OK before committing. Fixture times count from the game's first line (D-046), so durations and APM are tested too.

### Backend (local Supabase stack)

Needs Node.js and Docker Desktop. The CLI is pinned in `package.json`.

```
npm ci
npx supabase start                    # first run pulls the images (a few GB)
npx supabase test db                  # pgTAP: schema, row-level security, export
deno test supabase/functions/        # delete-account, upload-game, sweep
$env:TAVERN_SUPABASE_LOCAL = "1"; python -m unittest tests.test_supabase_local -v
$env:TAVERN_SUPABASE_LOCAL = "1"; cargo test -p uploader --features http --test local_stack -- --nocapture
pwsh tools/backup_supabase.ps1 -Local # backup into .local/backups/
```

The pgTAP and Deno tests also run in CI, on a stack that starts only what they need; the end-to-end tests run on the developer's machine only. The desktop end-to-end test sends one sign-in email and the website one sends two (sign-up, password reset); the local stack allows 30 an hour, as the hosted project. The daily sweep (files with no row, audit entries of deleted accounts or older than 90 days, expired counters) is the `sweep` function: `POST /functions/v1/sweep` with the service-role key, or with the sweep token that `pg_cron` sends every day at 03:17 UTC (migration `20261010090000_sweep_schedule.sql`; on the local stack the job sends nothing because Vault has no `project_url`). Each finished sweep leaves a row in `private.sweep_runs`.

Hosted project: migrations and the three functions will reach it through the Supabase GitHub integration on each merge to `main` once the user connects it; auth settings (sign-ups closed, site URL, redirect URL, password length) are set in the dashboard. A migration that adds summary columns goes in before the functions that write them (`SUMMARY_VERSION`): until then `put_game_stats` refuses the new keys, games are stored without a summary and the daily sweep fills them once the migration is in. The desktop app has the hosted URL and publishable key built in. Without `-Local`, the backup script backs up the linked hosted project, reading `SUPABASE_URL` and `SUPABASE_SECRET_KEY` (an `sb_secret_…` key; legacy keys are refused) from `.local/supabase.env`; the Free plan has no Supabase backups, so this is the only one.

### Website (`web/`)

Needs Node.js 22.12 or later. Copy `web/.env.example` to `web/.env` and fill in the local stack's API URL and anon key from `npx supabase status` (only the public key: the build refuses any other). `PUBLIC_SIGNUPS_OPEN=true` shows the sign-up form; unset, as on the live site, `/signin/` offers sign-in only.

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

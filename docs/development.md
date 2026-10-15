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

Upload to the website: the "Upload to the website" panel at the top of the window signs you in by email link (it opens the website's `/desktop-sign-in/` page in your browser, where the link is asked for behind the anti-robot check; with the local stack that is the local site, `http://127.0.0.1:3000`, so run `npm run dev` or `npm run preview` in `web/`), shows how many games are waiting, the last result and why uploading stopped, if it did, and holds the "Upload games" switch (off until you turn it on). The app uploads to the hosted project; a `server.json` in the data folder (`{"url": "...", "anon_key": "..."}`, the public key only) points it at another server, such as the local stack (`http://127.0.0.1:<port>`). Any other HTTPS server is used only by a development build or when the app is started with `--allow-other-server`, and the panel then says where uploads go. The sign-in is stored with the server that issued it and is never sent to another one: after switching servers, sign in again. `--data-dir <folder>` makes the app use another folder for the history and the upload settings, e.g. to try a build:

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

### Overlay gallery

To look at every overlay panel and state without a game (also from a phone), build the gallery. It needs only Node.js, no extra package:

```
node tools/overlay-gallery/build.js
```

This writes one self-contained page, `.local/overlay-gallery/overlay-gallery.html` (about 0.6 MB, never committed). It runs the real `crates/desktop/ui/overlay.html` and its scripts in a browser, with a stand-in for the app's bridge (`tools/overlay-gallery/bridge.js`) that answers from invented example data (`tools/overlay-gallery/scenarios/`), one scenario per frame, each marked "Example" (the hover scenarios `hover-keywords`, `hover-golden`, `hover-textures-none`, `hover-texture-bad` and `hover-no-webgl` show the minion drawing and its fallbacks) and captioned in English and Spanish. Open the file in a browser; add `?lang=es` for Spanish or `?scenario=<id>` (for example `?scenario=hover-duos`) for one frame. On a narrow screen the frames start zoomed in on the overlay.

The build only reads the overlay page. When the page asks for a command or an event the stand-in does not answer, or the theme colours in `overlay_layout.rs` change, `node --test "ui-tests/*.test.js"` (from `crates/desktop`) fails until `bridge.js` or `theme-vars.js` follows. A new panel slot of the page needs a scenario that shows it; the counters scenario appears by itself once the page has a `data-panel="counters"` slot.

### Overlay workbench

To work on the overlay page and see each change as you save it, start the workbench. It needs only Node.js, no extra package:

```powershell
node tools/overlay-dev/serve.js
```

Open <http://127.0.0.1:5178/> (`--port <n>` picks another port). The server listens on 127.0.0.1 only and serves `crates/desktop/ui/` straight from disk, with the gallery's stand-in for the app's bridge and its example data in front of the page's scripts (no copy, no build; card art is a placeholder, and the game's minion textures are read from the public host the app uses, so minions look as in the app; without network they are plain card art). Only `crates/desktop/ui/`, `tools/overlay-dev/`, `tools/overlay-gallery/` and `.local/overlay-dev/backgrounds/` are served, and the server writes nothing to disk.

- **Live reload.** Saving a stylesheet swaps it in place; saving anything else in `crates/desktop/ui/` or `tools/overlay-gallery/` reloads the overlay frame, keeping the scenario and the controls.
- **Controls.** Scenario (every gallery scenario), language, theme (including High contrast), window size (1920×1080, 2560×1440, 1280×720 or custom), backdrop, view and scale, and one button per bridge event the page listens to (`live-changed`, `overlay-hover`, `lobby-tribes-changed`…) to watch state changes. The view starts close-up on the part of the window the scenario is about; "Whole window" shows the full game window, scaled by "Scale". Every choice is kept in the address (for example `?scenario=hover-duos&lang=es&view=window`), so a view can be shared and reloaded.
- **Check overlaps.** The "Check overlaps" button goes through every scenario at 1920×1080, 2560×1440, 1280×720, 3440×1440 (ultrawide) and 1024×768 (4:3) and reports every pair of things the overlay shows (panels, the hover card, the marks beside the leaderboard) whose rendered rectangles share an area. It takes a few minutes; the page is the real one, so long names and Duos rows count. The default places are shares of the game's 16:9 picture, centred in a wider or taller window (`screen_fit::play_area`); `tools/overlay-gallery/default-layout.js` copies the app's numbers for the stand-in and a test keeps them equal.
- **Bare page.** `/bare?scenario=<id>&lang=<en|es>` shows only the overlay page at the size of the browser window.
- **Backgrounds.** Put your own screenshots (`.png`, `.jpg`, `.webp`, `.gif`) in `.local/overlay-dev/backgrounds/` (create the folder; it is never committed) and pick one under "Backdrop". Do not commit game art.
- **Limits.** Each open overlay page (the workbench, each `/bare` tab) holds one live-reload connection, and a browser allows about six per address: close old tabs if a page seems stuck. The workbench page's own files are not watched; refresh it by hand after editing them.

Its tests run with the app's ui-tests (`node --test "ui-tests/*.test.js"` from `crates/desktop`).

## Windows installer

Needs the same Rust toolchain as the app, Node.js 22.12 or later and an internet connection. From a clean checkout:

```
npm ci
npm run installer
```

`npm run installer` runs the pinned Tauri CLI (`@tauri-apps/cli` in `package.json`) as `tauri build --bundles nsis --no-sign -- --locked`, which builds the app in release mode from `Cargo.lock` as it is and makes `target\release\bundle\nsis\Tavern Ledger_<version>_x64-setup.exe` (about 6.5 MB). The version is `version` in `crates/desktop/tauri.conf.json`. To try an upgrade without changing it, build a second time from `crates\desktop` with `npx tauri build --bundles nsis --config '{"version":"0.1.1"}'`. The build also leaves `target\release\TavernLedger.exe`; `cargo build --release -p desktop` still makes `target\release\desktop.exe`.

It never signs anything. A release for players is built, signed for the app's updater and drafted by GitHub Actions from a `v*` tag, and published by hand: see [docs/release/checklist.md](release/checklist.md). `plugins.updater.pubkey` in `tauri.conf.json` is the release public key, so the installer a release builds asks GitHub for updates a minute after it starts and every six hours (turn "Check for updates automatically" off in Settings > Updates to stop it) and only accepts an installer signed by that key. A build whose key is not a real minisign public key has its updater off and never asks. So does every build you make yourself: a debug build (`cargo run`, `cargo test`) never checks, and neither does a release build from source (`target\release`, `npm run installer` on your PC), because it would replace your build with the published installer. Only a build made with the environment variable `TAVERN_LEDGER_OFFICIAL_BUILD=1`, which the release workflow sets and nothing else should, checks for updates. A build with updates off writes one line to the app log saying so, and Settings > Updates says automatic updates are not set up in this version. `cargo test -p desktop` runs the updater against a local test server with throwaway keys made in the tests.

On its first run the CLI downloads, into `%LOCALAPPDATA%\tauri`: NSIS 3.11 and the `nsis_tauri_utils` 0.5.3 plugin from Tauri's GitHub releases (checked against hashes built into the CLI), and Microsoft's WebView2 bootstrapper (not hash-checked; `Get-AuthenticodeSignature "$env:LOCALAPPDATA\tauri\MicrosoftEdgeWebview2Setup.exe"` should say `Valid` and `CN=Microsoft Corporation`).

What the installer does:

- **Per user, no administrator rights:** installs to `%LOCALAPPDATA%\Tavern Ledger` with a Start menu shortcut (and a desktop one if you tick it), and registers an uninstaller in Settings > Apps. Silent: `/S`; uninstall silently with `"%LOCALAPPDATA%\Tavern Ledger\uninstall.exe" /S`.
- **License page:** the fan-project notice, what the app reads and the privacy link, then `LICENSE` (`crates/desktop/installer/license.txt`; a test checks that it ends with `LICENSE` unchanged).
- **Compression:** zlib (NSIS's LZMA module is CPL-1.0, which does not mix with AGPL-3.0).
- **WebView2:** the bootstrapper is embedded (about 1.8 MB) and runs silently only when WebView2 is missing (Windows 11 and up-to-date Windows 10 have it); it still needs internet to fetch the runtime.
- **Upgrade:** a newer installer replaces the app in place. If the installed app is running, the installer closes that copy only (by its path), never a build run from source.
- **Your data:** install, upgrade and uninstall never touch `%APPDATA%\TavernLedger` (history, settings, upload settings). The uninstaller's "Delete the application data" box removes only the folders named after the app's identifier, `%APPDATA%\dev.tavernledger.app` (unused) and `%LOCALAPPDATA%\dev.tavernledger.app` (the window's WebView2 cache, shared with builds run from source), and the installer's own registry keys. Uninstalling also removes the "Start with Windows" entry, which a build run from source writes under the same name.
- **Not signed:** Windows SmartScreen warns before it runs, so a published build should list its SHA-256 next to the download (`Get-FileHash "<setup file>"` to compare). Updates are signed separately, for the app's updater (above).

## Development

```
python -m pip install --require-hashes --only-binary=:all: -r requirements.txt
python -m unittest discover -s tests
cargo test --workspace
```

CI runs the tests (the desktop app's on Windows, the Edge Functions' with Deno, the database's with pgTAP on the local stack), rebuilds the website's engine and compares it with the committed one, checks relative Markdown links and that every workflow action is pinned by commit SHA, and scans the history for secrets.

### Game build watch

A daily workflow (`.github/workflows/game-build-watch.yml`, also runnable by hand from the Actions tab) asks HearthstoneJSON, with one request, which game build it has card data for. If that build is newer than the newest one in `TESTED_BUILDS` (`crates/bg-parser/src/lib.rs`), it reads the card data of both builds and opens one issue, `New game build <N>`: the Battlegrounds cards added, changed and removed, the minions the simulator does not know and the ones it plays that changed, and a checklist (parser fixture and `TESTED_BUILDS`, simulator review, memory reader, card data). An issue with that title, open or closed, is never opened twice. The workflow can only read the repository and open issues. It does not read Blizzard's site; the issue links the official news page.

To see what it would do without opening anything: `python tools/game_build_watch.py`. Add `--open-issue` (needs `gh` and a token) to open the issue. If the simulator has a `CHECKED_BUILDS` list in `crates/combat-sim/src`, the report says when it is behind. The listing can be ahead of the game's own patch day, so read the build number in the issue before acting on it.

### Real-game fixtures

Copy the raw log into `.local/` (never commit it), build the parser and make one fixture per game; the script writes nothing if the log has a game played with the reconnect dev tool (D-043), if a name or id would be left, or if the fixture would read differently from the raw game:

```
cargo build --release -p bg-parser
python tools/make_fixture.py .local/fixtures-raw/Power_old.log --game 3 --session Hearthstone_<date> --out crates/bg-parser/tests/data/real/b<build>_duos.log --parser target/release/bg-parse
```

Check the report it writes next to the fixture (`.json`) by hand, add the build to `TESTED_BUILDS` if it is new (then make the fixture again, so the report has no "not tested" warning), and get the user's OK before committing. Fixture times count from the game's first line (D-046), so durations and APM are tested too. The simulator keeps its own list, `combat_sim::cards::CHECKED_BUILDS`: a build is added there only after every modelled card was compared with that build's card texts; until then the odds of its games carry a "new game build" warning (a test keeps the list inside `TESTED_BUILDS`).

### Backend (local Supabase stack)

Needs Node.js and Docker Desktop. The CLI is pinned in `package.json`.

```
npm ci
npx supabase start                    # first run pulls the images (a few GB)
npx supabase test db                  # pgTAP: schema, row-level security, export
                                       # lockdown.test.sql lists every table, view and function a browser can reach; a new object is closed to clients by default, so a migration that should open one grants it and adds it to that list
deno test supabase/functions/        # delete-account, upload-game, sweep
$env:TAVERN_SUPABASE_LOCAL = "1"; python -m unittest tests.test_supabase_local -v
$env:TAVERN_SUPABASE_LOCAL = "1"; cargo test -p uploader --features http --test local_stack -- --nocapture
pwsh tools/backup_supabase.ps1 -Local # one encrypted file in Documents\TavernLedger-backups (asks for a passphrase)
```

The pgTAP and Deno tests also run in CI, on a stack that starts only what they need; the end-to-end tests run on the developer's machine only. The desktop end-to-end test sends one sign-in email and the website one sends two (sign-up, password reset); the local stack allows 30 an hour, as the hosted project. The daily sweep (files with no row, audit entries of deleted accounts or older than 30 days, sessions beyond the newest 10 per user, expired counters) is the `sweep` function: `POST /functions/v1/sweep` with the service-role key, or with the signed token that `pg_cron` sends every day at 03:17 UTC (an HMAC of the time, valid for five minutes, keyed with the Vault secret; migrations `20261010090000_sweep_schedule.sql` and `20261014180000_sweep_token_signed.sql`; on the local stack the job sends nothing because Vault has no `project_url`). Each finished sweep leaves a row in `private.sweep_runs`.

Hosted project: migrations and the three functions will reach it through the Supabase GitHub integration on each merge to `main` once the user connects it; auth settings (sign-ups closed, site URL, redirect URL, password length) are set in the dashboard. A migration that adds summary columns goes in before the functions that write them (`SUMMARY_VERSION`): until then `put_game_stats` refuses the new keys, games are stored without a summary and the daily sweep fills them once the migration is in. The desktop app has the hosted URL and publishable key built in. Without `-Local`, the backup script backs up the linked hosted project, reading `SUPABASE_URL` and `SUPABASE_SECRET_KEY` (an `sb_secret_…` key; legacy keys are refused) from `.local/supabase.env`; the Free plan has no Supabase backups, so this is the only one. Each run writes one encrypted file, `tl-backups-<UTC time>.tlenc` (TLENC1: PBKDF2-SHA256 600 000 iterations + AES-256-GCM over a zip), into `Documents\TavernLedger-backups` (`-OutDir` changes it) and keeps files for `-KeepDays` (35). The plaintext only exists in a temp folder outside the repo that is deleted when the run ends, also on failure. The passphrase (12 characters or more) is asked twice, or read from `TAVERN_BACKUP_PASSPHRASE` for scheduled runs; it is never written anywhere, and without it a backup cannot be opened. Open one with `decrypt-backups.ps1` from the same folder. Plaintext folders that older versions left in `.local/backups/` can be deleted once one encrypted backup has been checked; until then the script still removes the ones older than `-KeepDays`. If Documents is synced to OneDrive, a deleted backup can stay in its recycle bin for a while; it is encrypted, so that is not a leak.

### Website (`web/`)

Needs Node.js 22.12 or later. Copy `web/.env.example` to `web/.env` and fill in the local stack's API URL and anon key from `npx supabase status` (only the public key: the build refuses any other). `PUBLIC_SIGNUPS_OPEN=true` shows the sign-up form; unset, as on the live site, `/signin/` offers sign-in only. `PUBLIC_TURNSTILE_SITE_KEY` is the anti-robot widget's site key on the forms that sign in, sign up or send an email: with the two Supabase settings set, the build stops without it; locally and in CI it is Cloudflare's published test key (`1x00000000000000000000AA`, always passes), on the live site the `tavernledger.net` widget's key. The local stack has CAPTCHA off, so it ignores the token (`[auth.captcha]` in `supabase/config.toml` says how to turn it on with Cloudflare's test secret).

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

### Minion drawing (shared with the website)

`crates/desktop/ui/minion-art/` draws a minion like the game from the game's texture set (frame, gems, one layer per keyword; Windfury and Stealth live with WebGL). It is a plain ES module with no app API inside: `minion-art.js` (`createMinionArt`), `layout.js` and `fx.js`, plus `minion-art.css`. The app's side is `crates/desktop/ui/minion-art-app.js`; the website imports the same files and passes its own texture URL. `node --test "ui-tests/minion-art.test.js"` checks the layer order, the fallbacks and that no page API creeps in.

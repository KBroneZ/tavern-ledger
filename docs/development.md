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

To see the overlay on a replayed game, follow the replayed folder with the app and force the overlay on (`--overlay-dev` turns it on, shows it whatever window is in front and does not save the on/off choice (layout and theme are saved in the data folder you give; `--overlay-unlock` starts it unlocked, and `--overlay-preview`, only together with `--overlay-dev`, starts with the tray's preview open); `--logs-dir` is for this and nothing else):

```
cargo run -p desktop -- --data-dir "$env:TEMP\replay\data" --logs-dir "$env:TEMP\replay\Logs" --overlay-dev
```

The overlay's default stack, at the right edge of the game's picture, is the status box (only when it has something to say), the opponents panel with the lobby's tribes as its header, then the counters (only while there is one). While the overlay is unlocked, a "?" next to "Layout editing" shows the legend. Ctrl+Shift+H, or "Hide overlay" in the tray menu, hides everything the overlay draws and shows it again; the key is registered only while `Hearthstone.exe` runs (always in an `--overlay-dev` run), hides only what is on screen (showing again always works), and a short note says which key brings it back; hiding lasts until the game ends, and unlocking a hidden overlay only shows it. "Preview overlay with example data" in the tray menu shows the overlay with its example data over the desktop, click-through; it does not open, and closes, while Hearthstone runs. Neither the hidden state nor the preview is saved.

### Overlay gallery

To look at every overlay panel and state without a game (also from a phone), build the gallery. It needs only Node.js, no extra package:

```
node tools/overlay-gallery/build.js
```

This writes one self-contained page, `.local/overlay-gallery/overlay-gallery.html` (about 0.6 MB, never committed). It runs the real `crates/desktop/ui/overlay.html` and its scripts in a browser, with a stand-in for the app's bridge (`tools/overlay-gallery/bridge.js`) that answers from invented example data (`tools/overlay-gallery/scenarios/`), one scenario per frame, each marked "Example" (the hover scenarios `hover-keywords`, `hover-golden`, `hover-textures-none`, `hover-texture-bad` and `hover-no-webgl` show the minion drawing and its fallbacks; `legend-open` is the layout editor with the legend open, `hide-all` hides and shows the overlay over and over, `preview` is the tray's preview) and captioned in English and Spanish. Open the file in a browser; add `?lang=es` for Spanish or `?scenario=<id>` (for example `?scenario=hover-duos`) for one frame. On a narrow screen the frames start zoomed in on the overlay.

`node tools/overlay-gallery/build.js --layouts` builds the layout comparison instead, `.local/overlay-gallery/overlay-layouts.html`: the same example games under each layout variant in `tools/overlay-gallery/layouts/` (the counters panel above, below or beside the opponents list, a hide-all key), plus the page as it ships (variant B, chosen in that round), in Solo and Duos, a Recruit phase and combat, at 1920×1080 and 2560×1440, over plain shapes standing in for the game window. A variant is applied inside the gallery's frame before the page's scripts run (it turns the counters panel on and adds its styles there); the overlay page and its files are not changed.

The build only reads the overlay page. When the page asks for a command or an event the stand-in does not answer, or the theme colours in `overlay_layout.rs` change, `node --test "ui-tests/*.test.js"` (from `crates/desktop`) fails until `bridge.js` or `theme-vars.js` follows. A new panel slot of the page needs a scenario that shows it; the counters scenario shows while the page's `data-panel="counters"` slot is switched on.

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

Needs the same Rust toolchain as the app, Node.js (the version in `web/.node-version`) and an internet connection. From a clean checkout:

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
- **WebView2:** the bootstrapper is embedded (about 1.8 MB) and runs silently only when WebView2 is missing (Windows 11 and up-to-date Windows 10 have it); it still needs internet to fetch the runtime. Before any window, both editions check that a runtime is usable: the version the registry lists (`pv`) is above 0.0.0.0 and the folder it points at (`EBWebView` under `ClientState`, else `location` and the version) holds `msedgewebview2.exe`. A registration whose files are not there (stale: the bootstrapper skips it because the key exists) or no registration at all shows a message in English and Spanish with Microsoft's page, logs one error line and exits, instead of the window library's raw dialog. A runtime chosen on purpose (a fixed runtime or channel through `WEBVIEW2_BROWSER_EXECUTABLE_FOLDER`, `WEBVIEW2_RELEASE_CHANNEL_PREFERENCE`, `WEBVIEW2_CHANNEL_SEARCH_KIND`, or anything under the `Software\Policies\Microsoft\Edge\WebView2` policy key) is never checked. Checked in Windows Sandbox on 2026-10-20 (the image registers 155.0.4283.45 with files 155.0.4283.62): the stale case shows the message before any window and no panic line; with the registry repaired the window opens; with the keys deleted the "not installed" message shows. Microsoft's standalone installer does not repair that stale image (it exits 0x80040902: a newer version is already there), so the message says to try it and then update Windows.
- **Upgrade:** a newer installer replaces the app. Its "Already installed" page defaults to "Uninstall before installing": it runs the installed copy's uninstaller (asking OK before it closes the running app, that copy only, by its path, never a build run from source), then installs the new version. `%APPDATA%\TavernLedger` is kept (checked on 0.2.3 to 0.2.4 in a clean Windows Sandbox).
- **Your data:** install, upgrade and uninstall keep `%APPDATA%\TavernLedger` (history, settings, upload settings, overlay layout, card cache, logs) unless you tick the uninstaller's box, "Also delete my games and settings (cannot be undone)" (unticked by default, and unticked in a silent `/S` uninstall; `installer/English.nsh` has the label, `installer/hooks.nsh` the deletion). Ticked, the uninstaller removes that folder, the folders named after the app's identifier, `%APPDATA%\dev.tavernledger.app` (unused) and `%LOCALAPPDATA%\dev.tavernledger.app` (the window's WebView2 cache, shared with builds run from source), and the installer's own registry keys. An update (the in-app updater runs the old uninstaller in update mode) never deletes any of it. Uninstalling also removes the "Start with Windows" entry, which a build run from source writes under the same name. A history kept elsewhere with `--data-dir` is never touched. If `%APPDATA%\TavernLedger` is a junction or a link the player made, only the link is removed, never what it points at (a link inside the folder too: the deletion is `rd /s /q`, not NSIS's `RmDir /r`, which follows links). The box also shows in the old uninstaller that the installer's "Uninstall before installing" choice runs (it is not an update there), so ticking it during an upgrade would wipe the data before the new version goes in. It also removes the data a build run from source shares with the installed app (the folder is the same), and a file held open by another running copy stays.
- **Not signed:** Windows SmartScreen warns before it runs, so a published build should list its SHA-256 next to the download (`Get-FileHash "<setup file>"` to compare). Updates are signed separately, for the app's updater (above).

## Microsoft Store package

The same app can be built as an MSIX package for the Microsoft Store. It is a separate edition, made by the `store` cargo feature and nothing else (`crates/desktop/src/edition.rs`; the app never guesses it at run time). Differences from the installer edition:

- **No self-update:** the updater plugin is not compiled in or registered, and Settings shows nothing about updates. The Store delivers updates.
- **Start with Windows** uses the package's start-up task (`windows.startupTask`), not a `Run` entry, which a package's registry virtualization would hide from Windows. A task the player turned off in Task Manager, or a policy, cannot be turned on from the app, and Settings says so.
- **Fix button:** a package's new files under `%LOCALAPPDATA%` can be redirected to the package's private store, where the game never looks. The `full` variant's manifest excludes `%LOCALAPPDATA%\Blizzard` and `%APPDATA%\TavernLedger` from that (and asks for the `unvirtualizedResources` capability, which Microsoft decides at certification), so Fix writes where the game reads and the history is the installer edition's folder. Fix is hidden before Windows 11. After a write the app looks for a copy in the private store and reports it as not done if there is one. The `manual` variant (`store-manual-setup` feature on top of `store`) asks for no restricted capability and has no Fix button: the setup box lists the lines to write by hand, and without an existing `%APPDATA%\TavernLedger` the history stays in the package's private store, which uninstalling the Store app deletes. Excluded folders stay after uninstall.

To build an unsigned package (needs the Windows SDK for `makeappx.exe`, and `npm ci --ignore-scripts` once):

```
pwsh tools/make-msix.ps1 [-Variant full|manual]
```

It builds into `target\store` (the installer edition's `target\release` is never packed), checks that the exe is the Store edition and holds no updater endpoint or key (`tools/store_package.py check-exe`), lays out the package folder (exe, logos from `crates/desktop/store/Assets`, `AppxManifest.xml` made from `crates/desktop/store/AppxManifest.xml.template`) and packs `target\store-package\TavernLedger_<version>_x64.msix`. Nothing is signed, uploaded or submitted: the Store signs what is uploaded in Partner Center.

- **Version:** the Store refuses a first part of 0 and reserves the fourth, so the package version is `(major+1).minor.patch.0` (`0.2.4` is `1.2.4.0`; `tools/store_version.py`). The installer edition's version is unchanged.
- **Identity:** name, publisher and publisher display name come from Partner Center (`-IdentityName`, `-Publisher`, `-PublisherDisplayName`, or the `STORE_IDENTITY_NAME`, `STORE_PUBLISHER` and `STORE_PUBLISHER_DISPLAY_NAME` variables). Without them the package carries a clearly fake local-test identity, and `-ForSubmission` refuses it.
- **Logos:** `python tools/build_icons.py --store` redraws them from `docs/brand/logo/`.
- **WebView2:** the package holds no runtime; the Evergreen runtime is on Windows 11 and nearly all Windows 10 PCs. The same check as the installer edition runs before any window.
- **CI:** the `Store package` workflow (by hand, from the Actions tab) builds the same package on a runner and keeps only the `.msix` as a workflow artifact for 7 days.

To try a package on a PC, a person must install it signed with a certificate the PC trusts (or in Developer Mode); do that only in a throwaway setup such as Windows Sandbox or with a test certificate removed afterwards, never next to the installed app's data. `-LocalTest` builds a debug package with a launch alias so a test can pass `--data-dir` and `--local-app-data`; it is never for submission.

## Development

```
python -m pip install --require-hashes --only-binary=:all: -r requirements.txt
python -m unittest discover -s tests
cargo test --workspace
```

CI runs the tests (the desktop app's on Windows, the Edge Functions' with Deno, the database's with pgTAP on the local stack), rebuilds the website's engine and compares it with the committed one, checks relative Markdown links and that every workflow action is pinned by commit SHA, and scans the history for secrets.

### Known advisories

`.github/workflows/advisories.yml` runs in CI (part of `check`), and `advisories-daily.yml` runs it every day on `main`; a failing daily run comments on the issue "Advisory scan failing on main" (or opens it). It asks [osv.dev](https://osv.dev) about every package pinned in `Cargo.lock`, `package-lock.json`, `web/package-lock.json` and `requirements.txt`, through osv-scanner (a pinned release, checked against its SHA-256), and `tools/check_advisories.py` decides: an advisory with a fixed version, of medium severity or higher, or of unknown severity fails; a low one with no fix is listed and passes. A scanner or network error, or a lockfile missing from the results, fails too. The scanner gets an empty config, so an `osv-scanner.toml` cannot hide an advisory, and a tracked one fails the check; so does a tracked lockfile that is neither scanned nor listed in `UNSCANNED` with a reason (today only the throwaway `spikes/overlay-tauri/Cargo.lock`). The Edge Functions have no lockfile because they import nothing from outside the repo; the check fails if one gets an `npm:`, `jsr:`, URL or bare-name import (only relative paths and `node:` built-ins pass), a relative import of code outside `supabase/functions` (JSON data may be outside), a Deno config, `package.json` or lockfile, or an `import_map` or an `entrypoint` outside `supabase/functions` in `supabase/config.toml`, so that cannot slip by unscanned. That check is a tripwire for an honest new dependency, read with regular expressions, not a JavaScript parser: code written to hide one (`eval`, `new Function`, `importScripts`, a template literal built to confuse the comment remover) is for review to catch.

An advisory that does not apply goes in `tools/advisory_ignores.toml` with its id, package, reason, evidence and a review date at most six months away; the check fails once the date passes, or when no advisory matches the entry any more. A crate's entry also needs `absent_from`, the builds it must stay out of (a target, minus some workspace crates), and the check reads `cargo metadata` for each, so the ignore stops holding the day the crate gets into one of them. To run it locally, download osv-scanner and pass it: `python tools/check_advisories.py --scanner <path>`.

Not covered: the toolchains themselves (Node.js in `web/.node-version`, Rust, Python, Deno), the GitHub Actions (Dependabot proposes their bumps), the Supabase Docker images and osv-scanner itself (bump `OSV_SCANNER_VERSION` and its SHA-256 by hand).

### Game build watch

A daily workflow (`.github/workflows/game-build-watch.yml`, also runnable by hand from the Actions tab) asks HearthstoneJSON, with one request, which game build it has card data for. If that build is newer than the newest one in `TESTED_BUILDS` (`crates/bg-parser/src/lib.rs`) and is not listed in `CLIENT_BUILDS` (next to it), it reads the card data of both builds and opens one issue, `New game build <N>`: the Battlegrounds cards added, changed and removed, the minions the simulator does not know and the ones it plays that changed, and a checklist (parser fixture and `TESTED_BUILDS`, simulator review, memory reader, card data). An issue with that title, open or closed, is never opened twice. The workflow can only read the repository and open issues. It does not read Blizzard's site; the issue links the official news page.

To see what it would do without opening anything: `python tools/game_build_watch.py`. Add `--open-issue` (needs `gh` and a token) to open the issue. If the simulator has a `CHECKED_BUILDS` list in `crates/combat-sim/src`, the report says when it is behind. The listing can be ahead of the game's own patch day, so read the build number in the issue before acting on it. The listing numbers the client, but a game writes its own number to `Power.log` (`BuildNumber=`), and a client build can write an older one: the 255130 client writes 254477. `TESTED_BUILDS` and the simulator's `CHECKED_BUILDS` hold the number the logs write; `CLIENT_BUILDS` pairs a client build with the log build its games write, and a pair is added only after real games on that client were read clean (a test keeps its log build inside `TESTED_BUILDS`). To find a client's build: `.product.db` in the game folder, or `Hearthstone/<version>` in `Net.log`.

### Simulator conformance corpus

Every logged combat can become a frozen test case for the combat simulator. `combat-corpus` reads your own saved games and writes one JSON line per combat that has a result: the build, Solo or Duos, both sides' boards, kits and game-long values, the result and the saved combat replay. Names, the lobby, the session string, when the game was played and the shop are left out; a key outside a fixed list, or a string that is not a card id or a game word, stops the export instead of being written. No card data goes in the file: whoever reads it gives each combat the `cards.json` of its own build.

```
cargo run --release -p combat-sim --bin combat-corpus -- --history "%APPDATA%\TavernLedger\games.jsonl" --out .local/corpus.jsonl.gz
```

The file is gzip-compressed when its name ends in `.gz`. Keep it under `.local/` (never commit a corpus made from real games without the project owner's OK, as with real-game fixtures).

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

The pgTAP and Deno tests also run in CI, on a stack that starts only what they need; the end-to-end tests run on the developer's machine only. The desktop end-to-end test sends one sign-in email and the website one sends two (sign-up, password reset); the local stack allows 30 an hour, as the hosted project. The daily sweep (files with no row, audit entries of deleted accounts or older than 30 days, sessions beyond the newest 10 per user, expired counters) is the `sweep` function: `POST /functions/v1/sweep` with the service-role key, or with the signed token that `pg_cron` sends every day at 03:17 UTC (an HMAC of the time, valid for five minutes, keyed with the Vault secret; migrations `20261010090000_sweep_schedule.sql`, `20261014180000_sweep_token_signed.sql` and `20261023100000_sweep_token_transition_over.sql`, after which the bare Vault secret is refused; on the local stack the job sends nothing because Vault has no `project_url`). Each finished sweep leaves a row in `private.sweep_runs`.

Hosted project: migrations and the three functions will reach it through the Supabase GitHub integration on each merge to `main` once the user connects it; auth settings (sign-ups closed, site URL, redirect URL, password length) are set in the dashboard. A migration that adds summary columns goes in before the functions that write them (`SUMMARY_VERSION`): until then `put_game_stats` refuses the new keys, games are stored without a summary and the daily sweep fills them once the migration is in. The desktop app has the hosted URL and publishable key built in. Without `-Local`, the backup script backs up the linked hosted project with the stored project URL and secret key (below); the Free plan has no Supabase backups, so this is the only one. Each run writes one encrypted file, `tl-backups-<UTC time>.tlenc` (TLENC1: PBKDF2-SHA256 600 000 iterations + AES-256-GCM over a zip), into `Documents\TavernLedger-backups` (`-OutDir` changes it) and keeps files for `-KeepDays` (35). The plaintext only exists in a temp folder outside the repo that is deleted when the run ends, also on failure. The passphrase (12 characters or more) is asked twice, or read from `TAVERN_BACKUP_PASSPHRASE` for scheduled runs; it is never written anywhere, and without it a backup cannot be opened. Open one with `decrypt-backups.ps1` from the same folder. If Documents is synced to OneDrive, a deleted backup can stay in its recycle bin for a while; it is encrypted, so that is not a leak.

The hosted project's secret key (`sb_secret_…`; legacy keys are refused) is stored with Windows DPAPI for the current user in `%LOCALAPPDATA%\tavern-ledger-dev\supabase-secret.json`, outside the repo, in a folder only that user can open; the project URL sits next to it in plain text (it is not a secret). `tools/backup_supabase.ps1` and `tools/sim_consented_games.ps1` decrypt it into memory when they run; they never read a plaintext file, and with no store they stop and say how to fill it. DPAPI protects the key at rest: backups and disk images of the PC, other Windows users, an upload or `cat` of the repo folder. It does not stop a program running as the same Windows user, AI sessions included, from decrypting it.

```
pwsh tools/supabase_secret.ps1                   # move .local/supabase.env into the store (once; safe to re-run), then check
pwsh tools/supabase_secret.ps1 -DeletePlaintext  # the same, then asks before deleting .local/supabase.env
pwsh tools/supabase_secret.ps1 -Check            # decrypt in memory and check
pwsh tools/supabase_secret.ps1 -SetKey           # after a key rotation: asks for the new key (hidden), checks it, asks before replacing
```

The move reads only `SUPABASE_URL` and `SUPABASE_SECRET_KEY` from the file, as data; any other line stops it, so nothing is lost when the file goes. A different key already in the store is never overwritten (only `-SetKey` replaces it, after asking). The check is one harmless authenticated read, `GET <SUPABASE_URL>/storage/v1/bucket/games` with the key in `apikey` and a 15-second timeout: it expects HTTP 200 with the private `games` bucket's settings, returns no user data and writes nothing. The store only opens for the Windows user that wrote it: on a new PC or user account, copy the key from the Supabase dashboard into a new `.local/supabase.env`, run the move and then `-DeletePlaintext`.

### Website (`web/`)

Needs Node.js: the version in `web/.node-version`, or another release `engines` in `web/package.json` allows (npm refuses the others, `web/.npmrc`). Copy `web/.env.example` to `web/.env` and fill in the local stack's API URL and anon key from `npx supabase status` (only the public key: the build refuses any other). `PUBLIC_SIGNUPS_OPEN=true` shows the sign-up form; unset, as on the live site, `/signin/` offers sign-in only. `PUBLIC_TURNSTILE_SITE_KEY` is the anti-robot widget's site key on the forms that sign in, sign up or send an email: with the two Supabase settings set, the build stops without it; locally and in CI it is Cloudflare's published test key (`1x00000000000000000000AA`, always passes), on the live site the `tavernledger.net` widget's key. The local stack has CAPTCHA off, so it ignores the token (`[auth.captcha]` in `supabase/config.toml` says how to turn it on with Cloudflare's test secret).

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

The site is an installable web app (T-637): `manifest.webmanifest` in each language, icons from our own mark (`public/icons/`; the PNGs are rendered from `public/favicon.svg` and `public/icons/icon-maskable.svg`), and, in the accounts-open build only, a service worker (`/sw.js`, written by the build from `src/lib/pwa/`) and `public/pwa.js`, which registers it and shows the footer's "Add to home screen" line. The worker keeps the shell (both offline pages, the hashed styles and scripts) at install and a copy of each page and file of ours you visit; it never keeps another origin's responses (Supabase, card art), an account, dashboard or sign-in page (`NEVER_CACHED` in `src/lib/pwa/build.ts`, from `NO_STORE_PAGES` and `TURNSTILE_PAGES`), an address with a query, or anything marked `no-store`. The rules are in `src/lib/pwa/rules.js` (unit tests: `test/unit/serviceWorker.test.ts`, which runs the generated worker in a VM). The build writes `/sw.js` last, with a version made from everything it keeps; it is served with `Cache-Control: no-cache` and its own Content-Security-Policy (`connect-src 'self'`). To check it in a real browser, build with the accounts-open settings and run the end-to-end test against Edge or Chrome (it serves `dist/` with the rules of `dist/_headers`, emulates an Android phone, asks the browser whether the site is installable, then stops the server to go offline): `$env:TAVERN_BROWSER = "C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe"; npm run test:local`.

Saved games (T-637, second part): on the games list a player can turn on "Keep my last 10 games on this device" (off by default; the choice is in this browser's `localStorage`, key `tl-keep-games`, holding the id of the player who made it, so it is on for that player only). While it is on, opening one of your own games (never someone else's public game) puts the game's report file in the cache `tl-replays` (`src/lib/pwa/saved.ts`, up to `SAVED_LIMIT`), and the page asks the worker by message to fetch the game page in both languages into the same cache, because the page's policy lets it connect to the account service only. Offline, the worker opens a saved game's address from that page (`savedGame` in `rules.js`; the pages carry `no-store`, so this is the one place the worker keeps a game page), the page reads the saved file when the account service cannot be reached, and the offline page lists the saved games. `tl-replays` does not start with `tl-v-`, so a new build never deletes it. Signing out (any tab, `SIGNED_OUT` in `src/scripts/client.ts`), a session that starts for another player than the one whose games are saved (`forgetOthers`), turning the choice off and the "Delete saved games" button remove it; so does the accounts-closed build's `/sw.js`. Known limit: a saved game's page names the scripts of the build it was saved from, so a new worker fetches the game page again when it activates (if the device is online then), and opening any game online refreshes it too; until one of those happens after a release, a saved game may open as an unstyled page. The offline copy is not tied to the current session, because offline the session cannot always be read.

On Cloudflare, `/sw.js` is served with `Cache-Control: max-age=14400` in place of the `no-cache` this build sets, because the zone's Browser Cache TTL rewrites the header of static files. It does no harm (the page registers the worker with `updateViaCache: "none"`, so the browser skips its HTTP cache when it checks for a new worker), but the setting "Respect Existing Headers" under Caching > Configuration would make the header what the build says.

### Minion drawing (shared with the website)

`crates/desktop/ui/minion-art/` draws a minion like the game from the game's texture set (frame, gems, one layer per keyword; Windfury and Stealth live with WebGL). It is a plain ES module with no app API inside: `minion-art.js` (`createMinionArt`), `layout.js` and `fx.js`, plus `minion-art.css`. The app's side is `crates/desktop/ui/minion-art-app.js`; the website imports the same files (`web/src/lib/art/`: `textureSet.ts` holds the set's version, `https://assets.tavernledger.net/<version>/` as the only host its policy allows in `img-src`, and a copy of that version's `layout.json`, since the policy lets pages fetch only from Supabase; a test fails if the version differs from the app's `VERSION` in `crates/cards/src/textures.rs`). Pages carry a minion as `data-tlm-*` attributes and an empty `.tlm-slot`; once the textures load the drawing fills the slot, and with no textures, no WebGL or a broken file the page keeps its own look (one console warning at most). `node --test "ui-tests/minion-art.test.js"` checks the layer order, the fallbacks and that no page API creeps in.

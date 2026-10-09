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

Backend (T-104a): the database schema, row-level security, export, account deletion and backups exist and are tested on a local Supabase stack. Clients can only read their own data and public profiles; game uploads go through one server function. **Hosted project** (session 022): Supabase in Frankfurt, sign-ups closed, nothing deployed yet; once Supabase's GitHub integration is connected (pending the user), each merge to `main` applies the migrations and deploys the server functions, and a daily job in the database runs the clean-up. Plan and live state: [docs/research/deploy.md](docs/research/deploy.md) section 10.

Robustness (session 014): the app and `check_logs.py` check that the game is set up to write a complete log (`log.config` and `client.config`), every saved game records which parser version read it, `tavern-watch --reparse` re-reads old sessions after a parser fix, and a dev tool replays a saved log so the app can be tested without playing.

Website (T-104c): a static Astro site in `web/` with sign-in by email, an account page (display name, public profile switch, download all your data, delete your account) and a public profile page that only exists when you turn it on. It shows no ratings and no other players' names. **Online at [tavernledger.net](https://tavernledger.net) with accounts closed** (session 021): home, privacy and terms drafts; sign-in, account and profile say "Accounts are not open yet" until the hosted backend is wired in. Hosted on Cloudflare Pages from `main`, no scripts, no analytics. Contact mail: `contact@` and `privacy@tavernledger.net` (D-030); deploy notes in [docs/research/deploy.md](docs/research/deploy.md).

Upload (T-104d): the desktop app can upload each finished game to your account. It is off until you sign in and turn it on. You sign in with a one-time link from your email, opened in your browser; the app never asks for your password and keeps the sign-in in Windows Credential Manager. Only the game's report goes up (heroes, card ids, places, boards), never player names, BattleTags or ratings, and the server checks that again before storing it. The app uses the hosted project unless its data folder has a `server.json` (for the local stack).

Game recap (T-202) and record against each opponent (T-203): a "Recap" button on each game, and it opens by itself when a new game ends. It shows hero (and teammate in Duos), place, health over the rounds, tribes seen in the tavern and the parser's warnings, and for each opponent in that game (by hero and seat, never by name) the combats won, lost, tied or unknown. The log never says who won, so results are worked out from health changes and marked inferred; when the numbers cannot say, the result is unknown. In Duos each round counts once for your team against the opposing team. Games saved before parser revision 2 show unknown results until re-read with `tavern-watch --reparse`.

Where values come from (T-109): every number or label in the window says where it comes from. Values straight from the game's log carry no mark; inferred ones (tribes seen in the tavern, skins grouped under one hero) and unknown ones carry a mark, every value has a tooltip, and a legend explains them. "Report a problem" (T-110): a button on each game shows the whole report file (game report, parser version, game build, app version, log setup check, warnings) and saves it to your Downloads folder if you choose. The app refuses to make it if it finds anything that looks like a player name, and it never sends it anywhere; you send it yourself.


Lobby tribes by hand (T-303): the log never says which five tribes are in the lobby, so you can pick them yourself from a fixed list, at hero select or later from the history (`Tribes` button on each game; you can change, clear or move the entry). If no game is on when you pick them, they go to the next game that starts. They are saved on your computer next to the history, not inside the game's report, so re-reading a game keeps them. They are always marked "entered by you" and shown next to the tribes seen in the tavern, never mixed with them; the stats count them in a separate table. Decision D-029.

Overlay (T-301): a small transparent window on top of the game with three panels: the tribes seen in the tavern (inferred) and the ones you entered (entered by you), each opponent by hero (health, the last board of theirs that entered play with the round it was seen, and your record against them, inferred from health), and a one-line status. It is off until you tick "Show overlay during a game" in the tray menu (the choice is kept), and it only shows while a Battlegrounds game is on and Hearthstone is the window in front (read from the front window's title and class only; nothing is opened or read in the game). Clicks go through it and it never takes focus. Every source is written on the panel (no mark: from the log); unknown values show `?`, an opponent not fought yet shows "not fought", and a game the parser cannot read says so instead of guessing. Boards show card ids and attack/health: the log names heroes only. **Exclusive fullscreen hides any overlay: run Hearthstone windowed or borderless.** Decision D-032.

Customizable overlay (T-302): from the tray, "Unlock overlay to move and resize panels" lets you drag the panels, resize them by their corner, pick a theme (Tavern dark, Parchment light, High contrast), set the opacity (80-100%), choose which panels show and reset the layout; the bar at the top has a "Lock overlay" button. While unlocked the overlay takes every click on the screen (a dim layer says so) and shows example data if no game is on. It locks itself again when a game starts and is always locked when the app starts. Your layout is saved per screen resolution in `overlay.json` and kept on screen if the resolution changes; if that file is broken the defaults are used and the app says so. There is no hotkey. Decision D-033.

Privacy and terms (T-104b): a data inventory written from the code, and drafts of the privacy policy and terms in [docs/legal/](docs/legal/), shown on the site at `/privacy/` and `/terms/`. They are not in force: the controller's name, some provider details and the legal checks of the email sender are still to be decided; the contact addresses exist. No analytics, minimum age 16.


Next tasks, in the user's order (2026-10-10; hub prompt [prompts/sessions/025-hub.md](prompts/sessions/025-hub.md)): (1) card and hero images in the overlay and the app (T-304, session 026); (2) redesign of the desktop app and overlay (T-305, session 027, after 026); the website redesign (T-104e, session 028) is done: direction "Combat Round", D-040; (3) an auth email sender that does not rewrite links (T-104f, session 029), which blocks opening sign-ups (D-037); then the follow-ups from session 022, T-D01, T-108, and what waits on the user (T-101, T-105, opening sign-ups).

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

To see the overlay on a replayed game, follow the replayed folder with the app and force the overlay on (`--overlay-dev` turns it on, shows it whatever window is in front and does not save the on/off choice (layout and theme are saved in the data folder you give; `--overlay-unlock` starts it unlocked); `--logs-dir` is for this and nothing else):

```
cargo run -p desktop -- --data-dir "$env:TEMP\replay\data" --logs-dir "$env:TEMP\replay\Logs" --overlay-dev
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
deno test supabase/functions/        # delete-account, upload-game, sweep
$env:TAVERN_SUPABASE_LOCAL = "1"; python -m unittest tests.test_supabase_local -v
$env:TAVERN_SUPABASE_LOCAL = "1"; cargo test -p uploader --features http --test local_stack -- --nocapture
pwsh tools/backup_supabase.ps1 -Local # backup into .local/backups/
```

These run on the developer's machine, not in CI. The desktop end-to-end test sends one sign-in email; the local stack allows two an hour. The daily sweep (files with no row, audit entries of deleted accounts, expired counters) is the `sweep` function: `POST /functions/v1/sweep` with the service-role key, or with the sweep token that `pg_cron` sends every day at 03:17 UTC (migration `20261010090000_sweep_schedule.sql`; on the local stack the job sends nothing because Vault has no `project_url`). Each finished sweep leaves a row in `private.sweep_runs`.

Hosted project (session 022, [deploy.md](docs/research/deploy.md) section 10): migrations and the three functions will reach it through the Supabase GitHub integration on each merge to `main` once the user connects it; auth settings (sign-ups closed, site URL, redirect URL, password length) are set in the dashboard. The desktop app has the hosted URL and publishable key built in. Without `-Local`, the backup script backs up the linked hosted project, reading `SUPABASE_URL` and `SUPABASE_SERVICE_ROLE_KEY` from `.local/supabase.env`; the Free plan has no Supabase backups, so this is the only one.

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

## License

MIT. See [LICENSE](LICENSE).

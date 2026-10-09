# Plan

Statuses: `pending` · `in progress` · `done` · `blocked`.

## F0 — Feasibility proof (40–80 h estimated)

Goal: show that a Battlegrounds game can be rebuilt from local logs alone.

| ID | Task | Status | Acceptance criteria |
|----|------|--------|---------------------|
| T-001 | Choose a name (P-001) and create the GitHub repo (with the user's OK) | done | Remote repo with README, MIT LICENSE and `.gitignore`. |
| T-002 | Enable logs (`log.config`) and collect 10+ own games as fixtures (Solo and Duos) | in progress | Fixtures in `fixtures/` with no third-party data that isn't public; recorded in `PROVENANCE.md`. |
| T-003 | Parser prototype with `hslog` (Python, MIT) to see what `Power.log` exposes in BG | done | Report: hero, lobby tribes, boards per round, opponents, final placement, Duos. What is missing. |
| T-004 | Research MMR sources (P-007): local logs + public leaderboard | done | Table of which data comes from where, with evidence. |
| T-005 | Decide the desktop stack (P-002) | done | Decision in `DECISIONS.md` with reason. |
| T-006 | Leaderboard client with cache and limits | done | Tests with synthetic responses (real responses are not stored, D-012); timeout, errors and the four D-011 states explicit. |

**Notes T-002 (2026-10-08).** `tools/check_logs.py` confirms that `log.config` already enables `Power` (LogLevel=1, FilePrinting, Verbose). The logs are in `C:\Battle.net\Battle.net\Hearthstone\Logs\Hearthstone_<date>\`. Inventory: 6 sessions (October 2 to 8), 23 game starts, all `GT_BATTLEGROUNDS_DUO`, build 253216. None of Solo. The game leaves a `Power_old.log` (120–563 MB per session). Missing: play Solo games and trim each game into a small fixture (needs the T-003 parser) and remove names of players that aren't public.

**Notes T-002 (2026-10-08, session 002).** No new Solo games. The T-003 tests use synthetic logs; the trimmed fixtures from own logs stay pending (they need the user's OK before being committed).

**Notes T-002 (2026-10-09, session 003).** First Solo game (`GT_BATTLEGROUNDS`, build 253216, session `Hearthstone_2026_10_09_00_26_56`). `parse_bg.py` reads it as `ok` but with two failures to investigate: the final health is 30 even though the player finished 7th (eliminated), and health goes up from 12 to 30 in the last round; also, another lobby player shows up with placement 7 too. Parser not touched in this session.

**Notes T-002 (2026-10-09, session 004).** `check_logs.py`: no new games (6 sessions, 22 Duos and 1 Solo).

**Notes T-002 (2026-10-09, session 005).** `check_logs.py`: no new games (6 sessions, 22 Duos and 1 Solo). Cause of the two Solo failures: when eliminated, the game copies the local player's hero (`COPIED_FROM_ENTITY_ID`, old placement, `DAMAGE=0`) and the parser read the copy. The player finished 8th, not 7th; the other "7" was correct. The same happened in the 12 lost Duos games (health 30; wrong placement in 4). Fixed in `parse_bg.py` with synthetic tests; minimum health 0. Solo (`GT_BATTLEGROUNDS`) becomes a proven type. Details in [`parser-hslog.md`](../research/parser-hslog.md). Remaining: more Solo games and the placement of opponents still alive when the player falls (it is the placement at that moment, not the final one).

**Notes T-005 (2026-10-09, session 006).** Report [`desktop-stack.md`](../research/desktop-stack.md): Tauri 2, .NET (WPF/Avalonia) and Electron compared on installer, overlay, UI shared with the web, auto-update and licenses for SignPath. Recommendation: Tauri 2 (Rust + TS); plan B WPF. Overlay test in `spikes/overlay-tauri/`: transparent, always on top and clicks pass through to the window below (checked with a real click, debug and release; 8.5 MB `.exe`). Also tested on top of Hearthstone in borderless fullscreen, without sending input to the game: the overlay stays on top and clicks go to the game. The user picks Tauri (D-014).

**Notes T-003 (2026-10-08).** `tools/parse_bg.py` + report [`docs/research/parser-hslog.md`](../research/parser-hslog.md). 23 of 23 real games (Duos, build 253216) in `ok`. Everything comes out except MMR and the exact tribe list (only inferred from the shop); 12% of the partner's combats in Duos are not visible in the local log. Solo untested with real logs.

**Notes T-004 (2026-10-09).** Report [`docs/research/mmr-sources.md`](../research/mmr-sources.md). No local log carries the MMR value; `Net.log` only marks when `NetCacheBaconRatingInfo` arrives (after each game). Leaderboard: endpoint, parameters, cutoff at 8000 and name matching documented. `[Net]` with `Verbose=true` tested with the user's OK (change made by them): it does not expose the rating. New pending P-008 (website terms). P-007 closed with D-011: leaderboard rating if it appears; otherwise, "below the cutoff" (< 8000).

**Notes T-006 (2026-10-09).** `tools/leaderboard.py` + report [`docs/research/leaderboard-client.md`](../research/leaderboard-client.md). P-008 closed with D-012 (only the user's own row, from the user's app); strategy in D-013. Measured with 10 requests: gzip (308 → 17 KB per page), `ETag` useless (changes on every response), 10–30 players per MMR point near the cutoff. Full Solo EU sweep: 174 requests, ~3 MB, ~7 min, at most once a day. Tests without network (synthetic responses and a local server for the transport). Measurement 10 (01:49): the leaderboard changes in batches, between minutes and under an hour. Remaining: check case and Unicode with the row of a real top user.

## F1 — Local history and minimal web

**Notes T-101 (2026-10-09, session 006/007).** Rust workspace with three crates:
- `crates/bg-parser`: port of the Python parser without `hslog` (same JSON output). Parity checked with 13 synthetic cases (in tests and CI) and with the 23 real games (local, `tools/compare_parsers.py`): identical. 590 MB log: 2.6 s versus 19.8 s in Python. Emits each game as soon as `STATE=COMPLETE` arrives.
- `crates/tracker` + CLI `tavern-watch`: finds the logs folder (registry or `--logs-dir`), follows `Power.log` of the most recent session (complete lines only; rotation to `Power_old.log` and new sessions) and saves to `%APPDATA%\TavernLedger\games.jsonl` (D-015). `--import` reads old sessions: 23 games in 8 s; none of the 149 player names in the logs appears in the history.
- `crates/desktop`: Tauri app (no npm yet) that follows the log in the background and shows the history with stats per mode (Solo 1–8, Duos 1–4 per team; top 4 / top 2). Tested with the real history.

Code and security reviews: no CRITICAL or HIGH; the MEDIUMs, fixed (lines over 1 MiB mark the game as `unsupported`; the follower reads each byte once, detects rotation also by the first bytes and doesn't lose the last line; pending games survive an error; the history withstands a cut write and invalid UTF-8; the app doesn't freeze, warns if the follower thread dies or if `Power.log` is missing; `reg.exe` with a fixed path; stricter CSP).

**Notes T-101 (2026-10-09, session 008).**
- Hero names (D-017): they come from `Power.log` itself, which writes each card's name in the `[entityName=… cardId=…]` references, in the player's client language. No external dataset: the HearthstoneJSON JSONs are "Copyright © Blizzard Entertainment - All Rights Reserved" (the wrapper is CC0) and `hsdata` has no license. The report gains the `card_names` field (only the heroes it mentions; Rust only, the Python prototype stays frozen and parity ignores it). With the 23 real games: 184 of 184 lobby heroes with a name, no player name in the output (checked against the log's `PlayerName` values without printing them) and parity with Python intact. The user's history was re-imported with `--import` (previous copy in `games.jsonl.bak-008`): 23 of 23 games with a name.
- Single instance (D-016): system lock on `games.lock` next to the history (`File::try_lock` from the standard library, no dependencies). The app and `tavern-watch` take it; the second process says so (the app shows the history read-only). Tested with two `tavern-watch`: the second exits with an error and, when the first closes, works again.
- Code and security reviews: no CRITICAL or HIGH. Fixed the MEDIUM they shared: the `cardId` was read outside the entity bracket, so an odd format could pair a BattleTag with a hero, and a long line of broken references cost 15 s (quadratic). Now the `cardId` is only read inside the same bracket and with id characters, BattleTag-shaped names are discarded, only hero ids are saved and the scan is bounded (milliseconds). `rust-version = "1.89"` because of `File::try_lock`.
- Live test: pending; the user hasn't played with the app open since session 007.
- Tray and start with Windows (D-018, with the user's OK for the dependencies): tray icon with "Open Tavern Ledger", "Start with Windows" and "Quit"; closing the window hides it and the app keeps reading the log. Start with Windows is off by default and only turned on from that menu; the startup entry opens the app with `--minimized`, straight into the tray. Review: no CRITICAL or HIGH; fixed the MEDIUMs (if the change fails, the window opens with the warning; with spaces in the path the option is disabled) and the flash when starting minimized (the window is born hidden). For T-105: the installer must use a folder without spaces or write the startup entry with quotes. Checked: the app starts in release and creates no entry in `HKCU\...\Run` until the user turns it on.

Remaining to close T-101: the live test (play a game with `cargo run --release -p desktop` open and see it appear by itself at the end). The signed build is T-105. CI doesn't build `desktop` (it needs system libraries on Linux).

**Notes T-101 (2026-10-09, session 009).** Live test still not done: no Battlegrounds game since session 008 (`check_logs.py`: the newest session folder has no Power log yet). `games.jsonl` holds 46 records for the same 23 games from 5 sessions, each saved twice by two imports; expected, since the store is append-only and the last record per game wins.

| ID | Task | Status |
|----|------|--------|
| T-101 | Desktop app: watch `Power.log` and save games locally | in progress |
| T-102 | Personal stats: heroes, average placements, tribes | done |
| T-103 | Decide web stack and backend (P-003) | done |
| T-104a | Supabase project in the EU (with the user's OK: account, terms, DPA), schema with row-level security (clients read only; one Edge Function writes), export, deletion and weekly backups until Pro | local done; hosted project waits for the user |
| T-104b | Privacy policy, processor list (Supabase, static host, email provider) and terms, before the first user | pending |
| T-104c | Website (Astro): sign-in, account page with export and deletion, public profile (private by default) | pending |
| T-104d | Desktop upload: sign-in through the browser (PKCE, loopback), upload queue with retries, upload off until the user turns it on | pending |
| T-105 | Code signing (P-005) and installer with auto-update | pending |
| T-106 | Log setup check: `client.config` `[Log] FileSizeLimit.Int=-1` (without it the game stops writing a log at about 10 MB, one game) and `log.config` `[Power]`, in `check_logs.py` and in the app, with fix instructions; the app never edits them by itself | done |
| T-107 | Save the parser version and game build with each game; when a parser fix lands, say which saved games to re-read and re-read them from the logs still on disk | done |
| T-108 | Fixtures from more than one game build (trimmed, no third-party names, with the user's OK) as regression tests for patches | pending |
| T-109 | Where each value comes from, shown in the app: from the log, inferred, entered by you, from the leaderboard, unknown | pending |
| T-110 | "Report a problem" bundle: one game's report, parser version, build and the parser's warnings, with names removed; the user sees it and sends it by hand | pending |

**Notes T-104a (2026-10-09, session 012).** Built and tested on the local Supabase stack only; the user chose to create the hosted project later (account, terms, DPA, Frankfurt, Free plan). Supabase CLI 2.118.0 as a pinned npm dev dependency and Docker Desktop, with the user's OK (D-021). Migrations in `supabase/migrations/`: `profiles` (private by default, display name of up to 32 letters, digits, spaces, `_`, `.` or `-`: no `#` or look-alike, so no BattleTag) and `games` keyed by `(user_id, session, game_index)` with checked mode, status, hero card id, place range per mode, build, date, tribes offered, `saved_at` revision, SHA-256, size and a derived storage path; no name, rating or MMR column (a test checks it). Private `games` bucket (64 KiB, `application/gzip`), files at `<user_id>/<session>-<index>.json.gz`. Row-level security: clients read their own rows and the rows and files of public profiles; their privileges are cut down to `select` plus updating their own `display_name` and `is_public`; no client write on games or the bucket. Export: `export_my_data()` (account without password hash or tokens, identities, sessions, second factors, auth events, profile, every game row, file list). Deletion: Edge Function `delete-account` (files through the Storage API, rows, auth user, then the `user_deleted` audit entry that the auth deletion itself writes with the email; repeatable after a failure, except that a failure in that last step leaves the entry until it is removed by hand or by the T-104d sweep). Backups: `tools/backup_supabase.ps1` (roles, schema, data without live sessions or refresh tokens, bucket and a SHA-256 manifest in `.local/backups/`, readable by the current Windows user only, kept 35 days; a failed run leaves no partial folder), checked against the local stack; `.local` should sit on a BitLocker disk, since `data.sql` holds emails and password hashes. Tests: 50 pgTAP (`npx supabase test db`), 14 Deno unit tests for the function, 4 end-to-end tests against the running stack (`TAVERN_SUPABASE_LOCAL=1`; after deletion nothing of the user remains in rows, auth tables, audit log or storage); a run with loosened policies fails 8 of the pgTAP tests. `supabase db lint`: no findings. Not in CI (D-021). When the hosted project exists: mirror the `config.toml` auth settings (password length 10, email confirmation, secure password change, no GraphQL schema), check that the `postgres` role may still delete from `auth.audit_log_entries` there (**unverified**), and add CORS for the site's origin plus a recent sign-in check to `delete-account` (T-104c). Reviews (code and security): no CRITICAL or HIGH; fixed the MEDIUM (the audit entry the auth deletion writes, retention running before the backup, CHECK evaluation order, backups holding live tokens) and most LOW findings. The upload Edge Function, quotas and the daily sweep of files with no row are T-104d.


**Notes T-102 (2026-10-09, session 010).** `tracker::stats` computes, per mode (Solo and Duos apart, other types listed without place numbers): totals, per-hero numbers (games, average place, top-half share, wins; most played first) and tribes seen in the tavern (games and offers). The app gets them through the `game_stats` command; `app.js` only renders. Only finished games with a place inside the mode's range count; unfinished and unreadable games are counted apart and shown as "not counted"; with no counted game the numbers are "—", never 0. Skins grouped by dropping a trailing `_SKIN_<x>` only (D-019): in the real history every such group names the same hero, while ids that share a number but not a prefix (`BG20_HERO_102` and `TB_BaconShop_HERO_102_SKIN_G`) are not merged. `NEUTRAL` and `ALL` in the tavern counts are minions with no tribe and with every tribe. Checked with the real history (22 Duos and 1 Solo; totals match an independent count) and against the 21 `PlayerName` values of the logs: none in the history, code or tests. 16 new unit tests in `tracker` (empty history, incomplete and unsupported games, Solo vs Duos, places out of range, hand-edited records, names, tribes) and 1 in `desktop`. Review: no CRITICAL or HIGH; fixed the MEDIUM (tavern offer counts saturate instead of overflowing) and two LOW (an older refresh can no longer overwrite a newer one; the "does this place count" rule lives in one function).

**Notes T-103 (2026-10-09, session 011).** Report [`web-stack.md`](../research/web-stack.md): requirements from the plan and decisions, four full stacks with free-plan limits, first paid step, EU region and DPA from the providers' pages, and the upload design (one gzipped game per `PUT /v1/games/{session}/{index}`, about 2.5 KB; idempotent by key and hash; backoff retries; nothing silent). Measured on the real history: 8.8-28.8 KB of JSON per game, 1.5-3.3 KB gzipped; the record already holds what the F2 viewer needs. Blizzard's Developer API terms rule out Battle.net login (no paid features, 30-day retention). The user picks Supabase (D-020), plan B Cloudflare. T-104 split into T-104a to T-104d.

**Notes T-106 (2026-10-09, session 014).** `tracker::setup` (Rust, used by the app and `tavern-watch`) and `check_logs.py` check `log.config` `[Power]` (`LogLevel=1`, `FilePrinting=true`, `Verbose=true`) and `client.config` `[Log] FileSizeLimit.Int=-1`, the latter in the game's install folder next to `Hearthstone.exe`. Each file has three different problem states, shown apart: file not found, file unreadable (with the error kind) and setting missing or with another value (the exact lines to add are listed). Read-only: the files are never written. The app shows a "Game log setup" panel with what to change, checked at start and again every 30 s, and hides it when all is fine. `tavern-watch` prints `Setup:` lines, but not with `--logs-dir` (a copy or a replay says nothing about the installed game). The user's setup is fine (`log.config` and `client.config` both correct).

**Notes T-107 (2026-10-09, session 014).** Each saved record now has `parser: {version, revision}` next to `saved_at` (not inside `report`, so reports still compare equal across versions): `version` is the `bg-parser` crate version and `revision` is `bg_parser::PARSER_REVISION`, which is bumped whenever a change makes the parser read the same log differently. The game build was already in `report.build`. Records written before this have no stamp and load as "unknown version" (the app shows it in the tooltip of the game's date), never as an error; so do records with a malformed stamp. `tavern-watch --reparse` re-reads every session in the history whose logs are on disk, saves only games whose report changed (last record wins, D-015) and lists the games it could not re-read: logs gone, a log unreadable, or logs incomplete. If the logs on disk hold fewer games than the history (the game rotated part of them away), game numbers could have shifted, so nothing of that session is overwritten (D-024). Tried on a copy of the real history: 5 sessions read, 0 games changed, 0 not re-read; the 46 existing records load as unknown version. **For T-104d:** the server schema has a `build` column but no parser version. Add a `parser_version` text column (for example `0.1.0+r1`) when the upload exists, so the server can tell which games an older parser wrote; `supabase/` was not touched here.

**Notes T-D02 (2026-10-09, session 014).** `tools/dev/replay_log.py` copies a saved `Power.log` or `Power_old.log` line by line, at `--lines-per-second` (0 = as fast as possible), into `<dest>/Logs/Hearthstone_<now>/Power.log`, and prints the command to follow it with `tavern-watch --logs-dir <dest>\Logs --data-dir <dest>\data`. It refuses a destination inside the game's install folder and prints paths and counts only, never log content. Tried on two real sessions with a running `tavern-watch` (400 000 lines/s, 2 million lines in 5 s): the replayed history had the same games as the original, 1 of 1 and 3 of 3, with reports identical to the saved ones; nothing from the logs was kept. Tests use the synthetic fixtures.

**Notes T-101 (2026-10-09, session 014).** Live test still not done: the history holds the same 23 games from 5 sessions (3, 7, 8, 1 and 4 games), no new game since session 011.

**Notes T-101 (2026-10-09, session 011).** Live test still not done: the history holds the same 23 games from 5 sessions (3, 7, 8, 1 and 4), no new game since session 010, with the app running.

## F2 — Replays and recaps

| ID | Task | Status |
|----|------|--------|
| T-201 | Turn-by-turn game viewer on the web (opponents' boards as seen) | pending |
| T-202 | Recap after each game | pending |
| T-203 | Record against each lobby opponent | pending |

## F3 — Minimal overlay

| ID | Task | Status |
|----|------|--------|
| T-301 | Overlay with tribes seen in the tavern (or entered by the user, labelled so), last seen board of each opponent and record. Needs windowed or borderless fullscreen (exclusive fullscreen hides any overlay) | pending |
| T-302 | Customizable overlay: lock/unlock button to move the panels wherever each user wants, choose which panels show and several themes | pending |
| T-303 | Lobby tribes by hand: the user picks the 5 tribes at hero select; shown as "entered by you", never mixed with the inferred ones | pending |

## F4 — Community and extras

| ID | Task | Status |
|----|------|--------|
| T-401 | Aggregate community stats (only with consent and enough volume) | pending |
| T-402 | Paid extras (P-006) | pending |

## Dev tools

Never shipped to users; for collecting test games and debugging.

| ID | Task | Status |
|----|------|--------|
| T-D01 | Reconnect/unplug dev tool under `tools/dev/` (D-022): hotkey that drops the game's TCP connection to skip combat, by hand, own account, admin; games played with it marked in the history. Needs the user's OK to start | pending |
| T-D02 | Log replay: feed a saved `Power.log` to the tracker at game speed, to test the app and the overlay without playing | done |

## Future ideas (from the GitHub survey)

Not planned yet; each needs its own task and, where noted, a decision.

- Read `LoadingScreen.log` to know when the player leaves a game, if `STATE=COMPLETE` turns out too late for recaps (T-202).
- Combat odds (F4 or later): only with the validation method other projects document (score predictions against real outcomes; replay the real attack order to find the first rule that differs) and showing "not modelled" instead of a number when a card is not supported. See [implementation-notes.md](../research/implementation-notes.md).
- Card images on the web (T-201): only under Blizzard's Fan Content Policy; HearthstoneJSON data is "All Rights Reserved", so it is never bundled in the repo.
- Community stats (T-401) opt-in and off by default, never switched on by an update.

## Out of scope for now

- Combat simulator (very high cost; only if the project is still alive after F4).
- Memory reading (D-004). It is how other trackers get exact lobby tribes at hero select and opponents' names (so their MMR); we show those as inferred or unknown instead.
- Injection or client modification, including the "minion dance": discarded for good (D-006).

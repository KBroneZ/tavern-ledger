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
| T-103 | Decide web stack and backend (P-003) | pending |
| T-104 | Accounts, game upload, public profile; privacy and deletion (GDPR) | pending |
| T-105 | Code signing (P-005) and installer with auto-update | pending |

**Notes T-102 (2026-10-09, session 010).** `tracker::stats` computes, per mode (Solo and Duos apart, other types listed without place numbers): totals, per-hero numbers (games, average place, top-half share, wins; most played first) and tribes seen in the tavern (games and offers). The app gets them through the `game_stats` command; `app.js` only renders. Only finished games with a place inside the mode's range count; unfinished and unreadable games are counted apart and shown as "not counted"; with no counted game the numbers are "—", never 0. Skins grouped by dropping a trailing `_SKIN_<x>` only (D-019): in the real history every such group names the same hero, while ids that share a number but not a prefix (`BG20_HERO_102` and `TB_BaconShop_HERO_102_SKIN_G`) are not merged. `NEUTRAL` and `ALL` in the tavern counts are minions with no tribe and with every tribe. Checked with the real history (22 Duos and 1 Solo; totals match an independent count) and against the 21 `PlayerName` values of the logs: none in the history, code or tests. 16 new unit tests in `tracker` (empty history, incomplete and unsupported games, Solo vs Duos, places out of range, hand-edited records, names, tribes) and 1 in `desktop`. Review: no CRITICAL or HIGH; fixed the MEDIUM (tavern offer counts saturate instead of overflowing) and two LOW (an older refresh can no longer overwrite a newer one; the "does this place count" rule lives in one function).

## F2 — Replays and recaps

| ID | Task | Status |
|----|------|--------|
| T-201 | Turn-by-turn game viewer on the web (opponents' boards as seen) | pending |
| T-202 | Recap after each game | pending |
| T-203 | Record against each lobby opponent | pending |

## F3 — Minimal overlay

| ID | Task | Status |
|----|------|--------|
| T-301 | Overlay with lobby tribes, last seen board of each opponent and record | pending |
| T-302 | Customizable overlay: lock/unlock button to move the panels wherever each user wants, choose which panels show and several themes | pending |

## F4 — Community and extras

| ID | Task | Status |
|----|------|--------|
| T-401 | Aggregate community stats (only with consent and enough volume) | pending |
| T-402 | Paid extras (P-006) | pending |

## Out of scope for now

- Combat simulator (very high cost; only if the project is still alive after F4).
- Memory reading (D-004).
- Injection or client modification, including the "minion dance": discarded for good (D-006).

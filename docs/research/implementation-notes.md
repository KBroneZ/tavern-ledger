# Implementation notes: how to do X

Practical notes gathered from our own work and from how other Battlegrounds tools do things ([github-landscape.md](github-landscape.md)). Each item says where it comes from. "Others" means other projects' READMEs or docs, not code we read. Date: 2026-10-09.

## Logs

**Turn on the Power log.** `%LOCALAPPDATA%\Blizzard\Hearthstone\log.config` needs a `[Power]` section with `LogLevel=1`, `FilePrinting=true`, `Verbose=true`. `tools/check_logs.py` checks it (ours). Others advise turning on only the loggers you need: all of them make the logs huge (openbg).

**Lift the log size limit.** Without `[Log]` `FileSizeLimit.Int=-1` in the game's `client.config` (install folder, next to `Hearthstone.exe`), the game stops writing a log at about 10 MB, about one Battlegrounds game (hsbgtracker). The user's install has it; nothing checks it yet (T-106). A game cut by this limit would look `incomplete` to our parser.

**Find the logs.** Install folder from the Windows uninstall registry (`InstallLocation`), then `Logs\Hearthstone_<date>\` (`tracker::discover`, ours). Each game launch makes a new session folder; `Power.log` rotates to `Power_old.log` (120–563 MB per session in our data).

**Other useful logs.** `LoadingScreen.log` says when the player leaves a game (hsbg-overlay uses it with `Power.log`). `GameNetLogger.log` holds the game server address (HsReconnector uses it). Neither is read by us yet.

**Never edit game files for the user** (D-004): show what to change and why; the user edits.

## Parsing and patches

- Read complete lines only, survive rotation, keep the last line (ours, T-101).
- One fresh parser per game; mark a game `unsupported` instead of guessing when something unknown shows up (ours, D-010).
- Patches break memory readers and hand-kept card data most (HSTracker issue after the 2026-06-02 patch; bgknowhow gave up on hand-kept card text). Log parsing breaks less, but still needs:
  - fixtures from several builds as regression tests (T-108; others replay stored logs in tests);
  - the parser version and game build saved with each game (T-107), so a fix knows what to re-read;
  - stats split by patch when they compare heroes or cards.

## Values we can and cannot get from the log

| Value | Log-only? | How |
|-------|-----------|-----|
| Hero, places, health, boards that played out | yes | `Power.log` (ours, [parser-hslog.md](parser-hslog.md)) |
| Hero names in the client's language | yes | `[entityName=… cardId=…]` references (D-017) |
| Tribes offered in the tavern | yes | `CARDRACE` of shop minions; not the lobby list (T-102) |
| Exact lobby tribes at hero select | no | others read memory (bgtracker's optional reader); fallback: user picks them (pink-replay; T-303) |
| Own MMR | no | public leaderboard, 8000+ only (D-011, [leaderboard-client.md](leaderboard-client.md)) |
| Opponents' names and MMR | no | others get names from memory (HDT plugins) or network traffic (HS Reconnect) and then look them up; we don't (D-004, D-012) |

Always say where a value comes from: from the log, inferred, entered by you, from the leaderboard, unknown (T-109). Others that skip this end up showing guesses as facts.

## Overlay on Windows

- Tauri 2: transparent, borderless window, `alwaysOnTop`, `set_ignore_cursor_events(true)` to let clicks through; switch it off at runtime to drag panels (ours, [desktop-stack.md](desktop-stack.md), spike in `spikes/overlay-tauri/`).
- Works over the game in **windowed or borderless fullscreen**; exclusive fullscreen hides any overlay (ours and openbg).
- Place panels relative to the game's 4:3 play area with small user nudges instead of guessing pixels (openbg's approach).
- The overlay only shows what the player has already seen (rule 3).

## Storage and uploads

- Local history first: JSONL with "last record wins" (D-015). Others use JSONL or SQLite; move to SQLite only if the stats need it.
- A CSV export failed for another tool when the file was open in a spreadsheet: write exports to a new file each time.
- Uploads opt-in, off by default and never switched on by an update (bgtracker turned community sharing on by default in 2026-08; HS Reconnect added telemetry later). Upload design: [web-stack.md](web-stack.md).

## Card data and images

- HearthstoneJSON: the website is CC0, the JSON card data is "Blizzard, All Rights Reserved". Don't bundle it in the repo (D-017).
- Firestone's stats feeds: public apps must ask the author first ([developer resources](https://github.com/Zero-to-Heroes/firestone/wiki/Developer-resources)).
- Card images only under Blizzard's Fan Content Policy, with the "Unofficial fan project" notice (rule 5).

## Combat odds (not planned)

How others do it: board model → combat engine → many random trials → distribution of outcomes (twanvl's simulator). The best-documented validation (hsbg-overlay `ACCURACY.md`):

1. Take each board just before the first attack, after start-of-combat effects.
2. Score predictions against real outcomes and damage, and check calibration.
3. Replay the real attack order to find the first rule that differs.

Duos and hidden hands lower accuracy (Firestone docs). More trials never fix a missing card rule: show "not modelled" instead of a number. For bug reports, Firestone asks for the board snapshot, turn and replay; our version is T-110.

## Reconnect/unplug (dev tool only, D-022)

What it is: dropping the game's TCP connection mid-combat so the client reconnects and skips the combat animation. How others do it, from their READMEs:

- **HsReconnector** (Windows, C#, Apache-2.0): reads the server address from `GameNetLogger.log` and closes that TCP connection with the Windows `SetTcpEntry` API; needs admin. Its README admits a ToS grey area.
- **HS Reconnect** (macOS): a Network Extension (system proxy) that closes the connection.
- **xinyao27's tool**: a hotkey "unplug" that needs admin; method not documented.

Known side effects: HDT marks reconnected games as unreliable, so a reconnect may confuse log parsing. T-D01 must check what the log shows around a reconnect and mark those games. Run by hand, own account, never shipped to users.

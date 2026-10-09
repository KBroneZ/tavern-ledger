# How others build Battlegrounds tools (GitHub landscape)

Date: 2026-10-09. Sources: the 100 most-starred results of the GitHub search "hearthstone battlegrounds", plus well-known trackers (HDT, HSTracker, Firestone, `python-hslog`). Two passes: README and metadata of 24 repos read by Claude (licences and push dates checked with the GitHub API), and a ChatGPT web pass (READMEs, docs, issues). **No source code was read**; Nomi's Kitchen was left out (rule 1). "Unclear" means the project's docs don't say.

## Summary

- **Log-only is common in small new tools** (openbg, hsbg-overlay, hsbgtracker, battlegrounds-companion, bgtracker by default). The big trackers (HDT, HSTracker, Firestone) also read the game's memory (HearthMirror or similar).
- **The two things nobody gets from the log alone:** the exact lobby tribes at hero select and the opponents' names (and so their MMR). Tools that show them read memory (bgtracker's optional reader, HDT plugins) or the game's network traffic (HS Reconnect). The rest show "inferred", let the user pick, or say nothing. This matches our findings (D-011, [parser-hslog.md](parser-hslog.md)).
- **Closest peers to us:** `AlexanderKempes/openbg` (Rust, `Power.log` only, MIT, egui transparent Win32 overlay) and `jmarlett93/battlegrounds-companion` (Rust workspace, `Power.log` only, SQLite, Linux overlay; no licence). The only Tauri app found, BgBuddy, is closed source and needs a "helper DLL".
- **Storage:** local-first everywhere (JSONL, SQLite, CSV). Servers appear only for leaderboard sites and bots (Supabase + Next.js, DynamoDB, MySQL).
- **Risky category to stay away from:** "reconnect" or "unplug" tools that cut the game's connection to skip combat animations (xinyao27's tool, HsReconnector, HS Reconnect for macOS). HDT marks reconnected games as unreliable. Out of the product (D-004); allowed only as a dev tool that never ships (D-022).

## Projects

| Repo | Stars | Licence | Last push | What it does | Data source | Stack | Storage | Game interaction |
|------|-------|---------|-----------|--------------|-------------|-------|---------|------------------|
| [HearthSim/python-hslog](https://github.com/HearthSim/python-hslog) | — | MIT | 2026-08 | `Power.log` parser library (packets → entities) | `Power.log` | Python | in memory | none |
| [HearthSim/Hearthstone-Deck-Tracker](https://github.com/HearthSim/Hearthstone-Deck-Tracker) | — | All Rights Reserved | active | Windows tracker with BG overlay | logs + HearthMirror (memory) | C#, WPF | local XML, HSReplay | reads memory |
| [HearthSim/HSTracker](https://github.com/HearthSim/HSTracker) | — | MIT (Blizzard assets excluded) | active | macOS tracker | logs + HearthMirror | Swift | HSReplay uploads | reads memory |
| [Zero-to-Heroes/firestone](https://github.com/Zero-to-Heroes/firestone) | — | unclear for reuse | active | overlays, history, simulator, meta stats | logs + memory; leaderboard by name for opponent MMR | Overwolf, Angular | online | reads memory |
| [xinyao27/hearthstone-battlegrounds-tools](https://github.com/xinyao27/hearthstone-battlegrounds-tools) | 296 | MIT | 2023-01 | daily stats, hero rates, opponent boards, OBS sync | logs (unclear which) | TypeScript, Electron | local (unclear) | **forces disconnect** ("unplug", needs admin) |
| [IBM5100o/HDT_BGrank](https://github.com/IBM5100o/HDT_BGrank) | 72 | MIT | 2026-08 | HDT plugin: opponents' MMR | leaderboard (8000+ only, "8000↓" otherwise); credits a memory reader (unity-spy) | C#, HDT plugin | local | via HDT / memory |
| [twanvl/hearthstone-battlegrounds-simulator](https://github.com/twanvl/hearthstone-battlegrounds-simulator) | 59 | MIT | 2020-01 | combat simulator, win % and positioning optimiser | boards typed by hand | C++ (+ web build) | files | none |
| [jawslouis/Battlegrounds-Match-Data](https://github.com/jawslouis/Battlegrounds-Match-Data) | 31 | none | 2022-12 | HDT plugin: games and combats to CSV/Sheets/dashboard; MMR after match | HDT state | C#, HDT plugin | CSV, Google Sheets, third-party server | via HDT |
| [HS-Tools/Wall_Lii](https://github.com/HS-Tools/Wall_Lii) | 21 | none | 2026-09 | Twitch/Discord bot: rank and daily MMR | leaderboard, polled over time | Python | DynamoDB | none |
| [BattlegroundsHelp/bgtracker](https://github.com/BattlegroundsHelp/bgtracker) | 3 | MIT (art and third-party data excluded) | 2026-08 | pick helper, session MMR, tribe inference, beta combat odds | `Power.log` by default; **optional memory reader** (off unless built); HearthstoneJSON art; community stats | Python/Tk, optional C# helper | local JSONL; anonymous per-game upload **on by default** since 2026-08-12 | none by default |
| [AlexanderKempes/openbg](https://github.com/AlexanderKempes/openbg) | 0 | MIT | 2026-09 | early-turn buy/roll advice, shop badges, pool browser by lobby tribes | `Power.log`; bundled Firestone stats snapshot; HearthstoneJSON art | Rust, egui/glow transparent click-through Win32 overlay | local cache | none |
| [Dimus99/hsbg-overlay](https://github.com/Dimus99/hsbg-overlay) | 1 | MIT | 2026-09 | macOS: fight odds, last-seen boards, lobby table, own stats | `Power.log` + `LoadingScreen.log`; HearthstoneJSON | Python/PyObjC | local | none (states: no memory, injection, traffic or input) |
| [RobinTo/hsbgtracker](https://github.com/RobinTo/hsbgtracker) | 0 | none | 2026-09 | last-seen boards, HP, Duos, history | `Power.log` only | Python/tkinter always-on-top window | `games_history.jsonl`; replay logs as tests | none |
| [jmarlett93/battlegrounds-companion](https://github.com/jmarlett93/battlegrounds-companion) | 0 | none | 2026-09 | Linux overlay: tier browser by lobby tribes "when known" | `Power.log`; HearthstoneJSON | Rust workspace, GTK4 layer-shell | SQLite | none |
| [nymann/pink-replay](https://github.com/nymann/pink-replay) | 0 | none | 2026-02 | card list for 5 tribes the **user picks** | manual | Rust + web UI (Tauri likely, unverified) | unclear | unclear |
| [ArKmatty/HSBGLdb](https://github.com/ArKmatty/HSBGLdb) | 0 | none (README says MIT) | 2026-07 | web leaderboard, MMR history | leaderboard API by cron; patch notes scraped | Next.js, Supabase, Vercel, GitHub Actions | Supabase | none |
| [O-Nemet/bgknowhow](https://github.com/O-Nemet/bgknowhow) | 7 | MIT | 2025-08 | BG info website (discontinued July 2025) | Blizzard API + own DB | PHP | MySQL | none |
| [kulibabkaaa/Hearthstone-Reconnect-MacOS](https://github.com/kulibabkaaa/Hearthstone-Reconnect-MacOS) | 7 | MIT | 2026-10 | reconnect hotkey, Solo lobby ratings | leaderboard + network capture | Swift, Network Extension | local; TelemetryDeck | **forces disconnect**; reads game traffic |
| [coutHan/HsReconnector](https://github.com/coutHan/HsReconnector) | 1 | Apache-2.0 | 2026-09 | tray hotkey that drops the game's TCP connection | `GameNetLogger.log` for the server address | C# .NET 8 | settings file | **forces disconnect** (admin); README admits a ToS grey area |
| rcbyron/BgBuddy-releases | 1 | closed source | 2026-04 | overlay with lobby names + MMR | "helper DLL" (likely memory, unverified) | Tauri | unclear | likely reads memory |

Simulators and clones (twanvl, battlegrounds-rs, Ghastcoiler, many 0-star repos) don't read the game; most are unfinished. Over a third of the 100 search results have no licence: ideas only, no code reuse.

## Data sources

- **`Power.log`** is the base of every log-only tool. Some also read `LoadingScreen.log` (to know when the player leaves a game) or `GameNetLogger.log`.
- **Log size limit:** the game stops writing a log at about 10 MB (roughly one BG game) unless `client.config` has `[Log] FileSizeLimit.Int=-1` (hsbgtracker README). The user's install has it; our `check_logs.py` and the app do not check it yet.
- **Memory** (HearthMirror, unity-spy) is what HDT, HSTracker and Firestone use for names, exact tribes and ratings. Out of scope for us (D-004).
- **Overwolf GEP** listed Hearthstone as deprecated (announced 2026-07-06, removal 2026-08-10, per [Overwolf docs](https://dev.overwolf.com/ow-native/live-game-data-gep/supported-games/deprecated/overview/)).
- **Card data:** HearthstoneJSON is everywhere. Its site is CC0, but the JSON data is "Blizzard, All Rights Reserved" (as we found in D-017). Firestone's stats feeds ask public apps to contact the author first ([developer resources](https://github.com/Zero-to-Heroes/firestone/wiki/Developer-resources)). openbg bundles a Firestone snapshot; whether that was allowed is unclear.

## Lobby tribes and MMR

| Field | How others do it | For us |
|-------|------------------|--------|
| Exact tribes at hero select | memory reader (bgtracker, optional) | not available (D-004) |
| Tribes seen | inferred from minions, labelled "partial" (bgtracker); "dominant tribe per board" (hsbgtracker, hsbg-overlay) | what we do: "tribes seen in the tavern" (T-102) |
| Tribes by hand | user picks 5 (pink-replay) | possible later, labelled "entered by you" |
| Own MMR | memory or HDT after the match | leaderboard only (D-011) |
| Opponents' MMR | names from memory or traffic, then the leaderboard; ambiguous names and the 8000 cut-off documented (HDT_BGrank, Firestone) | not possible: the log has no opponent names (and D-012 forbids storing other players' rows) |

## Patch resilience

- Patches break memory readers most (HSTracker issue after the 2026-06-02 patch) and manually kept card data (bgknowhow gave up on it).
- Good practices seen: replay tests on stored logs (hsbgtracker, bgtracker fixtures), "—" instead of invented numbers (bgtracker), accuracy notes and "not modelled" warnings (hsbg-overlay `ACCURACY.md`), stats split by patch.
- We already do the core of this (`unsupported` status, synthetic fixtures). Missing: fixtures from more than one build, and storing the parser version with each saved game.

## Combat simulators

Common design: board model → combat engine → many random trials → distribution (twanvl). The best-documented validation (hsbg-overlay) scores predictions against real outcomes and replays the real attack order to find the first rule that differs. Duos and hidden hands lower accuracy (Firestone docs). A simulator needs full card rules: big, patch-sensitive work. Not planned for us; if it ever is, start from that validation method.

## Ideas worth considering for Tavern Ledger

The useful ones are now tasks in the plan (T-106 to T-110, T-303, T-D01, T-D02) or listed under its "Future ideas". How-to details: [implementation-notes.md](implementation-notes.md).

1. **Check `client.config` `FileSizeLimit.Int=-1`** in `check_logs.py` and the app, and say so when it's missing (a game cut at 10 MB would otherwise look `incomplete`). Small, high value.
2. **Read `LoadingScreen.log`** to know when the player leaves a game, if we ever need it faster than `STATE=COMPLETE`.
3. **Store the parser version and game build** with each saved game, so a parser fix can say which games to re-read.
4. **Label where each value comes from**: from the log, inferred, entered by you, from the leaderboard, unknown (we already do this for MMR and tribes).
5. **Uploads opt-in, never on by default** (bgtracker turned community sharing on by default; HS Reconnect added telemetry later). Matches rule 6.
6. **Overlay (F3):** openbg's notes match our spike: a click-through window needs windowed or borderless fullscreen.
7. **Do not:** reconnect/unplug features in the product (dev tool only, D-022), memory readers, bundling HearthstoneJSON data or Firestone stats without permission.

ChatGPT's raw answer is kept locally in `.local/handoffs/github-landscape-answer.md` (not in git). Claims used here were checked against the repos' READMEs and the GitHub API; one conflict was resolved (bgtracker's licence file is MIT with exclusions for art and data, though GitHub shows "NOASSERTION").

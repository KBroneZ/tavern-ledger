# Exact lobby tribes (T-326)

Session 054, 2026-10-11. Build 253216. Research only, no product code.

The user's ask (2026-10-11): the exact five lobby tribes, as other apps show them, at hero select. Asked how, the user chose: **the log and public sources first; if those cannot give them, a read-only memory read**, as a new decision with the risk written down. Injection and client changes stay ruled out (D-006). The user also asked to see everything else other apps take from memory, to use what they get without trouble.

## Answers

1. **The log: no.** On all 11 raw games still on disk (2 Solo, 9 Duos, 11 different lobbies), every file, every tag on every entity: nothing names the lobby's tribes or changes with them before the first shop. Tag `4730` is `5` in every game and never changes; it holds no tribe. The log keeps giving only what the app already uses: the tribes of the minions Bob offers (D-050), which confirm all five only by shop turn 2 to 8, never at hero select. (Section 1.)
2. **Public sources: no.** HearthstoneJSON, Blizzard's Game Data API and the public leaderboard are card data or ratings; stats sites are aggregated. The tribes are drawn per game and only the client gets them. The game does show them on screen (at hero select, and in its own tavern guide since patch 33.2), so they are information the player sees. (Section 2.)
3. **Other apps: memory.** Every app that shows the exact tribes at hero select reads the game's memory: HDT through HearthSim's private HearthMirror (lobby data "already" in its Windows API), Firestone through its own reader (inferred; its docs only show a "Memory reading" error). None documents a log or server route. (Section 3.)
4. **So: a memory-read design for the user's decision** (section 4) and **D-063** (section 6), **approved by the user on 2026-10-11**, which amends D-004 for one read-only read of two on-screen values: the lobby's tribes and the player's own rating (section 5 explains why only those two). **The risk, plainly:** it goes against the letter of Blizzard's licence (§1.C.vi data mining, §1.C.i reverse engineering) with no written permission; Blizzard has tolerated the same reads by HDT and Firestone for over ten years with no known ban, but it can change that any day, and the account at risk is the player's.

## 1. The log, to the bottom

Only the user's own logs were read, with shared access, by read-only scripts kept outside the repo. Nothing below prints a player name, BattleTag or account id; only tag names and numbers, enum names, card ids and counts.

### What was read

- All 6 session folders still on disk (`Hearthstone_2026_10_09_00_26_56` to `Hearthstone_2026_10_10_19_11_29`), every file in them: `Power.log` or `Power_old.log` (the game renames the file when it grows), `Hearthstone.log`, `LoadingScreen.log` / `LoadingScreen_old.log`, `Achievements.log`, `Arena.log`, `Decks.log`, `Downloader.log`, `ExceptionReporter.log`, `FullScreenFX.log`, `GameNetLogger.log`, `Gameplay.log`, `Login.log`, `LuckyDraw.log`, `Net.log`, `Spells.log`, `Store.log`.
- **11 Battlegrounds games** (2 Solo, 9 Duos), 1 992 439 `GameState.DebugPrintPower` lines, one build (253216).
- Each game's lobby comes from the history (`games.jsonl`) by the inferred rule of D-050 (a single-tribe pool minion Bob offered proves its tribe). Every one of the 11 games gives exactly five tribes, and **all 11 lobbies are different**, so any log value tied to the lobby has to change between them. There is no hand entry (`lobby_tribes.json` does not exist).

| Game | Mode | Lobby (inferred, D-050) |
|---|---|---|
| 10-09 00:26 #1 | Solo | Aberration, Elemental, Mechanical, Murloc, Quilboar |
| 10-09 03:28 #1 | Duos | Aberration, Beast, Demon, Dragon, Mechanical |
| 10-09 03:28 #2 | Duos | Demon, Dragon, Mechanical, Pirate, Undead |
| 10-09 03:28 #3 | Duos | Aberration, Elemental, Mechanical, Murloc, Pirate |
| 10-09 03:28 #4 | Duos | Aberration, Demon, Elemental, Murloc, Undead |
| 10-10 04:02 #1 | Duos | Aberration, Dragon, Elemental, Murloc, Quilboar |
| 10-10 04:02 #2 | Duos | Beast, Demon, Dragon, Mechanical, Pirate |
| 10-10 14:58 #1 | Solo | Beast, Dragon, Murloc, Pirate, Quilboar |
| 10-10 18:11 #1 | Duos | Beast, Demon, Murloc, Pirate, Undead |
| 10-10 18:11 #2 | Duos | Aberration, Elemental, Murloc, Pirate, Quilboar |
| 10-10 19:11 #1 | Duos | Aberration, Beast, Demon, Elemental, Quilboar |

Naga is in none of the 11 (nor in any of the 19 games of the history with a shop record): the tribe looks out of this season's pool (the Aberrations' return in patch 36.6.1, 2026-09-22, is reported to have rotated Naga out; not re-checked here).

### Leads and results

| Lead | Check | Result |
|---|---|---|
| Tag `4730` on the game entity | value in each game; any `TAG_CHANGE` | `5` in all 11 games, set once at `CREATE_GAME`, never changed, on no other entity. It does not change with the lobby. A count of tribes in the lobby (5) fits, but nothing ties it to that; either way it holds no tribe. |
| Any tag whose values are tribes | for every tag on every entity, the set of values that are `Race` numbers or names of the 11 Battlegrounds tribes, compared with the lobby | **Before the first shop (`TURN=1`): no tag in any game has the lobby's set, or a set holding all five.** Over the whole game, leaving out number tags whose many values happen to cover the race numbers (`HEALTH`, `ATK`, `DAMAGE`, counters, unnamed tags such as `1323`), only `CARDRACE` holds all five (8 of 11 games; equal to the lobby in 3): it is the race of the minions that were created, the same signal D-050 already uses. No tag ever holds exactly the six tribes left out. |
| Anything that switches on and off with a tribe | every (entity kind, tag, value) and every card id created before `TURN=1` (1 542 features, 1 346 that vary between games), tested against each tribe: present in a game exactly when the tribe is in its lobby (or exactly when it is not) | **No feature predicts any tribe**, except two entity-id numbers for Undead (in 3 lobbies): plain entity numbering, a coincidence. |
| Game entity tags at the start | the 47 tags it has before `TURN=1` | 41 are the same in all games. The 6 that differ are `937` (2 values), `BACON_BARTENDER_CARD_ID` (Bob's skin), `BACON_GLOBAL_OLD_GOD_DBID` (2 values), `BG_COMBAT_SPEED_MAX_SPEED` (2 values), `ENTITY_ID` and `GAME_SEED` (different every game). None follows the tribes. `GAME_SEED` is the server's random seed; the tribes cannot be worked out from it without the server's code. |
| Player entity tags at the start | tags that differ between games | ids, team, teammate, next opponent, free rerolls, timeouts and the unnamed `1462`, `3010`, `3219`, `3675`, `4441`, `4709`. None holds a tribe value. |
| `BACON_SUBSET_<TRIBE>` | session 039 | A card attribute, also on tribes outside the lobby (see [lobby-tribes-log.md](lobby-tribes-log.md)). |
| Other `Power.log` line kinds | `DebugPrintGame`, `DebugPrintOptions`, `DebugPrintEntityChoices`, `DebugPrintEntitiesChosen`, `SendChoices`, `SendOption`, `DebugPrintPowerList` | Build, game type, format, scenario, options and choices (hero select offers heroes only). No tribe. |
| Other log files | lines with a tribe word, in English and in the client's language (Spanish), outside the game's card lines | `Hearthstone.log`: 170 lines, all card names in focus messages and the shop buff enchantments `BG_ShopBuff_Elemental_Ench` and `BG_ShopBuff_MultiRace_Ench` (asset warnings). Every other file: 0. |
| `log.config` sections | what is turned on, and what else exists | On: `Achievements`, `Arena`, `FullScreenFX`, `LoadingScreen`, `Power`, `Decks`, `Net` (`Power`, `Achievements` and `FullScreenFX` verbose). The game also writes `Hearthstone.log`, `GameNetLogger.log`, `Gameplay.log`, `Login.log`, `LuckyDraw.log`, `Store.log`, `Downloader.log`, `ExceptionReporter.log` and `Spells.log` on its own; all were read above. Other section names in public tracker docs (`Zone`, `Asset`, `Bob`; [Arena Tracker docs](https://triodo.gitbook.io/arena-tracker-documentation), older community articles) are not on: `Zone` repeats card moves `Power` already has, `Asset` is asset loading, `Bob` is described in an old article as legend rank data. No public source says any section logs the lobby's tribes. Not tried: a game with every section on and verbose (it needs the user to play); expected to add nothing, since the game-state section (`Power`) is already verbose and has no tribe. |

**The log does not have the lobby's tribes. No.** It keeps having only what D-050 uses: the tribes of the minions Bob offers.

### How early the inferred rule gets there

On the 19 games of the history with a shop record, the fifth tribe is confirmed by the shop turn below. At hero select, the rule has nothing yet.

| Shop turn | 2 | 3 | 4 | 5 | 6 | 7 | 8 |
|---|---|---|---|---|---|---|---|
| Games whose 5th tribe is confirmed on it | 2 | 2 | 2 | 4 | 3 | 5 | 1 |

Average tribes confirmed after shop turn 1 to 6: 1.7, 2.6, 3.5, 4.0, 4.4, 4.6. So the hero pick and the first turns, where the user wants them, are exactly where the log route cannot help.

### To recheck

The scripts were not kept in the repo (the counts above are the whole output). Steps: split each `Power*.log` at `GameState.DebugPrintPower() - CREATE_GAME`; for every `tag=X value=Y` line under `GameEntity`, `Player`, `FULL_ENTITY`, `SHOW_ENTITY`, `CHANGE_ENTITY` and every `TAG_CHANGE`, keep (entity kind, tag, value), separately for the lines before `TAG_CHANGE Entity=GameEntity tag=TURN value=1`; map values to the 11 tribes by `Race` name or number (`hearthstone` package, already pinned); compare with each game's lobby from `games.jsonl` and `card-cache/catalog.json` as in [lobby-tribes-log.md](lobby-tribes-log.md).

## 2. Public sources

None gives the current lobby's tribes. Every public source is static card data or aggregated stats; the five tribes are drawn per game by the server and only the game client receives them.

| Source | What it gives | Current lobby's tribes? |
|---|---|---|
| [HearthstoneJSON](https://hearthstonejson.com/) (already used, D-038) | Card data "automatically converted from the game files", per build: each minion's tribes, tier, pool flag. | No. Static per build. It is what lets the app turn Bob's offers into tribes (D-050). |
| Blizzard's Hearthstone Game Data API ([developer portal](https://community.developer.battle.net/documentation/hearthstone/game-data-apis); [Battlegrounds update thread](https://us.forums.blizzard.com/en/blizzard/t/hearthstone-api-battlegrounds/2792)) | `/hearthstone/metadata` (sets, classes, keywords, `minionTypes` as id, name, slug), `/hearthstone/cards` with a `gameMode` filter for Battlegrounds cards. OAuth client credentials. | No. Card catalogue only, no live or per-game endpoint. It would add nothing over HearthstoneJSON; D-020 already ruled out Battle.net login. |
| Blizzard's public leaderboard (`hearthstone.blizzard.com/.../api/community/leaderboardsData`, D-012) | Rank, display name, rating for 8000+. | No. No hero or tribe field. |
| HSReplay.net Tier7 ([tier7](https://hsreplay.net/battlegrounds/tier7)) | Paid stats, among them "Hero Stats by Available Minion Types", shown live in HDT's overlay. | No as a source: it is fed by the lobby tribes HDT itself reads on the player's PC (section 3), and it is a paid third-party service. |
| Firestone's meta stats ([wiki](https://github-wiki-see.page/m/Zero-to-Heroes/firestone/wiki/Battlegrounds-Meta-Stats-for-Heroes)) | Hero stats by tribe combination, aggregated. | No (aggregated). |
| Blizzard patch notes | How lobbies are drawn: since 36.6.3 (2026-10-01) "Aberrations are no longer guaranteed in every lobby" and have "the standard 50% chance to appear in lobbies"; new tribes get a guarantee of one week from now on ([36.6.3](https://hearthstone.blizzard.com/en-us/news/24303864/3663-patch-notes)). | No; rules only. Useful for a sanity check (five tribes, a newly added tribe in every lobby during its guarantee). |
| The game itself | Since 33.2 (2025-08-01) the client has a guide of "all available minions and Tavern spells in your lobby, organized by Tier and type" ([33.2](https://hearthstone.blizzard.com/en-us/news/24223019)), and it names the lobby's tribes at hero select (players describe the in and out lists: [Blizzard forum](https://us.forums.blizzard.com/en/hearthstone/t/battleground-should-tell-us-what-tribes-are-banned/107141)). | Yes, **on screen**. So the tribes are information the player sees (D-004's line), but the client does not write them to any log. |

## 3. How other apps get them

From docs, issues and forum posts only; no code of HDT, HSTracker, Firestone, HearthMirror or BobsBuddy was opened, and Nomi's Kitchen was not looked at (absolute rule 1).

- **HDT (HearthSim, Windows).** Shows the lobby's tribes from hero select on (players describe a "Not in this game" box and the tier list hiding unavailable tribes "even during hero selection": [Blizzard forum](https://us.forums.blizzard.com/en/hearthstone/t/battleground-should-tell-us-what-tribes-are-banned/107141), [HearthPwn](https://www.hearthpwn.com/forums/hearthstone-game-modes/battlegrounds/245438-hovering-over-to-see-missing-tribes)). Its log error "Unable to get available races" comes from a helper that "returns null when invoked too early in the game startup sequence", succeeding about 50 s later on retry ([issue 4685](https://github.com/hearthsim/hearthstone-deck-tracker/issues/4685), 2026-01-03): by our reading, a read of live game state, not a log line. HSTracker (HDT's macOS sibling) confirms the route: the issue's author says the Windows HearthMirror API "already supplies" Battlegrounds lobby data, and a HearthSim contributor answers there "were some additions to share the lobby information to the mirror, not that the interface is public" ([HSTracker issue 1423](https://github.com/HearthSim/HSTracker/issues/1423), 2026-07-11 and 2026-09-11). HearthMirror is HearthSim's private memory reader (its repository answers 404; [feasibility.md](feasibility.md), [github-landscape.md](github-landscape.md)). **Mechanism: memory reading (HearthMirror).**
- **HSReplay.net.** Its overlay FAQ says the tracker "only reads publicly available game log files, it never injects code or modifies the game client" (search snippet; the page did not load). That is the vendor's claim; it does not match the HearthMirror route above, and the log provably has no tribes (section 1).
- **Firestone (Overwolf).** Features: "Show the list of available / banned tribes as a widget" ([features wiki](https://github-wiki-see.page/m/Zero-to-Heroes/firestone/wiki/Firestone-features)). Its FAQ has a "Memory reading" error that "occurs when the app doesn't have the rights to access the game client's memory" and suggests running as admin ([FAQ](https://github-wiki-see.page/m/Zero-to-Heroes/firestone/wiki/FAQ---Troubleshooting), modified 2026-08-19). Its memory reader is called MindVision ([feasibility.md](feasibility.md)). The docs do not say which item comes from memory; tribes at hero select cannot come from the log, so **memory reading** (inferred, not stated).
- **Small open-source tools** ([github-landscape.md](github-landscape.md)): the log-only ones (openbg, hsbg-overlay, hsbgtracker, battlegrounds-companion) show tribes "when known", inferred, or picked by hand; bgtracker has an optional memory reader, off unless built; BgBuddy (closed) needs a "helper DLL".
- **unity-spy** (MIT, [hackf5/unityspy](https://github.com/hackf5/unityspy)), the generic Unity memory reader some HDT plugins credit, describes itself as "An open source MIT licensed rewrite of the now closed source HearthMirror". Even with an MIT licence it is a rewrite of HearthMirror, which rule 1 says is not copied or ported: **not usable**, not even to read.

**Summary: every app that shows exact tribes at hero select reads the game's memory.** No app documents a log or server route, and section 1 shows the log has none.

## 4. A memory read, for the user's decision (design only, not built)

Sections 1 to 3 leave no log or public route, so this is the design the user asked for. The user approved it (D-063, 2026-10-11); nothing is built yet (T-327, T-328).

### What would be read

- **Only:** the lobby's five tribes, once per game, at hero select; and (section 5) the player's own Battlegrounds rating as the game shows it on its Battlegrounds screen, before and after a game. Both are on the player's screen.
- **Read-only, from our own process:** `OpenProcess` with `PROCESS_VM_READ | PROCESS_QUERY_LIMITED_INFORMATION` only, `ReadProcessMemory`, handle closed right after. No write rights asked for, no `WriteProcessMemory`, no remote threads, no DLL, no hooks, no debugger attach, no packet capture, no elevation (if the game runs as administrator, the read is "unavailable", never retried with more rights). D-006 stays as it is.
- **Short:** a few reads at hero select (retried for up to about a minute, since the data appears some seconds into the game: [HDT issue 4685](https://github.com/hearthsim/hearthstone-deck-tracker/issues/4685)) and one on the Battlegrounds screen for the rating; never a continuous scan.
- **Narrow:** follow one fixed path of object fields to the values; never scan or copy memory in bulk (process memory holds other players' names and hidden game state, which we must not collect, even by accident).
- **Kept local:** the tribes go next to the hand-entered ones (`lobby_tribes.json`, D-029) with the source "from the game"; the rating stays on the PC. Neither goes into the uploaded report unless a later decision says so (the upload check refuses ratings today).

### What Blizzard's terms say, and how they treat trackers

The [Blizzard End User License Agreement](https://www.blizzard.com/en-us/legal/fba4d00f-c7e4-4883-b8b9-1b4500a402ea/blizzard-end-user-license-agreement) (last revised 2024-03-21, the version in force):

- §1.C.vi, data mining: you may not "use any unauthorized process or software that intercepts, collects, reads, or "mines" information generated or stored by the Platform; provided, however, that Blizzard may, at its sole and absolute discretion, allow the use of certain third-party user interfaces." **A memory read is exactly "reads … information … stored by the Platform".** (Read literally, log parsing is "generated by" the Platform too; it is tolerated, not authorised.)
- §1.C.i: you may not "reverse engineer, derive source code from, modify, disassemble, decompile" the Platform. **Finding where the tribes live in memory means studying the client's inner structure** (see "Clean room" below). That is a second clause, and it falls on us as developers, not only on the player.
- §1.C.ii.3, hacks: "accessing or modifying the software of the Platform in any manner not expressly authorized by Blizzard".
- §4: the game "may monitor your computer … memory for unauthorized third party programs running either concurrently with a game or out of process."
- Sanctions: suspension or revocation of the licence; Blizzard may end the agreement "for any reason, or for no reason".

How Blizzard has treated HDT and Firestone in practice:

| Date | What | Source |
|---|---|---|
| 2014-09-14 | Ben Brode (then lead designer): "any app that duplicates what you can do with a pencil and paper already is fine." | [tweet](https://twitter.com/bdbrode/status/511151446038179840), quoted in [HDT's FAQ](https://github.com/HearthSim/Hearthstone-Deck-Tracker/wiki/FAQ) |
| 2014-10-28 | "Several thousand" accounts banned for programs that **automate** gameplay. | [PCGamesN](https://pcgamesn.com/hearthstone-heroes-of-warcraft/several-thousand-bots-banned-from-hearthstone-till-2015-fair-play-is-the-core-of-the-hearthstone-experience) |
| 2015-04-17 | Community manager on deck trackers: "It's not against the TOS […]" | [tweet](https://twitter.com/CM_Zeriyah/status/589171381381672960), via HDT's FAQ |
| 2015 | On an Arena overlay: "Like any third party programs for Hearthstone, we can't condone or approve of their use. Our Terms of Use don't allow changing game files or automating game play." | [PCGamesN](https://pcgamesn.com/hearthstone-heroes-of-warcraft/hearthstone-arena-overlay-blizzard-say-we-can-t-condone-or-approve-of-their-use) |
| 2020-04-24 | Bob's Buddy (combat simulator) ships inside HDT; no Blizzard reaction found. | [HSReplay](https://articles.hsreplay.net/2020/04/24/introducing-bobs-buddy/) |
| 2026-08-03 (page date) | Tournament handbook §5.10 allows trackers where the event allows them. | [HearthSim help](https://help.hearthsim.net/en/articles/4345072-can-i-use-hearthstone-deck-tracker-in-an-official-hearthstone-tournament) |
| 2014 to 2026 | No public ban, warning or removal request tied to HDT, Firestone or a memory reader in Hearthstone was found; forum threads asking about it have no Blizzard reply ([2019](https://us.forums.blizzard.com/en/hearthstone/t/is-hsreplay-safe-to-use/7496), [2020](https://us.forums.blizzard.com/en/hearthstone/t/decktracker-a-cheatprogram/50704)). No public source says whether Hearthstone detects outside readers. | searches, 2026-10-11 |

The 2014 and 2015 statements are informal, over ten years old, and from people who have since left or changed roles; they are not policy.

### The risk to the user's account, said plainly

- **It breaks the written terms.** §1.C.vi forbids it in so many words, and no written permission exists. What protects HDT and Firestone users is tolerance: over ten years of it, with no known ban, but tolerance can end any day without notice, and §4 lets the game look for programs like ours.
- **The chance of a ban for reading on-screen information only is low** on the evidence: every documented Hearthstone ban wave was for automation or exploits, and the big trackers have read memory for years. Low is not zero, and the user, not the app, carries it.
- **It ends the app's simplest promise.** "Only reads the log files" is a selling point other log-only trackers use (Cairn, listed 2026-09-01: "never touches the game's memory and never injects anything", [AlternativeTo](https://alternativeto.net/software/cairn/about/)).
- **Risk goes up** if the read ever shows hidden information (opponents' hands or boards, anything before the game shows it), reads other players' names, BattleTags or ratings, scans memory broadly, asks for more than read rights, or ships to many users. Each of these is where Blizzard's statements draw the line, and most are already ruled out.
- A legal note, not legal advice: EU law (Directive 2009/24/EC, articles 5(3) and 6) lets a licensee observe a program while running it and, in narrow cases, decompile it for interoperability, and a contract cannot take that away. Whether it covers a tracker is untested; nothing here relies on it.

### Clean room: how we would find the data ourselves

Rule 1 stays whole: HearthMirror, HDT, HSTracker, Firestone (and its MindVision), BobsBuddy and unity-spy (a "rewrite of the now closed source HearthMirror") are not read, copied, ported or used.

1. **The runtime is public.** The game ships `MonoBleedingEdge` (seen in the install folder's listing; no game file opened): it runs on Unity's Mono, whose runtime is open source under MIT. Mono's own headers describe its domain, assembly, image, class, field and vtable structures. The reader walks those from the Mono module's exports to the game's assembly, then to a class by name and a field by name, and takes offsets from Mono's metadata at run time, so most patches move nothing we hard-code. Mono's code would be read, not copied, unless a part is worth taking under MIT and recorded in `PROVENANCE.md`.
2. **Finding the class and field names** is the reverse engineering §1.C.i speaks of; there is no way around it. The narrowest form: list class and field names through the same Mono walk while the user's own game is at hero select, and look for the field whose values are the five `Race` numbers the shop rule (D-050) later confirms for that game. No decompiler, no disassembler, no binary patching. The notes go in a research doc with the build number. This step is the most exposed part, and approving D-063 approves it too.
3. **Checks on every read:** exactly five distinct values, each one of the Battlegrounds tribes (D-029's list); and later in the same game, every tribe the shop rule confirms must be among them. Any failure: the value is dropped for that game, the app shows the inferred tribes as today and logs the reason.
4. **Patches:** the build number is already in the log (`DebugPrintGame() - BuildNumber`). The reader keeps a list of builds it has been checked on. On a new build it still reads, and shows the result only if every check passes (marked "unchecked build" in the tooltip). If the class or field is missing or a check fails, it says **"Lobby tribes: unsupported game version"** and shows the inferred tribes, never a guess. Tests replay saved hero-select values (lists of numbers only; no memory dumps in the repo).

### What changes in the docs, the site and the privacy text (rules 3 and 8)

If D-063 is approved, the PR that adds the reader also changes every "only reads the log files" claim. Found today:

- `CLAUDE.md` rule 3; `README.md` (first paragraph and "Lobby tribes and unmet opponents"); `docs/plan/plan.md` "Out of scope" (memory reading).
- Site home `web/src/pages/index.astro`: the description ("only reads the game's log files") and the line "It never reads the game's memory, adds code to it or changes its files" become, in substance: it reads the log files and, if you turn it on, two values the game shows you (the lobby's tribes, your rating) from the game's memory, read-only; it never changes the game, adds code to it or acts for you.
- `docs/legal/terms.md` ("Tavern Ledger only reads the log files … It does not read the game's memory") and `docs/legal/privacy.md` (what the app reads; the rating is the user's own data and stays on the PC). No new data about other players.
- Code comments saying the app only reads the log (`crates/desktop/src/main.rs`, `crates/desktop/src/overlay.rs`, `crates/tracker`).
- The app: a switch, **off by default**, "Read the lobby's tribes and your rating from the game", with one paragraph on the account risk; the value's source label "from the game" next to "inferred" and "entered by you" (T-109).

## 5. What else other apps take from memory

The user's request (2026-10-11): see what else the other apps get, to understand what they do without trouble, and use it too. From public docs, feature lists, changelogs and issues only. Few apps say which item comes from where, so "memory" below is often inferred from the item not being in the log ([log-inventory.md](log-inventory.md)); it says so when it is.

Columns: **On screen** = the game shows it to the player (D-004's line). **Others' data** = personal data of other players (rule 6, GDPR). **Blizzard** = any known reaction. Recommendation: **use** (on screen, our own or no personal data, low risk), **maybe** (needs a user call), **no**.

| Item | Who shows it, and where it comes from | On screen | Others' data | Blizzard | Recommendation |
|---|---|---|---|---|---|
| Lobby tribes (in and out) | HDT (HearthMirror lobby data, [HSTracker 1423](https://github.com/HearthSim/HSTracker/issues/1423); [HDT 4685](https://github.com/hearthsim/hearthstone-deck-tracker/issues/4685)), Firestone ("available / banned tribes" widget; memory inferred). Not in the log (section 1). | Yes: at hero select and in the game's tavern guide (33.2). | No. | Nothing found. | **Use** (D-063). |
| Your own Battlegrounds rating (Solo and Duos) | HDT ("Start MMR" per session, [HDT 4680](https://github.com/hearthsim/hearthstone-deck-tracker/issues/4680)), Firestone ("highest MMR", "Graph your MMR evolution", [features](https://github-wiki-see.page/m/Zero-to-Heroes/firestone/wiki/Firestone-features)). Not in the log ([mmr-sources.md](mmr-sources.md)); the public leaderboard has it only from 8000 (D-012). | Yes: "External rating: this rating is shown in your Battlegrounds lobby interface" (Blizzard developer insight, 2020-09-29, [via HearthstoneTopDecks](https://www.hearthstonetopdecks.com/developer-insights-hearthstone-battlegrounds-rating-system-update/)). The hidden internal MMR is not, and is not read. | No (the user's own). | Nothing found. | **Use** (D-063), kept on the PC. Covers players below 8000, who today see "below the leaderboard cut-off". |
| Main minion type and count per player (leaderboard hover) | HDT issue: "main tribe count" sits in the game's leaderboard object with "some information that shouldn't be exposed" ([HDT 4039](https://github.com/HearthSim/Hearthstone-Deck-Tracker/issues/4039), 2020). Not in the log ([lobby-tribes-log.md](lobby-tribes-log.md)). | Yes (hover a hero). | No (game facts per hero, not per person). | Nothing found. | **Maybe**: on screen and harmless, but a second read path in an object that also holds hidden data. Left out of D-063; a later call. |
| Win streak, last combat result per player | Same object ([HDT 4039](https://github.com/HearthSim/Hearthstone-Deck-Tracker/issues/4039)). | Yes (hover). | No. | Nothing found. | **No need**: results are already in the log (D-055), streaks follow from them. |
| Tavern tier, triples, trinkets per player | Same object. | Yes. | No. | — | **No need**: in the log (D-058). |
| Hero options, rerolls, quests, trinkets offered, Dark Gifts, Old God, next opponent | Trackers show them. | Yes. | No. | — | **No need**: in the log (D-055, D-058, D-061). |
| Anomaly | Firestone (Twitch tooltip). | Yes. | No. | — | **No need**: tag `BACON_GLOBAL_ANOMALY_DBID` exists for it; none this season ([log-inventory.md](log-inventory.md)). Check from the log when anomalies return. |
| Opponents' names, BattleTags, account ids | HearthMirror lobby data ("names, account IDs, hero card IDs", [HSTracker 1423](https://github.com/HearthSim/HSTracker/issues/1423)); Overwolf's game events gave a player "name" ([deprecated GEP](https://dev.overwolf.com/ow-native/live-game-data-gep/supported-games/deprecated/hearthstone)). | Partly (names on the leaderboard); account ids never. | **Yes.** | Nothing found. | **No.** The app never stores or shows other players' names (rule 6, D-012). |
| Opponents' ratings, lobby average rating | Firestone ("View the MMR of your opponents", by "a name lookup" on the official leaderboard; "won't work for low- or medium-MMR players"), HDT plugins (BGrank, Boonwin's "average rank"). Needs the names above. | **No**: the game never shows other players' ratings. | **Yes.** | Nothing found. | **No** (D-004, D-012, rule 6). |
| Opponents' boards or hands before they are shown | — | No. | — | Brode's line: only what "pencil and paper" give. | **No, ever.** |
| Duos teammate's name or rating | HearthMirror lobby data. | Name yes, rating no. | **Yes.** | — | **No.** |
| Your own BattleTag or account id | HDT and HSReplay (to link uploads). | Yes. | No (own), but not needed: our accounts use email (D-020). | — | **No.** |
| Collection, dust, decks, Arena drafts, ranked rank, quests and rewards track, achievements | HDT, Overwolf game events (`collection`, `decks`, `arena`, rank fields). | Yes. | No. | — | **No**: not Battlegrounds; nothing to use. |

So two items earn a memory read: **the lobby's tribes and the player's own rating**. Everything else other apps show is already in our log, is not on the player's screen, or is other players' personal data.

## 6. Decision D-063 (approved by the user on 2026-10-11)

Drafted here pending the user's OK; the user approved it as written the same day (through the hub), and it is in `DECISIONS.md` under "Made". Building it: T-327 (the reader) and T-328 (docs, site and privacy texts).

> **D-063.** Amends D-004 for one read-only memory read, for two values the game shows the player: **the lobby's five tribes** (at hero select) and **the player's own Battlegrounds rating** (Solo and Duos, as the game's Battlegrounds screen shows it). Nothing else is read from memory: no other players' names, BattleTags, account ids or ratings, no boards or hands, nothing the game does not show the player. **How:** from Tavern Ledger's own process, `OpenProcess` with read and query rights only and `ReadProcessMemory`, along one fixed path of fields found through Mono's public runtime structures; a few reads at hero select and on the Battlegrounds screen, never a bulk scan; no writes, no injection, no hooks, no debugger, no elevation (D-006 unchanged). **Clean room:** HearthMirror, HDT, HSTracker, Firestone/MindVision, BobsBuddy and unity-spy are not read or used; finding the class and field is done by us on the user's own client, written down per build, with no decompiler or disassembler. **Checks:** five distinct Battlegrounds tribes, matching every tribe the shop rule (D-050) confirms later in the game; on any failure or unknown structure the app shows "unsupported game version" and the inferred tribes, never a guess. **Users:** a switch, off by default, with the account risk in one paragraph; values marked "from the game" and kept on the PC (not uploaded). **Texts:** the README, the site, the terms, the privacy policy, `CLAUDE.md` rule 3 and code comments drop "only reads the log files" in the same PR. **Risk accepted by the user:** this goes against the letter of Blizzard's EULA (§1.C.vi data mining, §1.C.i reverse engineering) with no written permission; Blizzard has tolerated the same reads by HDT and Firestone for over ten years with no known ban, but it can stop at any time and the account at risk is the player's. **Left for later calls:** each player's main minion type and count (on screen, a second read path), anything else.

Reason: the user wants the exact tribes at hero select (2026-10-11, "all the other apps have them; I won't go without them") and chose log and public sources first, then a read-only memory read. Sections 1 to 3: no log or public route exists; every app that shows them reads memory. The own rating is the one other memory item that is on screen, the user's own data, and fills the gap below the leaderboard's 8000 cut-off.

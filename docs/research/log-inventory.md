# Inventory of everything the game's logs give (T-111)

Session 040, 2026-10-10. The user asked to "get as much information as the game gives us, to see what we can and cannot use". This page lists every file kind the game writes on the user's PC, every line kind in them, and every `tag=` of `Power.log`, with what it means, when it appears, how often, whether the parser reads it today, what it could be used for, whether showing it respects D-004 ("only what the player could see on their screen") and the effort. **Nothing here is implemented**: the user picks from the ranked list at the end.

Script: [`tools/log_inventory.py`](../../tools/log_inventory.py) (read-only, Python standard library). Tests: [`tests/test_log_inventory.py`](../../tests/test_log_inventory.py).

## Summary

- **Data:** the 6 session folders on disk (2026-10-09 to 2026-10-10), 15 file kinds plus `Power.log`. `Power.log`: 6 files, 4,727,438 lines, **11 Battlegrounds games (9 Duos, 2 Solo)**, all build 253216. No other game mode is in these logs.
- **`Power.log` has 792 distinct tags**: 388 with a name and 404 printed only as a number. 593 appear in Solo games, 784 in Duos games (the two Solo games are short, so most of the gap is sample size, not Duos-only data). The parser reads 24 of them today.
- **The other files add little for a Battlegrounds tracker.** `LoadingScreen.log` says when the client moves between the hub, the Battlegrounds lobby and a game (`prevMode`/`currMode`), a cheap signal that a game is about to start. `Hearthstone.log`, `Net.log`, `GameNetLogger.log` and `Login.log` hold network and account values and were read for keys and counts only. `Decks.log` holds deck names the user typed and deck codes: keys and counts only.
- **Worth adding first** (details at the end): the local player's combat result as the game records it (`BACON_WON_LAST_COMBAT`, `DAMAGE_DEALT_TO_HERO_LAST_TURN`) instead of working it out from health (D-028); each opponent's triples and trinkets from the leaderboard tags; the next opponent (`NEXT_OPPONENT_PLAYER_ID`); keywords on boards; the base hero of a skin (`BACON_SKIN_PARENT_ID`) instead of the D-019 rule.
- **Not usable (D-004):** the game's random seed, other players' trinket offers, hidden script data, the copies the game makes of a Duos combat it never plays back, anything in Bob's hand, and every network value. Marked in the tables.
- **Lobby tribes** (session 039's topic): no tag names the five lobby tribes. Tags `BACON_SUBSET_<TRIBE>` sit on cards (which tribe pool a card belongs to) and `CARDRACE` on minions; the game entity carries an unnamed tag `4730` with value 5 in every game. Listed below for session 039; not researched further here.

## Method and privacy

`log_inventory.py` reads a logs folder (one session folder or the folder with the `Hearthstone_<date>` folders). Files are opened for reading only, with Python's default sharing, so the game can keep writing. For each file kind (`Power_old.log` counts as `Power.log`) it counts line kinds: the `Logger.Method()` before ` - `, a `[Section]` prefix, or `<unstructured>`; and the `key=` names in them. For `Power.log` it follows each game from `CREATE_GAME` and counts, for every tag on `GameState` lines (the `PowerTaskList` lines repeat them), the entity types, zones and sides it appears on, its number range or enum values, the game phase (setup, hero select, shop = odd `TURN`, combat = even `TURN`, end = after `STATE=COMPLETE` or the local player's last `PLAYSTATE`), the `TURN` range, the cards it appeared on, and Solo and Duos apart. A tag listed in a `FULL_ENTITY` or `SHOW_ENTITY` block counts on that block's own type, zone and owner.

**Allow-list.** The output only holds line kinds, key and tag names, enum values (all capitals, not hex-like), entity types, card ids (a set code such as `BG36_` or `TB_`, or `Bacon_`), counts, number ranges, and line shapes with every other word replaced by `<w>` and every number by `<n>`. Tag values of 10,000,000 or more are only counted. Files that may hold network or account values (`Net.log`, `GameNetLogger.log`, `Login.log`, `Hearthstone.log`) and `Decks.log` give line kinds, keys and counts only. Keys whose name looks personal (`name`, `account`, `address`, `token`, `owner`, `deck`…) never keep values, not even in a shape. A line kind must look like the game's `Class.Method` (capital letters) or a known `[Section]`. Outside `Power.log`, a line kind, key, shape or enum value is shown only if it appears in two or more session folders; the rest is counted as "seen in one session only". Something that comes back session after session is the game's text, not a name someone typed.

**Guard** (`tools/log_inventory_guard.py`). While reading, the script keeps in memory (never prints) every player name it meets (`PlayerName=` lines, `Player=` in choices, and every name in `TAG_CHANGE Entity=<name>`), the account numbers of `GameAccountId`, and the Windows user name. Before printing, it walks the whole result and withholds any key, text or number that matches one of them, or looks like a BattleTag, an IP address, an e-mail, a path under `Users`, an account id or a deck code; then it checks the result again and prints nothing at all if anything is still there. On the user's logs the guard had nothing left to withhold. The JSON output stayed in the session's scratch folder; this page only carries what the allow-list lets through. The tests feed synthetic lines with fake BattleTags, names, account ids, IPs, paths, e-mails, deck codes, codes after sensitive keys, all-capitals names and names used as keys or line kinds, and check none of them reaches the JSON or the printed summary (28 tests). A code review (no CRITICAL; one HIGH, fixed: shapes kept values of sensitive keys) shaped these rules.

**What was found while building it (and is now handled):**
- Other players' names do appear in `Power.log`: during combat, Bob's player entity (`BACON_DUMMY_PLAYER`) is renamed to the current opponent (`TAG_CHANGE Entity=<opponent's BattleTag>`). Counted per game: 6 to 8 such names, almost all in combat lines. The parser already never stores them (D-017); the inventory counts those tags on `PLAYER_BOB`.
- One Solo game refers to a player only as the plain word `Player` (49 lines). It names nobody, so it is not guarded (otherwise every key called `player` would be withheld).
- `Decks.log` deck codes end in `=` and looked like `key=value` keys. Fixed: a key cannot start inside a base64 text, deck codes are refused, and the file is keys-and-counts only.

**Meaning.** "Means" in the tables is the tag's name read together with what the counts show (which entities carry it, its values, when it appears); for the tags the parser reads it was checked against raw lines in earlier sessions ([parser-hslog.md](parser-hslog.md), [shop-and-apm.md](shop-and-apm.md)). Tag names come from the log itself. No other tracker's code was read (absolute rule 1).

## Files on disk

| File kind | Files | Lines | Section in `log.config` | Read as |
|---|---|---|---|---|
| `Power.log` | 6 | 4,727,438 | `[Power]` (on) | full inventory (below) |
| `Hearthstone.log` | 6 | 35,707 | written always (Unity player log) | keys and counts |
| `LoadingScreen.log` | 6 | 11,526 | `[LoadingScreen]` (on) | kinds, keys, values, shapes |
| `Net.log` | 6 | 412 | `[Net]` (on) | keys and counts |
| `GameNetLogger.log` | 6 | 315 | written always | keys and counts |
| `Achievements.log` | 6 | 217 | `[Achievements]` (on) | kinds, keys, values, shapes |
| `Gameplay.log` | 5 | 136 | written always | kinds, keys, values, shapes |
| `LuckyDraw.log` | 6 | 121 | written always | kinds, shapes |
| `Decks.log` | 6 | 84 | `[Decks]` (on) | keys and counts |
| `ExceptionReporter.log` | 6 | 36 | written always | kinds, shapes |
| `FullScreenFX.log` | 6 | 33 | `[FullScreenFX]` (on) | kinds, shapes |
| `Login.log` | 6 | 18 | written always | keys and counts |
| `Downloader.log` | 6 | 6 | written always | kinds, shapes |
| `Store.log` | 6 | 6 | written always | kinds, shapes |
| `Spells.log` | 1 | 2 | written always | kinds, shapes |
| `Arena.log` | 1 | 1 | `[Arena]` (on) | kinds, shapes |

"Written always" means the file exists although its section is not in the user's `log.config` (which enables `[Achievements] [Arena] [FullScreenFX] [LoadingScreen] [Power] [Decks] [Net]`).

## Files other than `Power.log`

Columns as for `Power.log`: what it means, when, counts, used today, possible use, allowed (D-004), effort. "Kinds" are the `Logger.Method` names the lines carry; counts are over the 6 folders.

### `LoadingScreen.log`

| Item | Means | When | Count | Used today | Possible use | Allowed | Effort |
|---|---|---|---|---|---|---|---|
| `LoadingScreen.OnSceneLoaded` `prevMode`, `currMode` | Scene change. Values seen: `HUB`, `BACON` (Battlegrounds lobby), `GAMEPLAY`, `LOGIN`, `STARTUP`, `DRAFT`, `PACKOPENING` | each scene change | 80 | no | "game starting" / "back in lobby" signal for the app and overlay, before `Power.log` has a `CREATE_GAME` | yes (the player's own screen) | low |
| `LoadingScreen.OnSceneUnloaded`, `OnScenePreUnload` `prevMode`, `nextMode`, `m_phase` | The same change seen from the scene that closes | each scene change | 63, 41 | no | same | yes | low |
| `Gameplay.OnPowerHistory` `powerList=<n>` | A batch of power packets reached the client (size) | during games | 8,231 | no | none | n/a | - |
| `Gameplay.OnAllOptions` `id=<n>`, `Gameplay.OnEntityChoices` `id=<n>` | Options and choices reached the client (same ids as `Power.log`) | during games | 2,511, 242 | no | none (`Power.log` has them in full) | n/a | - |
| `MulliganManager.HandleGameStart`, `WaitForHeroesAndStartAnimations` | Hero select starts | each game | 22, 11 | no | hero-select start time | yes | low |
| `Gameplay.Awake`/`Start`/`Unload`/`OnDestroy`, `Box.Awake`/`OnDestroy` | Game scene and main menu created or destroyed | each game | 11 each, 17 | no | same as scene change | yes | low |
| `LoadingScreen.*` timestamps (`m_assetLoad*Timestamp`), fades, transitions | Loading-screen timing (large numbers, only counted) | each scene change | 21 to 44 each | no | none | n/a | - |

### `Achievements.log`

| Item | Means | When | Count | Used today | Possible use | Allowed | Effort |
|---|---|---|---|---|---|---|---|
| `NetCache.OnProfileNotices` `NoticeID`, `Type`, `Origin`, `OriginData`, `Date` | Account notices at login; the only `Type` seen is `MERCENARIES_SEASON_ROLL` (`NOTICE_ORIGIN_MERCENARIES`) | at login | 177 | no | none for Battlegrounds | n/a | - |
| seen in one session only | Lines of a kind that came up in one session (`AchieveManager.*`) | menus | 22 | no | none | n/a | - |
| `<unstructured>` with `total=` | Counter lines (value always 0) | at login | 18 | no | none | n/a | - |

### `Gameplay.log`

| Item | Means | When | Count | Used today | Possible use | Allowed | Effort |
|---|---|---|---|---|---|---|---|
| `<unstructured>` (130 of the same shape) | Client gameplay messages quoting an object name; a few quote an entity (`id`, `cardId`, `zone`, `zonePos`, `player`) | during games | 136 | no | none (`Power.log` has the same entities) | n/a | - |

### `LuckyDraw.log`, `Store.log`, `Downloader.log`, `ExceptionReporter.log`, `FullScreenFX.log`, `Spells.log`, `Arena.log`

| File | Item | Means | Count | Used today | Possible use | Allowed | Effort |
|---|---|---|---|---|---|---|---|
| `LuckyDraw.log` | `<unstructured>` (shapes `<w…> <n> <w>: <w…>`, one with `DFT`) | Battlegrounds "Lucky Draw" event messages at login | 121 | no | none | n/a | - |
| `Store.log` | `<unstructured>` | One store message per session | 6 | no | none | n/a | - |
| `Downloader.log` | `<unstructured>` ending in `DBF` | Game data download check, one per session | 6 | no | none | n/a | - |
| `ExceptionReporter.log` | `<unstructured>` incl. `UUID:` | Crash-reporter start-up (an install id; withheld as a word) | 36 | no | none | no: an install id | - |
| `FullScreenFX.log` | `<unstructured>` | Screen effects switched on and off | 33 | no | none | n/a | - |
| `Spells.log` | seen in one session only | One file, 2 lines (shape not shown) | 2 | no | none | n/a | - |
| `Arena.log` | seen in one session only | One file, 1 line (shape not shown) | 1 | no | none | n/a | - |

### Keys and counts only: `Hearthstone.log`, `Net.log`, `GameNetLogger.log`, `Login.log`, `Decks.log`

| File | Line kinds (count) | Keys seen | Possible use | Allowed | Effort |
|---|---|---|---|---|---|
| `Hearthstone.log` | `<unstructured>` (34,058; keys `id`, `cardId`, `zone`, `zonePos`, `player`, `tempGuardianVars`, `dir`), `BaconTrinketWidget.UpdatePortrait` (707), `[GameNetLogger]` (314), `DefLoader.GetEntityDef` (188), `[Gameplay]` (136), `[LuckyDraw]` (121), `AttackSpellController.DetermineAttackType` (85: `prevAttackType`, `prevAttacker`, `prevDefender`), `[Startup]` (24), `[Login]` (18), `Gameplay.Awake` (11: `CurrentMode`, `PrevMode`), 6 more kinds with 2 to 8 lines, and 12 lines of kinds seen in one session only | entity references, network keys (`address`, `game`, `client`, `spectateKey`, `reconnecting`) | none: it repeats the other logs and adds network values | **no** for network values; the rest is in other files | - |
| `Net.log` | `<unstructured>` (411), one line of a kind seen in one session only | none in `key=` form | none | **no** (network) | - |
| `GameNetLogger.log` | `GameMgr.ChangeFindGameState` (88), `RpcController.SendData` (63), `Network.GotoGameServe` (24: `address`, `game`, `client`, `spectateKey`, `reconnecting`), `Network.OnGameServerConnectEvent` (24), `Network.DisconnectFromGameServer` (16), `Network.SendGameServerHandshake` (12), `Network.OnGameServerDisconnectEvent` (12), `GameMgr.OnGameSetup` (11), `<unstructured>` (64), one line of a kind seen in one session only | network keys | matchmaking start and game-server connect times (queue time); reconnects | **no** for addresses and keys; the line kinds and times alone would be allowed but need a strict reader (medium) | medium |
| `Login.log` | `<unstructured>` (18) | none | none | **no** (account) | - |
| `Decks.log` | `<unstructured>` (84) | none (deck codes are not keys) | none for Battlegrounds | **no** (deck names typed by the user) | - |

## `Power.log`

### Line kinds

| Line kind | Lines | What it is | Used today |
|---|---|---|---|
| `GameState.DebugPrintPower` | 1,992,450 | Every game change: `TAG_CHANGE` (1,003,740), entity tags `tag=` (764,318), `BLOCK_START`/`BLOCK_END` (46,693/46,537), `FULL_ENTITY` (38,988), `SHOW_ENTITY` (20,888), `HIDE_ENTITY` (18,857), `META_DATA` (12,565) with `Info[]` (21,141), `SUB_SPELL_START`/`END` (4,726 each) with `Source`/`Targets[]`, `CHANGE_ENTITY` (77), `CREATE_GAME`/`GameEntity` (11), `Player` (22) | yes (the parser's input) |
| `PowerTaskList.DebugPrintPower` | 1,995,182 | The same packets again as the client plays them back (so with display timing) | no (repeats) |
| `GameState.DebugPrintOptions` | 275,812 | The options the client offers the local player: `option` (93,701), `target` (179,321), `subOption`; each with `type` (`POWER` 91,119, `END_TURN` 2,582) and `error` (why not playable: `REQ_ENOUGH_MANA` 9,433, `REQ_MINION_CAP` 1,537, `REQ_YOUR_TURN`…) | no |
| `PowerTaskList.DebugDump`, `PowerProcessor.*` | 217,592 + 232,528 | Client task-list bookkeeping | no |
| `GameState.DebugPrintPowerList` | 8,231 | `Count=` of each packet batch | no |
| `GameState.SendOption` | 2,303 | One option the local client sent | yes (APM, D-046) |
| `GameState.DebugPrintEntityChoices` | 1,242 | A choice shown to the local player: `ChoiceType=MULLIGAN` (hero select, 11) or `GENERAL` (discover and the like, 231), its `Source` and every offered `Entities[]` | no |
| `GameState.DebugPrintEntitiesChosen`, `GameState.SendChoices` | 484, 482 | What the local player picked | yes (`SendChoices` count, D-046) |
| `ChoiceCardMgr.*` | 626 | Client showing and hiding choices | no |
| `GameState.DebugPrintGame` | 66 | `GameType`, `FormatType`, `ScenarioID`, `BuildNumber`, and `PlayerID=…, PlayerName=…` (names: never kept) | yes (mode, build) |
| `<unstructured>` | 440 | Lines without a logger (counted only) | no |

### Blocks, metadata, options and choices

| Item | Means | Count | Used today | Possible use | Allowed | Effort |
|---|---|---|---|---|---|---|
| `BLOCK_START BlockType=TRIGGER` | Triggered effects (37,055; mostly the turn-start `TB_BaconShop_8P_PlayerE` and the triple checker) | 37,055 | no | none | n/a | - |
| `BlockType=POWER` | Effects of buttons and hero powers (roll refreshes, sells, triples) | 3,546 | partly (D-046) | none more | yes | - |
| `BlockType=PLAY` | A card or button played (buy, sell, roll, Dark Gift, minions) | 1,902 | yes (shop record) | discovered and played cards in recap | own | low |
| `BlockType=ATTACK` | One attack in combat | 2,049 | yes (boards at first attack) | combat replay | yes | high |
| `BlockType=DEATHS` | Deaths resolved | 1,744 | no | combat replay | yes | high |
| `BlockType=MOVE_MINION` | Local player moved a minion | 354 | yes (APM) | none more | own | - |
| `BlockType=DECK_ACTION` | Duos pass and similar | 43 | counted as "other" | Duos passes in shop record | own | low |
| `META_DATA Meta=` | `SLUSH_TIME` 4,510, `DAMAGE` 3,942, `CONTROLLER_AND_ZONE_CHANGE` 1,823, `ARTIFICIAL_PAUSE` 670, `HISTORY_TARGET` 463, `TARGET` 276, `OVERRIDE_HISTORY` 272, `SHOW_BIG_CARD` 92, `SPEND_HEALTH` 56, `POISONOUS` 37, `EFFECT_TIMING` 19, … | 12,565 | no | `DAMAGE`: damage per hit for a combat replay | yes | high |
| `DebugPrintOptions … error=` | Why each option was not playable now | 101,672 `NONE` + 23 other values | no | none (UI state) | own | - |
| `DebugPrintEntityChoices ChoiceType=MULLIGAN` + `Entities[]` | The heroes offered at hero select | 11 | no | stats: heroes offered vs picked, per hero pick rate | own (shown to the player) | medium |
| `DebugPrintEntityChoices ChoiceType=GENERAL` + `Entities[]` | Discover offers (minions, spells, trinkets) | 231 | no | recap: what was offered and what was picked | own | medium |

### Entity types and cards

| Card type | Entities | Distinct cards | Most common |
|---|---|---|---|
| `MINION` | 9,408 | 479 | `BG36_345`, `BGS_034`, `BGS_119` |
| `ENCHANTMENT` | 16,108 | 278 | `TB_BaconShopBadsongE`, `BG36_301te`, `BG_ShopBuff_Ench` |
| `SPELL` | 2,689 | 150 | `BG_OldGod`, `TB_BaconShop_CheckTriples`, `BG20_GEM_No_Impact` |
| `MOVE_MINION_HOVER_TARGET` | 2,359 | 3 | `TB_BaconShop_DragBuy`, `…DragBuy_Spell`, `…DragSell` |
| `HERO` | 814 | 105 | lobby heroes, their combat copies and Bob (`TB_BaconShopBob_SKIN_*`) |
| `BATTLEGROUND_TRINKET` | 792 | 120 | `BG30_Trinket_1st`/`2nd` (the trinket discovers), `BG30_MagicItem_*` |
| `BATTLEGROUND_SPELL` | 771 | 64 | tavern spells (`BG31_886`, `BG34_444`, `BG28_168`) |
| `HERO_POWER` | 484 | 78 | `BG36_HERO_002p`, `TB_BaconShop_HP_048` |
| `GAME_MODE_BUTTON` | 404 | 8 | roll, freeze, tier-up 2 to 6, `BG36_Button_DarkGift` |
| `BATTLEGROUND_QUEST_REWARD` | 3 | 2 | `BG28_Reward_509`, `BG24_Reward_306` |
| `PET` | 1 | 1 | `PET_3_4` (a cosmetic pet, zone `COSMETIC`) |

No anomaly entity is in these logs (no `BATTLEGROUND_ANOMALY` card type and no anomaly card ids). This season's mechanics show instead as Dark Gifts (`BG36_Button_DarkGift`, `HAS_DARK_GIFT`), an Old God per game (`BG_OldGod`, `BACON_GLOBAL_OLD_GOD_DBID`), mid-game effects (`BG36_MidGameEffect_*`), Timewarped taverns (`BG34_Giant_*`, `BACON_TIMEWARPED`) and trinkets.

### Tags

One table per group. Columns: **On** = entity types that carry it (`GAME`, `PLAYER` = local player, `PLAYER_BOB` = Bob's player, which stands for the combat opponent in combat, or a card type); **When** = game phases and the `TURN` range (odd = shop, even = combat); **Count** = all games (Solo / Duos; number of games with it); **Allowed** = whether showing it keeps D-004 ("yes (own)" = about the local player; "yes (on screen)" = the game shows it to the player; "n/a" = a technical flag with no meaning to show; "no" = the player never sees it). Effort "-" = nothing worth doing.

#### Game (33 tags)

| Tag | Means | On | Values | When | Count (Solo/Duos; games) | Used today | Possible use | Allowed (D-004) | Effort |
|---|---|---|---|---|---|---|---|---|---|
| `MISSION_EVENT` | Counter of scripted events (0..129) | GAME | 0..129 | shop, combat; TURN 1-30 | 3006 (S 402 / D 2604; 11 g) | no | none | n/a (no player meaning) | - |
| `NEXT_STEP` | Step the engine moves to next | GAME | MAIN_READY, MAIN_START_TRIGGERS, MAIN_START, MAIN_CLEANUP, MAIN_NEXT, MAIN_ACTION… | hero select, shop, combat, end; TURN 1-30 | 1698 (S 262 / D 1436; 11 g) | no | none | n/a (no player meaning) | - |
| `STEP` | Engine step of the turn (MAIN_READY, MAIN_ACTION, MAIN_END…) | GAME | MAIN_READY, MAIN_START_TRIGGERS, MAIN_END, MAIN_CLEANUP, MAIN_NEXT, MAIN_START… | hero select, shop, combat, end; TURN 1-30 | 1687 (S 260 / D 1427; 11 g) | no | shop/combat timing | yes | low |
| `TURN` | Game turn: odd = shop, even = combat; also on both players | GAME, PLAYER, PLAYER_BOB | 1..30 | hero select, shop, combat; TURN 1-30 | 492 (S 76 / D 416; 11 g) | rounds and shop turns (`collector.rs`) | already used | yes | - |
| `STATE` | Game state: RUNNING, COMPLETE | GAME | RUNNING, COMPLETE | hero select, end; TURN 16-30 | 22 (S 4 / D 18; 11 g) | game end (`collector.rs`) | already used | yes | - |
| `BACON_MAX_LEADERBOARD_ARMOR` | Maximum armor a hero can start with (20..40) | GAME | 20..40 | hero select | 20 (S 2 / D 18; 11 g) | no | none | yes | - |
| `END_TURN_BUTTON_ALTERNATIVE_APPEARANCE` | UI flag | GAME | 3..3 | setup | 11 (S 2 / D 9; 11 g) | no | none | n/a (no player meaning) | - |
| `COIN_MANA_GEM` | UI: gold shown as coins | GAME | 1..1 | setup | 11 (S 2 / D 9; 11 g) | no | none | n/a (no player meaning) | - |
| `TECH_LEVEL_MANA_GEM` | UI flag | GAME | 1..1 | setup | 11 (S 2 / D 9; 11 g) | no | none | n/a (no player meaning) | - |
| `DISABLE_TURN_INDICATORS` | UI flag | GAME | 1..1 | setup | 11 (S 2 / D 9; 11 g) | no | none | n/a (no player meaning) | - |
| `ALWAYS_USE_FAST_ACTOR_TRIGGERS` | Engine flag | GAME | 1..1 | setup | 11 (S 2 / D 9; 11 g) | no | none | n/a (no player meaning) | - |
| `DISABLE_NONHERO_GOLDEN_ANIMATIONS` | UI flag | GAME | 1..1 | setup | 11 (S 2 / D 9; 11 g) | no | none | n/a (no player meaning) | - |
| `ALLOW_GAME_SPEEDUP` | Engine flag | GAME | 1..1 | setup | 11 (S 2 / D 9; 11 g) | no | none | n/a (no player meaning) | - |
| `COIN_MANA_GEM_FOR_CHOICE_CARDS` | UI flag | GAME | 1..1 | setup | 11 (S 2 / D 9; 11 g) | no | none | n/a (no player meaning) | - |
| `GAME_SEED` | Random seed of the game | GAME |  | setup | 11 (S 2 / D 9; 11 g) | no | none | no: never on screen, could help predict | - |
| `BG_COMBAT_SPEED_START_TIME` | Combat speed-up setting | GAME | 10..10 | setup | 11 (S 2 / D 9; 11 g) | no | none | n/a (no player meaning) | - |
| `BG_COMBAT_SPEED_ACCELERATION` | Combat speed-up setting | GAME | 30..30 | setup | 11 (S 2 / D 9; 11 g) | no | none | n/a (no player meaning) | - |
| `BG_COMBAT_SPEED_DECELERATION` | Combat speed-up setting | GAME | 300..300 | setup | 11 (S 2 / D 9; 11 g) | no | none | n/a (no player meaning) | - |
| `BG_COMBAT_SPEED_MAX_SPEED` | Combat speed-up setting | GAME | 2200..2400 | setup | 11 (S 2 / D 9; 11 g) | no | none | n/a (no player meaning) | - |
| `BG_COMBAT_SPEED_ATTACKS_REMAINING_BEFORE_SLOW_DOWN_MIN` | Combat speed-up setting | GAME | 5..5 | setup | 11 (S 2 / D 9; 11 g) | no | none | n/a (no player meaning) | - |
| `BACON_MULLIGAN_HERO_REROLL_ACTIVE` | Hero reroll at hero select is on | GAME | 1..1 | setup | 11 (S 2 / D 9; 11 g) | no | none | yes | - |
| `BG_COMBAT_SPEED_MIN_COMBAT_EVENTS_REMAINING_TO_START` | Combat speed-up setting | GAME | 10..10 | setup | 11 (S 2 / D 9; 11 g) | no | none | n/a (no player meaning) | - |
| `BG_COMBAT_SPEED_ATTACKS_REMAINING_BEFORE_SLOW_DOWN_MAX` | Combat speed-up setting | GAME | 8..8 | setup | 11 (S 2 / D 9; 11 g) | no | none | n/a (no player meaning) | - |
| `BACON_FACTION_BANNERS_ENABLED` | UI flag | GAME | 1..1 | setup | 11 (S 2 / D 9; 11 g) | no | none | n/a (no player meaning) | - |
| `ALLOW_MOVE_MINION` | Engine flag: minions can be moved | GAME | 1..1 | hero select | 11 (S 2 / D 9; 11 g) | no | none | n/a (no player meaning) | - |
| `ALLOW_MOVE_BACON_SPELL` | Engine flag | GAME | 1..1 | hero select | 11 (S 2 / D 9; 11 g) | no | none | n/a (no player meaning) | - |
| `FORCE_NO_CUSTOM_SUMMON_SPELLS` | UI flag | GAME | 1..1 | hero select | 11 (S 2 / D 9; 11 g) | no | none | n/a (no player meaning) | - |
| `FORCE_NO_CUSTOM_LIFETIME_SPELLS` | UI flag | GAME | 1..1 | hero select | 11 (S 2 / D 9; 11 g) | no | none | n/a (no player meaning) | - |
| `USE_FAST_ACTOR_TRANSITION_ANIMATIONS` | UI flag | GAME | 1..1 | hero select | 11 (S 2 / D 9; 11 g) | no | none | n/a (no player meaning) | - |
| `DONT_SUPPRESS_KEYWORD_VO` | Voice flag | GAME | 1..1 | hero select | 11 (S 2 / D 9; 11 g) | no | none | n/a (no player meaning) | - |
| `BACON_USE_COIN_BASED_BUDDY_METER` | UI flag | GAME | 1..1 | hero select | 11 (S 2 / D 9; 11 g) | no | none | n/a (no player meaning) | - |
| `BACON_NUM_MAX_REROLL_PER_HERO` | Rerolls allowed per hero at hero select | GAME | 1..1 | hero select | 11 (S 2 / D 9; 11 g) | no | none | yes | - |
| `TUTORIAL_HERO_POWER_TARGET_MINION_ANIM` | Tutorial flag (Solo) | GAME | 20..20 | hero select | 2 (S 2 / D 0; 2 g) | no | none | n/a (no player meaning) | - |

#### Player and hero (55 tags)

| Tag | Means | On | Values | When | Count (Solo/Duos; games) | Used today | Possible use | Allowed (D-004) | Effort |
|---|---|---|---|---|---|---|---|---|---|
| `HEALTH` | Health (heroes and minions) | MINION, HERO | 0..80391 | hero select, shop, combat; TURN 1-30 | 38667 (S 2883 / D 35784; 11 g) | hero health, minion stats (`collector.rs`) | already used | yes (on screen) | - |
| `DAMAGE` | Damage taken | MINION, HERO | 0..46911 | shop, combat; TURN 2-30 | 8837 (S 503 / D 8334; 11 g) | hero health, minion stats (`collector.rs`) | already used | yes (on screen) | - |
| `PLAYER_TECH_LEVEL` | Tavern tier of each lobby hero | HERO, ENCHANTMENT, PLAYER | 0..6 | setup, shop, combat, end; TURN 1-30 | 2879 (S 208 / D 2671; 11 g) | tier and tier-ups, live tier (`collector.rs`, `shop.rs`, `lib.rs`) | already used | yes (on screen) | - |
| `ARMOR` | Armor (heroes; a few minions) | HERO, MINION, SPELL | 0..32 | hero select, shop, combat, end; TURN 1-30 | 2554 (S 193 / D 2361; 11 g) | hero health (`collector.rs`) | already used | yes (on screen) | - |
| `NUM_OPTIONS_PLAYED_THIS_TURN` | Options sent this turn (local) | PLAYER | 0..98 | shop; TURN 1-29 | 2409 (S 289 / D 2120; 11 g) | no | cross-check APM | yes (own) | low |
| `BACON_MAX_PLAYER_TECH_LEVEL` | Highest tier a hero can reach (6, 7 with effects) | HERO, ENCHANTMENT, PLAYER | 0..6 | setup, shop, combat; TURN 1-30 | 2362 (S 117 / D 2245; 11 g) | no | none | yes (on screen) | - |
| `CORPSES` | Undead corpses (per side) | PLAYER_BOB, PLAYER | 1..214 | shop, combat; TURN 2-30 | 2159 (S 118 / D 2041; 11 g) | no | none | yes (on screen) | - |
| `HERO_POWER` | Database id of a hero's hero power | HERO, HERO_POWER | 0..134672 | hero select, shop, combat, end; TURN 1-30 | 2088 (S 136 / D 1952; 11 g) | no | recap: hero power used | yes (on screen) | low |
| `BACON_HEROPOWER_BASE_HERO_ID` | Base hero of a hero power | HERO_POWER | 0..134504 | hero select, shop, combat, end; TURN 1-30 | 1469 (S 94 / D 1375; 11 g) | no | skin grouping check | yes | low |
| `BACON_HERO_CAN_BE_DRAFTED` | Hero is in the draftable pool | HERO | 0..1 | hero select, combat; TURN 2-30 | 1177 (S 109 / D 1068; 11 g) | no | none | yes | - |
| `PLAYER_LEADERBOARD_PLACE` | Place on the leaderboard | HERO | 1..8 | hero select, shop, combat, end; TURN 1-30 | 1104 (S 289 / D 815; 11 g) | place, live leaderboard (`collector.rs`, `lib.rs`) | already used | yes (on screen) | - |
| `RESOURCES_USED` | Gold used this turn | PLAYER | 0..18 | shop; TURN 1-29 | 1071 (S 135 / D 936; 11 g) | no | cross-check gold spent | yes (own) | low |
| `NUM_CARDS_PLAYED_THIS_TURN` | Cards played this turn (local) | PLAYER | 0..45 | shop; TURN 1-29 | 967 (S 116 / D 851; 11 g) | no | shop record | yes (own) | low |
| `PLAYER_ID` | Lobby seat of a hero (1..8; 9..16 on combat copies) | HERO, PLAYER, PLAYER_BOB | 1..16 | setup, hero select, combat; TURN 2-30 | 760 (S 60 / D 700; 11 g) | lobby seats (`collector.rs`) | already used | yes | - |
| `HERO_ENTITY` | Hero a player entity is playing (swaps in combat) | PLAYER_BOB, PLAYER | 25..44745 | setup, hero select, combat; TURN 2-30 | 552 (S 42 / D 510; 11 g) | who fights whom (`collector.rs`) | already used | yes | - |
| `NUM_MINIONS_PLAYED_THIS_TURN` | Minions played this turn (local) | PLAYER | 0..23 | shop; TURN 1-29 | 552 (S 55 / D 497; 11 g) | no | shop record | yes (own) | low |
| `CURRENT_PLAYER` | Whose turn it is | PLAYER_BOB, PLAYER | 0..1 | hero select, shop, combat; TURN 1-29 | 503 (S 78 / D 425; 11 g) | no | none | n/a (no player meaning) | - |
| `NUM_SPELLS_PLAYED_THIS_GAME` | Spells played this game (local) | PLAYER | 1..127 | shop; TURN 1-29 | 414 (S 60 / D 354; 11 g) | no | stats | yes (own) | low |
| `TAVERN_SPELL_ATTACK_INCREASE` | Tavern spell attack bonus (per side) | PLAYER, PLAYER_BOB, ENCHANTMENT | 0..218 | shop, combat; TURN 6-30 | 414 (S 2 / D 412; 10 g) | no | recap: bonus over time | own yes; opponent check | low |
| `TAVERN_SPELL_HEALTH_INCREASE` | Tavern spell health bonus (per side) | PLAYER, PLAYER_BOB, ENCHANTMENT | 0..34 | shop, combat; TURN 6-30 | 278 (S 0 / D 278; 8 g) | no | recap: bonus over time | own yes; opponent check | low |
| `TIMEOUT` | Turn timer in seconds (25..125) | PLAYER | 25..125 | hero select, shop; TURN 1-29 | 262 (S 28 / D 234; 11 g) | no | shop record: time per turn | yes (own) | low |
| `NEXT_OPPONENT_PLAYER_ID` | Seat of the next combat opponent | HERO, PLAYER | 1..8 | setup, shop, combat, end; TURN 1-29 | 255 (S 40 / D 215; 11 g) | no | overlay: next opponent's last board | yes (game marks next opponent) | low |
| `PLAYER_TRIPLES` | Triples made by each lobby hero | HERO | 1..8 | shop, combat, end; TURN 5-27 | 229 (S 27 / D 202; 11 g) | no | overlay hover, recap per opponent | yes (leaderboard tooltip) | low |
| `SUPPRESS_SUMMON_VO_FOR_PLAYER` | Voice flag | PLAYER, PLAYER_BOB | 0..1 | setup, combat; TURN 2-30 | 219 (S 2 / D 217; 11 g) | no | none | n/a (no player meaning) | - |
| `COMBO_ACTIVE` | A card was already played this turn | PLAYER | 0..1 | shop; TURN 1-29 | 210 (S 28 / D 182; 11 g) | no | none | n/a (no player meaning) | - |
| `SKIP_ARMOR_ANIMATION` | UI flag | HERO | 0..1 | shop; TURN 11-23 | 136 (S 0 / D 136; 3 g) | no | none | n/a (no player meaning) | - |
| `BACON_HERO_POWER_ACTIVATED` | Hero power was used (on/off powers) | HERO_POWER | 0..1 | hero select, shop, combat; TURN 2-30 | 135 (S 2 / D 133; 7 g) | no | with the above | yes | low |
| `HERO_POWER_ENTITY` | Entity id of a hero's hero power | HERO | 122..42486 | hero select, shop, end; TURN 1-29 | 128 (S 15 / D 113; 11 g) | no | none | yes | - |
| `EXTRA_BATTLECRIES_BASE` | Battlecries trigger extra times (per side) | PLAYER, PLAYER_BOB | 0..2 | shop, combat; TURN 12-30 | 119 (S 6 / D 113; 11 g) | no | none | yes (on screen) | - |
| `TEMP_RESOURCES` | Extra gold this turn | PLAYER | 0..6 | shop; TURN 3-23 | 119 (S 11 / D 108; 10 g) | no | shop record: full gold | yes (own) | low |
| `MAXHANDSIZE` | Max hand size | PLAYER, PLAYER_BOB | 10..99 | setup, shop; TURN 5-25 | 94 (S 10 / D 84; 11 g) | no | none | yes | - |
| `USE_LEADERBOARD_AS_SPAWN_ORIGIN` | UI flag for lobby heroes | HERO | 1..1 | hero select | 77 (S 14 / D 63; 11 g) | no | none | n/a (no player meaning) | - |
| `BACON_BLOODGEMBUFFHEALTHVALUE` | Blood gem health value (per side) | PLAYER_BOB, PLAYER | 0..36 | shop, combat; TURN 6-30 | 74 (S 10 / D 64; 5 g) | no | recap: blood gem level | own yes; opponent check | low |
| `PLAYSTATE` | PLAYING, LOSING, LOST, WON of a player | PLAYER_BOB, PLAYER | PLAYING, LOSING, LOST, WON | hero select, combat, end; TURN 16-30 | 71 (S 10 / D 61; 11 g) | no | elimination time, win marker | yes (own) | low |
| `EXTRA_DEATHRATTLES_ADDITIONAL` | Deathrattles trigger extra times (per side) | PLAYER, PLAYER_BOB | 0..2 | shop, combat; TURN 12-28 | 67 (S 7 / D 60; 9 g) | no | none | yes (on screen) | - |
| `BACON_BLOODGEMBUFFATKVALUE` | Blood gem attack value (per side) | PLAYER_BOB, PLAYER | 0..26 | shop, combat; TURN 10-30 | 58 (S 12 / D 46; 5 g) | no | recap: blood gem level | own yes; opponent check | low |
| `HERO_POWER_DISABLED` | Hero power disabled | HERO_POWER | 0..1 | combat; TURN 2-26 | 46 (S 9 / D 37; 7 g) | no | none | yes | - |
| `MULLIGAN_STATE` | Hero-select state (INPUT, DEALING, WAITING, DONE) | PLAYER | INPUT, DEALING, WAITING, DONE | hero select | 44 (S 8 / D 36; 11 g) | no | hero-select time | yes (own) | low |
| `HEROPOWER_ACTIVATIONS_THIS_TURN` | Hero power uses this turn | PLAYER, HERO_POWER | 0..1 | shop; TURN 1-27 | 42 (S 34 / D 8; 2 g) | no | shop record: hero power per turn | yes (own) | low |
| `BACON_PLAYER_EXTRA_GOLD_NEXT_TURN` | Extra gold promised for next turn | PLAYER | 0..4 | shop; TURN 1-23 | 37 (S 7 / D 30; 8 g) | no | overlay: next turn gold | yes (own) | low |
| `STARTHANDSIZE` | Start hand size | PLAYER, PLAYER_BOB | 4..4 | setup | 22 (S 4 / D 18; 11 g) | no | none | yes | - |
| `TEAM_ID` | Team of a player entity | PLAYER, PLAYER_BOB | 1..16 | setup | 22 (S 4 / D 18; 11 g) | no | none | yes | - |
| `CANT_DRAW` | Engine flag on players | PLAYER, PLAYER_BOB | 1..1 | setup | 22 (S 4 / D 18; 11 g) | no | none | n/a (no player meaning) | - |
| `MAX_EXCAVATE_TIER` | Constructed mechanic default | PLAYER, PLAYER_BOB | 2..2 | setup | 22 (S 4 / D 18; 11 g) | no | none | n/a (no player meaning) | - |
| `NUM_TURNS_LEFT` | Engine counter | PLAYER, PLAYER_BOB | 1..1 | hero select | 22 (S 4 / D 18; 11 g) | no | none | n/a (no player meaning) | - |
| `FIRST_PLAYER` | Engine: Bob's player goes first | PLAYER_BOB | 1..1 | hero select | 11 (S 2 / D 9; 11 g) | no | none | n/a (no player meaning) | - |
| `MAXRESOURCES` | Max gold (99) | PLAYER | 99..99 | shop; TURN 1-5 | 11 (S 2 / D 9; 11 g) | no | none | yes | - |
| `NUM_TIMES_HERO_POWER_USED_THIS_GAME` | Hero power uses this game | PLAYER | 1..9 | shop; TURN 1-25 | 11 (S 9 / D 2; 2 g) | no | stats per hero | yes (own) | low |
| `BACON_PREMIUM_FREE_REROLLS` | Player has premium (Tavern Pass) free rerolls | PLAYER | 1..1 | setup | 10 (S 1 / D 9; 10 g) | no | none (account perk) | own, but not needed | - |
| `BACON_NUM_FREE_REROLLS_USED` | Free hero rerolls used | PLAYER | 1..1 | hero select | 10 (S 1 / D 9; 10 g) | no | hero-select record | yes (own) | low |
| `BACON_NUM_MULLIGAN_REFRESH_USED` | Hero rerolls used at hero select | HERO | 1..1 | hero select | 10 (S 1 / D 9; 10 g) | no | hero-select record | yes (own) | low |
| `BACON_LOCKED_MULLIGAN_HERO` | A hero option locked at hero select (Solo) | HERO | 1..1 | hero select | 2 (S 2 / D 0; 1 g) | no | hero-select record | yes (own) | low |
| `BACON_COMEONECOMEALL` | Hero power counter | HERO_POWER | 1..2 | shop; TURN 21-27 | 2 (S 0 / D 2; 1 g) | no | none | yes (own) | - |
| `ADDITIONAL_HERO_POWER_ENTITY_1` | Second hero power entity | HERO | 1893..1893 | shop; TURN 7 | 1 (S 1 / D 0; 1 g) | no | none | yes | - |
| `IS_SHOP_CHOICE` | Hero power that offers a shop choice | HERO_POWER | 1..1 | hero select | 1 (S 0 / D 1; 1 g) | no | none | yes | - |

#### Minions (88 tags)

| Tag | Means | On | Values | When | Count (Solo/Duos; games) | Used today | Possible use | Allowed (D-004) | Effort |
|---|---|---|---|---|---|---|---|---|---|
| `SPAWN_TIME_COUNT` | Engine counter on creation | MINION, ENCHANTMENT, SPELL | 0..1 | setup, hero select, shop, combat, end; TURN 1-30 | 72160 (S 6027 / D 66133; 11 g) | no | none | n/a (no player meaning) | - |
| `ATK` | Attack | MINION, HERO | 0..46129 | shop, combat; TURN 1-30 | 38228 (S 2849 / D 35379; 11 g) | minion stats (`collector.rs`) | already used | yes (on screen) | - |
| `TECH_LEVEL` | Tier of a card | MINION, BATTLEGROUND_SPELL, SPELL | 0..7 | hero select, shop, combat; TURN 1-30 | 33235 (S 2705 / D 30530; 11 g) | no | recap: tier of bought minions; possible-minions pool check | yes | low |
| `ZONE_POSITION` | Slot on board or in hand | MINION, SPELL, BATTLEGROUND_SPELL | 0..10 | hero select, shop, combat; TURN 1-30 | 32337 (S 2684 / D 29653; 11 g) | board order (`collector.rs`) | already used | yes | - |
| `BACON_TRIPLE_UPGRADE_MINION_ID` | Database id of the golden version | MINION | 0..134717 | shop, combat; TURN 1-30 | 24834 (S 2045 / D 22789; 11 g) | no | none | yes | - |
| `EXHAUSTED` | Can't act this turn | MINION, SPELL, HERO_POWER | 0..1 | shop, combat; TURN 1-30 | 15445 (S 1217 / D 14228; 11 g) | no | none | n/a (no player meaning) | - |
| `NUM_TURNS_IN_PLAY` | Turns on board | MINION, GAME_MODE_BUTTON, BATTLEGROUND_TRINKET | 0..31 | hero select, shop, combat; TURN 1-30 | 11388 (S 1403 / D 9985; 11 g) | no | recap: how long a minion stayed | yes (own) | low |
| `CARDRACE` | Tribe of a minion | MINION | ABERRATION, PIRATE, DRAGON, UNDEAD, DEMON, MURLOC… | shop, combat; TURN 1-30 | 9402 (S 785 / D 8617; 11 g) | tribes seen in the tavern (`collector.rs`) | already used | yes (on screen) | - |
| `CANT_ATTACK` | Can't attack (shop minions) | MINION, BATTLEGROUND_SPELL, SPELL | 0..1 | shop, combat; TURN 1-30 | 5428 (S 656 / D 4772; 11 g) | no | none | n/a (no player meaning) | - |
| `BACON_EVOLUTION_CARD_ID` | Card it evolves into | MINION, SPELL, BATTLEGROUND_TRINKET | 0..134496 | hero select, shop, combat; TURN 1-30 | 5121 (S 366 / D 4755; 11 g) | no | none | yes | - |
| `DEATHRATTLE` | Keyword | MINION, SPELL | 0..1 | shop, combat; TURN 1-30 | 5074 (S 282 / D 4792; 11 g) | no | none | yes (on screen) | low |
| `BATTLECRY` | Keyword | MINION | 0..1 | shop, combat; TURN 1-30 | 4012 (S 336 / D 3676; 11 g) | no | none | yes (on screen) | low |
| `DIVINE_SHIELD` | Keyword (value can be >1) | MINION, SPELL, BATTLEGROUND_SPELL | 0..3 | shop, combat; TURN 1-30 | 3449 (S 231 / D 3218; 11 g) | no | boards with keywords (overlay, recap) | yes (on screen) | low |
| `NUM_TURNS_IN_HAND` | Turns in hand | MINION, SPELL, BATTLEGROUND_SPELL | 0..11 | hero select, shop, combat, end; TURN 1-30 | 3362 (S 288 / D 3074; 11 g) | no | none | yes (own) | - |
| `BACON_SUBSET_DRAGON` | Card belongs to the Dragon pool | MINION, BATTLEGROUND_TRINKET, BATTLEGROUND_SPELL | 0..1 | shop, combat; TURN 2-28 | 3284 (S 245 / D 3039; 10 g) | no | check lobby-tribe research (session 039) | yes | low |
| `BACON_SUBSET_PIRATE` | Card belongs to the Pirate pool | MINION, SPELL, BATTLEGROUND_TRINKET | 0..1 | hero select, shop, combat; TURN 1-30 | 3174 (S 424 / D 2750; 9 g) | no | check lobby-tribe research (session 039) | yes | low |
| `BACON_SUBSET_ABERRATION` | Card belongs to the Aberration pool | MINION, BATTLEGROUND_SPELL, BATTLEGROUND_TRINKET | 0..1 | hero select, shop, combat; TURN 1-30 | 3153 (S 120 / D 3033; 7 g) | no | check lobby-tribe research (session 039) | yes | low |
| `ELITE` | Legendary-style frame | MINION, HERO | 0..1 | hero select, shop, combat; TURN 1-30 | 3070 (S 134 / D 2936; 11 g) | no | none | yes | - |
| `BACON_TRIPLED_BASE_MINION_ID` | Base minion of a golden (1st copy) | MINION | 0..134693 | shop, combat; TURN 5-30 | 2659 (S 107 / D 2552; 11 g) | no | recap: what was tripled | yes (on screen) | low |
| `BACON_SUBSET_DEMON` | Card belongs to the Demon pool | MINION, BATTLEGROUND_TRINKET, SPELL | 0..1 | hero select, shop, combat; TURN 1-28 | 2634 (S 22 / D 2612; 11 g) | no | check lobby-tribe research (session 039) | yes | low |
| `BACON_SUBSET_UNDEAD` | Card belongs to the Undead pool | MINION, SPELL, BATTLEGROUND_TRINKET | 0..1 | shop, combat; TURN 1-28 | 2289 (S 4 / D 2285; 7 g) | no | check lobby-tribe research (session 039) | yes | low |
| `START_OF_COMBAT` | Keyword | MINION, SPELL, HERO_POWER | 0..1 | hero select, shop, combat; TURN 1-30 | 2250 (S 152 / D 2098; 11 g) | no | none | yes (on screen) | low |
| `REBORN` | Keyword | MINION, SPELL, BATTLEGROUND_SPELL | 0..1 | shop, combat; TURN 1-30 | 2235 (S 52 / D 2183; 11 g) | no | none | yes (on screen) | low |
| `BACON_RALLY` | Keyword (Rally) | MINION | 0..1 | shop, combat; TURN 1-30 | 2123 (S 271 / D 1852; 11 g) | no | none | yes (on screen) | low |
| `BACON_SUBSET_ELEMENTALS` | Card belongs to the Elemental pool | MINION, SPELL, BATTLEGROUND_SPELL | 0..1 | hero select, shop, combat; TURN 1-30 | 1995 (S 101 / D 1894; 8 g) | no | check lobby-tribe research (session 039) | yes | low |
| `BACON_TRIPLED_BASE_MINION_ID2` | Base minion of a golden (2nd copy) | MINION | 0..134693 | shop, combat; TURN 5-30 | 1960 (S 75 / D 1885; 11 g) | no | recap: what was tripled | yes (on screen) | low |
| `AURA` | Card has an aura | MINION, BATTLEGROUND_TRINKET, HERO_POWER | 0..1 | hero select, shop, combat; TURN 2-30 | 1877 (S 182 / D 1695; 11 g) | no | none | yes | - |
| `TAUNT` | Keyword | MINION | 0..1 | shop, combat; TURN 1-30 | 1873 (S 159 / D 1714; 11 g) | no | boards with keywords (overlay, recap) | yes (on screen) | low |
| `BACON_SUBSET_MURLOC` | Card belongs to the Murloc pool | MINION, BATTLEGROUND_TRINKET, BATTLEGROUND_SPELL | 0..1 | shop, combat; TURN 1-30 | 1864 (S 396 / D 1468; 10 g) | no | check lobby-tribe research (session 039) | yes | low |
| `END_OF_TURN_TRIGGER` | Keyword (end of turn) | MINION, BATTLEGROUND_TRINKET | 0..1 | shop, combat; TURN 1-30 | 1853 (S 166 / D 1687; 11 g) | no | none | yes (on screen) | low |
| `BACON_TRIPLED_BASE_MINION_ID3` | Base minion of a golden (3rd copy) | MINION | 0..134693 | shop, combat; TURN 5-30 | 1834 (S 75 / D 1759; 11 g) | no | recap: what was tripled | yes (on screen) | low |
| `BACON_SUBSET_QUILLBOAR` | Card belongs to the Quilboar pool | MINION, BATTLEGROUND_TRINKET, SPELL | 0..1 | shop, combat; TURN 1-30 | 1625 (S 348 / D 1277; 7 g) | no | check lobby-tribe research (session 039) | yes | low |
| `BACON_SUBSET_MECH` | Card belongs to the Mech pool | MINION, HERO, BATTLEGROUND_TRINKET | 0..1 | hero select, shop, combat; TURN 1-30 | 1616 (S 118 / D 1498; 10 g) | no | check lobby-tribe research (session 039) | yes | low |
| `JUST_PLAYED` | Played this turn | MINION, SPELL, BATTLEGROUND_TRINKET | 0..1 | shop, combat; TURN 1-30 | 1534 (S 183 / D 1351; 11 g) | no | none | yes (own) | - |
| `BACON_SUBSET_BEAST` | Card belongs to the Beast pool | MINION, BATTLEGROUND_TRINKET, BATTLEGROUND_SPELL | 0..1 | shop, combat; TURN 1-30 | 1050 (S 170 / D 880; 7 g) | no | check lobby-tribe research (session 039) | yes | low |
| `BACON_GUIDE_RELATED_CARD` | Related card in the in-game guide | MINION | 0..122285 | shop, combat; TURN 1-28 | 959 (S 52 / D 907; 11 g) | no | none | yes | - |
| `CHOOSE_ONE` | Keyword | MINION, BATTLEGROUND_SPELL, SPELL | 0..1 | shop, combat; TURN 3-30 | 854 (S 100 / D 754; 11 g) | no | none | yes | - |
| `MODULAR` | Keyword (Magnetic) | MINION, ENCHANTMENT | 0..1 | shop, combat; TURN 1-28 | 778 (S 74 / D 704; 7 g) | no | none | yes (on screen) | low |
| `BACON_OVERRIDE_BG_COST` | Buy cost override (not 3) | MINION, BATTLEGROUND_SPELL, SPELL | 0..2 | shop, combat; TURN 1-28 | 746 (S 0 / D 746; 3 g) | no | shop record: buy cost | yes (own) | low |
| `SCORE_VALUE_1` | Counter shown on a card | MINION, HERO_POWER, BATTLEGROUND_TRINKET | 0..99 | hero select, shop, combat; TURN 2-30 | 742 (S 27 / D 715; 11 g) | no | none | yes (on screen) | - |
| `WINDFURY` | Keyword | MINION | 0..1 | shop, combat; TURN 1-30 | 712 (S 96 / D 616; 11 g) | no | none | yes (on screen) | low |
| `WAS_DISCOVER_OPTION` | Came from a discover | MINION, BATTLEGROUND_SPELL, SPELL | 0..1 | shop, combat; TURN 1-30 | 666 (S 34 / D 632; 11 g) | no | recap: discovered cards | yes (own) | low |
| `CAN_TARGET_CARDS_IN_HAND` | Targeting rule | MINION, HERO_POWER | 0..1 | shop, combat; TURN 3-29 | 646 (S 22 / D 624; 8 g) | no | none | n/a (no player meaning) | - |
| `SCORE_VALUE_2` | Counter shown on a card | MINION, BATTLEGROUND_TRINKET, HERO_POWER | 0..4 | combat; TURN 2-28 | 635 (S 21 / D 614; 10 g) | no | none | yes (on screen) | - |
| `STEALTH` | Keyword | MINION, SPELL, BATTLEGROUND_SPELL | 0..1 | shop, combat; TURN 2-30 | 531 (S 36 / D 495; 11 g) | no | none | yes (on screen) | low |
| `AVENGE` | Keyword | MINION, BATTLEGROUND_TRINKET | 0..1 | shop, combat; TURN 2-28 | 527 (S 10 / D 517; 6 g) | no | none | yes (on screen) | low |
| `VENOMOUS` | Keyword | MINION | 0..1 | shop, combat; TURN 4-30 | 397 (S 32 / D 365; 11 g) | no | none | yes (on screen) | low |
| `FRENZY` | Keyword | MINION, SPELL, BATTLEGROUND_SPELL | 0..0 | shop, combat; TURN 2-30 | 345 (S 20 / D 325; 11 g) | no | none | yes (on screen) | - |
| `BACON_RALLY_CANT_BE_ARTIFICIALLY_TRIGGERED` | Rally rule flag | MINION | 0..1 | shop, combat; TURN 2-30 | 308 (S 28 / D 280; 11 g) | no | none | yes | - |
| `ONLY_GOLD_IN_GUIDE` | Guide flag | MINION | 0..1 | shop, combat; TURN 1-30 | 306 (S 8 / D 298; 6 g) | no | none | n/a (no player meaning) | - |
| `BACON_CHROMADRAKE` | Minion family flag | MINION | 0..1 | combat; TURN 8-28 | 255 (S 0 / D 255; 3 g) | no | none | yes | - |
| `DISCOVER` | Card discovers | SPELL, BATTLEGROUND_SPELL, HERO_POWER | 0..1 | hero select, shop, combat; TURN 1-29 | 227 (S 37 / D 190; 11 g) | no | none | yes | - |
| `BACON_PAIR_CANDIDATE` | Minion would make a pair (UI highlight) | MINION, HERO | 0..1 | hero select, shop, combat; TURN 1-29 | 218 (S 22 / D 196; 11 g) | no | none | local side only | - |
| `TO_BE_DESTROYED` | Marked to die | MINION, HERO | 0..1 | shop, combat, end; TURN 4-30 | 204 (S 10 / D 194; 10 g) | no | none | yes | - |
| `BACON_SUBSET_NAGA` | Card belongs to the Naga pool | MINION | 0..1 | shop, combat; TURN 1-28 | 138 (S 4 / D 134; 8 g) | no | check lobby-tribe research (session 039) | yes | low |
| `MINION_TYPE_MASK` | Tribe mask (all-tribe minions) | MINION | 0..1 | shop, combat; TURN 15-28 | 101 (S 0 / D 101; 5 g) | no | none | yes | - |
| `PROTOSS` | StarCraft hero minion type | MINION | 0..1 | combat; TURN 2-26 | 90 (S 0 / D 90; 2 g) | no | none | yes | - |
| `ADDITIONAL_PLAY_REQS_1` | Play requirement | MINION | 0..132153 | shop, combat; TURN 17-28 | 90 (S 0 / D 90; 2 g) | no | none | n/a (no player meaning) | - |
| `TIMES_BEEN_TRANSFORMED` | Times transformed | MINION, BATTLEGROUND_TRINKET, HERO | 1..2 | hero select, shop, combat; TURN 9-30 | 79 (S 9 / D 70; 11 g) | no | none | yes | - |
| `WHELP` | Whelp subtype | MINION | 0..1 | shop, combat; TURN 9-25 | 70 (S 8 / D 62; 5 g) | no | none | yes | - |
| `BACON_TRIPLE_CANDIDATE` | Minion would complete a triple (UI highlight) | MINION, HERO | 0..1 | hero select, shop, combat; TURN 9-29 | 64 (S 8 / D 56; 11 g) | no | none | local side only | - |
| `STARSHIP` | StarCraft starship flag | MINION, BATTLEGROUND_TRINKET, HERO | 0..0 | hero select, shop, combat; TURN 11-30 | 64 (S 7 / D 57; 11 g) | no | none | yes | - |
| `MAGNETIC_TO_RACE` | Magnetic to a tribe | MINION | 0..11 | shop, combat; TURN 11-26 | 64 (S 0 / D 64; 5 g) | no | none | yes | - |
| `DYNAMIC_KEYWORD1` | Variable keyword | MINION | 0..3423 | shop, combat; TURN 14-28 | 51 (S 10 / D 41; 5 g) | no | none | yes | - |
| `MODULAR_ENTITY_PART_1` | Part 1 of a built minion | MINION, ENCHANTMENT | 0..103579 | shop, combat; TURN 11-28 | 47 (S 0 / D 47; 1 g) | no | none | yes (own) | - |
| `MODULAR_ENTITY_PART_2` | Part 2 of a built minion | MINION, ENCHANTMENT | 0..126947 | shop, combat; TURN 11-28 | 47 (S 0 / D 47; 1 g) | no | none | yes (own) | - |
| `EXTRA_BATTLECRIES_ADDITIONAL` | Minion makes battlecries trigger extra times | MINION | 0..2 | combat; TURN 18-24 | 40 (S 0 / D 40; 2 g) | no | none | yes (on screen) | - |
| `OVERRIDECARDNAME` | Built minion's name override | MINION | 0..97817 | shop, combat; TURN 11-28 | 39 (S 0 / D 39; 1 g) | no | none | yes (own) | - |
| `OVERRIDECARDTEXTBUILDER` | Built minion's text override | MINION | 0..14 | shop, combat; TURN 11-28 | 39 (S 0 / D 39; 1 g) | no | none | yes (own) | - |
| `ZOMBEAST` | Custom-built minion (Zombeast) | MINION | 0..1 | shop, combat; TURN 11-28 | 39 (S 0 / D 39; 1 g) | no | none | yes (own) | - |
| `BACON_PUTRICIDES_CREATION_TOOLTIP` | Built minion tooltip | MINION | 0..1 | shop, combat; TURN 11-28 | 39 (S 0 / D 39; 1 g) | no | none | yes (own) | - |
| `GHOSTLY` | Ghostly minion flag | MINION | 0..0 | shop, combat; TURN 9-30 | 38 (S 2 / D 36; 9 g) | no | none | yes | - |
| `TERRAN` | StarCraft hero minion type | MINION | 0..1 | combat; TURN 2-26 | 33 (S 0 / D 33; 1 g) | no | none | yes | - |
| `BACON_EVOLUTION_CARD_TEXT_ADDITIONAL_ATK` | Text bonus shown on card | MINION | 0..44 | shop, combat; TURN 17-27 | 33 (S 0 / D 33; 3 g) | no | none | yes | - |
| `BACON_EVOLUTION_CARD_TEXT_ADDITIONAL_HEALTH` | Text bonus shown on card | MINION | 0..44 | shop, combat; TURN 17-27 | 33 (S 0 / D 33; 3 g) | no | none | yes | - |
| `BACON_SELL_VALUE` | Gold when sold (if not 1) | MINION | 0..5 | shop, combat; TURN 13-23 | 29 (S 5 / D 24; 7 g) | no | shop record: sell gold | yes (own) | low |
| `ATTACKABLE_BY_RUSH` | Engine flag | MINION | 0..0 | shop, combat; TURN 11-30 | 27 (S 1 / D 26; 7 g) | no | none | n/a (no player meaning) | - |
| `SCORE_LABELID_1` | Label of a counter | HERO_POWER | 0..0 | combat; TURN 2-24 | 18 (S 1 / D 17; 5 g) | no | none | n/a (no player meaning) | - |
| `SCORE_LABELID_2` | Label of a counter | HERO_POWER | 0..0 | combat; TURN 2-24 | 18 (S 1 / D 17; 5 g) | no | none | n/a (no player meaning) | - |
| `SCORE_LABELID_3` | Label of a counter | HERO_POWER | 0..0 | combat; TURN 2-24 | 18 (S 1 / D 17; 5 g) | no | none | n/a (no player meaning) | - |
| `SCORE_VALUE_3` | Counter shown on a card | HERO_POWER | 0..0 | combat; TURN 2-24 | 18 (S 1 / D 17; 5 g) | no | none | yes (on screen) | - |
| `IS_A_SHOP_CHOICE_CARD` | Card offered as a shop choice | MINION, BATTLEGROUND_SPELL | 0..1 | shop, combat; TURN 15-16 | 18 (S 0 / D 18; 1 g) | no | none | yes (own) | - |
| `REAL_TIME_TRANSFORM` | Engine flag | MINION | 0..0 | shop; TURN 9-29 | 15 (S 2 / D 13; 7 g) | no | none | n/a (no player meaning) | - |
| `IMMOLATING` | Immolate effect | MINION | 0..1 | shop, combat; TURN 23-26 | 5 (S 0 / D 5; 2 g) | no | none | yes | - |
| `RACE_ALSO_UPDATE_COUNT` | Engine flag | MINION | 0..1 | combat; TURN 16-22 | 4 (S 0 / D 4; 1 g) | no | none | n/a (no player meaning) | - |
| `BACON_AWAITING_HAND_PROTECTED_SPAWN` | Engine flag | MINION | 0..1 | shop; TURN 21-25 | 4 (S 0 / D 4; 1 g) | no | none | n/a (no player meaning) | - |
| `UNTOUCHABLE` | Can't be targeted (dormant) | MINION | 0..0 | shop; TURN 11-19 | 3 (S 1 / D 2; 3 g) | no | none | yes | - |
| `DORMANT_AWAKENED_THIS_TURN` | Woke this turn | MINION | 0..0 | shop; TURN 15 | 1 (S 0 / D 1; 1 g) | no | none | yes | - |

#### Shop and Bob (38 tags)

| Tag | Means | On | Values | When | Count (Solo/Duos; games) | Used today | Possible use | Allowed (D-004) | Effort |
|---|---|---|---|---|---|---|---|---|---|
| `IS_BACON_POOL_MINION` | Minion is from the shared pool | MINION | 0..1 | shop, combat; TURN 1-30 | 23018 (S 2008 / D 21010; 11 g) | shop offers, tribes seen (`collector.rs`, `shop.rs`) | already used | yes | - |
| `COST` | Gold cost (buttons, spells, buy) | MOVE_MINION_HOVER_TARGET, SPELL, BATTLEGROUND_SPELL | 0..13 | hero select, shop, combat, end; TURN 1-30 | 12606 (S 1330 / D 11276; 11 g) | no | shop record: cost of each action | yes (own) | low |
| `BACON_ACTION_CARD` | Shop action entity (buy, sell, roll…) | MOVE_MINION_HOVER_TARGET, GAME_MODE_BUTTON | 0..1 | shop, combat; TURN 1-30 | 8181 (S 1095 / D 7086; 11 g) | no | none | yes (own) | - |
| `MOVE_MINION_HOVER_TARGET_SLOT` | Drag target slot | MOVE_MINION_HOVER_TARGET | 0..2 | shop, combat; TURN 1-30 | 7069 (S 909 / D 6160; 11 g) | no | none | yes (own) | - |
| `TAG_LAST_KNOWN_COST_IN_HAND` | Cost in hand | SPELL, BATTLEGROUND_SPELL, BATTLEGROUND_TRINKET | 0..13 | hero select, shop, combat, end; TURN 1-30 | 4153 (S 380 / D 3773; 11 g) | no | none | yes (own) | - |
| `GOLDSPARKLES_HINT` | UI hint | BATTLEGROUND_SPELL, SPELL | 0..1 | shop, combat; TURN 6-30 | 2516 (S 32 / D 2484; 10 g) | no | none | n/a (no player meaning) | - |
| `HAS_DRAG_TO_BUY` | Shop card can be dragged to buy | MINION, BATTLEGROUND_SPELL | 0..1 | shop; TURN 1-29 | 2495 (S 316 / D 2179; 11 g) | no | none | yes (own) | - |
| `BACON_TAVERN_SPELL_TOOLTIP` | Tavern spell tooltip flag | BATTLEGROUND_SPELL, SPELL | 0..1 | shop, combat; TURN 6-30 | 2467 (S 0 / D 2467; 9 g) | no | none | n/a (no player meaning) | - |
| `USE_DISCOVER_VISUALS` | UI flag | MINION, SPELL, BATTLEGROUND_SPELL | 0..1 | hero select, shop, combat; TURN 1-30 | 1606 (S 187 / D 1419; 11 g) | no | none | n/a (no player meaning) | - |
| `RESOURCES` | Gold of the turn (local player) | PLAYER, ENCHANTMENT, PLAYER_BOB | 0..22 | shop, combat; TURN 1-30 | 1280 (S 75 / D 1205; 11 g) | turn gold (`shop.rs`) | already used | yes (own) | - |
| `GAME_MODE_BUTTON_SLOT` | Shop button slot | GAME_MODE_BUTTON | 0..5 | shop, combat; TURN 1-30 | 1076 (S 179 / D 897; 11 g) | no | none | yes (own) | - |
| `HIDE_COST` | Card hides its cost | HERO_POWER, SPELL, MINION | 0..1 | hero select, shop, combat, end; TURN 1-30 | 1026 (S 99 / D 927; 11 g) | no | none | n/a (no player meaning) | - |
| `FROZEN` | Frozen | MINION, SPELL, BATTLEGROUND_SPELL | 0..1 | shop, combat; TURN 1-30 | 960 (S 99 / D 861; 11 g) | frozen offers, freezes (`shop.rs`) | already used | yes (own) | - |
| `NUM_RESOURCES_SPENT_THIS_GAME` | Gold spent so far (local player) | PLAYER | 1..182 | shop; TURN 1-29 | 645 (S 80 / D 565; 11 g) | gold spent (`shop.rs`) | already used | yes (own) | - |
| `BIG_CARD_AS_TOOLTIP_USE_TOOKTIP_SYSTEM` | UI flag on the roll button | GAME_MODE_BUTTON | 0..1 | shop, combat; TURN 1-30 | 373 (S 57 / D 316; 11 g) | no | none | n/a (no player meaning) | - |
| `BACON_FREE_REFRESH_COUNT` | Free rolls available | GAME_MODE_BUTTON, ENCHANTMENT | 0..9 | shop, combat; TURN 8-23 | 216 (S 43 / D 173; 10 g) | no | shop record: why a roll was free | yes (own) | low |
| `LITERALLY_UNPLAYABLE` | Card can't be played | MINION, SPELL, HERO_POWER | 0..1 | shop, combat; TURN 2-28 | 203 (S 19 / D 184; 10 g) | no | none | n/a (no player meaning) | - |
| `CANT_DISCARD` | Rule flag | MINION, SPELL | 0..1 | shop, combat; TURN 2-28 | 196 (S 15 / D 181; 10 g) | no | none | n/a (no player meaning) | - |
| `UNPLAYABLE_VISUALS` | UI | MINION, SPELL | 0..1 | shop, combat; TURN 2-28 | 188 (S 14 / D 174; 10 g) | no | none | n/a (no player meaning) | - |
| `COIN_CARD` | Coin card | SPELL, BATTLEGROUND_SPELL | 0..1 | shop, combat; TURN 1-24 | 182 (S 8 / D 174; 9 g) | no | none | yes (own) | - |
| `POWERED_UP` | Card glows (condition met) | SPELL, MINION, BATTLEGROUND_SPELL | 0..1 | shop, combat; TURN 9-28 | 168 (S 2 / D 166; 9 g) | no | none | n/a (no player meaning) | - |
| `BACON_SPELLCRAFT_ID` | Spellcraft spell of a minion | MINION, BATTLEGROUND_TRINKET | 0..133396 | shop, combat; TURN 11-29 | 114 (S 15 / D 99; 8 g) | no | none | yes | - |
| `LOCK_VISUAL` | UI lock | HERO_POWER, GAME_MODE_BUTTON | 0..1 | hero select, shop, combat, end; TURN 1-22 | 80 (S 12 / D 68; 11 g) | no | none | n/a (no player meaning) | - |
| `COPIED_BY_KHADGAR` | Copy flag | SPELL, MINION, BATTLEGROUND_TRINKET | 0..0 | shop, combat; TURN 4-29 | 80 (S 9 / D 71; 11 g) | no | none | yes | - |
| `SPELLCRAFT_HINT` | Spellcraft hint | SPELL | 0..1 | shop; TURN 11-29 | 56 (S 0 / D 56; 4 g) | no | none | n/a (no player meaning) | - |
| `CARD_ALTERNATE_COST` | Alternative cost (health, etc.) | MOVE_MINION_HOVER_TARGET, GAME_MODE_BUTTON, MINION | 0..5 | shop, combat; TURN 7-29 | 54 (S 4 / D 50; 9 g) | no | none | yes (own) | - |
| `BACON_COSTS_HEALTH_TO_BUY` | Card costs health to buy | BATTLEGROUND_SPELL, SPELL | 0..1 | shop, combat; TURN 7-29 | 53 (S 6 / D 47; 9 g) | no | shop record | yes (own) | low |
| `BACON_FODDERS_IN_REFRESH` | Extra minions in a roll | GAME_MODE_BUTTON | 0..3 | shop, combat; TURN 5-23 | 30 (S 0 / D 30; 2 g) | no | none | yes (own) | - |
| `HIDE_STATS` | Bob's hero hides stats | HERO | 1..1 | hero select, combat; TURN 2 | 22 (S 4 / D 18; 11 g) | no | none | n/a (no player meaning) | - |
| `HIDE_HEALTH` | Bob's hero hides health | HERO | 1..1 | hero select, combat; TURN 2 | 22 (S 4 / D 18; 11 g) | no | none | n/a (no player meaning) | - |
| `EVIL_GLOW` | Card glow | SPELL, MINION | 0..1 | shop, combat; TURN 11-22 | 16 (S 0 / D 16; 2 g) | no | none | n/a (no player meaning) | - |
| `BACON_OVERRIDE_COST_COLOR` | UI | BATTLEGROUND_TRINKET | 0..1 | shop; TURN 11-17 | 14 (S 2 / D 12; 6 g) | no | none | n/a (no player meaning) | - |
| `LOCK_VISUAL_STATE` | UI lock | GAME_MODE_BUTTON, HERO_POWER | 1..1 | shop; TURN 3-13 | 13 (S 3 / D 10; 11 g) | no | none | n/a (no player meaning) | - |
| `SHOW_DISCOVER_FROM_DECK` | UI flag | ENCHANTMENT | 1..1 | hero select | 11 (S 2 / D 9; 11 g) | no | none | n/a (no player meaning) | - |
| `GAME_MODE_BUTTON_TYPE` | Shop button kind | GAME_MODE_BUTTON | 2..2 | shop; TURN 1 | 11 (S 2 / D 9; 11 g) | no | none | yes (own) | - |
| `GAME_MODE_BUTTON_IS_EXHAUSTABLE` | Button can run out | GAME_MODE_BUTTON | 1..1 | shop; TURN 1 | 11 (S 2 / D 9; 11 g) | no | none | yes (own) | - |
| `CANT_READY` | Button not ready | GAME_MODE_BUTTON | 1..1 | shop; TURN 15-19 | 9 (S 2 / D 7; 9 g) | no | none | n/a (no player meaning) | - |
| `BACON_SHOW_COST_ON_DISCOVER` | UI flag | BATTLEGROUND_TRINKET | 0..1 | combat; TURN 20 | 4 (S 0 / D 4; 1 g) | no | none | n/a (no player meaning) | - |

#### Combat (28 tags)

| Tag | Means | On | Values | When | Count (Solo/Duos; games) | Used today | Possible use | Allowed (D-004) | Effort |
|---|---|---|---|---|---|---|---|---|---|
| `PREDAMAGE` | Damage about to be dealt (combat) | MINION, HERO | 0..46129 | shop, combat; TURN 2-30 | 15768 (S 904 / D 14864; 11 g) | no | combat replay | yes (on screen) | medium |
| `PROPOSED_ATTACKER` | Entity id of the next attacker in combat | GAME | 0..44862 | combat; TURN 2-30 | 3928 (S 240 / D 3688; 11 g) | no | combat replay | yes (on screen) | medium |
| `PROPOSED_DEFENDER` | Entity id of the attacked entity | GAME | 0..44923 | combat; TURN 2-30 | 3928 (S 240 / D 3688; 11 g) | no | combat replay | yes (on screen) | medium |
| `ATTACKING` | Entity is attacking | MINION, HERO | 0..1 | combat; TURN 2-30 | 3928 (S 240 / D 3688; 11 g) | no | combat replay | yes (on screen) | medium |
| `DEFENDING` | Entity is attacked | MINION, HERO | 0..1 | combat; TURN 2-30 | 3928 (S 240 / D 3688; 11 g) | no | combat replay | yes (on screen) | medium |
| `NUM_ATTACKS_THIS_TURN` | Attacks made this combat | MINION, HERO | 0..5 | shop, combat; TURN 2-30 | 3629 (S 225 / D 3404; 11 g) | no | combat replay | yes (on screen) | medium |
| `NUM_FRIENDLY_MINIONS_THAT_DIED_THIS_TURN` | Own minions dead this turn (per side) | PLAYER_BOB, PLAYER | 0..36 | shop, combat; TURN 2-30 | 2534 (S 141 / D 2393; 11 g) | no | combat recap | yes (on screen) | low |
| `NUM_MINIONS_PLAYER_KILLED_THIS_TURN` | Minions killed by a side this turn | PLAYER_BOB, PLAYER | 0..42 | shop, combat; TURN 2-30 | 2380 (S 141 / D 2239; 11 g) | no | combat recap | yes (on screen) | low |
| `NUM_MINIONS_KILLED_THIS_TURN` | Minions killed this turn (game-wide) | GAME | 0..64 | shop, combat; TURN 2-30 | 2281 (S 133 / D 2148; 11 g) | no | combat recap | yes (on screen) | low |
| `NUM_FRIENDLY_MINIONS_THAT_DIED_THIS_GAME` | Own minions dead this game (per side) | PLAYER_BOB, PLAYER | 1..214 | shop, combat; TURN 2-30 | 2159 (S 118 / D 2041; 11 g) | no | stats | yes (on screen) | low |
| `NUM_FRIENDLY_MINIONS_THAT_ATTACKED_THIS_TURN` | Own minions that attacked (per side) | PLAYER, PLAYER_BOB | 0..28 | shop, combat; TURN 2-30 | 2058 (S 129 / D 1929; 11 g) | no | combat recap | yes (on screen) | low |
| `DIVINE_SHIELD_DAMAGE` | Damage blocked by shields | MINION, SPELL, BATTLEGROUND_SPELL | 0..3 | shop, combat; TURN 2-30 | 1387 (S 84 / D 1303; 11 g) | no | none | yes (on screen) | - |
| `BACON_DIED_LAST_COMBAT` | Own minion died last combat | MINION | 0..1 | shop, combat; TURN 2-30 | 653 (S 83 / D 570; 11 g) | no | recap | yes (own) | low |
| `BACON_CURRENT_COMBAT_PLAYER_ID` | Seat fought this combat (per side) | PLAYER_BOB, PLAYER | 0..8 | shop, combat; TURN 2-30 | 648 (S 72 / D 576; 11 g) | combat opponent, ghosts (`collector.rs`) | already used | yes (on screen) | - |
| `BACON_COMBAT_PHASE_HERO` | Hero copy that fights in combat | HERO | 1..1 | combat; TURN 2-30 | 639 (S 38 / D 601; 11 g) | no | none (parser uses HERO_ENTITY) | yes | - |
| `HAS_BEEN_REBORN` | Minion came back with reborn | MINION | 0..1 | shop, combat; TURN 2-30 | 395 (S 6 / D 389; 10 g) | no | none | yes (on screen) | - |
| `START_WITH_1_HEALTH` | Reborn copy starts at 1 health | MINION | 0..1 | shop, combat; TURN 2-28 | 394 (S 6 / D 388; 10 g) | no | none | yes | - |
| `HIGHLIGHT_ATTACKING_MINION_DURING_COMBAT` | UI flag | GAME | 0..1 | hero select, combat; TURN 2-30 | 277 (S 44 / D 233; 11 g) | no | none | n/a (no player meaning) | - |
| `BOARD_VISUAL_STATE` | Board look in combat (1, 2) | GAME | 1..2 | combat; TURN 2-30 | 239 (S 36 / D 203; 11 g) | no | none | n/a (no player meaning) | - |
| `BACON_WON_LAST_COMBAT` | 1 if the local player won the last combat | PLAYER, HERO | 0..1 | combat; TURN 2-30 | 236 (S 32 / D 204; 11 g) | no | replace inferred results by a read value | yes (own) | low |
| `BACON_IN_COMBAT_PHASE` | 1 while a combat plays | GAME | 0..1 | shop, combat; TURN 2-30 | 235 (S 36 / D 199; 11 g) | no | phase marker for live state | yes | low |
| `DAMAGE_DEALT_TO_HERO_LAST_TURN` | Damage the local hero took last combat | PLAYER | 0..30 | combat; TURN 2-30 | 121 (S 16 / D 105; 11 g) | no | recap: damage per round | yes (own) | low |
| `BACON_ODD_PLAYER_OUT` | Hero/team with no opponent this round (fights a ghost) | HERO, PLAYER | 0..1 | shop, combat, end; TURN 16-27 | 46 (S 2 / D 44; 8 g) | no | mark ghost rounds | yes (on screen) | low |
| `BACON_COMBAT_DAMAGE_CAP` | Cap on hero damage per combat (2..15 by turn) | GAME | 2..15 | hero select, shop; TURN 1-15 | 26 (S 8 / D 18; 11 g) | no | overlay: max damage you can take | yes if the game shows it (check on screen) | low |
| `CANT_BE_DESTROYED` | Ghost hero can't die | HERO | 0..1 | hero select, combat; TURN 22-26 | 23 (S 2 / D 21; 11 g) | no | with ghost | yes | - |
| `BACON_IS_KEL_THUZAD` | The ghost opponent (eliminated player) in combat | HERO | 0..1 | hero select, combat; TURN 22-26 | 23 (S 2 / D 21; 11 g) | no | mark ghost fights | yes (on screen) | low |
| `EXTRA_ATTACKS_THIS_TURN` | Extra attacks | MINION | 0..0 | shop, combat; TURN 11-30 | 23 (S 0 / D 23; 5 g) | no | none | yes (on screen) | - |
| `BACON_COMBAT_DAMAGE_CAP_ENABLED` | Damage cap switched on or off (late game) | GAME | 0..1 | hero select, combat; TURN 22-26 | 15 (S 2 / D 13; 11 g) | no | with the cap | same as cap | low |

#### Trinkets (18 tags)

| Tag | Means | On | Values | When | Count (Solo/Duos; games) | Used today | Possible use | Allowed (D-004) | Effort |
|---|---|---|---|---|---|---|---|---|---|
| `BACON_TURNS_LEFT_TO_DISCOVER_TRINKET` | Turns until the trinket discover | BATTLEGROUND_TRINKET | 0..8 | hero select, shop, combat; TURN 1-30 | 1681 (S 155 / D 1526; 11 g) | no | overlay: trinket countdown | yes (own) | low |
| `BACON_IS_MAGIC_ITEM_DISCOVER` | Entity is a trinket discover | BATTLEGROUND_TRINKET, HERO_POWER | 0..1 | hero select, shop, combat; TURN 1-22 | 1530 (S 129 / D 1401; 11 g) | no | recap: own trinket choices | local side only | low |
| `BACON_IS_POTENTIAL_TRINKET` | Trinket offered in a trinket discover | BATTLEGROUND_TRINKET | 0..1 | hero select, shop, combat; TURN 1-30 | 1517 (S 125 / D 1392; 11 g) | no | recap: own trinket choices | own offers only; others' offers no | low |
| `BACON_TRINKET` | Card is a trinket | BATTLEGROUND_TRINKET | 0..1 | shop, combat; TURN 11-30 | 1307 (S 104 / D 1203; 11 g) | no | recap: own trinkets | yes (on screen) | low |
| `BACON_TRIGGER_UPBEAT` | Trinket/hero power triggers on Upbeat | BATTLEGROUND_TRINKET, HERO_POWER | 0..1 | hero select, shop, combat, end; TURN 1-30 | 372 (S 24 / D 348; 10 g) | no | none | yes | - |
| `ADDITIONAL_HERO_POWER_INDEX` | Index of an extra hero power (trinkets act as one) | BATTLEGROUND_TRINKET, MINION, HERO_POWER | 0..1 | hero select, shop, combat; TURN 2-30 | 354 (S 38 / D 316; 11 g) | no | none | yes | - |
| `BACON_HERO_FIRST_TRINKET_LEADERBOARD_SDN1` | Leaderboard data of a hero's first trinket | HERO | 0..126 | shop, combat; TURN 11-30 | 242 (S 12 / D 230; 11 g) | no | overlay hover: trinkets | yes (leaderboard tooltip) | medium |
| `BACON_HERO_SECOND_TRINKET_LEADERBOARD_SDN1` | Leaderboard data of a hero's second trinket | HERO | 0..43 | shop, combat, end; TURN 17-29 | 231 (S 11 / D 220; 10 g) | no | overlay hover: trinkets | yes (leaderboard tooltip) | medium |
| `BACON_FIRST_TRINKET_DATABASE_ID` | Database id of a hero's first trinket | HERO | 110966..134748 | shop; TURN 11-23 | 105 (S 16 / D 89; 11 g) | no | overlay hover, recap: opponents' trinkets | yes (leaderboard tooltip) | low |
| `BACON_SECOND_TRINKET_DATABASE_ID` | Database id of a hero's second trinket | HERO | 111207..134757 | shop; TURN 17 | 79 (S 16 / D 63; 10 g) | no | overlay hover, recap: opponents' trinkets | yes (leaderboard tooltip) | low |
| `BACON_HERO_FIRST_TRINKET_LEADERBOARD_ALT_TEXT` | Tooltip variant flag | HERO | 0..1 | shop, combat; TURN 11-27 | 54 (S 2 / D 52; 10 g) | no | none | yes | - |
| `BACON_HERO_SECOND_TRINKET_LEADERBOARD_SDN3` | Same, third value | HERO | 1..36 | shop; TURN 17-29 | 40 (S 0 / D 40; 3 g) | no | overlay hover: trinkets | yes (leaderboard tooltip) | medium |
| `BACON_HERO_FIRST_TRINKET_LEADERBOARD_SDN2` | Same, second value | HERO | 0..15 | shop; TURN 11-19 | 27 (S 4 / D 23; 11 g) | no | overlay hover: trinkets | yes (leaderboard tooltip) | medium |
| `BACON_HERO_SECOND_TRINKET_LEADERBOARD_SDN2` | Same, second value | HERO | 1..24 | shop; TURN 17-23 | 21 (S 4 / D 17; 8 g) | no | overlay hover: trinkets | yes (leaderboard tooltip) | medium |
| `BACON_TRINKETS_ACTIVE` | Trinkets are on in this season | GAME | 1..1 | setup | 11 (S 2 / D 9; 11 g) | no | season detection | yes | low |
| `BACON_HERO_SECOND_TRINKET_LEADERBOARD_ALT_TEXT` | Tooltip variant flag | HERO | 0..1 | shop; TURN 19-21 | 5 (S 0 / D 5; 3 g) | no | none | yes | - |
| `BACON_HERO_FIRST_TRINKET_LEADERBOARD_SDN3` | Same, third value | HERO | 0..50 | shop; TURN 11-17 | 2 (S 0 / D 2; 1 g) | no | overlay hover: trinkets | yes (leaderboard tooltip) | medium |
| `BACON_HEROPOWER_TRINKET_DATABASE_ID` | Trinket that became a hero power | HERO | 131308..131308 | shop; TURN 15 | 1 (S 1 / D 0; 1 g) | no | overlay hover, recap: opponents' trinkets | yes (leaderboard tooltip) | low |

#### Quests (13 tags)

| Tag | Means | On | Values | When | Count (Solo/Duos; games) | Used today | Possible use | Allowed (D-004) | Effort |
|---|---|---|---|---|---|---|---|---|---|
| `QUEST_PROGRESS_TOTAL` | Quest goal | SPELL, BATTLEGROUND_SPELL, BATTLEGROUND_TRINKET | 0..17 | shop, combat; TURN 1-30 | 878 (S 47 / D 831; 11 g) | no | overlay: quest progress | yes (own) | low |
| `OBJECTIVE` | Objective card (Old God, quests) | SPELL, BATTLEGROUND_SPELL | 0..1 | shop, combat; TURN 1-30 | 846 (S 47 / D 799; 11 g) | no | none | yes | - |
| `QUEST_HIDE_PROGRESS` | UI flag | SPELL, BATTLEGROUND_SPELL | 0..1 | shop, combat; TURN 1-30 | 815 (S 41 / D 774; 9 g) | no | none | n/a (no player meaning) | - |
| `QUEST_PROGRESS` | Quest progress | SPELL | 0..8 | shop, combat; TURN 2-30 | 333 (S 15 / D 318; 7 g) | no | overlay: quest progress | yes (own) | low |
| `SECRET` | Secret card | SPELL | 0..1 | combat; TURN 2-30 | 149 (S 9 / D 140; 8 g) | no | none | own only | - |
| `SECRET_LOCKED` | Secret locked | SPELL | 0..0 | combat; TURN 2-30 | 143 (S 9 / D 134; 8 g) | no | none | own only | - |
| `BACON_IS_HEROPOWER_QUESTREWARD` | Reward from a hero-power quest | SPELL, BATTLEGROUND_QUEST_REWARD | 0..1 | shop, combat; TURN 1-8 | 7 (S 0 / D 7; 1 g) | no | recap | yes (own) | low |
| `QUEST` | Card is a quest | SPELL | 0..1 | shop, combat; TURN 1-8 | 6 (S 0 / D 6; 1 g) | no | recap: quest and reward | yes (own) | low |
| `QUEST_REWARD_DATABASE_ID` | Database id of the quest's reward | SPELL | 0..110309 | shop, combat; TURN 1-8 | 4 (S 0 / D 4; 1 g) | no | recap | yes (own) | low |
| `BACON_HERO_HEROPOWER_QUEST_REWARD_COMPLETED` | Hero-power quest done | HERO | 1..1 | combat; TURN 8 | 4 (S 0 / D 4; 1 g) | no | recap | yes (on screen) | low |
| `BACON_DOUBLE_QUEST_HERO_POWER` | Hero power with two quests | HERO_POWER | 0..1 | hero select, combat; TURN 8 | 3 (S 0 / D 3; 1 g) | no | none | yes | - |
| `BACON_QUEST_COMPLETED` | Quest done (hero) | HERO | 1..1 | combat; TURN 8 | 2 (S 0 / D 2; 1 g) | no | recap | yes (on screen) | low |
| `BACON_HERO_HEROPOWER_QUEST_REWARD_DATABASE_ID` | Quest reward of a hero-power quest | HERO | 110309..110309 | shop; TURN 1 | 1 (S 0 / D 1; 1 g) | no | recap | yes (own) | low |

#### Anomalies and season mechanics (24 tags)

| Tag | Means | On | Values | When | Count (Solo/Duos; games) | Used today | Possible use | Allowed (D-004) | Effort |
|---|---|---|---|---|---|---|---|---|---|
| `INTERACTABLE_OBJECT` | Clickable minion (this season) | MINION | 0..1 | shop, combat; TURN 1-30 | 3411 (S 325 / D 3086; 11 g) | no | none | yes | - |
| `INTERACTABLE_OBJECT_PASSIVE_ANIMATION_TYPE` | UI | MINION | 0..2 | shop, combat; TURN 1-30 | 3134 (S 273 / D 2861; 11 g) | no | none | n/a (no player meaning) | - |
| `WAS_EVER_AN_INTERACTABLE_OBJECT` | Engine flag | MINION | 0..1 | shop, combat; TURN 1-30 | 2895 (S 255 / D 2640; 11 g) | no | none | n/a (no player meaning) | - |
| `INTERACTABLE_OBJECT_COST` | Cost of clicking it | MINION | 0..2 | shop, combat; TURN 1-30 | 2498 (S 251 / D 2247; 11 g) | no | none | yes | - |
| `HAS_DARK_GIFT` | Minion carries a Dark Gift | MINION | 0..1 | shop, combat; TURN 4-30 | 1724 (S 144 / D 1580; 11 g) | no | recap: Dark Gifts taken | yes (on screen) | low |
| `IS_NIGHTMARE_BONUS` | Nightmare bonus (mid-game effect) | ENCHANTMENT, SPELL | 0..1 | shop, combat; TURN 4-30 | 1214 (S 124 / D 1090; 11 g) | no | none | yes | - |
| `WAR_MODE_ACTIVE` | Mode flag on minions | MINION | 0..1 | shop, combat; TURN 1-30 | 804 (S 98 / D 706; 11 g) | no | none | yes | - |
| `USE_EVOLUITON_BIG_CARD_AS_PRIMARY` | UI flag (Old God card) | SPELL | 0..1 | shop, combat; TURN 1-30 | 803 (S 37 / D 766; 7 g) | no | none | n/a (no player meaning) | - |
| `BACON_DEITY_SIGIL` | Old God sigil card | SPELL | 0..1 | shop, combat; TURN 1-30 | 803 (S 37 / D 766; 7 g) | no | none | yes | - |
| `BACON_EVOLUTION_CARD_OVERWRITE_ATK` | Old God card shown attack | SPELL | 0..2275 | shop, combat; TURN 1-30 | 625 (S 19 / D 606; 7 g) | no | none | yes | - |
| `BACON_EVOLUTION_CARD_OVERWRITE_HEALTH` | Old God card shown health | SPELL | 0..1413 | shop, combat; TURN 1-30 | 625 (S 19 / D 606; 7 g) | no | none | yes | - |
| `BACON_OLD_GOD_ATTACK` | Old God attack (per side) | PLAYER_BOB, PLAYER | 0..2275 | shop, combat; TURN 1-30 | 526 (S 19 / D 507; 7 g) | no | recap: Old God growth | own yes; opponent check | low |
| `BACON_OLD_GOD_HEALTH` | Old God health (per side) | PLAYER_BOB, PLAYER | 0..1413 | shop, combat; TURN 1-30 | 526 (S 19 / D 507; 7 g) | no | recap: Old God growth | own yes; opponent check | low |
| `BACON_COIN_ON_ENEMY_MINIONS` | Season rule flag (coins on enemy minions) | GAME | 0..1 | shop, combat; TURN 1-30 | 246 (S 38 / D 208; 11 g) | no | none | yes | - |
| `DARK_GIFT_ENTITY` | Dark Gift attached to a minion | MINION | 0..28356 | shop, combat; TURN 7-24 | 201 (S 36 / D 165; 11 g) | no | recap: Dark Gifts taken | yes (own) | low |
| `BACON_TIMEWARPED` | Timewarped card | MINION, BATTLEGROUND_SPELL, SPELL | 0..1 | shop, combat; TURN 15-28 | 143 (S 0 / D 143; 3 g) | no | recap: Timewarped tavern buys | yes (own) | low |
| `BACON_OLD_GOD` | Minion is an Old God | MINION | 0..1 | combat; TURN 8-26 | 93 (S 0 / D 93; 6 g) | no | none | yes (on screen) | - |
| `DARKMOON_TICKET` | Darkmoon ticket card | MINION, BATTLEGROUND_SPELL, SPELL | 0..1 | shop, combat; TURN 17-27 | 92 (S 0 / D 92; 1 g) | no | none | yes | - |
| `BACON_DARK_GIFT_PRESSABLE_VFX` | Dark Gift button can be pressed | GAME_MODE_BUTTON | 0..1 | shop; TURN 5-25 | 61 (S 12 / D 49; 11 g) | no | none | yes (own) | - |
| `HAS_TIMEWARPED_TAVERN_ALT_TEXT` | UI text flag | HERO_POWER | 0..1 | hero select, shop, combat; TURN 2-27 | 46 (S 9 / D 37; 4 g) | no | none | n/a (no player meaning) | - |
| `BACON_DARK_GIFTS_ACTIVE` | Dark Gifts are on in this season | GAME | 1..1 | setup | 11 (S 2 / D 9; 11 g) | no | season detection | yes | low |
| `BACON_GLOBAL_OLD_GOD_DBID` | Database id of the game's Old God | GAME | 130610..132532 | setup | 11 (S 2 / D 9; 11 g) | no | recap: which Old God | check: is it shown before it acts? | low |
| `BACON_ALT_TAVERN_IN_PROGRESS` | An alternative tavern (Timewarped) is open | GAME | 0..1 | shop; TURN 15 | 2 (S 0 / D 2; 1 g) | no | shop record: mark alt tavern turns | yes (own) | low |
| `BACON_ALT_TAVERN_COIN` | Currency of an alternative tavern | PLAYER | 2..2 | shop; TURN 15 | 1 (S 0 / D 1; 1 g) | no | shop record | yes (own) | low |

#### Duos (12 tags)

| Tag | Means | On | Values | When | Count (Solo/Duos; games) | Used today | Possible use | Allowed (D-004) | Effort |
|---|---|---|---|---|---|---|---|---|---|
| `BACON_DUO_PASSABLE` | Card can be passed to the teammate | MINION, SPELL | 0..1 | shop, combat; TURN 1-30 | 11979 (S 4 / D 11975; 10 g) | no | none | yes (own) | - |
| `DECK_ACTION_COST` | Cost of a pass (Duos) | MINION, SPELL | 0..1 | shop, combat; TURN 1-30 | 2595 (S 0 / D 2595; 9 g) | no | none | yes (own) | - |
| `BACON_DUO_PLAYER_FIGHTS_FIRST_NEXT_COMBAT` | Which teammate fights first next combat | HERO, PLAYER | 0..1 | setup, hero select, combat, end; TURN 2-28 | 696 (S 0 / D 696; 9 g) | no | overlay: who fights first | yes (shown in Duos UI) | low |
| `BACON_DUO_PAIR_CANDIDATE_TEAMMATE` | Minion pairs with the teammate's | MINION, HERO | 0..1 | hero select, shop, combat; TURN 1-29 | 176 (S 0 / D 176; 9 g) | no | none | local side only | - |
| `BACON_PASS_TOOLTIP` | Pass tooltip flag | MINION, HERO_POWER, BATTLEGROUND_TRINKET | 0..1 | hero select, shop, combat; TURN 2-30 | 141 (S 0 / D 141; 7 g) | no | none | n/a (no player meaning) | - |
| `NEXT_OPPONENT_TEAMMATE_PLAYER_ID` | Seat of the next opponent's teammate | PLAYER | 1..8 | setup, combat, end; TURN 2-28 | 106 (S 0 / D 106; 9 g) | no | overlay: next opposing team | yes (game marks next opponents) | low |
| `IS_USING_PASS_OPTION` | Card is being passed | MINION, SPELL | 0..1 | shop; TURN 7-27 | 86 (S 0 / D 86; 9 g) | no | shop record: passes to teammate | yes (own) | low |
| `BACON_DUO_TEAM_ID` | Duos team of a hero | HERO, PLAYER | 1..4 | setup, hero select | 72 (S 0 / D 72; 9 g) | Duos teams (`lib.rs`) | already used | yes (on screen) | - |
| `BACON_DUO_TRIPLE_CANDIDATE_TEAMMATE` | Minion triples with the teammate's | MINION, HERO | 0..1 | hero select, shop, combat; TURN 5-29 | 62 (S 0 / D 62; 9 g) | no | none | local side only | - |
| `BACON_TEAMMATE_BONUS_MINION_DAMAGE_LAST_COMBAT` | Teammate bonus last combat | PLAYER, PLAYER_BOB | 0..1 | combat; TURN 2-28 | 57 (S 0 / D 57; 9 g) | no | none | yes (on screen) | - |
| `BACON_DUOS_PUNISH_LEAVERS` | Duos leaver rule flag (also in Solo) | GAME | 0..1 | setup, combat, end; TURN 20-26 | 18 (S 2 / D 16; 11 g) | no | none | yes | - |
| `BACON_DUO_TEAMMATE_PLAYER_ID` | Teammate's seat | PLAYER | 1..7 | setup | 9 (S 0 / D 9; 9 g) | teammate (`lib.rs`) | already used | yes (on screen) | - |

#### Cosmetics and other (79 tags)

| Tag | Means | On | Values | When | Count (Solo/Duos; games) | Used today | Possible use | Allowed (D-004) | Effort |
|---|---|---|---|---|---|---|---|---|---|
| `ZONE` | Zone (PLAY, HAND, SETASIDE…) | ENCHANTMENT, UNKNOWN, MINION | SETASIDE, PLAY, REMOVEDFROMGAME, GRAVEYARD, HAND, SECRET… | setup, hero select, shop, combat, end; TURN 1-30 | 138803 (S 9874 / D 128929; 11 g) | boards, shop offers, elimination (`collector.rs`, `shop.rs`, `state.rs`) | already used | yes | - |
| `CONTROLLER` | Owner's PlayerID | UNKNOWN, ENCHANTMENT, MINION | 1..16 | setup, hero select, shop, combat, end; TURN 1-30 | 65118 (S 4853 / D 60265; 11 g) | side of an entity (`collector.rs`, `shop.rs`) | already used | yes | - |
| `ENTITY_ID` | Entity id | UNKNOWN, ENCHANTMENT, MINION | 1..45426 | setup, hero select, shop, combat, end; TURN 1-30 | 59909 (S 4397 / D 55512; 11 g) | no | none | n/a (no player meaning) | - |
| `CREATOR` | Entity that created it | ENCHANTMENT, MINION, SPELL | 0..44892 | hero select, shop, combat, end; TURN 1-30 | 47091 (S 3646 / D 43445; 11 g) | no | recap: what made a card | yes | medium |
| `CREATOR_DBID` | Database id of the creator | ENCHANTMENT, MINION, SPELL | 0..134748 | hero select, shop, combat, end; TURN 1-30 | 43689 (S 3494 / D 40195; 11 g) | no | recap: what made a card | yes | medium |
| `TAG_SCRIPT_DATA_NUM_1` | Card script number (counters shown on some cards) | ENCHANTMENT, MINION, SPELL | 0..132955 | hero select, shop, combat, end; TURN 1-30 | 40642 (S 2973 / D 37669; 11 g) | no | none | only where the card shows it | - |
| `HAS_ACTIVATE_POWER` | Engine flag | MINION, MOVE_MINION_HOVER_TARGET, BATTLEGROUND_SPELL | 0..1 | hero select, shop, combat, end; TURN 1-30 | 39336 (S 3641 / D 35695; 11 g) | no | none | n/a (no player meaning) | - |
| `CARDTYPE` | Entity type | ENCHANTMENT, MINION, SPELL | ENCHANTMENT, MINION, SPELL, MOVE_MINION_HOVER_TARGET, BATTLEGROUND_TRINKET, BATTLEGROUND_SPELL… | setup, hero select, shop, combat, end; TURN 1-30 | 37966 (S 3118 / D 34848; 11 g) | hero, minion, button kinds (`collector.rs`, `shop.rs`) | already used | yes | - |
| `SUPPRESS_ALL_SUMMON_VO` | Voice flag | MINION, MOVE_MINION_HOVER_TARGET, BATTLEGROUND_TRINKET | 0..1 | setup, hero select, shop, combat, end; TURN 1-30 | 31184 (S 2823 / D 28361; 11 g) | no | none | n/a (no player meaning) | - |
| `TAG_SCRIPT_DATA_NUM_2` | Card script number (counters shown on some cards) | ENCHANTMENT, MINION, SPELL | 0..134405 | hero select, shop, combat; TURN 1-30 | 28822 (S 2073 / D 26749; 11 g) | no | none | only where the card shows it | - |
| `CLASS` | Card class | ENCHANTMENT, MINION, SPELL | NEUTRAL, WARRIOR, ROGUE, DEATHKNIGHT, PRIEST, WARLOCK… | hero select, shop, combat; TURN 1-30 | 27499 (S 2245 / D 25254; 11 g) | no | none | yes | - |
| `COPIED_FROM_ENTITY_ID` | Copy of another entity | ENCHANTMENT, MINION, HERO | 0..44774 | hero select, shop, combat; TURN 1-30 | 24653 (S 1504 / D 23149; 11 g) | elimination copy (`collector.rs`) | already used | yes | - |
| `ATTACHED` | Enchantment attached to | ENCHANTMENT | 2..44892 | hero select, shop, combat; TURN 1-30 | 16750 (S 1048 / D 15702; 11 g) | no | none | n/a (no player meaning) | - |
| `LAST_AFFECTED_BY` | Last entity that affected it | MINION, SPELL, HERO | 0..44923 | setup, hero select, shop, combat; TURN 1-30 | 14551 (S 951 / D 13600; 11 g) | no | none | n/a (no player meaning) | - |
| `TRIGGER_VISUAL` | Card shows a trigger icon | MINION, BATTLEGROUND_TRINKET, HERO_POWER | 0..1 | hero select, shop, combat; TURN 1-30 | 9442 (S 654 / D 8788; 11 g) | no | none | n/a (no player meaning) | - |
| `SPELL_SCHOOL` | Spell school | BATTLEGROUND_TRINKET, SPELL, BATTLEGROUND_SPELL | 0..12 | hero select, shop, combat; TURN 1-30 | 7643 (S 656 / D 6987; 11 g) | no | none | yes | - |
| `FACTION` | Card faction | HERO, BATTLEGROUND_TRINKET, MINION | NEUTRAL, INVALID, ALLIANCE | hero select, shop, combat; TURN 1-30 | 5838 (S 486 / D 5352; 11 g) | no | none | yes | - |
| `IS_EXTRA_TRIGGERED_POWER` | Engine flag | MINION, ENCHANTMENT, PLAYER | 0..1 | shop, combat; TURN 12-28 | 5586 (S 504 / D 5082; 10 g) | no | none | n/a (no player meaning) | - |
| `TAG_SCRIPT_DATA_NUM_3` | Card script number (counters shown on some cards) | MINION, SPELL, BATTLEGROUND_SPELL | 0..133438 | hero select, shop, combat; TURN 1-30 | 5211 (S 408 / D 4803; 11 g) | no | none | only where the card shows it | - |
| `TRANSIENT_ENTITY` | Temporary entity (combat copies) | MINION, HERO, HERO_POWER | 0..1 | shop, combat; TURN 2-30 | 4172 (S 265 / D 3907; 11 g) | no | none | n/a (no player meaning) | - |
| `TAG_SCRIPT_DATA_NUM_6` | Card script number (counters shown on some cards) | BATTLEGROUND_TRINKET, SPELL, ENCHANTMENT | 0..132532 | hero select, shop, combat; TURN 1-30 | 4170 (S 303 / D 3867; 11 g) | no | none | only where the card shows it | - |
| `ENCHANTMENT_INVISIBLE` | Hidden enchantment | ENCHANTMENT | 1..1 | shop, combat; TURN 1-30 | 3063 (S 246 / D 2817; 11 g) | no | none | n/a (no player meaning) | - |
| `TAG_SCRIPT_DATA_NUM_4` | Card script number (counters shown on some cards) | BATTLEGROUND_TRINKET, MINION, SPELL | 0..200 | shop, combat; TURN 1-30 | 2855 (S 191 / D 2664; 11 g) | no | none | only where the card shows it | - |
| `PREMIUM` | Golden art flag (cosmetic, not the golden minion; see parser notes) | MINION, ENCHANTMENT, BATTLEGROUND_TRINKET | 0..1 | hero select, shop, combat; TURN 1-30 | 2742 (S 165 / D 2577; 11 g) | no | none | yes | - |
| `PARENT_CARD` | Parent card | SPELL | 0..44786 | shop, combat; TURN 3-30 | 2230 (S 264 / D 1966; 11 g) | no | none | yes | - |
| `BACON_TRIGGER_XY` | Card triggers on a Battlegrounds condition (engine) | MINION, SPELL, HERO_POWER | 0..1 | hero select, shop, combat; TURN 1-30 | 2180 (S 109 / D 2071; 11 g) | no | none | n/a (no player meaning) | - |
| `FALLBACK_ENCHANTMENT_PORTRAIT_DBID` | Enchantment icon | ENCHANTMENT | 57110..127884 | shop, combat; TURN 3-30 | 1446 (S 6 / D 1440; 11 g) | no | none | n/a (no player meaning) | - |
| `BATTLEGROUNDS_FAVORITE_FINISHER` | Hero's finisher (cosmetic) | HERO | 0..107 | hero select, combat; TURN 2-30 | 1409 (S 99 / D 1310; 11 g) | no | none | yes | - |
| `LINKED_ENTITY` | Linked entity | HERO, MINION, SPELL | 0..44745 | hero select, shop, combat; TURN 2-30 | 1345 (S 122 / D 1223; 11 g) | no | none | n/a (no player meaning) | - |
| `USE_ALTERNATE_CARD_TEXT` | Text variant | BATTLEGROUND_TRINKET, MINION, SPELL | 0..1 | hero select, shop, combat; TURN 1-30 | 1329 (S 86 / D 1243; 11 g) | no | none | n/a (no player meaning) | - |
| `CARD_TARGET` | Target of a card | MOVE_MINION_HOVER_TARGET, SPELL, MINION | 0..42311 | shop, combat; TURN 1-29 | 1115 (S 116 / D 999; 11 g) | no | none | n/a (no player meaning) | - |
| `BACON_COMPANION_ID` | Database id of a hero's buddy | HERO | 0..123075 | hero select, combat; TURN 2-30 | 948 (S 88 / D 860; 11 g) | no | none | yes | - |
| `TAG_SCRIPT_DATA_NUM_5` | Card script number (counters shown on some cards) | MINION, BATTLEGROUND_TRINKET, SPELL | 0..7 | shop, combat; TURN 1-30 | 941 (S 55 / D 886; 11 g) | no | none | only where the card shows it | - |
| `BACON_SKIN_PARENT_ID` | Database id of the base hero of a skin | HERO | 0..107183 | hero select, combat; TURN 2-30 | 872 (S 28 / D 844; 11 g) | no | skin grouping without guessing (D-019) | yes | low |
| `BACON_SKIN` | Hero is a skin | HERO | 0..1 | hero select, combat; TURN 2-30 | 854 (S 28 / D 826; 11 g) | no | skin grouping (D-019) | yes | low |
| `RARITY` | Rarity of the hero card | HERO | FREE, LEGENDARY, EPIC, RARE | hero select, combat; TURN 2-30 | 798 (S 67 / D 731; 11 g) | no | none | yes | - |
| `PET_EVENT_ID` | Pet animation event | PET | 0..43 | hero select, shop, combat; TURN 1-24 | 786 (S 0 / D 786; 1 g) | no | none | n/a (no player meaning) | - |
| `MULTIPLE_CLASSES` | Class mask | MINION, SPELL | 0..8704 | shop, combat; TURN 1-30 | 737 (S 28 / D 709; 11 g) | no | none | yes | - |
| `TAG_SCRIPT_DATA_ENT_2` | Card script entity reference | MINION, ENCHANTMENT, SPELL | 0..1256 | shop, combat; TURN 2-30 | 526 (S 22 / D 504; 11 g) | no | none | n/a (no player meaning) | - |
| `REVEALED` | Entity was revealed (heroes at select, spells) | HERO, HERO_POWER, BATTLEGROUND_SPELL | 1..1 | hero select, shop, combat, end; TURN 1-30 | 503 (S 61 / D 442; 11 g) | no | none | n/a (no player meaning) | - |
| `TAG_SCRIPT_DATA_ENT_1` | Card script entity reference | MINION, SPELL, ENCHANTMENT | 0..17639 | shop, combat; TURN 1-30 | 502 (S 22 / D 480; 11 g) | no | none | n/a (no player meaning) | - |
| `DISPLAYED_CREATOR` | Creator shown on a card | SPELL, MINION, UNKNOWN | 224..43657 | shop, combat; TURN 5-30 | 390 (S 51 / D 339; 11 g) | no | none | yes | - |
| `CUSTOMTEXT3` | Custom text slot | MINION, SPELL, ENCHANTMENT | 0..1 | shop, combat; TURN 2-30 | 354 (S 21 / D 333; 11 g) | no | none | n/a (no player meaning) | - |
| `CUSTOM_KEYWORD_EFFECT` | Custom keyword | MINION, SPELL, BATTLEGROUND_SPELL | 0..2 | shop, combat; TURN 2-30 | 353 (S 20 / D 333; 11 g) | no | none | n/a (no player meaning) | - |
| `CUSTOMTEXT2` | Custom text slot | MINION, SPELL, BATTLEGROUND_SPELL | 0..1 | shop, combat; TURN 2-30 | 351 (S 21 / D 330; 11 g) | no | none | n/a (no player meaning) | - |
| `CUSTOMTEXT1` | Custom text slot | MINION, SPELL, BATTLEGROUND_SPELL | 0..0 | shop, combat; TURN 2-30 | 348 (S 21 / D 327; 11 g) | no | none | n/a (no player meaning) | - |
| `DEATHRATTLE_RETURN_ZONE` | Rule | MINION, SPELL, BATTLEGROUND_SPELL | 0..0 | shop, combat; TURN 2-30 | 345 (S 20 / D 325; 11 g) | no | none | n/a (no player meaning) | - |
| `ONE_SIDED_GHOSTLY` | Rule | MINION, SPELL, BATTLEGROUND_SPELL | 0..0 | shop, combat; TURN 2-30 | 345 (S 20 / D 325; 11 g) | no | none | n/a (no player meaning) | - |
| `OPPONENT_SIDE_GHOSTLY` | Rule | MINION, SPELL, BATTLEGROUND_SPELL | 0..0 | shop, combat; TURN 2-30 | 345 (S 20 / D 325; 11 g) | no | none | n/a (no player meaning) | - |
| `TITAN_ABILITY_USED_1` | Constructed default | MINION, SPELL, BATTLEGROUND_SPELL | 0..0 | shop, combat; TURN 2-30 | 345 (S 20 / D 325; 11 g) | no | none | n/a (no player meaning) | - |
| `TITAN_ABILITY_USED_2` | Constructed default | MINION, SPELL, BATTLEGROUND_SPELL | 0..0 | shop, combat; TURN 2-30 | 345 (S 20 / D 325; 11 g) | no | none | n/a (no player meaning) | - |
| `TITAN_ABILITY_USED_3` | Constructed default | MINION, SPELL, BATTLEGROUND_SPELL | 0..0 | shop, combat; TURN 2-30 | 345 (S 20 / D 325; 11 g) | no | none | n/a (no player meaning) | - |
| `PREPARE` | Rule | MINION, SPELL, BATTLEGROUND_SPELL | 0..0 | shop, combat; TURN 2-30 | 345 (S 20 / D 325; 11 g) | no | none | n/a (no player meaning) | - |
| `BACON_CHOSEN_BOARD_SKIN_ID` | Board skin shown in combat | GAME | 0..45 | combat; TURN 2-30 | 235 (S 36 / D 199; 11 g) | no | cosmetic | yes | - |
| `TAG_NOT_SET` | Placeholder | MINION, BATTLEGROUND_TRINKET, HERO | 0..0 | hero select, shop, combat; TURN 9-30 | 77 (S 8 / D 69; 11 g) | no | none | n/a (no player meaning) | - |
| `LETTUCE_CONTROLLER` | Mercenaries engine default | SPELL | 0..0 | shop, combat; TURN 4-27 | 70 (S 6 / D 64; 8 g) | no | none | n/a (no player meaning) | - |
| `LETTUCE_ABILITY_OWNER` | Mercenaries engine default | SPELL | 0..0 | shop, combat; TURN 4-27 | 70 (S 6 / D 64; 8 g) | no | none | n/a (no player meaning) | - |
| `DONT_SUPPRESS_SUMMON_VO` | Voice flag | MINION | 0..1 | combat; TURN 8-26 | 52 (S 0 / D 52; 6 g) | no | none | n/a (no player meaning) | - |
| `OVERRIDE_EMOTE_0` | Emote slot | PLAYER, PLAYER_BOB | 15..15 | setup | 22 (S 4 / D 18; 11 g) | no | none | n/a (no player meaning) | - |
| `OVERRIDE_EMOTE_1` | Emote slot | PLAYER, PLAYER_BOB | 16..16 | setup | 22 (S 4 / D 18; 11 g) | no | none | n/a (no player meaning) | - |
| `OVERRIDE_EMOTE_2` | Emote slot | PLAYER, PLAYER_BOB | 17..17 | setup | 22 (S 4 / D 18; 11 g) | no | none | n/a (no player meaning) | - |
| `OVERRIDE_EMOTE_3` | Emote slot | PLAYER, PLAYER_BOB | 18..18 | setup | 22 (S 4 / D 18; 11 g) | no | none | n/a (no player meaning) | - |
| `OVERRIDE_EMOTE_4` | Emote slot | PLAYER, PLAYER_BOB | 19..19 | setup | 22 (S 4 / D 18; 11 g) | no | none | n/a (no player meaning) | - |
| `OVERRIDE_EMOTE_5` | Emote slot | PLAYER, PLAYER_BOB | 20..20 | setup | 22 (S 4 / D 18; 11 g) | no | none | n/a (no player meaning) | - |
| `BACON_BOB_SKIN` | Bob skin hero | HERO | 1..1 | hero select, combat; TURN 2 | 20 (S 2 / D 18; 10 g) | no | cosmetic | yes | - |
| `BACON_BARTENDER_CARD_ID` | Database id of the Bob skin | GAME | 57110..127886 | setup | 11 (S 2 / D 9; 11 g) | no | cosmetic in recap | yes | low |
| `BACON_DUMMY_PLAYER` | Bob's player | PLAYER_BOB | 1..1 | setup | 11 (S 2 / D 9; 11 g) | Bob's player (`collector.rs`) | already used | yes | - |
| `CANT_BE_SILENCED` | Rule flag | ENCHANTMENT | 1..1 | shop, combat; TURN 11-27 | 11 (S 0 / D 11; 1 g) | no | none | n/a (no player meaning) | - |
| `PET_MEDIUM_XP_TRIGGER_COUNT` | Pet XP counter | PET | 1..5 | combat; TURN 2-24 | 5 (S 0 / D 5; 1 g) | no | none | n/a (no player meaning) | - |
| `HIDDEN_SCRIPT_DATA_1` | Hidden script number | MINION | 0..0 | shop, combat; TURN 15-30 | 4 (S 0 / D 4; 3 g) | no | none | no: never on screen | - |
| `HIDDEN_SCRIPT_DATA_2` | Hidden script number | MINION | 0..0 | shop, combat; TURN 15-30 | 4 (S 0 / D 4; 3 g) | no | none | no: never on screen | - |
| `HIDDEN_SCRIPT_DATA_3` | Hidden script number | MINION | 0..0 | shop, combat; TURN 15-30 | 4 (S 0 / D 4; 3 g) | no | none | no: never on screen | - |
| `HIDDEN_SCRIPT_DATA_4` | Hidden script number | MINION | 0..0 | shop, combat; TURN 15-30 | 4 (S 0 / D 4; 3 g) | no | none | no: never on screen | - |
| `HIDDEN_SCRIPT_DATA_5` | Hidden script number | MINION | 0..0 | shop, combat; TURN 15-30 | 4 (S 0 / D 4; 3 g) | no | none | no: never on screen | - |
| `HIDDEN_SCRIPT_DATA_6` | Hidden script number | MINION | 0..0 | shop, combat; TURN 15-30 | 4 (S 0 / D 4; 3 g) | no | none | no: never on screen | - |
| `VOODOO_LINK` | Engine link | ENCHANTMENT | 0..0 | combat; TURN 16-22 | 3 (S 1 / D 2; 3 g) | no | none | n/a (no player meaning) | - |
| `DORMANT_AWAKEN_CONDITION_ENCHANT` | Engine link | ENCHANTMENT | 0..0 | combat; TURN 16-22 | 3 (S 1 / D 2; 3 g) | no | none | n/a (no player meaning) | - |
| `PET_VARIANT_ID` | Pet variant | PET | 8..8 | hero select | 1 (S 0 / D 1; 1 g) | no | none | yes | - |
| `PET_ID` | Pet (cosmetic) id | PET | 3..3 | hero select | 1 (S 0 / D 1; 1 g) | no | none | yes | - |

#### Unnamed tags (404 tags)

The log prints these as numbers: the game client has no name for them in its log output. Meaning unknown; listed so nothing is left out. None is used today; none should be used before its meaning is known (D-010: unknown is not data).

| Tag | On | Values | When | Count (Solo/Duos; games) | Cards it appeared on |
|---|---|---|---|---|---|
| 10 | GAME | 70..90 | hero select | 20 (S 2 / D 18; 11 g) |  |
| 323 | ENCHANTMENT | 1..1 | shop, combat; TURN 6-30 | 738 (S 16 / D 722; 11 g) | BG25_011e2, BG36_524e, BG36_372e |
| 324 | ENCHANTMENT | 1..1 | shop, combat; TURN 6-30 | 738 (S 16 / D 722; 11 g) | BG25_011e2, BG36_524e, BG36_372e |
| 335 | MINION, BATTLEGROUND_SPELL, SPELL | 0..1 | shop, combat; TURN 5-29 | 333 (S 10 / D 323; 9 g) | BG36_301t, BG25_008, BGDUO31_208 |
| 341 | BATTLEGROUND_TRINKET | 0..0 | shop; TURN 11-17 | 23 (S 4 / D 19; 11 g) | BG35_MagicItem_154, BG30_MagicItem_888, BG30_MagicItem_416 |
| 430 | PLAYER | 0..22 | shop; TURN 1-29 | 479 (S 67 / D 412; 11 g) |  |
| 464 | PLAYER, PLAYER_BOB | 0..41 | shop, combat; TURN 2-30 | 328 (S 36 / D 292; 11 g) |  |
| 479 | MINION, HERO, BATTLEGROUND_TRINKET | 0..46129 | hero select, shop, combat; TURN 1-30 | 19237 (S 1415 / D 17822; 11 g) | BG36_345, BG33_155, BGS_034 |
| 542 | MINION | 0..1 | shop, combat; TURN 15-28 | 118 (S 0 / D 118; 6 g) | BG34_Giant_331, BG23_318 |
| 755 | PLAYER, PLAYER_BOB | 0..2 | shop, combat; TURN 16-24 | 27 (S 0 / D 27; 4 g) |  |
| 851 | MINION, BATTLEGROUND_SPELL | 0..1 | shop; TURN 1-29 | 916 (S 76 / D 840; 11 g) | BG36_345, BG25_010, BG35_883 |
| 858 | PET | 122617..122617 | hero select | 1 (S 0 / D 1; 1 g) | PET_3_4 |
| 937 | GAME | 3459..5173 | setup | 11 (S 2 / D 9; 11 g) |  |
| 1037 | ENCHANTMENT, MINION, SPELL | 0..16 | hero select, shop, combat, end; TURN 1-30 | 43648 (S 3427 / D 40221; 11 g) | TB_BaconShopBadsongE, TB_BaconShop_DragBuy, BG30_Trinket_2nd |
| 1067 | MINION, BATTLEGROUND_SPELL | 0..24 | shop; TURN 1-29 | 5212 (S 622 / D 4590; 11 g) | BG36_345, BGDUO_114, BGS_131 |
| 1068 | ENCHANTMENT, MINION, SPELL | 0..7 | hero select, shop, combat, end; TURN 1-30 | 109366 (S 8479 / D 100887; 11 g) | TB_BaconShopBadsongE, TB_BaconShop_DragBuy, BG36_301te |
| 1069 | MINION, BATTLEGROUND_TRINKET, HERO | 0..134748 | hero select, shop, combat; TURN 9-30 | 218 (S 23 / D 195; 11 g) | BG30_Trinket_1st, BG30_Trinket_2nd, BG36_520t |
| 1153 | PLAYER, PLAYER_BOB | 1..59 | shop, combat; TURN 2-30 | 201 (S 3 / D 198; 10 g) |  |
| 1173 | MINION, HERO | 0..44923 | shop, combat; TURN 2-30 | 7884 (S 452 / D 7432; 11 g) | BGS_034, BGS_119, BG_ICC_026t |
| 1196 | MINION, BATTLEGROUND_TRINKET | 0..3 | shop, combat; TURN 1-30 | 9544 (S 792 / D 8752; 11 g) | BG36_345, BGS_034, BGS_119 |
| 1200 | SPELL | 0..1 | shop, combat; TURN 1-30 | 876 (S 123 / D 753; 11 g) | TB_BaconShop_CheckTriples, TB_BaconShop_UpdateDmgCap, BGDUO31_205s |
| 1234 | ENCHANTMENT | 2..44827 | shop, combat; TURN 1-30 | 15125 (S 856 / D 14269; 11 g) | TB_BaconShopBadsongE, BG36_301te, BG_ShopBuff_Ench |
| 1254 | MINION, BATTLEGROUND_SPELL, BATTLEGROUND_TRINKET | 0..44827 | shop, combat; TURN 1-30 | 9364 (S 851 / D 8513; 11 g) | BG36_345, BG_ICC_026t, BGS_034 |
| 1270 | GAME_MODE_BUTTON | 0..1 | shop, combat; TURN 1-30 | 300 (S 57 / D 243; 9 g) | TB_BaconShopLockAll_Button |
| 1271 | MINION, SPELL, BATTLEGROUND_TRINKET | 0..0 | shop, combat; TURN 2-30 | 422 (S 29 / D 393; 11 g) | BG32_330, BG31_893, BG36_301t |
| 1292 | PLAYER_BOB, PLAYER | 0..1 | hero select, shop, combat; TURN 1-29 | 503 (S 78 / D 425; 11 g) |  |
| 1304 | MINION | 0..132153 | shop, combat; TURN 17-28 | 186 (S 0 / D 186; 2 g) | BG33_890t |
| 1323 | GAME | 1..373 | shop; TURN 1-29 | 2299 (S 272 / D 2027; 11 g) |  |
| 1355 | PLAYER | 1..1 | setup | 11 (S 2 / D 9; 11 g) |  |
| 1396 | BATTLEGROUND_QUEST_REWARD | 300..302 | shop; TURN 1 | 2 (S 0 / D 2; 1 g) | BG28_Reward_509, BG24_Reward_306 |
| 1413 | PET | 122617..122617 | hero select | 1 (S 0 / D 1; 1 g) | PET_3_4 |
| 1420 | PLAYER, PLAYER_BOB | 0..69 | shop, combat; TURN 2-30 | 910 (S 40 / D 870; 11 g) |  |
| 1449 | MINION, SPELL, BATTLEGROUND_SPELL | 0..0 | shop, combat; TURN 2-30 | 345 (S 20 / D 325; 11 g) | BG32_330, BG31_893, BG36_301t |
| 1453 | GAME | 0..1 | shop, combat; TURN 1-30 | 2644 (S 252 / D 2392; 11 g) |  |
| 1462 | PLAYER | 1..1 | setup | 10 (S 1 / D 9; 10 g) |  |
| 1466 | GAME | 0..1 | shop, combat; TURN 1-30 | 246 (S 38 / D 208; 11 g) |  |
| 1469 | PLAYER, PLAYER_BOB | 0..12 | shop, combat; TURN 3-29 | 181 (S 26 / D 155; 11 g) |  |
| 1474 | HERO | 0..15 | combat; TURN 22-26 | 4 (S 0 / D 4; 2 g) | TB_BaconShop_HERO_02_SKIN_F, BG34_HERO_000 |
| 1475 | MINION, ENCHANTMENT | 0..3 | shop, combat; TURN 5-30 | 2336 (S 95 / D 2241; 11 g) | TB_BaconUps_045, BG36_344_G, BG36_342_G |
| 1483 | PLAYER | 0..104433 | combat; TURN 2-30 | 1907 (S 117 / D 1790; 11 g) |  |
| 1484 | PLAYER | 0..11 | combat; TURN 2-30 | 79 (S 11 / D 68; 11 g) |  |
| 1485 | PLAYER |  | shop; TURN 1 | 11 (S 2 / D 9; 11 g) |  |
| 1486 | PLAYER, HERO | 0..1 | combat; TURN 2-22 | 36 (S 8 / D 28; 7 g) | BG36_HERO_105, TB_BaconShop_HERO_41_SKIN_E, BG21_HERO_000_SKIN_D |
| 1487 | ENCHANTMENT, PLAYER_BOB, PLAYER | 0..1 | shop, combat; TURN 2-30 | 587 (S 63 / D 524; 11 g) | Bacon_TagTransferPlayerE, TB_BaconShop_HERO_27_SKIN_C, BG20_HERO_242_SKIN_C |
| 1488 | GAME | 0..1 | setup, combat; TURN 2-30 | 250 (S 38 / D 212; 11 g) |  |
| 1489 | ENCHANTMENT | 96812..130796 | shop, combat; TURN 8-28 | 227 (S 10 / D 217; 6 g) | BG_DEEP_015e, BG34_170t2e, BG34_170te |
| 1500 | MINION, SPELL, BATTLEGROUND_TRINKET | 0..1 | shop, combat; TURN 1-30 | 2441 (S 227 / D 2214; 11 g) | BG_OldGod, BG32_330, BG26_963 |
| 1506 | HERO_POWER | 0..1 | hero select, shop, combat; TURN 2-30 | 117 (S 1 / D 116; 7 g) | TB_BaconShop_HP_024, BG22_HERO_002p, BG22_HERO_003p |
| 1513 | GAME_MODE_BUTTON, MOVE_MINION_HOVER_TARGET | 0..1 | shop, combat; TURN 7-24 | 22 (S 4 / D 18; 5 g) | TB_BaconShop_8p_Reroll_Button, TB_BaconShop_DragBuy_Spell |
| 1543 | GAME | 1..1 | setup | 11 (S 2 / D 9; 11 g) |  |
| 1549 | MINION | 0..2 | shop; TURN 21-23 | 6 (S 0 / D 6; 1 g) | BGS_020 |
| 1555 | MINION | 0..1 | combat; TURN 4-22 | 42 (S 0 / D 42; 2 g) | BG36_101, BG31_803, BGS_119 |
| 1557 | HERO_POWER | 0..1 | hero select, combat; TURN 6-18 | 13 (S 0 / D 13; 1 g) | TB_BaconShop_HP_069 |
| 1567 | HERO_POWER | 0..1 | hero select, shop, combat; TURN 1-30 | 134 (S 1 / D 133; 8 g) | TB_BaconShop_HP_024, BG22_HERO_002p, BG22_HERO_003p |
| 1569 | PLAYER, PLAYER_BOB | 0..7 | shop, combat; TURN 1-26 | 114 (S 7 / D 107; 11 g) |  |
| 1570 | MINION, SPELL, BATTLEGROUND_SPELL | 0..0 | shop, combat; TURN 2-30 | 345 (S 20 / D 325; 11 g) | BG32_330, BG31_893, BG36_301t |
| 1573 | PLAYER_BOB, PLAYER | 1..9 | shop, combat; TURN 6-30 | 44 (S 4 / D 40; 11 g) |  |
| 1575 | PLAYER, PLAYER_BOB | 0..1 | shop, combat; TURN 6-30 | 125 (S 20 / D 105; 11 g) |  |
| 1576 | MINION | 0..1 | shop, combat; TURN 11-24 | 9 (S 0 / D 9; 1 g) | BG26_ICC_028 |
| 1588 | ENCHANTMENT, PLAYER, PLAYER_BOB | 0..4 | shop, combat; TURN 1-30 | 281 (S 35 / D 246; 8 g) | Bacon_TagTransferPlayerE |
| 1620 | HERO_POWER | 0..1 | shop, combat; TURN 13-26 | 8 (S 0 / D 8; 2 g) | TB_BaconShop_HP_041i, TB_BaconShop_HP_041l |
| 1627 | MINION, SPELL, BATTLEGROUND_SPELL | 0..0 | shop, combat; TURN 2-30 | 345 (S 20 / D 325; 11 g) | BG32_330, BG31_893, BG36_301t |
| 1640 | HERO, PLAYER_BOB | 0..1 | shop, combat; TURN 16-30 | 70 (S 4 / D 66; 11 g) | BGDUO_HERO_100, BG36_HERO_002, BG36_HERO_105 |
| 1710 | GAME | 0..1 | combat; TURN 2-30 | 6936 (S 394 / D 6542; 11 g) |  |
| 1711 | SPELL, MINION | 0..1 | shop, combat; TURN 7-24 | 411 (S 72 / D 339; 11 g) | BG36_MidGameEffect_000t11, BG36_MidGameEffect_000t13, BG36_MidGameEffect_000t18 |
| 1715 | SPELL, MINION, HERO | 0..42189 | shop, combat; TURN 1-30 | 1994 (S 260 / D 1734; 11 g) | TB_BaconShop_CheckTriples, BG20_GEM_No_Impact, TB_BaconShop_UpdateDmgCap |
| 1724 | MINION, SPELL, BATTLEGROUND_TRINKET | 0..1 | hero select, shop, combat; TURN 1-30 | 12299 (S 1055 / D 11244; 11 g) | BG30_Trinket_2nd, BG36_345, BG30_Trinket_1st |
| 1739 | PLAYER | 0..1 | shop; TURN 1-27 | 21 (S 17 / D 4; 2 g) |  |
| 1745 | ENCHANTMENT | 1..1 | shop, combat; TURN 6-30 | 264 (S 44 / D 220; 5 g) | BG20_GEMe2 |
| 1804 | MINION | 0..6 | shop, combat; TURN 8-30 | 207 (S 12 / D 195; 4 g) | BG32_324, BG36_364, BG35_883 |
| 1853 | PLAYER, PLAYER_BOB | 2..149 | shop, combat; TURN 2-30 | 210 (S 19 / D 191; 11 g) |  |
| 1854 | PLAYER_BOB, PLAYER | 1..133 | shop, combat; TURN 2-28 | 123 (S 17 / D 106; 11 g) |  |
| 1882 | GAME | 1..30 | shop, combat; TURN 1-30 | 246 (S 38 / D 208; 11 g) |  |
| 1914 | PLAYER | 0..8 | shop; TURN 1-29 | 250 (S 35 / D 215; 11 g) |  |
| 1927 | ENCHANTMENT | 1..1 | shop, combat; TURN 6-30 | 695 (S 0 / D 695; 9 g) | BG25_011e2, BG33_155e, BG32_MagicItem_232e |
| 1999 | ENCHANTMENT | 0..1 | shop, combat; TURN 12-28 | 26 (S 1 / D 25; 8 g) | BG36_356e, BGFYM_011e, BG32_330_Ge |
| 2016 | PLAYER | 2..39 | combat; TURN 2-30 | 45 (S 7 / D 38; 11 g) |  |
| 2022 | GAME | 0..1 | combat; TURN 2-30 | 246 (S 38 / D 208; 11 g) |  |
| 2029 | SPELL, MINION, PLAYER | 0..15 | shop, combat; TURN 2-30 | 1033 (S 93 / D 940; 11 g) | BG_OldGod, BG32_330, BG26_963 |
| 2046 | MINION | 0..59202 | combat; TURN 4-22 | 33 (S 0 / D 33; 1 g) | TB_BaconShop_HP_033t_SKIN_B |
| 2110 | ENCHANTMENT | 1..1 | shop, combat; TURN 16-26 | 14 (S 0 / D 14; 5 g) | BG36_308e, BG34_888e |
| 2125 | HERO | 0..102925 | hero select, combat; TURN 2-22 | 50 (S 0 / D 50; 2 g) | TB_BaconShop_HERO_33_SKIN_E, TB_BaconShop_HERO_23_SKIN_C |
| 2156 | ENCHANTMENT | 1..1 | shop, combat; TURN 8-30 | 929 (S 1 / D 928; 10 g) | BG_ShopBuff_Ench, BGDUO_121e, BG_ShopBuff_Elemental_Ench |
| 2169 | HERO_POWER | 0..8 | hero select, shop, end; TURN 1-27 | 172 (S 17 / D 155; 11 g) | BG20_HERO_202p, BG28_HERO_400p, BG28_HERO_400p2 |
| 2181 | GAME | 0..1 | shop, combat; TURN 1-30 | 1120 (S 106 / D 1014; 11 g) |  |
| 2187 | MINION, HERO | 0..44923 | shop, combat; TURN 2-30 | 7830 (S 448 / D 7382; 11 g) | BGS_034, BGS_119, BG_ICC_026t |
| 2210 | HERO | 0..45 | hero select, combat; TURN 2-30 | 1409 (S 99 / D 1310; 11 g) | BG36_HERO_002, TB_BaconShop_HERO_18, BG22_HERO_004_SKIN_G |
| 2245 | PLAYER, ENCHANTMENT, PLAYER_BOB | 0..47 | shop, combat; TURN 1-30 | 1929 (S 150 / D 1779; 11 g) | Bacon_TagTransferPlayerE |
| 2262 | HERO | 0..5 | hero select, combat; TURN 2-28 | 378 (S 57 / D 321; 11 g) | BG36_HERO_105, TB_BaconShop_HERO_43, BG36_HERO_002 |
| 2263 | HERO | 0..30 | hero select, combat; TURN 2-30 | 1234 (S 88 / D 1146; 11 g) | TB_BaconShop_HERO_18, BG36_HERO_002, BG22_HERO_004_SKIN_G |
| 2267 | PLAYER | 1..8 | combat; TURN 2-30 | 123 (S 19 / D 104; 11 g) |  |
| 2268 | PLAYER | 1..10 | shop; TURN 1-27 | 31 (S 10 / D 21; 6 g) |  |
| 2330 | HERO_POWER | 0..126 | shop, combat; TURN 13-26 | 8 (S 0 / D 8; 2 g) | TB_BaconShop_HP_041i, TB_BaconShop_HP_041l |
| 2335 | PLAYER, PLAYER_BOB | 1..11 | combat; TURN 2-30 | 114 (S 17 / D 97; 11 g) |  |
| 2336 | PLAYER, ENCHANTMENT, PLAYER_BOB | 0..16 | shop, combat; TURN 1-30 | 1599 (S 105 / D 1494; 11 g) | Bacon_TagTransferPlayerE |
| 2358 | PLAYER, ENCHANTMENT, PLAYER_BOB | 0..59 | shop, combat; TURN 1-30 | 697 (S 37 / D 660; 11 g) | Bacon_TagTransferPlayerE |
| 2374 | GAME, MINION | 0..1 | shop, combat; TURN 2-30 | 1039 (S 80 / D 959; 11 g) | BG27_017, BG29_813, BG21_015_G |
| 2423 | SPELL | 0..1 | shop, combat; TURN 11-30 | 82 (S 0 / D 82; 4 g) | BG30_MagicItem_416t, BG35_MagicItem_733t, BGDUO_120t |
| 2433 | MINION | 0..1 | shop, combat; TURN 12-19 | 4 (S 0 / D 4; 2 g) | BG27_084 |
| 2436 | MINION | 0..1 | shop, combat; TURN 11-27 | 42 (S 0 / D 42; 3 g) | BG32_340, BG28_303 |
| 2442 | MOVE_MINION_HOVER_TARGET | 0..42317 | shop, combat; TURN 1-30 | 4464 (S 568 / D 3896; 11 g) | TB_BaconShop_DragBuy, TB_BaconShop_DragBuy_Spell |
| 2443 | BATTLEGROUND_SPELL, SPELL | 0..5 | shop, combat; TURN 11-30 | 119 (S 0 / D 119; 7 g) | BG31_243, BG31_244, BG31_242 |
| 2466 | SPELL | 0..1 | shop, combat; TURN 1-8 | 6 (S 0 / D 6; 1 g) | BG24_Quest_124, BG24_Quest_151 |
| 2467 | BATTLEGROUND_QUEST_REWARD | 75..115 | shop, combat; TURN 1-8 | 3 (S 0 / D 3; 1 g) | BG28_Reward_509, BG24_Reward_306 |
| 2471 | MINION | 0..1 | shop, combat; TURN 4-28 | 166 (S 6 / D 160; 8 g) | BG33_890t, BG26_LOOT_534, BG34_697 |
| 2474 | HERO_POWER | 0..1 | hero select, combat; TURN 6-14 | 13 (S 5 / D 8; 3 g) | BG22_HERO_201p, TB_BaconShop_HP_044 |
| 2485 | MINION, BATTLEGROUND_TRINKET, SPELL | 0..0 | hero select, shop, combat; TURN 2-30 | 1173 (S 89 / D 1084; 11 g) | BG30_Trinket_2nd, BG30_Trinket_1st, BG36_624 |
| 2496 | SPELL | 0..1 | shop; TURN 1 | 3 (S 0 / D 3; 1 g) | BG24_Quest_151 |
| 2523 | MINION | 0..1 | shop, combat; TURN 9-23 | 126 (S 8 / D 118; 7 g) | BG27_005, BG27_005_G |
| 2534 | MINION | 0..1 | shop, combat; TURN 11-26 | 68 (S 0 / D 68; 5 g) | BG_DEEP_015, BG26_361, BG_DEEP_015_G |
| 2536 | MINION, BATTLEGROUND_TRINKET | 0..1 | shop, combat; TURN 11-22 | 11 (S 0 / D 11; 2 g) | BG_LOE_077, BG32_MagicItem_416 |
| 2537 | MINION | 0..1 | shop, combat; TURN 1-30 | 327 (S 22 / D 305; 10 g) | BG32_821, BG31_330, BG28_633 |
| 2539 | MINION | 0..1 | shop, combat; TURN 5-30 | 124 (S 0 / D 124; 7 g) | BG36_764, BG36_764_G |
| 2540 | MINION | 0..1 | shop, combat; TURN 11-24 | 100 (S 0 / D 100; 6 g) | BG34_500, BG34_500_G, BG34_142 |
| 2542 | MINION | 0..1 | shop, combat; TURN 9-30 | 175 (S 12 / D 163; 7 g) | BG36_763 |
| 2553 | MINION | 0..1 | shop, combat; TURN 19-28 | 38 (S 4 / D 34; 5 g) | BG32_820 |
| 2564 | ENCHANTMENT | 1..1 | combat; TURN 20 | 2 (S 0 / D 2; 1 g) | BG31_812e |
| 2594 | ENCHANTMENT | 1..1 | combat; TURN 4-22 | 17 (S 0 / D 17; 3 g) | TB_BaconShop_HP_024e2, BG31_812e |
| 2632 | MINION | 0..1 | shop; TURN 9-29 | 30 (S 4 / D 26; 7 g) | BG36_524, BG36_524_G, BG36_369 |
| 2641 | BATTLEGROUND_QUEST_REWARD | 1..1 | shop, combat; TURN 1-8 | 3 (S 0 / D 3; 1 g) | BG28_Reward_509, BG24_Reward_306 |
| 2643 | SPELL | 0..80 | shop, combat; TURN 1-8 | 3 (S 0 / D 3; 1 g) | BG24_Quest_124 |
| 2644 | SPELL | 0..90 | shop, combat; TURN 1-8 | 3 (S 0 / D 3; 1 g) | BG24_Quest_124 |
| 2646 | SPELL | 0..90 | shop, combat; TURN 1-8 | 3 (S 0 / D 3; 1 g) | BG24_Quest_124 |
| 2652 | SPELL | 0..80 | shop, combat; TURN 1-8 | 3 (S 0 / D 3; 1 g) | BG24_Quest_124 |
| 2653 | BATTLEGROUND_QUEST_REWARD | 100..100 | shop, combat; TURN 1-8 | 2 (S 0 / D 2; 1 g) | BG28_Reward_509 |
| 2703 | HERO | 0..126 | hero select, combat; TURN 4-28 | 59 (S 7 / D 52; 4 g) | BG20_HERO_301, TB_BaconShop_HERO_08, TB_BaconShop_HERO_10 |
| 2711 | MINION | 0..1 | combat; TURN 18-30 | 92 (S 0 / D 92; 2 g) | BG36_344_G, BG33_822_G, BG36_344 |
| 2717 | PLAYER_BOB, PLAYER, ENCHANTMENT | 0..36 | shop, combat; TURN 2-30 | 2695 (S 141 / D 2554; 11 g) | Bacon_TagTransferPlayerE |
| 2718 | HERO | 1..1 | combat; TURN 8 | 1 (S 0 / D 1; 1 g) | BG24_HERO_100_SKIN_E |
| 2727 | BATTLEGROUND_TRINKET | 0..1 | shop, combat; TURN 17-26 | 38 (S 0 / D 38; 3 g) | BG30_MagicItem_995, BG35_MagicItem_732, BG36_MagicItem_600 |
| 2747 | MINION, ENCHANTMENT, HERO | 0..43686 | hero select, shop, combat; TURN 1-30 | 408 (S 56 / D 352; 11 g) | TB_BaconShop_3ofKindChecke, BG34_Giant_331, BG28_309 |
| 2753 | PLAYER_BOB, PLAYER, ENCHANTMENT | 0..134716 | shop, combat; TURN 2-30 | 2860 (S 148 / D 2712; 11 g) | Bacon_TagTransferPlayerE |
| 2755 | HERO | 92549..92549 | shop; TURN 1 | 1 (S 0 / D 1; 1 g) | BG24_HERO_100_SKIN_E |
| 2766 | HERO | 8..8 | shop; TURN 1 | 1 (S 0 / D 1; 1 g) | BG24_HERO_100_SKIN_E |
| 2787 | MINION | 0..130805 | shop, combat; TURN 1-26 | 541 (S 64 / D 477; 7 g) | BG26_146, BG34_170t3, BG34_170t2 |
| 2793 | PLAYER | 1..55 | shop; TURN 5-29 | 316 (S 26 / D 290; 11 g) |  |
| 2813 | PLAYER | 1..4 | shop; TURN 15-25 | 21 (S 1 / D 20; 8 g) |  |
| 2814 | MINION | 1..1 | shop; TURN 7 | 1 (S 0 / D 1; 1 g) | BG20_101 |
| 2843 | PLAYER_BOB, PLAYER | 0..39 | combat; TURN 2-30 | 123 (S 19 / D 104; 11 g) |  |
| 2860 | ENCHANTMENT | 1..1 | shop, combat; TURN 1-30 | 1082 (S 58 / D 1024; 11 g) | BG29_813e, Bacon_TagTransferPlayerE, BG26_159pe |
| 2861 | MINION | 0..1 | combat; TURN 12-26 | 18 (S 0 / D 18; 2 g) | BG31_HERO_802pt1_G, BGDUO31_201_G |
| 2863 | MINION | 0..1 | shop, combat; TURN 11-28 | 124 (S 0 / D 124; 3 g) | BG33_890t, BG26_LOOT_534, BG34_697 |
| 2873 | MINION | 0..1 | combat; TURN 2-30 | 965 (S 55 / D 910; 11 g) | BGS_034, BGS_119, BG32_236 |
| 2874 | MINION | 0..44923 | combat; TURN 24-30 | 8 (S 0 / D 8; 1 g) | BG36_523_G |
| 2878 | PLAYER, ENCHANTMENT, PLAYER_BOB | 0..23 | shop, combat; TURN 2-30 | 557 (S 15 / D 542; 11 g) | Bacon_TagTransferPlayerE |
| 2882 | BATTLEGROUND_TRINKET, ENCHANTMENT, MINION | 0..1 | hero select, shop, combat; TURN 1-30 | 357 (S 34 / D 323; 11 g) | BG36_MidGameEffect_000t18e, BG30_MagicItem_435, BGDUO_118 |
| 2888 | HERO_POWER | 1..1 | hero select | 1 (S 1 / D 0; 1 g) | BG21_HERO_000p |
| 2944 | MINION | 0..1 | shop, combat; TURN 17-30 | 186 (S 0 / D 186; 2 g) | BG36_523_G, BG36_352_G, BG28_633_G |
| 2949 | GAME | 0..1 | shop; TURN 7-23 | 112 (S 8 / D 104; 5 g) |  |
| 2954 | HERO | 0..10 | hero select, combat; TURN 2-30 | 95 (S 7 / D 88; 2 g) | TB_BaconShop_HERO_18, TB_BaconShop_HERO_10 |
| 2955 | GAME | 1..1 | setup | 11 (S 2 / D 9; 11 g) |  |
| 2956 | GAME | 1..1 | setup | 11 (S 2 / D 9; 11 g) |  |
| 2986 | PLAYER | 0..1 | combat; TURN 2-30 | 73 (S 0 / D 73; 9 g) |  |
| 3010 | PLAYER | 9..15 | setup | 9 (S 0 / D 9; 9 g) |  |
| 3025 | HERO | 30..60 | hero select | 88 (S 16 / D 72; 11 g) | BG36_HERO_105, TB_BaconShop_HERO_43, BG31_HERO_006 |
| 3026 | HERO | 5..18 | hero select | 85 (S 16 / D 69; 11 g) | BG36_HERO_105, TB_BaconShop_HERO_43, BG31_HERO_006 |
| 3027 | PLAYER | 91..119 | combat; TURN 2 | 9 (S 0 / D 9; 9 g) |  |
| 3032 | GAME | 1..1 | setup | 11 (S 2 / D 9; 11 g) |  |
| 3033 | GAME | 1..1 | setup | 11 (S 2 / D 9; 11 g) |  |
| 3049 | ENCHANTMENT | 1..1 | combat; TURN 12-18 | 10 (S 0 / D 10; 3 g) | BG36_506e, BG26_149e |
| 3050 | PLAYER | 1..41 | shop; TURN 5-29 | 267 (S 31 / D 236; 11 g) |  |
| 3062 | MINION | 0..18 | shop, combat; TURN 13-26 | 48 (S 1 / D 47; 4 g) | BG26_175 |
| 3068 | HERO | 0..1 | shop, combat; TURN 6-26 | 139 (S 0 / D 139; 6 g) | TB_BaconShopBob_SKIN_R4, TB_BaconShop_HERO_102_SKIN_G, TB_BaconShop_HERO_37_SKIN_D |
| 3075 | PLAYER | 0..1 | shop, combat; TURN 1-30 | 246 (S 38 / D 208; 11 g) |  |
| 3080 | ENCHANTMENT | 101280..101280 | shop, combat; TURN 17-30 | 36 (S 0 / D 36; 2 g) | BG26_175e |
| 3081 | BATTLEGROUND_SPELL, SPELL | 0..1 | shop, combat; TURN 1-30 | 3362 (S 358 / D 3004; 11 g) | BG28_810, BG36_624, BG36_301t |
| 3083 | MINION | 0..1 | shop, combat; TURN 2-30 | 1045 (S 45 / D 1000; 11 g) | BG36_362_G, BG31_815, BG34_638t |
| 3084 | GAME | 1..1 | setup | 11 (S 2 / D 9; 11 g) |  |
| 3085 | MINION, HERO, BATTLEGROUND_TRINKET | 0..80391 | hero select, shop, combat; TURN 1-30 | 23226 (S 1672 / D 21554; 11 g) | BG36_345, BG33_155, BGS_034 |
| 3087 | PLAYER, PLAYER_BOB | 0..6 | shop, combat; TURN 1-25 | 19 (S 2 / D 17; 11 g) |  |
| 3088 | PLAYER, ENCHANTMENT, PLAYER_BOB | 0..234 | shop, combat; TURN 1-30 | 1401 (S 85 / D 1316; 11 g) | Bacon_TagTransferPlayerE |
| 3089 | PLAYER_BOB, PLAYER | 0..44748 | shop, combat; TURN 6-30 | 135 (S 15 / D 120; 5 g) |  |
| 3090 | ENCHANTMENT | 3089..3245 | shop, combat; TURN 1-30 | 483 (S 29 / D 454; 11 g) | Bacon_TagTransferPlayerE, BG26_159pe |
| 3092 | MINION | 0..101308 | shop, combat; TURN 13-26 | 48 (S 1 / D 47; 4 g) | BG26_175 |
| 3096 | MINION, HERO_POWER | 0..1 | hero select, shop, combat; TURN 1-30 | 57 (S 0 / D 57; 6 g) | BGDUO_125, TB_BaconShop_HP_086, BG22_HERO_000p_Alt |
| 3110 | PLAYER, PLAYER_BOB | 1..149 | shop, combat; TURN 2-30 | 135 (S 10 / D 125; 11 g) |  |
| 3112 | PLAYER | 0..6 | shop; TURN 1-29 | 131 (S 13 / D 118; 11 g) |  |
| 3143 | PLAYER_BOB, PLAYER | 0..35 | shop, combat; TURN 2-30 | 394 (S 43 / D 351; 11 g) |  |
| 3148 | PLAYER | 10..19 | shop; TURN 1-25 | 43 (S 8 / D 35; 11 g) |  |
| 3164 | MINION, SPELL, BATTLEGROUND_SPELL | 0..10 | shop, combat; TURN 5-30 | 770 (S 107 / D 663; 9 g) | BG34_170t3, BG34_638t, BG34_170t2 |
| 3165 | GAME | 1..1 | setup | 11 (S 2 / D 9; 11 g) |  |
| 3166 | MINION, BATTLEGROUND_SPELL, HERO | 0..1 | hero select, shop, combat; TURN 1-30 | 1982 (S 0 / D 1982; 9 g) | BGDUO_114, BGDUO_108, BGDUO_700 |
| 3167 | BATTLEGROUND_TRINKET | 0..1 | combat; TURN 18 | 4 (S 4 / D 0; 1 g) | BG36_MagicItem_211 |
| 3169 | BATTLEGROUND_TRINKET, SPELL, HERO_POWER | 0..1 | setup, hero select, shop, combat; TURN 1-30 | 2244 (S 239 / D 2005; 11 g) | BG30_Trinket_2nd, BG30_Trinket_1st, TB_BaconShop_CheckTriples |
| 3170 | GAME | 1..1 | setup | 11 (S 2 / D 9; 11 g) |  |
| 3173 | HERO | 0..1 | combat; TURN 16-24 | 10 (S 0 / D 10; 5 g) | BG28_HERO_400_SKIN_B, BG31_HERO_802, TB_BaconShop_HERO_75 |
| 3177 | MINION, SPELL, BATTLEGROUND_SPELL | 0..3 | shop, combat; TURN 1-29 | 131 (S 0 / D 131; 9 g) | BG28_550, BG36_515, BGDUO_114 |
| 3179 | MINION, SPELL | 0..1 | shop, combat; TURN 2-28 | 126 (S 11 / D 115; 9 g) | BG31_HERO_802pt, BG32_330_G, BG31_HERO_802pt7 |
| 3181 | SPELL | 0..1 | shop, combat; TURN 11-30 | 64 (S 0 / D 64; 3 g) | BG30_MagicItem_416t, BGDUO_120t |
| 3184 | MINION, SPELL, BATTLEGROUND_SPELL | 0..8 | shop, combat; TURN 1-29 | 131 (S 0 / D 131; 9 g) | BG28_550, BG36_515, BGDUO_114 |
| 3186 | MINION | 0..1 | shop, combat; TURN 11-29 | 60 (S 0 / D 60; 6 g) | BGDUO_120, BG31_835 |
| 3190 | HERO | 0..100 | hero select, combat; TURN 2-30 | 501 (S 24 / D 477; 11 g) | TB_BaconShop_HERO_18, BG33_HERO_001, BG31_HERO_802 |
| 3191 | PLAYER_BOB | 0..130856 | combat; TURN 22 | 2 (S 0 / D 2; 1 g) |  |
| 3206 | HERO | 0..18 | hero select, combat; TURN 2-30 | 930 (S 102 / D 828; 11 g) | BG36_HERO_002, TB_BaconShop_HERO_18, BG33_HERO_001 |
| 3219 | PLAYER, PLAYER_BOB | 0..1 | hero select, combat; TURN 2-26 | 54 (S 0 / D 54; 3 g) |  |
| 3224 | ENCHANTMENT | 1..1 | shop, combat; TURN 2-30 | 168 (S 0 / D 168; 9 g) | TB_BaconShop_HP_040e, BG36_356e, BGFYM_011e |
| 3225 | MINION | 0..44952 | shop, combat; TURN 6-30 | 517 (S 81 / D 436; 5 g) | BG36_240, BG36_344_G, BG33_885 |
| 3226 | MINION | 0..40547 | shop, combat; TURN 6-28 | 109 (S 9 / D 100; 4 g) | BG33_883, BG36_331, BG31_326 |
| 3234 | HERO | 1..1 | hero select | 22 (S 4 / D 18; 11 g) | TB_BaconShop_HERO_PH |
| 3235 | MINION, SPELL | 0..2 | shop, combat; TURN 1-29 | 110 (S 0 / D 110; 9 g) | BG28_550, BG36_515, BG31_881 |
| 3236 | PLAYER, ENCHANTMENT, PLAYER_BOB | 0..55 | shop, combat; TURN 1-30 | 980 (S 39 / D 941; 11 g) | Bacon_TagTransferPlayerE |
| 3245 | PLAYER_BOB, PLAYER | 0..44747 | shop, combat; TURN 1-30 | 817 (S 40 / D 777; 11 g) |  |
| 3261 | ENCHANTMENT | 1..1 | shop, combat; TURN 1-26 | 789 (S 0 / D 789; 5 g) | BG20_HERO_102pe, BG32_873e, BG30_MagicItem_841e |
| 3323 | HERO | 0..18634 | hero select, combat; TURN 2-24 | 132 (S 15 / D 117; 6 g) | BGDUO_HERO_100, TB_BaconShop_HERO_34, BGDUO_HERO_104 |
| 3388 | BATTLEGROUND_TRINKET | 0..1 | shop, combat; TURN 11-30 | 610 (S 49 / D 561; 11 g) | BG30_MagicItem_435, BG36_MagicItem_370, BG30_MagicItem_996 |
| 3391 | SPELL, BATTLEGROUND_SPELL, MINION | 0..1 | shop, combat; TURN 8-30 | 1333 (S 133 / D 1200; 11 g) | BG35_911, BG28_604, BG33_813 |
| 3437 | BATTLEGROUND_SPELL | 1..4 | shop; TURN 1-29 | 85 (S 8 / D 77; 11 g) | BG28_810, BG28_827, BG28_518 |
| 3443 | BATTLEGROUND_TRINKET | 0..1 | shop, combat; TURN 11-28 | 234 (S 33 / D 201; 11 g) | BG32_MagicItem_350, BG36_MagicItem_203, BG30_MagicItem_700 |
| 3452 | GAME | 0..1 | combat; TURN 2-30 | 886 (S 76 / D 810; 11 g) |  |
| 3455 | BATTLEGROUND_QUEST_REWARD | 1..1 | combat; TURN 8 | 2 (S 0 / D 2; 1 g) | BG28_Reward_509 |
| 3475 | GAME | 1..1 | hero select | 2 (S 2 / D 0; 2 g) |  |
| 3488 | GAME | 1..1 | hero select | 11 (S 2 / D 9; 11 g) |  |
| 3491 | PLAYER, ENCHANTMENT, PLAYER_BOB | 0..28 | shop, combat; TURN 1-30 | 1264 (S 101 / D 1163; 11 g) | Bacon_TagTransferPlayerE |
| 3493 | MINION | 0..1 | shop, combat; TURN 15-30 | 316 (S 0 / D 316; 2 g) | BG35_881_G, BGDUO_114, BG36_342_G |
| 3518 | HERO, PLAYER | 0..1 | combat; TURN 4-28 | 64 (S 0 / D 64; 8 g) | BG34_HERO_000, TB_BaconShop_HERO_102_SKIN_G, TB_BaconShop_HERO_27_SKIN_C |
| 3519 | BATTLEGROUND_TRINKET | 0..1 | shop; TURN 17 | 3 (S 0 / D 3; 1 g) | BG30_MagicItem_403 |
| 3521 | MINION | 0..1 | shop, combat; TURN 9-28 | 232 (S 30 / D 202; 11 g) | BG36_346, BG28_550, BG28_707 |
| 3522 | PLAYER | 1..12 | shop; TURN 5-25 | 25 (S 0 / D 25; 5 g) |  |
| 3524 | BATTLEGROUND_TRINKET | 0..250 | shop, combat; TURN 11-30 | 213 (S 11 / D 202; 10 g) | BG30_MagicItem_435, BG36_MagicItem_370, BG30_MagicItem_888 |
| 3528 | ENCHANTMENT | 0..1 | shop, combat; TURN 1-30 | 9055 (S 589 / D 8466; 11 g) | BG36_301te, BG_ShopBuff_Ench, BG33_812e |
| 3530 | MINION | 0..1 | shop, combat; TURN 16-28 | 31 (S 0 / D 31; 3 g) | BG29_813, BG29_813_G, BG27_017 |
| 3531 | MINION | 0..1 | shop, combat; TURN 16-24 | 86 (S 0 / D 86; 4 g) | BG36_849, BG34_Giant_314, BG36_240 |
| 3533 | GAME | 0..1 | combat; TURN 2-30 | 640 (S 38 / D 602; 11 g) |  |
| 3534 | GAME | 0..1 | combat; TURN 2-30 | 246 (S 38 / D 208; 11 g) |  |
| 3536 | MINION | 0..16756 | combat; TURN 8-18 | 76 (S 12 / D 64; 5 g) | BG35_814, BG26_963, BGS_034 |
| 3547 | MINION | 0..11320 | shop, combat; TURN 15-20 | 25 (S 25 / D 0; 1 g) | BG26_810, BG33_822, BG36_342 |
| 3549 | MINION | 0..5113 | shop, combat; TURN 9-12 | 19 (S 0 / D 19; 1 g) | BG26_135, BG25_010, BG25_011 |
| 3551 | BATTLEGROUND_TRINKET | 0..1 | shop; TURN 11-17 | 6 (S 0 / D 6; 2 g) | BG36_MagicItem_220, BG36_MagicItem_373 |
| 3557 | SPELL, MINION | 0..10 | shop, combat; TURN 1-30 | 1370 (S 165 / D 1205; 11 g) | BG35_911, TB_BaconShop_Triples_01, BG28_604 |
| 3558 | PLAYER | 0..1 | shop, combat; TURN 11-24 | 13 (S 0 / D 13; 1 g) |  |
| 3566 | BATTLEGROUND_SPELL, SPELL | 0..1 | shop, combat; TURN 3-29 | 231 (S 8 / D 223; 10 g) | BG31_886, BG31_880, BG35_912 |
| 3580 | MINION | 0..1 | combat; TURN 2-28 | 39 (S 0 / D 39; 3 g) | TB_BaconShop_HP_105t |
| 3584 | MINION | 0..0 | shop; TURN 15 | 1 (S 0 / D 1; 1 g) | BG35_883 |
| 3614 | MINION, BATTLEGROUND_TRINKET, ENCHANTMENT | 0..1 | shop, combat; TURN 1-30 | 448 (S 45 / D 403; 11 g) | BG25_001, BG29_300, BG31_803 |
| 3616 | BATTLEGROUND_TRINKET, ENCHANTMENT, MINION | 0..0 | shop, combat; TURN 11-30 | 69 (S 6 / D 63; 11 g) | BG36_372e, BG34_170e, BG25_011e2 |
| 3617 | MINION, BATTLEGROUND_TRINKET, ENCHANTMENT | 0..3 | shop, combat; TURN 1-30 | 962 (S 82 / D 880; 11 g) | BGS_034, BGS_119, BG32_236 |
| 3618 | BATTLEGROUND_TRINKET, ENCHANTMENT, MINION | 0..0 | shop, combat; TURN 11-30 | 69 (S 6 / D 63; 11 g) | BG36_372e, BG34_170e, BG25_011e2 |
| 3619 | MINION, BATTLEGROUND_TRINKET, ENCHANTMENT | 0..1 | shop, combat; TURN 2-30 | 121 (S 11 / D 110; 11 g) | BG34_604, BG32_330_G, BG36_372e |
| 3620 | BATTLEGROUND_TRINKET, ENCHANTMENT, MINION | 0..0 | shop, combat; TURN 11-30 | 69 (S 6 / D 63; 11 g) | BG36_372e, BG34_170e, BG25_011e2 |
| 3621 | MINION, BATTLEGROUND_TRINKET, ENCHANTMENT | 0..1 | shop, combat; TURN 1-30 | 411 (S 13 / D 398; 11 g) | BGS_034, BG25_010t, BG36_113 |
| 3622 | MINION, BATTLEGROUND_TRINKET, ENCHANTMENT | 0..1 | shop, combat; TURN 1-30 | 207 (S 29 / D 178; 11 g) | BGS_119, BG25_016, BG36_240 |
| 3639 | BATTLEGROUND_TRINKET | 0..1 | hero select, shop, combat; TURN 1-22 | 1496 (S 124 / D 1372; 11 g) | BG30_Trinket_2nd, BG30_Trinket_1st, BG35_MagicItem_154 |
| 3652 | MINION | 0..27328 | shop, combat; TURN 23-24 | 12 (S 0 / D 12; 1 g) | BG32_821_G, BG36_360t9, BG22_403 |
| 3654 | MINION | 0..1 | combat; TURN 2-26 | 38 (S 2 / D 36; 6 g) | BG32_330_G, BG34_604, BG34_697 |
| 3657 | MINION | 0..1 | combat; TURN 4-30 | 90 (S 10 / D 80; 5 g) | TB_BaconShop_HP_033t_SKIN_B, BGS_131, BG33_318 |
| 3663 | GAME | 0..1 | shop; TURN 1-29 | 228 (S 0 / D 228; 9 g) |  |
| 3669 | MINION, ENCHANTMENT, HERO | 0..1 | hero select, shop, combat; TURN 1-30 | 15588 (S 1032 / D 14556; 11 g) | BG29_813e, Bacon_TagTransferPlayerE, BG36_345 |
| 3670 | MINION, ENCHANTMENT, PLAYER_BOB | 0..22 | shop, combat; TURN 8-28 | 424 (S 16 / D 408; 6 g) | Bacon_TagTransferPlayerE, BG36_854, BG30_125_G |
| 3673 | PLAYER, ENCHANTMENT, PLAYER_BOB | 0..24 | shop, combat; TURN 1-30 | 1556 (S 100 / D 1456; 11 g) | Bacon_TagTransferPlayerE |
| 3675 | PLAYER | 0..2 | setup, combat, end; TURN 2-24 | 55 (S 0 / D 55; 9 g) |  |
| 3676 | ENCHANTMENT | 0..12 | shop, combat; TURN 1-30 | 4995 (S 465 / D 4530; 11 g) | BG28_168e, BG33_155e, BG36_246e |
| 3685 | PLAYER, ENCHANTMENT, PLAYER_BOB | 0..67 | shop, combat; TURN 1-30 | 727 (S 39 / D 688; 11 g) | Bacon_TagTransferPlayerE |
| 3687 | PLAYER, PLAYER_BOB | 1..55 | shop, combat; TURN 2-30 | 135 (S 10 / D 125; 11 g) |  |
| 3735 | ENCHANTMENT | 1..1 | combat; TURN 4-22 | 15 (S 0 / D 15; 2 g) | TB_BaconShop_HP_024e2 |
| 3736 | SPELL | 0..2 | shop, combat; TURN 1-30 | 602 (S 28 / D 574; 9 g) | BG_OldGod, TB_Bacon_Secrets_15, BG34_889 |
| 3768 | MINION | 0..1 | shop, combat; TURN 11-26 | 165 (S 0 / D 165; 4 g) | BG36_362_G, BG36_364_G, BG36_180_G |
| 3769 | MINION | 0..1 | shop, combat; TURN 11-26 | 159 (S 0 / D 159; 4 g) | BG36_362_G, BG36_364_G |
| 3770 | MINION | 0..1 | shop, combat; TURN 5-30 | 641 (S 25 / D 616; 10 g) | BG36_362_G, BG36_345_G, BG36_114_G |
| 3771 | MINION | 0..1 | shop, combat; TURN 14-30 | 172 (S 0 / D 172; 8 g) | BG36_523_G, BG34_858_G, BG36_372_G |
| 3772 | MINION | 0..1 | combat; TURN 16-30 | 73 (S 0 / D 73; 3 g) | BG36_523_G, BG26_810_G, BG36_209_G |
| 3799 | MINION | 0..2068 | combat; TURN 24-30 | 10 (S 0 / D 10; 1 g) | BG36_523_G |
| 3801 | MINION | 0..2816 | combat; TURN 24-30 | 10 (S 0 / D 10; 1 g) | BG36_523_G |
| 3803 | PLAYER_BOB | 0..3 | combat; TURN 12-24 | 6 (S 0 / D 6; 1 g) |  |
| 3809 | PLAYER, ENCHANTMENT, PLAYER_BOB | 0..331 | shop, combat; TURN 1-30 | 1546 (S 134 / D 1412; 11 g) | Bacon_TagTransferPlayerE |
| 3819 | HERO | 17..126 | shop; TURN 11-17 | 5 (S 1 / D 4; 4 g) | BG20_HERO_242, TB_BaconShop_HERO_64_SKIN_E, BG36_HERO_002 |
| 3820 | HERO | 11..43 | shop; TURN 17 | 4 (S 0 / D 4; 4 g) | TB_BaconShop_HERO_02_SKIN_F, TB_BaconShop_HERO_27_SKIN_C, TB_BaconShop_HERO_08 |
| 3822 | BATTLEGROUND_TRINKET | 0..1 | shop, combat; TURN 12-30 | 59 (S 3 / D 56; 6 g) | BG30_MagicItem_426, BG30_MagicItem_426t, BG32_MagicItem_901 |
| 3824 | MINION | 0..25860 | combat; TURN 16-22 | 7 (S 0 / D 7; 4 g) | BG23_318 |
| 3828 | PLAYER_BOB | 0..10 | combat; TURN 22 | 2 (S 0 / D 2; 1 g) |  |
| 3830 | PLAYER_BOB | 0..8 | combat; TURN 18-24 | 4 (S 0 / D 4; 1 g) |  |
| 3834 | PLAYER_BOB, PLAYER | 0..4 | shop, combat; TURN 15-30 | 31 (S 2 / D 29; 6 g) |  |
| 3840 | MINION | 1..1 | shop; TURN 21-23 | 2 (S 0 / D 2; 2 g) | BGDUO31_211 |
| 3852 | MINION, BATTLEGROUND_SPELL, SPELL | 0..1 | shop; TURN 5-29 | 188 (S 6 / D 182; 11 g) | BG28_810, BG28_550, BGDUO_108 |
| 3857 | MINION | 0..228 | combat; TURN 20-24 | 26 (S 0 / D 26; 2 g) | BG33_885, BG28_583, BG30_123 |
| 3858 | MINION | 0..240 | combat; TURN 20-24 | 26 (S 0 / D 26; 2 g) | BG33_885, BG28_583, BG30_123 |
| 3861 | BATTLEGROUND_TRINKET | 0..18722 | shop, combat; TURN 11-26 | 42 (S 4 / D 38; 7 g) | BG30_MagicItem_700, BG30_MagicItem_707, BGDUO_MagicItem_008 |
| 3873 | PLAYER, ENCHANTMENT, PLAYER_BOB | 0..154 | shop, combat; TURN 1-30 | 1187 (S 39 / D 1148; 11 g) | Bacon_TagTransferPlayerE |
| 3874 | ENCHANTMENT, PLAYER_BOB, PLAYER | 0..4 | combat; TURN 6-20 | 40 (S 11 / D 29; 3 g) | Bacon_TagTransferPlayerE |
| 3908 | MINION, SPELL, HERO | 0..0 | hero select, shop, combat; TURN 2-30 | 433 (S 21 / D 412; 11 g) | BG36_624, BG32_330, BG31_893 |
| 3924 | ENCHANTMENT, PLAYER_BOB, PLAYER | 0..2 | combat; TURN 6-20 | 40 (S 11 / D 29; 3 g) | Bacon_TagTransferPlayerE |
| 3927 | PLAYER, ENCHANTMENT, PLAYER_BOB | 0..134496 | shop, combat; TURN 1-30 | 1103 (S 74 / D 1029; 11 g) | Bacon_TagTransferPlayerE |
| 3929 | ENCHANTMENT | 1..1 | combat; TURN 14-28 | 4 (S 0 / D 4; 2 g) | BGDUO31_208pe |
| 3962 | ENCHANTMENT, PLAYER, PLAYER_BOB | 0..29 | shop, combat; TURN 2-30 | 453 (S 22 / D 431; 11 g) | Bacon_TagTransferPlayerE |
| 3970 | MINION | 0..1 | shop, combat; TURN 17-28 | 68 (S 0 / D 68; 4 g) | BG22_403, BG22_403_G |
| 3977 | MINION | 0..23648 | shop, combat; TURN 5-24 | 19 (S 0 / D 19; 1 g) | BGS_004, TB_BaconUps_079 |
| 3980 | GAME | 1..1 | setup | 11 (S 2 / D 9; 11 g) |  |
| 4018 | PET | 1..1 | hero select | 1 (S 0 / D 1; 1 g) | PET_3_4 |
| 4020 | HERO_POWER | 0..1 | hero select, shop, combat; TURN 2-26 | 121 (S 0 / D 121; 5 g) | TB_BaconShop_HP_035, TB_BaconShop_HP_033, BG31_HERO_801p |
| 4021 | HERO_POWER | 0..1 | hero select, combat; TURN 2-26 | 78 (S 5 / D 73; 6 g) | BG31_HERO_802p, BG27_HERO_801p2, BG36_HERO_101p |
| 4080 | PLAYER | 1..1 | shop; TURN 3-5 | 11 (S 2 / D 9; 11 g) |  |
| 4101 | PLAYER, ENCHANTMENT, PLAYER_BOB | 0..22 | shop, combat; TURN 1-30 | 1105 (S 77 / D 1028; 11 g) | Bacon_TagTransferPlayerE |
| 4112 | PLAYER, ENCHANTMENT, PLAYER_BOB | 0..39 | shop, combat; TURN 1-30 | 936 (S 40 / D 896; 11 g) | Bacon_TagTransferPlayerE |
| 4137 | ENCHANTMENT | 1..1 | shop, combat; TURN 4-30 | 143 (S 0 / D 143; 5 g) | BG28_309e, TB_BaconShop_HP_024e2, BG36_MidGameEffect_000t12e |
| 4163 | MINION, HERO, SPELL | 0..7 | shop, combat; TURN 2-30 | 5571 (S 309 / D 5262; 11 g) | BGS_034, BGS_119, BG25_010t |
| 4177 | BATTLEGROUND_TRINKET | 0..1 | shop, combat; TURN 11-16 | 64 (S 0 / D 64; 7 g) | BG30_MagicItem_888, BG30_MagicItem_891 |
| 4185 | BATTLEGROUND_TRINKET | 0..4 | shop, combat; TURN 11-22 | 53 (S 4 / D 49; 9 g) | BG35_MagicItem_890, BG30_MagicItem_418, BG35_MagicItem_840 |
| 4200 | GAME | 0..1 | shop; TURN 5-29 | 534 (S 62 / D 472; 11 g) |  |
| 4206 | BATTLEGROUND_TRINKET | 0..1 | combat; TURN 12-26 | 24 (S 0 / D 24; 1 g) | BG32_MagicItem_821 |
| 4211 | PLAYER, PLAYER_BOB | 0..1 | combat; TURN 2-24 | 26 (S 0 / D 26; 2 g) |  |
| 4212 | PLAYER, ENCHANTMENT, PLAYER_BOB | 0..305 | shop, combat; TURN 1-30 | 1846 (S 137 / D 1709; 11 g) | Bacon_TagTransferPlayerE |
| 4214 | PLAYER_BOB | 0..1 | combat; TURN 8-20 | 6 (S 0 / D 6; 1 g) |  |
| 4230 | PLAYER, ENCHANTMENT, PLAYER_BOB | 0..150 | shop, combat; TURN 10-30 | 225 (S 31 / D 194; 6 g) | Bacon_TagTransferPlayerE |
| 4231 | SPELL, BATTLEGROUND_SPELL | 0..1 | shop, combat; TURN 8-30 | 249 (S 49 / D 200; 6 g) | BG33_813, BG33_814, BG33_815 |
| 4235 | MINION, BATTLEGROUND_TRINKET, ENCHANTMENT | 0..5630 | shop, combat; TURN 6-30 | 354 (S 76 / D 278; 11 g) | BG33_883, BG36_331, BG31_326 |
| 4236 | MINION, BATTLEGROUND_TRINKET, ENCHANTMENT | 0..7845 | shop, combat; TURN 6-30 | 354 (S 76 / D 278; 11 g) | BG33_883, BG36_331, BG31_326 |
| 4237 | ENCHANTMENT | 1..1 | combat; TURN 28 | 2 (S 0 / D 2; 1 g) | BG20_HERO_282e2 |
| 4243 | MINION | 0..1 | shop, combat; TURN 5-24 | 275 (S 58 / D 217; 5 g) | BG34_170t3, BG34_170t2, BG34_170t |
| 4254 | BATTLEGROUND_SPELL, SPELL | 0..1 | shop, combat; TURN 1-30 | 1961 (S 186 / D 1775; 11 g) | BG35_911, BG36_624, BG36_301t |
| 4256 | GAME | 1..1 | setup | 11 (S 2 / D 9; 11 g) |  |
| 4269 | MINION, HERO | 0..1 | combat; TURN 2-30 | 1862 (S 110 / D 1752; 11 g) | BG36_345, BG25_010t, BGS_034 |
| 4270 | MINION | 0..5630 | shop, combat; TURN 6-30 | 676 (S 130 / D 546; 5 g) | BG33_883, BG36_331, BG36_240 |
| 4271 | MINION | 0..7845 | shop, combat; TURN 6-30 | 676 (S 130 / D 546; 5 g) | BG33_883, BG36_331, BG36_240 |
| 4290 | SPELL, BATTLEGROUND_SPELL | 0..1 | shop, combat; TURN 3-29 | 611 (S 33 / D 578; 11 g) | BG31_886t2, BG31_886, BG31_880t2 |
| 4298 | GAME | 9..9 | hero select | 11 (S 2 / D 9; 11 g) |  |
| 4299 | PLAYER_BOB, PLAYER, ENCHANTMENT | 0..3 | shop, combat; TURN 4-22 | 34 (S 3 / D 31; 4 g) | Bacon_TagTransferPlayerE |
| 4301 | GAME | 4..4 | hero select | 11 (S 2 / D 9; 11 g) |  |
| 4303 | SPELL | 0..1 | combat; TURN 6-28 | 381 (S 24 / D 357; 4 g) | BG20_GEM_No_Impact |
| 4304 | HERO_POWER, BATTLEGROUND_SPELL, SPELL | 0..1 | hero select, shop, combat; TURN 4-28 | 51 (S 8 / D 43; 9 g) | EBG_Spell_032, BG20_HERO_301p |
| 4325 | SPELL | 0..3 | shop, combat; TURN 7-24 | 24 (S 6 / D 18; 6 g) | BG36_MidGameEffect_000t13 |
| 4330 | MINION, BATTLEGROUND_SPELL, BATTLEGROUND_QUEST_REWARD | 0..5 | shop, combat; TURN 9-19 | 31 (S 0 / D 31; 8 g) | BG35_150, BG36_354, BGS_034 |
| 4344 | SPELL, BATTLEGROUND_SPELL | 0..1 | shop, combat; TURN 12-28 | 151 (S 14 / D 137; 9 g) | BG28_604, EBG_Spell_032, BG28_573 |
| 4348 | ENCHANTMENT | 0..2018 | shop, combat; TURN 1-29 | 1280 (S 120 / D 1160; 11 g) | BG36_364e, BG_Consumed, BG36_246e |
| 4349 | ENCHANTMENT | 0..1774 | shop, combat; TURN 1-29 | 1236 (S 111 / D 1125; 11 g) | BG36_364e, BG_Consumed, BG34_854e |
| 4350 | ENCHANTMENT | 0..1 | shop, combat; TURN 10-28 | 442 (S 5 / D 437; 5 g) | BG36_364e, BG30_MagicItem_995e, BG36_246e |
| 4408 | MINION, BATTLEGROUND_SPELL, SPELL | 0..1 | shop, combat; TURN 15-28 | 143 (S 0 / D 143; 3 g) | BG34_Giant_331, BG34_Giant_314, BG34_Giant_038 |
| 4418 | GAME | 1..1 | shop; TURN 5-15 | 11 (S 2 / D 9; 11 g) |  |
| 4419 | GAME | 2..6 | shop; TURN 5-23 | 26 (S 4 / D 22; 11 g) |  |
| 4421 | MINION, BATTLEGROUND_SPELL, SPELL | 0..1 | shop, combat; TURN 1-30 | 437 (S 11 / D 426; 8 g) | BG32_236, BG34_Giant_331, BG36_372_G |
| 4441 | PLAYER, PLAYER_BOB | 0..6 | hero select, shop, combat; TURN 2-26 | 36 (S 0 / D 36; 2 g) |  |
| 4446 | MINION | 0..1 | shop, combat; TURN 4-22 | 256 (S 0 / D 256; 2 g) | BG32_236, BG33_823, BG25_010t |
| 4449 | PLAYER, ENCHANTMENT, PLAYER_BOB | 0..5 | shop, combat; TURN 5-30 | 346 (S 12 / D 334; 11 g) | Bacon_TagTransferPlayerE |
| 4453 | PLAYER, PLAYER_BOB | 0..15 | shop, combat; TURN 2-30 | 328 (S 36 / D 292; 11 g) |  |
| 4463 | MINION, BATTLEGROUND_SPELL, SPELL | 0..1 | shop, combat; TURN 15-20 | 11 (S 0 / D 11; 2 g) | BG34_Giant_038, BG34_Giant_042, BG34_Treasure_933 |
| 4466 | MINION | 0..1 | combat; TURN 20 | 4 (S 0 / D 4; 1 g) | BG34_Giant_038 |
| 4467 | MINION, BATTLEGROUND_SPELL, SPELL | 0..1 | shop, combat; TURN 15-28 | 139 (S 0 / D 139; 2 g) | BG34_Giant_331, BG34_Giant_314, BG34_Giant_042 |
| 4468 | PLAYER, ENCHANTMENT, PLAYER_BOB | 0..32 | shop, combat; TURN 8-28 | 90 (S 5 / D 85; 5 g) | Bacon_TagTransferPlayerE |
| 4469 | ENCHANTMENT, PLAYER_BOB, PLAYER | 0..41 | shop, combat; TURN 6-28 | 91 (S 5 / D 86; 5 g) | Bacon_TagTransferPlayerE |
| 4472 | MINION, BATTLEGROUND_TRINKET, SPELL | 0..5831 | hero select, shop, combat; TURN 1-30 | 2578 (S 166 / D 2412; 11 g) | BG30_Trinket_2nd, BG30_Trinket_1st, BG25_010 |
| 4482 | GAME | 1..1 | setup | 11 (S 2 / D 9; 11 g) |  |
| 4500 | PLAYER | 0..26 | shop; TURN 1-29 | 464 (S 66 / D 398; 11 g) |  |
| 4526 | GAME | 0..1 | shop; TURN 15 | 2 (S 0 / D 2; 1 g) |  |
| 4551 | PLAYER_BOB, PLAYER | 0..96 | shop, combat; TURN 2-30 | 521 (S 19 / D 502; 11 g) |  |
| 4552 | PLAYER | 0..405 | shop; TURN 5-29 | 370 (S 31 / D 339; 11 g) |  |
| 4553 | PLAYER | 0..373 | shop; TURN 5-29 | 372 (S 31 / D 341; 11 g) |  |
| 4580 | HERO | 1..3 | shop; TURN 7-23 | 7 (S 2 / D 5; 5 g) | TB_BaconShop_HERO_102_SKIN_G, BG21_HERO_000_SKIN_D, BG22_HERO_002 |
| 4603 | BATTLEGROUND_TRINKET | 0..19564 | shop, combat; TURN 12-18 | 11 (S 0 / D 11; 2 g) | BG35_MagicItem_801, BG30_MagicItem_801 |
| 4607 | MINION | 0..1 | shop, combat; TURN 2-23 | 235 (S 6 / D 229; 5 g) | BG35_814, BG35_814_G |
| 4610 | SPELL | 0..1 | shop, combat; TURN 5-30 | 214 (S 9 / D 205; 11 g) | TB_BaconShop_Triples_01 |
| 4630 | PLAYER | 0..1 | combat; TURN 22-28 | 4 (S 0 / D 4; 2 g) |  |
| 4631 | BATTLEGROUND_SPELL | 0..1 | shop, combat; TURN 21-29 | 9 (S 0 / D 9; 2 g) | EBG_Spell_017 |
| 4639 | PLAYER, PLAYER_BOB, ENCHANTMENT | 0..227 | shop, combat; TURN 2-30 | 1396 (S 52 / D 1344; 11 g) | Bacon_TagTransferPlayerE |
| 4640 | PLAYER, PLAYER_BOB, ENCHANTMENT | 0..227 | shop, combat; TURN 2-30 | 1396 (S 52 / D 1344; 11 g) | Bacon_TagTransferPlayerE |
| 4651 | SPELL | 0..1 | shop, combat; TURN 6-20 | 97 (S 51 / D 46; 4 g) | BG20_GEM |
| 4681 | GAME | 1..1 | setup | 11 (S 2 / D 9; 11 g) |  |
| 4684 | PLAYER | 0..30 | shop; TURN 13-23 | 72 (S 0 / D 72; 3 g) |  |
| 4685 | MINION | 0..1 | combat; TURN 24-30 | 18 (S 0 / D 18; 1 g) | BG36_523_G |
| 4692 | BATTLEGROUND_TRINKET | 0..1 | shop, combat; TURN 11-30 | 287 (S 17 / D 270; 11 g) | BG30_MagicItem_435, BG36_MagicItem_370, BG30_MagicItem_426 |
| 4695 | PLAYER, PLAYER_BOB | 0..2 | shop, combat; TURN 17-30 | 40 (S 11 / D 29; 3 g) |  |
| 4696 | BATTLEGROUND_TRINKET, HERO_POWER | 0..1 | shop, combat; TURN 2-30 | 296 (S 28 / D 268; 11 g) | BG30_Trinket_2nd, BG30_Trinket_1st, BG36_HERO_002p |
| 4697 | BATTLEGROUND_TRINKET, HERO_POWER | 0..0 | combat; TURN 2-30 | 287 (S 28 / D 259; 11 g) | BG30_Trinket_2nd, BG30_Trinket_1st, BG36_HERO_002p |
| 4698 | BATTLEGROUND_TRINKET, HERO_POWER | 0..0 | combat; TURN 2-30 | 287 (S 28 / D 259; 11 g) | BG30_Trinket_2nd, BG30_Trinket_1st, BG36_HERO_002p |
| 4701 | BATTLEGROUND_TRINKET, SPELL, HERO_POWER | 0..1 | hero select, shop, combat; TURN 2-30 | 1032 (S 73 / D 959; 11 g) | TB_BaconShop_Triples_01, BG36_MidGameEffect_010, BG30_MagicItem_435 |
| 4707 | MINION | 0..1 | combat; TURN 20-26 | 12 (S 0 / D 12; 1 g) | BG28_309_G, BG30_125_G, BGDUO31_208 |
| 4709 | PLAYER_BOB, PLAYER | 0..1 | hero select, shop, combat; TURN 2-30 | 168 (S 3 / D 165; 9 g) |  |
| 4713 | MINION | 0..1 | shop, combat; TURN 9-24 | 131 (S 12 / D 119; 5 g) | BG21_015, BG21_015_G |
| 4729 | PLAYER_BOB | 0..6 | combat; TURN 18-30 | 10 (S 0 / D 10; 1 g) |  |
| 4730 | GAME | 5..5 | setup | 11 (S 2 / D 9; 11 g) |  |
| 4732 | PLAYER_BOB, PLAYER | 0..2 | shop, combat; TURN 13-30 | 44 (S 12 / D 32; 4 g) |  |
| 4741 | MINION, SPELL, ENCHANTMENT | 0..57 | setup, hero select, shop, combat; TURN 1-30 | 24992 (S 1621 / D 23371; 11 g) | BG36_345, BGS_034, BG29_813e |
| 4768 | ENCHANTMENT, PLAYER_BOB, PLAYER | 0..20 | shop, combat; TURN 2-30 | 227 (S 10 / D 217; 9 g) | Bacon_TagTransferPlayerE |
| 4773 | PLAYER, PLAYER_BOB | 0..2 | shop, combat; TURN 15-30 | 40 (S 0 / D 40; 3 g) |  |
| 4784 | MINION, SPELL, BATTLEGROUND_SPELL | 0..1 | shop; TURN 1-29 | 176 (S 0 / D 176; 9 g) | BG28_550, BGDUO_114, BG36_515 |
| 4799 | PLAYER, ENCHANTMENT, PLAYER_BOB | 0..38 | shop, combat; TURN 2-30 | 605 (S 20 / D 585; 11 g) | Bacon_TagTransferPlayerE |
| 4803 | PLAYER_BOB, ENCHANTMENT | 0..14 | combat; TURN 10-18 | 21 (S 7 / D 14; 3 g) | Bacon_TagTransferPlayerE |
| 4809 | PLAYER, ENCHANTMENT, PLAYER_BOB | 0..14 | shop, combat; TURN 2-30 | 356 (S 20 / D 336; 11 g) | Bacon_TagTransferPlayerE |
| 4810 | PLAYER, ENCHANTMENT, PLAYER_BOB | 0..29 | shop, combat; TURN 2-30 | 456 (S 24 / D 432; 11 g) | Bacon_TagTransferPlayerE |
| 4811 | SPELL, BATTLEGROUND_SPELL | 0..1 | shop, combat; TURN 1-30 | 917 (S 68 / D 849; 11 g) | BG35_911, BG28_503, BG28_604 |
| 4825 | PLAYER, ENCHANTMENT, PLAYER_BOB | 0..41 | shop, combat; TURN 5-26 | 218 (S 11 / D 207; 8 g) | Bacon_TagTransferPlayerE |
| 4826 | PLAYER, PLAYER_BOB, ENCHANTMENT | 0..49 | shop, combat; TURN 2-30 | 521 (S 13 / D 508; 10 g) | Bacon_TagTransferPlayerE |
| 4836 | PLAYER, ENCHANTMENT, PLAYER_BOB | 0..10 | shop, combat; TURN 1-30 | 710 (S 34 / D 676; 11 g) | Bacon_TagTransferPlayerE |
| 4837 | PLAYER, ENCHANTMENT, PLAYER_BOB | 0..77 | shop, combat; TURN 1-30 | 1480 (S 136 / D 1344; 11 g) | Bacon_TagTransferPlayerE |
| 4853 | SPELL | 0..9 | shop, combat; TURN 7-24 | 291 (S 48 / D 243; 11 g) | BG36_MidGameEffect_000t11, BG36_MidGameEffect_000t18, BG36_MidGameEffect_000t10 |
| 4854 | SPELL | 0..11 | shop, combat; TURN 7-22 | 192 (S 36 / D 156; 11 g) | BG36_MidGameEffect_000t18, BG36_MidGameEffect_000t52, BG36_MidGameEffect_000t5 |
| 4855 | SPELL | 0..1 | shop, combat; TURN 7-24 | 315 (S 54 / D 261; 11 g) | BG36_MidGameEffect_000t11, BG36_MidGameEffect_000t13, BG36_MidGameEffect_000t18 |
| 4856 | SPELL | 0..200 | shop, combat; TURN 11-24 | 27 (S 0 / D 27; 8 g) | BG36_MidGameEffect_000t16, BG36_MidGameEffect_000t22, BG36_MidGameEffect_000t66 |
| 4868 | MINION | 0..1 | shop, combat; TURN 13-28 | 149 (S 0 / D 149; 3 g) | BG36_511 |
| 4874 | SPELL | 0..1 | shop, combat; TURN 7-24 | 138 (S 24 / D 114; 11 g) | BG36_MidGameEffect_000t13, BG36_MidGameEffect_000t10, BG36_MidGameEffect_000t |
| 4901 | MINION, ENCHANTMENT, SPELL | 0..1 | hero select, shop, combat, end; TURN 1-30 | 54042 (S 4635 / D 49407; 11 g) | TB_BaconShop_DragBuy, TB_BaconShopBadsongE, TB_BaconShop_DragBuy_Spell |
| 4904 | MINION, SPELL, BATTLEGROUND_TRINKET | 0..5 | shop, combat; TURN 2-30 | 1115 (S 83 / D 1032; 11 g) | BG30_Trinket_2nd, BG30_Trinket_1st, BG36_624 |
| 4908 | PLAYER_BOB | 0..1 | combat; TURN 20 | 2 (S 0 / D 2; 1 g) |  |
| 4912 | MINION | 0..1 | shop, combat; TURN 1-30 | 306 (S 8 / D 298; 6 g) | BG32_236 |
| 4921 | SPELL | 0..1 | shop, combat; TURN 7-24 | 66 (S 15 / D 51; 10 g) | BG36_MidGameEffect_000t13, BG36_MidGameEffect_000t, BG36_MidGameEffect_000t2 |
| 4925 | SPELL | 0..1 | shop, combat; TURN 17-18 | 2 (S 0 / D 2; 1 g) | BG36_MidGameEffect_010 |
| 4929 | MINION | 0..19780 | combat; TURN 8-26 | 93 (S 0 / D 93; 6 g) | BGFYM_000, BGFYM_011 |
| 4937 | MINION | 0..1 | shop, combat; TURN 1-29 | 145 (S 32 / D 113; 9 g) | BG36_511, BG36_345, BG36_342 |
| 4939 | MINION | 0..1 | combat; TURN 14-26 | 45 (S 0 / D 45; 1 g) | BG36_362_G, BG36_362 |
| 4940 | MINION | 0..1 | shop, combat; TURN 8-25 | 110 (S 22 / D 88; 7 g) | BG36_360t3, BG36_360t6, BG36_360t9 |
| 4944 | MINION | 0..1 | combat; TURN 18 | 6 (S 0 / D 6; 1 g) | BG26_149 |
| 4946 | SPELL | 0..1 | shop, combat; TURN 1-30 | 803 (S 37 / D 766; 7 g) | BG_OldGod |
| 4953 | MINION | 0..1 | shop, combat; TURN 5-29 | 992 (S 35 / D 957; 11 g) | BG36_311, BG36_511, BG36_312 |
| 4955 | MINION | 0..1 | shop, combat; TURN 2-29 | 804 (S 100 / D 704; 10 g) | BG24_715, BG36_098, BG31_816 |
| 4964 | MINION | 0..1 | combat; TURN 8-26 | 56 (S 0 / D 56; 6 g) | BGFYM_000, BGFYM_011 |
| 4965 | MINION | 0..1 | combat; TURN 8-26 | 56 (S 0 / D 56; 6 g) | BGFYM_000, BGFYM_011 |
| 4997 | MINION, BATTLEGROUND_TRINKET, SPELL | 0..5162 | hero select, shop, combat; TURN 1-30 | 2486 (S 175 / D 2311; 11 g) | BG30_Trinket_2nd, BG30_Trinket_1st, BG36_624 |

## Worth adding next (for the user to choose)

Ranked by value for the product over effort. All are inside D-004 as described; none is implemented.

| # | What | Tags or lines | Where it shows | Allowed | Effort |
|---|---|---|---|---|---|
| 1 | **Combat result as the game records it** for the local player, instead of working it out from health (D-028 marks results "inferred"; 2 of the 4 outcomes can be "unknown") | `BACON_WON_LAST_COMBAT`, `DAMAGE_DEALT_TO_HERO_LAST_TURN` (on the local player and hero, every combat) | recap, record against each opponent, stats | yes (own) | low |
| 2 | **Opponents' triples and trinkets** in the hover card and the recap, as the game's leaderboard tooltip shows them | `PLAYER_TRIPLES`, `BACON_FIRST_TRINKET_DATABASE_ID`, `BACON_SECOND_TRINKET_DATABASE_ID`, `BACON_HERO_*_TRINKET_LEADERBOARD_SDN*` (on lobby heroes) | overlay hover, recap | yes (leaderboard tooltip) | low |
| 3 | **Next opponent** marked in the overlay with their last board (Duos: the next opposing team) | `NEXT_OPPONENT_PLAYER_ID`, `NEXT_OPPONENT_TEAMMATE_PLAYER_ID`, `BACON_DUO_PLAYER_FIGHTS_FIRST_NEXT_COMBAT` | overlay | yes (the game marks it) | low |
| 4 | **Keywords on boards** (taunt, divine shield, reborn, venomous, windfury, stealth, deathrattle, golden by card id) | `TAUNT`, `DIVINE_SHIELD`, `REBORN`, `VENOMOUS`, `WINDFURY`, `STEALTH`, `DEATHRATTLE`, `BACON_RALLY`, … on minions that entered play | overlay boards, recap boards | yes (on screen) | low |
| 5 | **Skins grouped by the game's own link** instead of the D-019 name rule | `BACON_SKIN`, `BACON_SKIN_PARENT_ID` (database id of the base hero) | per-hero stats | yes | low (needs a database-id to card-id map from the card data already cached, D-038) |
| 6 | **Hero select record**: heroes offered, rerolls used, hero picked | `DebugPrintEntityChoices ChoiceType=MULLIGAN` + `Entities[]`, `BACON_NUM_MULLIGAN_REFRESH_USED` | stats (pick rate, placement when offered) | yes (own) | medium |
| 7 | **Shop record extras**: extra gold, why a roll was free, buy and sell prices, turn timer, Duos passes | `TEMP_RESOURCES`, `BACON_FREE_REFRESH_COUNT`, `BACON_OVERRIDE_BG_COST`, `BACON_SELL_VALUE`, `TIMEOUT`, `IS_USING_PASS_OPTION`, `DECK_ACTION` blocks | recap "Shop and actions" | yes (own) | low |
| 8 | **Season choices in the recap**: own trinkets and the trinket offers, quests and rewards, Dark Gifts, the Old God, Timewarped buys | `BACON_TRINKET`, `BACON_IS_POTENTIAL_TRINKET` (local side only), `QUEST*`, `QUEST_REWARD_DATABASE_ID`, `HAS_DARK_GIFT`, `DARK_GIFT_ENTITY`, `BACON_GLOBAL_OLD_GOD_DBID`, `BACON_TIMEWARPED` | recap, stats | yes (own; check the Old God is shown before it acts) | medium |
| 9 | **Discover picks**: what was offered and what was taken | `DebugPrintEntityChoices GENERAL` + `DebugPrintEntitiesChosen` | recap | yes (own) | medium |
| 10 | **Lobby/queue signal** from scene changes: "in the Battlegrounds lobby", "game loading" | `LoadingScreen.log` `prevMode`/`currMode` | app status, overlay start | yes | low |
| 11 | **Damage cap** of the current combat | `BACON_COMBAT_DAMAGE_CAP`, `…_ENABLED` | overlay | check first that the game shows it | low |
| 12 | **Combat replay** (attack by attack) | `ATTACK`/`DEATHS` blocks, `PROPOSED_ATTACKER`/`DEFENDER`, `META_DATA DAMAGE`, `PREDAMAGE` | replays (F2) | yes for combats the game plays back; Duos combats it hides stay "not visible" | high |

**Do not use (D-004 or no meaning):** `GAME_SEED`; `HIDDEN_SCRIPT_DATA_*`; trinket offers and cards on Bob's side outside play (other players' choices, Bob's hand); the `SETASIDE` copies of a Duos combat the game does not play back (already excluded, parser-hslog.md); every network or account value in `Net.log`, `GameNetLogger.log`, `Hearthstone.log`, `Login.log`; deck names in `Decks.log`; the 404 unnamed tags until their meaning is known.

## Log sections not enabled today

The user's `log.config` enables `[Achievements] [Arena] [FullScreenFX] [LoadingScreen] [Power] [Decks] [Net]`. Other section names appear in public tracker documentation. What each holds is only "said to" here: no page found gives a reliable description, and nothing was tried. Enabling one is the user's edit (the app never writes `log.config`, T-106) and changes nothing about D-004: it is still a local log.

| Section | Said to hold | Source | Might help with |
|---|---|---|---|
| `[Zone]` | Zone changes of cards (hand, play, graveyard) as the client shows them; older trackers needed it | [HS-DeckTrack README](https://github.com/bru7us/HS-DeckTrack) (name only), search summary of [hearthstone.gg](http://www.hearthstone.gg/articles/log-file/) | nothing new: `Power.log` has every `ZONE` change |
| `[Bob]` | Client state, legend rank (search summary) | same | perhaps rank or mode changes; no MMR expected (T-004) |
| `[Asset]` | Unity asset loading | search summary of hearthstone.gg | nothing for the product |
| `[Rachelle]`, `[Kyle]`, `[Sound]`, `[Packets]` | Named in the same list; Kyle "card back information" | search summary of hearthstone.gg | unknown; `[Packets]` may hold network data (would be keys-and-counts only) |

Suggestion: if the user wants, enable `[Zone]` and `[Bob]` for one session and run `log_inventory.py` again; the script already handles any new file kind.

## Sources

- The user's own logs (6 session folders, 2026-10-09 to 2026-10-10), read with `tools/log_inventory.py`.
- [parser-hslog.md](parser-hslog.md), [shop-and-apm.md](shop-and-apm.md) for the tags the parser reads.
- Public pages, for section names only: [HS-DeckTrack README](https://github.com/bru7us/HS-DeckTrack); [hearthstone.gg, "Introduction to the Hearthstone Log File"](http://www.hearthstone.gg/articles/log-file/) (unreachable on 2026-10-10; known only from a search summary). No tracker code was read.

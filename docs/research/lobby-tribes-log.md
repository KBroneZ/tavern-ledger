# Lobby tribes and per-player tribe hints in the log (T-310)

Session 039, 2026-10-10. Build 253216. Question from the user: the lobby's five tribes are in the log at the start of the game, and the game shows a minion type and a number of minions for each player; where are they?

**Answers.**

1. **The lobby's tribes are not in the log as a list**, at the start of the game or later. What the log does give is enough to work them out exactly in practice: every pool minion Bob offers is in the shop record (D-046), and a minion that has exactly one tribe in the card data proves that tribe is in the lobby. On all 18 games of the user's history that have a shop record, this gives exactly five tribes per game, and every dual-tribe offer shares a tribe with them. The app uses it as `inferred` (it needs the card data and is not a line of the log), shown as "Lobby (N of 5 confirmed)".
2. **The per-player minion type and count are not in the log.** No tag on the lobby heroes or the player entities holds a tribe and a count that change after each combat. The game draws them from data the client does not write to any log file, so they are out of reach (D-004: no memory, no pixel reading). The board guess for an opponent not met yet uses the lobby tribes and their tier only (D-050).

Only the user's own logs were read, with shared access. Nothing below prints a player name, BattleTag or account id; only tag names, entity types, card ids and counts.

## Data

| Session folder | Mode | Games | Lines of the game (`GameState.DebugPrintPower`) | Tavern tribes (`CARDRACE` of Bob's offers, `shop_tribes`) |
|---|---|---|---|---|
| `Hearthstone_2026_10_10_14_58_17` | Solo | 1 | 84 497 | Pirate 12, Dragon 9, Beast 6, Murloc 5, Quilboar 5 (+ Neutral 5) |
| `Hearthstone_2026_10_10_18_11_27` | Duos | 2 | 111 554 and 318 892 | 1: Demon 8, Beast 7, Undead 7, Pirate 6, Murloc 5, **Mechanical 2, Naga 1** (+ Neutral 8); 2: Aberration 12, Pirate 13, Elemental 11, Murloc 10, Quilboar 6 (+ Neutral 14, All 2) |

Older raw logs are gone (the game keeps a few sessions); the history's shop records (`games.jsonl`, parser revision 3) cover 18 games. The user has no hand entry (`lobby_tribes.json` does not exist), so there was nothing to compare with.

## Where the lobby tribes could be, and what is there

Checked on the three raw games:

- **Before the first shop** (the lines before `TAG_CHANGE Entity=GameEntity tag=TURN value=1`: 942, 1014 and 1007 lines): no tag on any entity has a set of values that names the lobby's tribes, by name or by `Race` number (UNDEAD 11, MURLOC 14, DEMON 15, MECHANICAL 17, ELEMENTAL 18, BEAST 20, PIRATE 23, DRAGON 24, QUILBOAR 43, NAGA 92, ABERRATION 126). The entities created there are the heroes, Bob, the trinket and hero-select cards and hidden `SETASIDE` entities. One `BACON_SUBSET_PIRATE=1` appears, on a hero card offered at hero select (`TB_BaconShop_HERO_18`, Duos game 2).
- **The game entity:** across the three games only these tags differ: `10`, `937`, `1323`, `1710`, `2022`, `2374`, `3452`, `3475`, `3533`, `3534`, `BACON_BARTENDER_CARD_ID`, `BACON_CHOSEN_BOARD_SKIN_ID`, `BACON_COMBAT_DAMAGE_CAP`, `BACON_GLOBAL_OLD_GOD_DBID`, `BACON_MAX_LEADERBOARD_ARMOR`, `BG_COMBAT_SPEED_MAX_SPEED`, `ENTITY_ID`, `GAME_SEED`, `HIGHLIGHT_ATTACKING_MINION_DURING_COMBAT`, `MISSION_EVENT`, `PROPOSED_ATTACKER`, `PROPOSED_DEFENDER`, `TUTORIAL_HERO_POWER_TARGET_MINION_ANIM`. None follows the tribes. `BACON_FACTION_BANNERS_ENABLED=1` is a switch, the same in all games.
- **`BACON_SUBSET_<TRIBE>`** (the hub's lead): a card attribute. It sits on minions, spells, trinkets and a few heroes and hero powers of that tribe or that work with it, from the first shop on, and it carries tribes that are **not** in the lobby in every game:

  | Game | Entities with `BACON_SUBSET_*=1` (by tribe) | Outside the lobby |
  |---|---|---|
  | Solo | Pirate 152, Dragon 77, Murloc 68, Beast 54, Quillboar 40, Demon 5, Naga 1, Undead 1 | Demon 5 (minions), Naga 1 (a dragon), Undead 1 (a trinket) |
  | Duos 1 | Pirate 136, Demon 102, Beast 81, Undead 77, Murloc 36, Mech 6, Naga 2 | Mech 6, Naga 2 |
  | Duos 2 | Pirate 343, Elementals 287, Quillboar 162, Aberration 143, Murloc 108, Mech 17, Beast 7, Dragon 7, Demon 4 | Mech 17, Beast 7, Dragon 7, Demon 4 |

  So it does not mark the lobby (nor the banned tribes); the very different counts per tribe the hub saw are how often cards of each tribe were created.
- **Other tags looked at:** `CARDRACE` (one race per card; the first of a dual-tribe minion), `IS_BACON_POOL_MINION` (pool minion yes/no), `BACON_MAX_PLAYER_TECH_LEVEL` (6 on every hero and player). None names the lobby.
- **Other log files** of the Duos session (`Hearthstone.log`, `LoadingScreen.log`, `Gameplay.log`, `Net.log`, `GameNetLogger.log`, and the rest): no tribe names besides card names in focus messages and `BG_ShopBuff_Elemental_Ench` (a shop buff). `Power.log`'s other line kinds (`DebugPrintOptions`, `DebugPrintEntityChoices`, `DebugPrintGame`: build, format, game type, scenario) hold nothing on tribes.

## Working the tribes out from the tavern (found)

The shop record lists every pool minion entering Bob's side. The log's `CARDRACE` gives one race per card, so a Mechanical-Undead minion counts as Mechanical; that is why tribes outside the lobby show up in the tavern counts (Duos 1 above: Mechanical 2, Naga 1). The card data (D-038) has every tribe of a minion. Rule: **a tribe is in the lobby once Bob offers a pool minion whose only tribe it is** (`ALL` and neutral minions prove nothing; a dual-tribe minion proves nothing on its own).

Checked on the 18 games of the history with a shop record (the card data of 2026-10-10, 303 pool minions; no offer was missing from it):

- every game gives exactly **5** confirmed tribes (Solo 2, Duos 16);
- every dual-tribe offer has a tribe among the five (for example Duos 1: Demon-Naga 1, Mechanical-Undead 2, Beast-Pirate 1; Solo: Dragon-Pirate 2, Demon-Dragon 2, Dragon-Naga 1, Demon-Quilboar 1);
- the three raw games give the lobbies Beast, Dragon, Murloc, Pirate, Quilboar (Solo); Beast, Demon, Murloc, Pirate, Undead (Duos 1); Aberration, Elemental, Murloc, Pirate, Quilboar (Duos 2).

On the Solo game the five were all confirmed by shop turn 5 (18 offers so far). If the rule ever gives more than five tribes (a card data change, an effect that offers an off-lobby minion), the app claims nothing from it and falls back to the tavern counts.

To recheck: for each game in `games.jsonl`, take `shop.turns[].offers[].card_id`, look each up in `card-cache/catalog.json` (`pool.<id>.tribes`), and collect the tribes of the offers with exactly one tribe other than `ALL`.

## The per-player tribe and count

The game shows, for each player, their most common minion type and how many. Looked for on the lobby heroes (the entities with `PLAYER_LEADERBOARD_PLACE`; 9, 9 and 8 in the three games): the only tags that change on all of them through the game are `PLAYER_LEADERBOARD_PLACE`, `PLAYER_TECH_LEVEL`, `PLAYER_TRIPLES`, `ARMOR`, `DAMAGE`, `LAST_AFFECTED_BY`, `LINKED_ENTITY`, `4741` (one step per hero, 1 to 11), `3669` (0 or 1), and in Duos `BACON_DUO_PLAYER_FIGHTS_FIRST_NEXT_COMBAT` and the trinket leaderboard numbers. None holds a tribe; none is a count of minions. The player entities in the log are only the user's and Bob's (`BACON_DUMMY_PLAYER`); the opponent's player entity exists only during a fight. Tags whose value is a `Race` number on heroes (`2703`, `3026`, `3206`, `2263`) are set once or step by 2 and do not follow the boards.

**Not in the log.** Nothing on screen is read to get it (D-004, D-006).

## Sources

- The game's tavern hint described by players: [TheGamer, "things to know before playing Hearthstone Battlegrounds"](https://www.thegamer.com/things-to-know-before-playing-hearthstone-battlegrounds/) (hovering a portrait shows the dominant minion type, or "Mixed Minions"); [HSReplay, Battlegrounds guide, April 2020](https://articles.hsreplay.net/2020/04/15/battlegrounds-guide-april-2020/). Read for what the game shows, not for any code.
- `Race` and `GameTag` numbers: the `hearthstone` Python package (MIT, already in `requirements.txt`).

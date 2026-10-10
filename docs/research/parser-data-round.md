# Parser data round: combat results, hero pick, shop extras, skin links

Session 044, 2026-10-10. Tasks T-206, T-209, T-210 and T-214 from the log inventory ([log-inventory.md](log-inventory.md), "Worth adding next"). Decision: D-055. Parser revision 4.

## Method

- **Logs:** the user's own `Power.log` and `Power_old.log` files on this PC (6 files, build 253216): **11 Battlegrounds games, 2 Solo and 9 Duos**, 123 combats, 123 shop turns. The same games the inventory counted.
- **Reading:** read-only scripts in the session's scratch folder replayed the `GameState` power lines (entities, tags, top-level blocks, the hero choice lines) and printed counts only. Player names were mapped to "local player" or "other" and never printed; no BattleTag, account id or name left the scripts.
- **Cross-check:** every count below was compared with the Rust parser's report of the same games (`bg-parse --json`), and the two real fixtures (`crates/bg-parser/tests/data/real/`) pin the numbers of their games in `tests/real_games.rs`.
- **Card data:** one download of HearthstoneJSON's `cards.json` (the file the app already uses, D-038) to check the skin links; not kept.

To re-check: run `bg-parse --json` (the release build of `crates/bg-parser`) on the raw logs and count the new fields, or read the two real fixtures' reports.

## T-206: combat result as the game records it

**What the log gives.** Two tags on the **local player entity** (and `BACON_WON_LAST_COMBAT` also on its leaderboard hero, with the same values), only in combat turns (even `TURN`):

| Tag | Changes to 0 | Changes to a value | Values |
|-----|--------------|--------------------|--------|
| `BACON_WON_LAST_COMBAT` | 57 | 61 (to 1) | 0, 1 |
| `DAMAGE_DEALT_TO_HERO_LAST_TURN` | 46 | 75 | 0 to 30 |

Both are set back to 0 when a combat starts (logged only when they were not 0 already) and set during the combat. So at the end of a combat (the next shop turn's start, or `STATE=COMPLETE` after the last combat):

- `BACON_WON_LAST_COMBAT` = 1 and no damage: **won**;
- damage above 0 and not won: **lost**;
- both 0: **tie**;
- anything else (won and damage both set, other values): **unknown** (never seen).

**Checked against the health rule (D-028).** Of the 123 combats, the health rule gives a result for 102: **all 102 agree** (Solo 17 of 17, Duos 85 of 85). The other 21, unknown by health (Duos round 1 has no starting pool; a side gained health; both sides lost health, e.g. a hero that pays health in the shop; a ghost), all get a result from the game's record. Over the 123 combats: 61 won, 53 lost, 9 tied. In Duos the tags are the team's result, as the health rule counts it.

**Left out: the damage amount.** `DAMAGE_DEALT_TO_HERO_LAST_TURN` is not a clean "damage taken": in Duos it counts the hit once per teammate hero (a 2 damage loss reads 4; the value was above the health lost in 22 of the 40 Duos losses whose health the log has), and in Solo it includes damage past 0 (overkill). The health drop per round is already in the report, so only the result is kept.

**Guard.** A game whose log never sets either tag on the local player (an older client, or a patch that drops them) gets no read result at all, since every combat would look like a tie; the recap then falls back to the health rule.

## T-209: hero pick

- Each game has **one** `DebugPrintEntityChoices` with `ChoiceType=MULLIGAN`, listing **4 heroes**, all controlled by the local player. The client prints only its own choice: no other player's offer is in the log.
- A **reroll** is a `CHANGE_ENTITY` of one of the offered heroes to a new card before the pick; its packet carries `BACON_NUM_MULLIGAN_REFRESH_USED` = 1. **10 rerolls in 11 games** (one in each of 10 games, none in one).
- The **pick** is the `DebugPrintEntitiesChosen` line with the same choice id: it is the report's hero in **11 of 11** games.
- Recorded per game: every hero shown (the first four, then each reroll's hero), the rerolls and the hero picked.

## T-210: shop record extras

Gold the player has = `RESOURCES - RESOURCES_USED + TEMP_RESOURCES` on the local player entity. A top-level block gains what that gold grew by plus what it spent (`NUM_RESOURCES_SPENT_THIS_GAME`).

| Item | Rule | Count over 11 games |
|------|------|---------------------|
| Turn's own gold | set in a top-level `TRIGGER` block of the player entity (gold in any such block is the turn's own) | 123 of 123 shop turns: the turn's first block; the player entity's later `TRIGGER` blocks (744) changed no gold |
| Buy price | the change of `NUM_RESOURCES_SPENT_THIS_GAME` over the buy | 176 minion buys: 47 at 2 (all with `BACON_OVERRIDE_BG_COST` 2), 129 at 3 (none with it); 481 gold |
| Tavern spell price | same | 86 spells, 0 to 4 gold; 141 gold |
| Sell gold | the gain of the sell block (a sell at full gold goes to `TEMP_RESOURCES`) | 314 sells: 310 at 1, 4 at 5 (all 4 with `BACON_SELL_VALUE` 5); 330 gold |
| Extra gold | the gain of every other block, the turn's own gold aside | 121 gold |
| Free roll from the button | a roll during which the roll button's `BACON_FREE_REFRESH_COUNT` falls; unknown for a game whose log never gives that count | 67 rolls; none spent gold. Another 22 Duos rolls were free for another reason, and 176 cost 1 gold. 1 game of 11 never gives the count (no free roll there): unknown |
| Pass to the teammate (Duos) | a top-level `DECK_ACTION` block of the player's own card (any other is "other") | 43 blocks, each with `IS_USING_PASS_OPTION` 0 to 1 and the card leaving the hand |

**Gold check.** On all 122 shop turns with gold in the log: gold at the end = gold after the turn's own gold is set + sell gold + extra gold - gold spent. A first rule that added every rise of a gold tag counted a hero power that takes the gold and gives it back as extra gold (off on 15 turns); counting each block's net gain fixed it.

`BACON_OVERRIDE_BG_COST` and `BACON_SELL_VALUE` are not stored: the gold change gives the price directly and agrees with them every time.

**Left out: the turn timer.** `TIMEOUT` (25 to 125) is set at the end of each shop turn, sometimes two or three times in a row. The odd `TURN` that holds a shop also holds the playback of the combat before it, so its length is not the shop's: the values differed from it by -4 to +98 seconds. Without a clean shop length to check it against, it is not used.

## T-214: skins by the game's own link

- Hero entities that are skins carry `BACON_SKIN_PARENT_ID`, the database id of the base hero (and usually `BACON_SKIN` = 1). **54 skin card ids** in the 11 games: each has exactly **one** link; no base hero id has one.
- In the card data, all 54 links are hero cards. **53** give the same base hero as the D-019 rule (drop `_SKIN_<x>`); **1** differs: `TB_BaconShop_HERO_102_SKIN_G` links to `BG20_HERO_102`, which D-019 kept apart (D-019 listed this pair as "ids that only share a number").
- The parser keeps the link per card id; the card data (HearthstoneJSON's `dbfId`) turns it into the base hero's card id on the PC. The D-019 rule stays only for a game without the link (older records) or a link the card data does not know, and the stats list those ids apart.

## Fixtures

`tools/make_fixture.py` now also keeps every top-level block with its end (the gold each block gives), the tags `BACON_WON_LAST_COMBAT`, `DAMAGE_DEALT_TO_HERO_LAST_TURN`, `TEMP_RESOURCES`, `RESOURCES_USED`, `BACON_FREE_REFRESH_COUNT` and `BACON_SKIN_PARENT_ID`, and the hero choice lines rewritten to ids (`id=1 ChoiceType=MULLIGAN`, `Entities[0]=103`, `id=1 EntitiesCount=1`). Both real fixtures were made again from the same games: every field of the earlier report is unchanged, and the Rust parser reads the fixture and the raw game the same. The Python prototype cannot read the rewritten hero choice lines; it is not run on real fixtures.

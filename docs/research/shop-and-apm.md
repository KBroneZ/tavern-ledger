# Shop record and logged actions per minute (T-204, T-205, D-046)

Session 034, 2026-10-10. Everything here was checked on the user's own `Power.log` files (build 253216: 2 Solo and 14 Duos games in 5 session folders) with scripts that printed counts, tag names and card ids only, never a player name or a log line with one. No other tracker's code was read: the APM idea comes from other tools' descriptions only (absolute rule 1).

Only `GameState.*` lines are read (not `PowerTaskList`, which repeats them). "Top level" means a `GameState.DebugPrintPower()` line with no indentation after `- `: the game indents each nested block by four spaces.

## What the log gives

| Value | Source in the log | Certainty |
|-------|-------------------|-----------|
| Shop turn | `TAG_CHANGE Entity=GameEntity tag=TURN`, odd values. Shop turn *n* = (TURN + 1) / 2, combat round *n* = TURN / 2. | Certain (all 16 games). |
| Shop offers | A pool minion (`CARDTYPE=MINION`, `IS_BACON_POOL_MINION=1`) that enters `ZONE=PLAY` under the shop player's (Bob's, `BACON_DUMMY_PLAYER`) `CONTROLLER` during a shop turn. New offers are created with the local player as controller in `SETASIDE`, then `SHOW_ENTITY` moves them to Bob's side, so the parser looks at `SHOW_ENTITY`/`FULL_ENTITY` and at `ZONE`/`CONTROLLER` changes. | Certain. Tavern spells are offered the same way (`BATTLEGROUND_SPELL`); they are not listed as offers, only counted when bought. |
| Frozen offers | A frozen shop does **not** keep its entities: at the next shop turn's start the frozen minions come back as new entities, get `FROZEN=1` and then `FROZEN=0` in the turn-start `TRIGGER`, before any action of the player. An offer that gets `FROZEN=1` before the player's first own action of the turn is marked "frozen" (kept from the last turn). | Seen on every freeze checked (Solo game of 2026-10-09: 6 minions frozen, 5 pool minions back frozen). A hero or card that freezes at the start of a turn would also mark them; that is still "frozen at the start of the turn". |
| Roll | A top-level `BLOCK_START BlockType=PLAY` whose entity is the player's `TB_BaconShop_8p_Reroll_Button` (`CARDTYPE=GAME_MODE_BUTTON`). Turn-start refreshes are `POWER` blocks of the same button inside `TRIGGER`s and are not rolls. | Certain: per game the parser's count equals an independent count of those lines (51, 21, 41, 16, 17, 10, 36, 55, 10, 37, 23, 8, 31, 41, 32, 21). |
| Free roll | A roll during which `NUM_RESOURCES_SPENT_THIS_GAME` of the local player did not go up. `BACON_FREE_REFRESH_COUNT` is set on an enchantment and on the button, not on the player, so it is not used. | Certain for "no gold spent"; why it was free (hero power, card) is not read. A few roll blocks have no top-level `BLOCK_END` (2 in one Duos game); they end at the next top-level block. |
| Buy | Top-level `PLAY` block of `TB_BaconShop_DragBuy` (`MOVE_MINION_HOVER_TARGET`); its `Target=` is the bought shop minion (card id kept). `TB_BaconShop_DragBuy_Spell` buys a tavern spell (counted only). | Certain. |
| Sell | Top-level `PLAY` block of `TB_BaconShop_DragSell`; `Target=` is the sold minion. | Certain. |
| Freeze | Top-level `PLAY` block of `TB_BaconShopLockAll_Button`; it sets `FROZEN=1` on the shop. A press that only clears `FROZEN` is an unfreeze. | Freeze certain; no unfreeze press was seen in the user's logs (synthetic test only). |
| Tier-up | `PLAYER_TECH_LEVEL` going up on the local player's leaderboard hero during a shop turn (the `TB_BaconShopTechUpNN_Button` `PLAY` block is the usual cause). The first set to 1 is not a tier-up. | Certain (4 or 5 per game, matching the buttons pressed). |
| Gold | `RESOURCES` on the local player entity, set in the turn-start `TRIGGER` (3, 4, 5 … then more from effects, for example 14). Extra gold for the turn is `TEMP_RESOURCES` (not added). `RESOURCES_USED` goes up when spending and down when selling. | Certain for the turn's gold; extra gold is left out. |
| Gold spent | Change of `NUM_RESOURCES_SPENT_THIS_GAME` on the local player over the turn. Its sum over the game equals the last value of the tag (78 in the Solo game). Gold from sells is spent too, so a turn can spend more than its gold. | Certain. |
| Actions | Each `GameState.SendOption() - selectedOption=…` line is one option the client sent; each `GameState.SendChoices() - id=… ChoiceType=GENERAL` one pick (discover and the like). `ChoiceType=MULLIGAN` is the hero pick, before the first shop: not counted. The kind is the next top-level block: the shop buttons above, a hero power (`CARDTYPE=HERO_POWER`), a card played, `MOVE_MINION` (a minion moved on the board) or other (`DECK_ACTION`, or no block). | Certain: all options and choices of all 16 games fall in shop turns. A few options per game are answered by a `DECK_ACTION` block or by a `TRIGGER` first; they count as "other". |
| Times | Each line's `hh:mm:ss.fffffff`, as ms from the game's first line (midnight handled). The game's end is `STATE=COMPLETE`, else the last line (the record says the log stops early). | Certain. Only durations are kept, never a clock time. |

Duos: the local log has only two player entities (the local player and Bob); the teammate's shop, actions and gold are not in it. Bob's side during a shop turn is the local player's own shop.

Moving minions: the log records each move as `MOVE_MINION` after a `SendOption`, so APM includes it. Hovering, dragging without dropping and other mouse moves are not in the log; no input hook was needed or used.

## Per game (counts from the parser, checked against the raw lines)

| Session (local start) | Game | Mode | Shop turns | Rolls / free | Buys + spell buys | Sells | Freezes | Offers | Tier-ups | Gold spent | Actions | Minutes | APM |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 2026-10-08 00:05 | 1 | Duos | 14 | 51 / 0 | 25 + 19 | 54 | 0 | 370 | 4 | 235 | 366 | 30.4 | 12.0 |
| 2026-10-08 00:05 | 2 | Duos | 11 | 21 / 16 | 9 + 13 | 26 | 1 | 161 | 5 | 119 | 277 | 22.0 | 12.6 |
| 2026-10-08 00:05 | 3 | Duos | 13 | 41 / 14 | 29 + 22 | 72 | 4 | 317 | 5 | 223 | 482 | 25.3 | 19.0 |
| 2026-10-08 00:05 | 4 | Duos | 12 | 16 / 2 | 9 + 10 | 20 | 2 | 129 | 4 | 113 | 232 | 23.7 | 9.8 |
| 2026-10-08 00:05 | 5 | Duos | 11 | 17 / 0 | 11 + 6 | 22 | 1 | 128 | 4 | 110 | 243 | 21.2 | 11.5 |
| 2026-10-08 00:05 | 6 | Duos | 10 | 10 / 0 | 8 + 4 | 10 | 2 | 99 | 5 | 103 | 164 | 18.1 | 9.1 |
| 2026-10-08 00:05 | 7 | Duos | 12 | 36 / 14 | 19 + 8 | 39 | 1 | 240 | 4 | 150 | 244 | 24.7 | 9.9 |
| 2026-10-08 00:05 | 8 | Duos | 11 | 55 / 38 | 11 + 12 | 28 | 0 | 326 | 5 | 121 | 215 | 21.1 | 10.2 |
| 2026-10-09 00:26 | 1 | Solo | 9 | 10 / 5 | 11 + 3 | 8 | 2 | 85 | 4 | 78 | 127 | 14.1 | 9.0 |
| 2026-10-09 03:28 | 1 | Duos | 12 | 37 / 28 | 15 + 14 | 21 | 1 | 230 | 4 | 117 | 193 | 25.3 | 7.6 |
| 2026-10-09 03:28 | 2 | Duos | 14 | 23 / 2 | 19 + 10 | 37 | 1 | 175 | 4 | 167 | 293 | 31.9 | 9.2 |
| 2026-10-09 03:28 | 3 | Duos | 8 | 8 / 4 | 9 + 6 | 14 | 1 | 71 | 4 | 79 | 105 | 12.7 | 8.2 |
| 2026-10-09 03:28 | 4 | Duos | 13 | 31 / 4 | 18 + 12 | 17 | 0 | 165 | 4 | 125 | 246 | 28.1 | 8.8 |
| 2026-10-10 04:02 | 1 | Duos | 12 | 41 / 12 | 21 + 7 | 55 | 2 | 292 | 5 | 158 | 299 | 23.9 | 12.5 |
| 2026-10-10 04:02 | 2 | Duos | 11 | 32 / 8 | 29 + 11 | 51 | 0 | 159 | 4 | 151 | 332 | 21.3 | 15.6 |
| 2026-10-10 14:58 | 1 | Solo | 10 | 21 / 8 | 13 + 5 | 17 | 2 | 145 | 5 | 117 | 173 | 16.2 | 10.7 |

Actions = `SendOption` lines + `SendChoices` GENERAL lines of the game, exactly, in all 16 games. "Minutes" runs from the first shop turn to `STATE=COMPLETE`.

## Guessed or left out

- Why a roll was free (hero power, card, trinket): not read.
- Extra gold of a turn (`TEMP_RESOURCES`) is not added to "gold".
- `DECK_ACTION` blocks (a few per Duos game) are kept as "other": what they are was not checked.
- Tavern spells offered in the shop are not listed; spells bought are counted.
- APM counts what the client sends. A drag that the game refuses, or a hover, is not an action in the log.

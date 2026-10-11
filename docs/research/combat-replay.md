# Combat replay from the log (T-215)

Session 086, parser revision 8, decision D-099. What the game's `Power.log` says about each combat it plays back on screen, and how the parser turns it into a replay. Counts only: no names, BattleTags or account ids were printed or kept.

## 1. What the log has

A combat turn (even `TURN`) starts with the game making copies of both boards (`FULL_ENTITY` minions in `ZONE=PLAY`, controlled by the local player and by the shop's "dummy" player that stands in for the opponent). Then, at the top level of the log:

| Log shape | Meaning | Read as |
|-----------|---------|---------|
| `BLOCK_START BlockType=TRIGGER` from a minion, hero power or trinket, before the first attack | start-of-combat effect | `trigger` `start_of_combat` |
| `BLOCK_START BlockType=ATTACK Entity=<minion>` | an attack | `attack` |
| `TAG_CHANGE Entity=GameEntity tag=PROPOSED_DEFENDER` inside it | the defender | `attack.defender` |
| `TAG_CHANGE … tag=DAMAGE` going up on a board minion | damage taken | `damage` (amount, health left) |
| `tag=DIVINE_SHIELD` 1 → 0 (or 0 → 1) | shield popped (or given) | `shield` |
| `tag=ATK` / `tag=HEALTH` changes, `DAMAGE` going down | buffs and heals | `stats` (one per minion for a run of changes in a row) |
| `tag=ZONE value=GRAVEYARD` on a board minion | death | `death` |
| a minion made this combat (`FULL_ENTITY`, `SHOW_ENTITY`, or `SETASIDE` → `PLAY`) entering a board | summon | `summon` (card, stats, slot) |
| `BLOCK_START BlockType=TRIGGER … TriggerKeyword=DEATHRATTLE` / `AVENGE` / `BACON_RALLY` | the trigger the game names | `trigger` |
| `HERO_ENTITY` of a side changing mid-combat (Duos) | the teammate's board comes in; its minions may be the player's own shop minions moved to `PLAY` | `board_in`, then `summon`s |
| `BLOCK_START BlockType=ATTACK Entity=<hero>` with the hero's `ATK` | the end-of-combat hero hit | `hero_damage` |

After the hero hit (or, in a tie, after the last death) the game cleans up in the same turn: copies go to `REMOVEDFROMGAME`, the shop minions come back to `PLAY`, and in Duos the heroes switch back. None of it is a combat event: the replay stops at the hero hit, only minions made during the combat (or a Duos board coming in) count as summons, and a hero switch with no fight after it is dropped.

The fixtures made by `tools/make_fixture.py` keep top-level blocks only, so nested triggers (rallies inside an attack) and `PROPOSED_DEFENDER` are not in them; there the defender is the first enemy minion the attack damages or strips of its shield, as for `Round::attacks` (T-603b).

## 2. Checked on the user's logs

The six sessions on this PC on 2026-10-12 (8 Duos games, build 253216) and the two real fixtures (one Solo, one Duos), parsed with the new code and checked by a throwaway script that printed counts only:

| Check | Result |
|-------|--------|
| Combats with a replay | 84 of 84 in the logs (Duos), 17 of 17 in the fixtures |
| Attacks with their defender named | 1308 of 1308 |
| Attacker and defender on the board, on opposite sides | all |
| Damage events whose health left adds up | all |
| A death or summon of a minion not on the board | none |
| Won: no enemy minion left; lost: none of yours left | 54 of 54 and 27 of 27 (Duos), 5 and 3 of 3 (Solo fixture) |
| Hero hit on the loser's side | 81 of 81 |
| Hero hit equal to the player's health drop | 21 of 24 (the 3 others: the player died with less health left, or something else changed health the same turn) |
| Triggers named | 452 deathrattle, 273 avenge, 208 rally, 115 start of combat |
| Duos boards coming in | 138 |

Deaths with health left above 0 (36) are minions killed by Venomous or by an effect that destroys, which the log does not show as damage.

## 3. Size

The largest game's record is 196 KB compact (19.5 KB gzipped), 82 KB of it without the replays. upload-game reads at most 64 KB gzipped (unchanged) and now inflates up to 1 MiB (was 512 KiB), as do the sweep and the website's reader. Bounds per combat: 200 minions and 2000 events (the longest combat seen had 466 events and 62 minions).

## 4. Not done

- The website's replay (T-536): the record is uploaded and checked, the page is a later task.
- `CHANGE_ENTITY` of a board minion during a combat (a minion that turns into another card) keeps its first card in the replay.
- Damage to heroes during the combat (not at the end) and hero power effects other than start-of-combat triggers are not events.

# Parser prototype with hslog (T-003)

Date: 2026-10-08. Prototype: [`tools/parse_bg.py`](../../tools/parse_bg.py). Tests: [`tests/test_parse_bg.py`](../../tests/test_parse_bg.py) (synthetic logs).

## Summary

- **A Battlegrounds game can be rebuilt from `Power.log` alone.** Own hero and teammate's hero, lobby with heroes and teams, health per round, opponent of each combat, boards at the start of each combat and final placement all come from the log, with no memory reading.
- Tested with the user's **23 real games** (6 sessions, build 253216): 22 Duos (`GT_BATTLEGROUNDS_DUO`) and 1 Solo (`GT_BATTLEGROUNDS`). 23 of 23 in `ok` state, 8-player lobby in all, no repeated placements (in Duos, 1-4 by pairs), 0 unidentified opponents.
- When eliminated, the game **copies the local player's hero** with an old placement and reset health. Until session 005 the prototype read the copy: final health 30 in the 13 lost games (12 Duos and the Solo one) and wrong placement in 5 (see "Local player elimination", below).
- **Not in the log:** MMR and the exact list of lobby tribes. Tribes can only be inferred from what the tavern offers; `CARDRACE` counts alone are not reliable (see below), the card data's full tribe lists make them exact in practice ([lobby-tribes-log.md](lobby-tribes-log.md), T-310).
- In Duos, **some of the teammate's combats are not visible** in the local log: 141 of 1145 combat entries (12 %). The prototype marks them "not visible" instead of showing them empty.
- `hslog` fails on logs with several Duos games (`InconsistentPlayerIdError`). Avoided with a new parser per game.
- Performance: 15 s for the largest log (563 MB, 8 games), processing one game at a time.

## Dependencies

| Package | Version | License | Latest release | Notes |
|---------|---------|---------|----------------|-------|
| `hslog` | 1.20.0 | MIT | 2026-08-15 | HearthSim `Power.log` parser. |
| `hearthstone` | 9.21.1 | MIT | 2026-09-23 | Enums (`GameTag`, `Race`, `GameType`). Already knows `GT_BATTLEGROUNDS_DUO` and the Duos `BACON_*` tags; a few new tags appear only as a number (e.g. `4901`), with no effect on the prototype. |

They pull in `aniso8601`, `requests`, `urllib3`, `certifi`, `idna` and `charset-normalizer` (licenses in [`PROVENANCE.md`](../../PROVENANCE.md)). The prototype makes no network requests; `requests` is only used by `hearthstone` to download the card database, which is not used here.

Versions are pinned with hashes in [`requirements.txt`](../../requirements.txt) (pure Python wheels only, `--require-hashes`).

## What comes out of the log

Evidence: game of 2026-10-02 (`Hearthstone_2026_10_02_15_44_55/Power_old.log`, 1 game). Lines are approximate and refer to that file.

| Data | Available? | How | Evidence |
|------|------------|-----|----------|
| Game type and build | Yes | `GameState.DebugPrintGame()`: `GameType=`, `BuildNumber=` | lines 250-251 |
| Local player and "shop" player | Yes | `CREATE_GAME` has two `Player` entries: the local one and a dummy one (`BACON_DUMMY_PLAYER=1`, `lo=0`) that controls Bob, the shop and the opponents in combat | lines 2-97 |
| Own hero | Yes | Leaderboard hero (`CARDTYPE=HERO` with `PLAYER_LEADERBOARD_PLACE`) whose `PLAYER_ID` is the local player's | line 871 onward |
| Teammate in Duos | Yes | `BACON_DUO_TEAMMATE_PLAYER_ID` on the local player entity → leaderboard hero with that `PLAYER_ID` | line 72 |
| Lobby: heroes, team, tavern tier | Yes | The 8 leaderboard heroes: `PLAYER_ID`, `BACON_DUO_TEAM_ID`, `PLAYER_TECH_LEVEL`. The own hero has no `BACON_DUO_TEAM_ID`; it is on the player entity | lines 75, 871 |
| Health per round (own and each opponent's) | Yes | `HEALTH + ARMOR − DAMAGE` of each leaderboard hero when the combat closes, with a minimum of 0 (the final hit can overshoot: `DAMAGE` 33 with `HEALTH` 30). It can go up (healing, armor) | 281 rounds, none without data |
| Final placement | Yes | `PLAYER_LEADERBOARD_PLACE` of the own hero with the game in `STATE=COMPLETE`. In Duos it is the team's placement (1-4). If the player was eliminated, the copy of their hero must be ignored (below) | line 910647 |
| Rounds | Yes | `TURN` of `GameEntity`: odd = tavern, even = combat; round = `TURN // 2`. `TURN` 0 is the hero pick | — |
| Opponent of each combat | Yes | In combat, the dummy player changes its `HERO_ENTITY` to a copy of the opponent's hero; it is identified by the `card_id` of the lobby hero and, if that does not match (ghosts of eliminated players, e.g. `TB_BaconShop_HERO_KelThuzad`), by `BACON_CURRENT_COMBAT_PLAYER_ID` | line 7880 |
| Who fights on your side (you or your teammate) | Yes | The local player changes its `HERO_ENTITY` to the copy of the teammate's hero and back to its own | — |
| Own and opponent board at combat start | Yes, except hidden combats | Minions in `ZONE=PLAY` on each side in the first `BLOCK_START BlockType=ATTACK` block after the hero change: `card_id`, `ATK`, `HEALTH − DAMAGE`, `ZONE_POSITION` | line 9044 (first attack; 257 in the game) |
| Golden minion | Yes | By `card_id` (`…_G` or `TB_BaconUps_…`). **The `PREMIUM` tag is no use**: it is cosmetic (it appears on freshly bought tier 1 minions) | 895 golden minions seen in the 23 games |
| Lobby tribes | Inferred only | `CARDRACE` of the shop minions (`IS_BACON_POOL_MINION=1`, controlled by the dummy player in the tavern turn) | lines 2573, 2580 |
| MMR | **No** | It does not appear in `Power.log` (search for `MMR`, `Rating`: no results). See T-004 | — |

### Combats that are not visible (Duos)

Before each combat the game creates in `SETASIDE` a copy of all participants (hero, trinkets, minions), with their `card_id` visible. Afterwards, some of the teammate's combats are not played back: the opposing minions are hidden (`HIDE_ENTITY`, line 5309 onward) and moved to `ZONE=HAND`, and there is no `BLOCK_START BlockType=ATTACK`.

- The prototype **only uses boards that enter play** and takes them at the first attack. It does not use the `SETASIDE` copies, even though they have the `card_id`s, so as not to show something the player does not see on screen (rule 3 of `CLAUDE.md`).
- A combat with no attack at all ends up with `board = null` ("not visible in the log"), never as an empty board.
- In the 23 games: 1145 combat entries, 141 not visible.

### Local player elimination

Session 005 (2026-10-09). Evidence: Solo game `Hearthstone_2026_10_09_00_26_56/Power_old.log` (22 MB, 1 game; approximate lines) and the 22 Duos games. No names: entities by number.

Sequence when the local player dies (same in Solo and Duos):

1. The final hit leaves the own leaderboard hero with more damage than health: `DAMAGE=33` with `HEALTH=30`, `ARMOR=0` (line 159936). In Duos, between 31 and 59 damage.
2. The game creates **a copy of the hero** (new `FULL_ENTITY`, line 159967) and gives it `PLAYER_LEADERBOARD_PLACE` with the placement it had at that moment (7), the same `PLAYER_ID`, `DAMAGE=33`, `COPIED_FROM_ENTITY_ID` = the original hero, and then `DAMAGE=0` (lines 159988-160005). It stays in `SETASIDE`.
3. The original moves to `ZONE=GRAVEYARD` and the player to `PLAYSTATE=LOSING` and then `LOST` (lines 160010-162403).
4. The other heroes get their placement at that moment (lines 162417-162427) and, right before `STATE=COMPLETE`, **the original gets the final placement**: 8 (lines 164051-164059). The copy keeps the old one.

The two failures in the first Solo game came from there: `leaderboard_heroes()` kept the last entity per `PLAYER_ID`, which was the copy. Hence health 30 (copy with `DAMAGE=0`) and placement 7 (old), repeated with that of the player who really finished 7th. The local player finished **8th** (first eliminated; the other 7 still had health > 0), not 7th.

In Duos the same happens when the local team falls: 12 of 22 games have the copy (all the non-won ones) and in 4 the copy's placement differed from the final one (2 instead of 3 three times, 1 instead of 2 once). With the copy, the local player got a different placement from their teammate; with the original, placements come out by pairs in all 22.

How to tell the copy: `COPIED_FROM_ENTITY_ID` points to another leaderboard hero. **Having `COPIED_FROM_ENTITY_ID` is not enough**: the opponents' leaderboard heroes are also copies (in Solo and Duos), but of entities that are not leaderboard heroes. The prototype discards only copies of another leaderboard hero.

Eliminated opponents do not generate a copy (0 in the 23 games). Their health used to come out negative (down to −36); now it is 0.

### Tribes: why the shop is not enough

Counting the minions offered per game, only in 17 of 23 games do exactly 5 tribes appear with 5 or more appearances. In the rest, tribes with 1-4 appearances show up that are not in the lobby or that might be (effects that generate minions of other tribes, dual-tribe minions with a single `CARDRACE`). The prototype gives the counts as they are, labeled as inferred. The product needs another source (another log, another tag) or to show it as "probable".

**Update (T-310, 2026-10-10):** no log has the list (checked tag by tag, [lobby-tribes-log.md](lobby-tribes-log.md)). With the card data, a single-tribe minion offered by Bob proves its tribe: exactly 5 tribes in each of the 18 games with a shop record; the dual-tribe minions explain the extra `CARDRACE` tribes.

## Failures and limits found

| Finding | Impact | What the prototype does |
|---------|--------|-------------------------|
| `hslog` raises `InconsistentPlayerIdError` in the 2nd game of a log with several Duos games: it keeps player state for the whole file and the same name comes back with a different `PlayerID` | 4 of 6 logs could not be read | Splits the log at each `CREATE_GAME` and uses a new `LogParser` per game. Also, a broken game does not hide the following ones |
| `hslog` treats only `GT_BATTLEGROUNDS` as Battlegrounds in its name heuristic (`player.py`), not the Duos variants | None seen with a per-game parser | Nothing; watch when upgrading |
| Thousands of `Broken option nesting` warnings | Noise | Silenced (`ERROR` level) |
| New unnamed tags in `hearthstone` 9.21.1 | None for this data | Nothing |
| `hslog` exceptions quote the log line, which may contain BattleTags | Names leaking in errors | Only the exception type is reported |

## Prototype guarantees

- **Nothing made up.** If something the report needs is missing (local player, leaderboard hero, reasonable lobby size) or `hslog` fails, the game comes out as `unsupported` with no data. A game without `STATE=COMPLETE` comes out as `incomplete` and with no placement.
- **Untested build** (≠ 253216) or **untested game type** (anything except `GT_BATTLEGROUNDS` and `GT_BATTLEGROUNDS_DUO`): it is processed, but with a warning.
- **Privacy:** the output only carries `card_id`, lobby player numbers (1-8) and numbers. Checked on the JSON output of the 23 games: no BattleTags, no `PlayerName`, no `GameAccountId`.
- Read-only access to the log file. No network.

## What it means for the product

- **History and stats (F1):** viable now. Hero, teammate, placement, health per round, opponents.
- **Replays (F2):** viable with boards per combat. In Duos, the teammate's hidden combats will be shown as "not visible". Card names and images need the card database (HearthstoneJSON or another source with a compatible license), pending.
- **Overlay (F3):** the last board seen of each opponent comes from the log in real time if the file is followed while it grows. Lobby tribes, not reliably.
- **MMR:** not in `Power.log`; left for T-004 (other logs and public leaderboard).
- **Stack (P-002):** the log format is well understood and the prototype fits in a file under 500 lines. Porting it to C# or Rust is viable without depending on `hslog`, writing it from this report and not from other trackers' code. Python + `hslog` is fine for prototypes and tests.

## Pending

- More Solo games: there is only one real one, in which the local player falls first. Still to see: a won Solo game and one with opponents eliminated before the local player (covered only with synthetic logs).
- **Placement of opponents still alive** when the local player falls: the log gives them their placement at that moment, not the final one (the game goes on without the player). The prototype gives it as `final_place`, same as for those already eliminated. They need to be told apart (e.g. health > 0 at the end) and their placement marked as unknown or "placement at exit".
- Trimmed fixtures from own logs, with no third-party names, with the user's OK.
- Exact source of the lobby tribes: none in the log (T-310); worked out from the tavern with the card data.
- Reconnections mid-game: if the game writes a new `CREATE_GAME` for the same game, the prototype would count it as two. It has not happened in the 23 logs (the count matches `check_logs.py`), but it is untested.
- Spectator mode: untested.

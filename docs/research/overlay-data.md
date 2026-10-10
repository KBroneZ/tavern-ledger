# Overlay data: golden by PREMIUM, next opponent, leaderboard values, scenes, damage cap

Session 052, 2026-10-11. Tasks T-312 to T-315, T-208's overlay side and 049's follow-up (golden by `PREMIUM`). Decision: D-061. Parser revision 6, server check version 6.

## Method

- **Logs:** the user's own logs on this PC: 6 session folders (2026-10-09 to 2026-10-10, build 253216), **11 Battlegrounds games, 2 Solo and 9 Duos** (the games of [parser-round-2.md](parser-round-2.md)), each with its `Power.log` (or `Power_old.log`) and `LoadingScreen.log` (or `LoadingScreen_old.log`).
- **Reading:** read-only scripts in the session's scratch folder replayed the power lines with `hslog` (the prototype's own reader, `tools/parse_bg.py`) and printed counts, card ids and lobby seats (1 to 8) only; `LoadingScreen` lines were reduced to their mode names. No name, BattleTag or account id was printed or kept.
- **Card data:** the app's own cached copy of HearthstoneJSON (D-038) to name the card ids below.
- **Whether the game shows something:** Blizzard's own pages and patch notes, and two player guides; noted per item.

## Golden board minions by `PREMIUM` (049's follow-up)

At the first attack of each combat (the moment the parser takes the boards), every minion on both boards:

| Side | Golden card id and `PREMIUM` | `PREMIUM` only | Golden card id only | Neither |
|------|------------------------------|----------------|---------------------|---------|
| Own | 180 | 11 | 0 | 791 |
| Opponent | 154 | 19 | 0 | 825 |

The 30 with `PREMIUM` only are cards that are always golden or golden tokens, which have no golden card id: Aureate Laureate `BG32_236` (27), Red Chromadrake `BG34_638_Gt`, Green Chromadrake `BG34_636_Gt` and Helping Hand `BG25_010_Gt` (1 each). The same holds for every minion of every game at its last state (976 with both, 108 with `PREMIUM` only, all of them those four cards, 0 golden ids without it).

An older note ([parser-hslog.md](parser-hslog.md)) called `PREMIUM` cosmetic because it showed up on freshly bought minions. The logs it came from are gone, but the effect is visible here: `PREMIUM` is set to 1 and back to 0 **49 times** on minions without a golden id, always on the player's own minions in `SETASIDE` during a shop turn, for at most 5 tag changes (the copies set aside while a triple is made), and **never** on a minion in play at an attack. The parser only reads minions in play at the first attack, so it never meets them.

**Rule (parser revision 6):** a board minion is golden when it has `PREMIUM` or a golden card id. The Python prototype follows the same rule, so the parity tests still hold. Both real fixtures were made again with `PREMIUM` kept: the Solo game is unchanged, and in the Duos game exactly the two Aureate Laureates become golden; every other field is the same.

## T-313: the next opponent

`NEXT_OPPONENT_PLAYER_ID` (lobby seat) is set:

| On | When | Times |
|----|------|-------|
| the local leaderboard hero | once at each shop turn | 123 (every shop turn of the 11 games) |
| the local player entity | during combats | 114 |
| combat copies of heroes | during combats | 7 |

The value on the local leaderboard hero at the end of each shop turn, against the opponent met in the next combat (the hero that steps in on Bob's side):

| Mode | Names the hero met | Names the teammate of the hero met | Other |
|------|--------------------|-----------------------------------|-------|
| Solo | 19 | - | 0 |
| Duos | 102 | 2 | 0 |

So it is the game's own "next opponent" for the shop turn, and in Duos it names one hero of the next opposing team (the overlay marks the whole team). `NEXT_OPPONENT_TEAMMATE_PLAYER_ID` is set only during combats (97 times) and is not needed.

**Does the game show it?** Yes. Blizzard's launch post: "The status of all other players (including which opponent you will face off against next) will be visible at all times on the Leaderboard"; player guides describe the next opponent's portrait highlighted in red and pushed out of the column during the recruit phase. The overlay shows the same mark ("Next" in the shop; "Now" during that combat, since the game keeps its mark on the opponent being fought until the next shop turn) with that opponent's last board seen, which the player already saw in an earlier combat. Live only: it is not saved or uploaded.

## T-312: opponents' triples and trinkets

Already read by parser revision 5 (D-058, [parser-round-2.md](parser-round-2.md)): every lobby hero's `PLAYER_TRIPLES` and trinkets, as the leaderboard tooltip shows them, each change with its turn. The overlay shows, per hero, the newest count and the newest trinket of each slot (first, second, hero power). A count the log never gave is said in words ("no triple in the log yet"), never 0, because the game does not print a zero tag. Trinkets are database ids; the app names them with the card data (`CardStore::card_of_dbf`, session 051), and one the card data does not know shows its id, marked unknown.

## T-314: scenes from `LoadingScreen.log`

Lines read (only the mode names are kept):

```text
LoadingScreen.OnScenePreUnload() - prevMode=BACON nextMode=GAMEPLAY m_phase=INVALID
LoadingScreen.OnSceneLoaded() - prevMode=BACON currMode=GAMEPLAY
```

- Modes seen: `STARTUP`, `LOGIN`, `HUB` (main menu), `BACON` (Battlegrounds lobby), `GAMEPLAY`, `DRAFT`, `PACKOPENING`.
- **Every one of the 11 games** has a `BACON` to `GAMEPLAY` change: the pre-unload line 1.1 to 2.4 s before the game's `CREATE_GAME` in `Power.log`, the loaded line 0.3 to 0.6 s after it. Each game ends with a `GAMEPLAY` to `BACON` change (11 of 11) 3 to 16 s after the log's last `STATE=COMPLETE` line (the player leaving the end screen). So when `CREATE_GAME` arrives the scene is already "loading", and a game the log never closed is left as soon as that change comes.
- **Use:** the app's status says "In the main menu", "In the Battlegrounds lobby", "Battlegrounds game loading" or "In a game"; the overlay starts with the loading screen ("Battlegrounds game loading…"); and a game the log never closed (cut off without `STATE=COMPLETE`) ends as soon as the player leaves the game screen, instead of staying "on" until the next game (the known limit of D-029 and D-032). A game left that way stays over when the next game's loading screen comes.
- **Setup:** the file exists only when `log.config` has a `[LoadingScreen]` section (the user's has it). The app never writes `log.config` (D-004); without the section it works from `Power.log` alone, as before, and the setup box shows one line marked "Optional" with the two lines to add (`LogLevel=1`, `FilePrinting=true`). A read error leaves the scene unknown, never a stale one.
- **Known limit:** a reconnect that goes through another scene in the middle of a game would end the overlay's game early; not seen in the user's logs.

## T-315: combat damage cap (left out)

`BACON_COMBAT_DAMAGE_CAP` on the game entity: in the 2 Solo games 5 from the start, 10 from game turn 7 (shop turn 4) and 15 from game turn 15 (shop turn 8); in the 9 Duos games 15 the whole game. `BACON_COMBAT_DAMAGE_CAP_ENABLED` goes to 0 late (turns 22 to 26, 4 times). That matches Blizzard's rule: 5, then 10 on turn 4 and 15 on turn 8 in Solo since Season 9 (patch 31.2), a flat 15 in Duos, removed in the top 4.

**Left out:** neither Blizzard's patch notes nor the wiki say the game **shows** the cap on screen, and no source found describes an indicator. The task allows it only after checking that the game shows it, so the overlay does not show it. The hub asked the user to look in a game; if they see it on screen, it becomes a follow-up (the tags above are enough).

## T-208: keywords in the overlay

The boards already carry each minion's keywords (D-058). The hover card and the next-opponent block show them as short letters under each minion (T Taunt, DS Divine Shield, R Reborn, V Venomous, W Windfury, S Stealth, DR Deathrattle, Ra Rally), each line read out in full to screen readers, and the legend spells them out; golden keeps its star. Letters, never colour alone; the marks wrap inside the 76 px minion chips.

## Sources

- The user's own logs (6 session folders, 2026-10-09 to 2026-10-10).
- [Introducing Hearthstone Battlegrounds](https://hearthstone.blizzard.com/en-us/news/23156373) (Blizzard): the leaderboard shows who you face next.
- [Hearthstone Battlegrounds Mechanics Guide](https://www.icy-veins.com/hearthstone/hearthstone-battlegrounds-mechanics-guide) (Icy Veins) and [Battlegrounds](https://hearthstone.wiki.gg/wiki/Battlegrounds) (wiki): the next opponent highlighted on the leaderboard.
- [31.2 Patch Notes](https://news.blizzard.com/en-us/article/24163459/31-2-patch-notes) (Blizzard) and [Battlegrounds/Hero](https://hearthstone.wiki.gg/wiki/Battlegrounds/Hero) (wiki): the damage cap's rule; no word on showing it.
- [HearthstoneJSON](https://hearthstonejson.com/) card data (the app's cached copy), for card names.
- [log-inventory.md](log-inventory.md), [parser-round-2.md](parser-round-2.md), [parser-hslog.md](parser-hslog.md).

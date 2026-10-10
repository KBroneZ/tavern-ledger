# Parser round 2: leaderboard triples and trinkets, board keywords, season choices, discover picks

Session 049, 2026-10-10. Tasks T-207, T-208, T-211, T-212 and T-213 from the log inventory ([log-inventory.md](log-inventory.md), "Worth adding next"). Decision: D-058. Parser revision 5.

## Method

- **Logs:** the user's own `Power.log` and `Power_old.log` files on this PC (6 files, build 253216): **11 Battlegrounds games, 2 Solo and 9 Duos**, the same games as [parser-data-round.md](parser-data-round.md).
- **Reading:** read-only scripts in the session's scratch folder replayed the `GameState` power and choice lines (entities, tags, blocks, choices) and printed counts and card ids only. Player names were mapped to "local player" or "other" and never kept. (One early sample of raw lines printed a choice header's `Player=` field to the session's own terminal before the redaction covered it; it was not saved or sent anywhere, and the redaction was fixed.)
- **Card data:** one download of HearthstoneJSON's `cards.json` (the file the app already uses, D-038) to turn database ids into cards and compare keywords with the cards' own; not kept.
- **Cross-check:** the release build of `bg-parse` read every raw game after the change, and its new fields give the counts below; the two real fixtures pin their games' numbers in `crates/bg-parser/tests/real_games.rs`.

## T-207: opponents' triples and trinkets, as the leaderboard shows them

**What the log gives.** Tags on each lobby hero (the leaderboard's hero entities, in `SETASIDE` for opponents, in `PLAY` for the local hero):

| Tag | Changes on leaderboard heroes | On other heroes | Values |
|-----|-------------------------------|-----------------|--------|
| `PLAYER_TRIPLES` | 222 (220 in shop turns) | 7, on the copies made for a combat | 1 to 7 |
| `BACON_FIRST_TRINKET_DATABASE_ID` | 105 | 0 | trinket database ids |
| `BACON_SECOND_TRINKET_DATABASE_ID` | 79 | 0 | trinket database ids |
| `BACON_HEROPOWER_TRINKET_DATABASE_ID` | 1 | 0 | a trinket database id |

- **Triples:** on the local hero the count equals the triple rewards the player got (`TB_BaconShop_Triples_01` created in the player's hand in a shop turn) in **6 of 11 games, both Solo games among them**; in the other 5 (Duos) the player got more reward cards than triples (passed by the teammate or given by other cards), never fewer.
- **Trinkets:** all 185 values are trinket cards (`BATTLEGROUND_TRINKET`) in the card data. On the local hero the value equals the trinket the player took from the trinket offer **21 of 21 times**. A value set again later is an upgrade (18 times in 7 games), so each change is kept with its turn.
- **Kept:** per lobby hero, each change of the triple count and each trinket set, with its shop turn (`lobby[].triples`, `lobby[].trinkets`). Combat copies are not the leaderboard and are left out.
- **Left out:** `BACON_HERO_*_TRINKET_LEADERBOARD_SDN*` and `_ALT_TEXT` (counters and text variants in the tooltip; their meaning per trinket was not checked).

## T-208: keywords on the boards seen

At the first attack of each combat, the 1153 minions on the two boards (11 games), each keyword tag compared with the card's own keywords in the card data:

| Tag | Tag and card | Tag only (gained) | Card only |
|-----|--------------|-------------------|-----------|
| `TAUNT` | 40 | 73 | 0 |
| `DIVINE_SHIELD` | 104 | 66 | 1 |
| `REBORN` | 33 | 46 | 0 |
| `VENOMOUS` | 6 | 10 | 0 |
| `WINDFURY` | 29 | 32 | 0 |
| `STEALTH` | 4 | 8 | 0 |
| `DEATHRATTLE` | 190 | 27 | 0 |
| `BACON_RALLY` | 93 | 12 | 0 |

The tags are the minion's state as it enters the combat: they hold every keyword the card has (the one divine shield missing had lost it before the first attack) plus the ones it gained, which is what the board shows. `DIVINE_SHIELD` is 3 on three minions (more than one shield); only "has it" is kept. `POISONOUS`, `MEGA_WINDFURY` and `CLEAVE` never appear. **Kept:** `keywords` on every board minion (`taunt`, `divine_shield`, `reborn`, `venomous`, `windfury`, `stealth`, `deathrattle`, `rally`). Over every board the parser keeps (each leg of a Duos combat), the 1980 minions carry 377 deathrattle, 292 divine shield, 203 taunt, 169 rally, 147 reborn, 96 windfury, 37 venomous and 18 stealth.

**Golden.** `golden` stays the card-id rule: `PREMIUM` is on all 188 minions with a golden card id and on 20 more that are golden without one (18 Aureate Laureate, a minion that is always golden, and 2 golden tokens `_Gt`). Switching `golden` to `PREMIUM` would fix those 20 but change an earlier field and the Python prototype's parity; it is a follow-up.

## T-212: discover picks

- `DebugPrintEntityChoices` with `ChoiceType=GENERAL`: **231 choices** in 11 games, all the local player's own (the header names the player), all in shop turns, all `CountMin=1 CountMax=1`, with 2 (2 choices), 3 (207), 4 (21) or 5 (1) cards, every card's id in the log.
- `DebugPrintEntitiesChosen` with the same id: **231 of 231** answered, always with one of the offered cards. (`SendChoices` lines: 230; one game has one choice more than it sent.)
- Sources: triple rewards 49, Dark Discovery 35, `BG30_MagicItem_416t` 18, `BG28_550` 17, `BG26_525` 15, the first trinket offer 11, `BG36_342` 11, the second trinket offer 10, and 24 other cards and hero powers.
- **Kept:** `discovers`, one per choice: the shop turn, the source card, each offered card and the index of what was taken (empty when the log does not say, never a guess). The parser keeps only cards controlled by the player.

## T-211: season choices

| Item | What the log gives | Kept |
|------|--------------------|------|
| Trinket offers and own trinkets | the trinket offers are choices whose source is `BG30_Trinket_1st` or `_2nd` (21 choices, 4 trinkets each); the trinket taken is the leaderboard's value every time (21 of 21) | yes: in `discovers`, and the own trinkets in `lobby[].trinkets` |
| Dark Gifts | the Dark Discovery choices (source `BG36_MidGameEffect_010`, 35 choices, 105 minions): every offered minion's `DARK_GIFT_ENTITY` names the gift's card (105 of 105). The button (`BG36_Button_DarkGift`) was pressed 30 times; the other 5 choices came another way (not checked which card) | yes: each offer's `gift` in `discovers` |
| Quests and rewards | one game only: a hero-power quest choice (`BG24_QuestsPlayerEnch_t`, two quests, each with `QUEST_REWARD_DATABASE_ID`), completed on turn 4 (`BACON_QUEST_COMPLETED`) | the choice only (in `discovers`); reward and completion **left out**: one game is too little to check them |
| The Old God | `BACON_GLOBAL_OLD_GOD_DBID` on the game entity at the start, in 11 of 11 games (C'Thun 8, Y'Shaarj 3); in 7 games the player's own "Secret Deity" (`BG_OldGod`, in the player's secret zone) names the same Old God, the other 4 have no such entity | yes: `old_god` (database id) |
| Timewarped buys | no bought card carried `BACON_TIMEWARPED`; a Timewarped tavern was open in one game only (`BACON_ALT_TAVERN_IN_PROGRESS`) | **left out**: how a Timewarped buy shows in the log is not clear from one game |

## T-213: Duos combats the game does not play back (left out)

Before each combat the game copies every participant (hero, hero power, trinkets, minions) into `SETASIDE`, controlled by the local player. For a player who steps into the fight, some of those minion copies are moved to their `HAND` (261 copies in 125 groups) and the rest become the board; **no tag separates the two** (no tag differs by more than 30% between the two kinds), and the hand copies always come first in the group (125 of 125), but for a player who never steps in nothing says how many are hand. 15 of the 18 opposing groups that never step in hold more than 7 minions, so hand cards are certainly among them. Showing those copies as a "board" would be false, and an opponent's hand goes beyond D-054, which allows boards only. **The user chose to leave T-213 out for now** (2026-10-10, through the hub).

Also seen: in a Duos relay, a second opponent can step in without a hero switch on Bob's side (their minions just enter play after the first opponent's die); the parser then keeps no board for that opponent in that round. That is older behaviour and is left for the combat replay (T-215).

## Fixtures

`tools/make_fixture.py` now also keeps the tags above, spells, tavern spells, trinkets and quest rewards (the cards a choice offers), and every `GENERAL` choice with its source, rewritten to ids (`id=2 ChoiceType=GENERAL`, `Source=812`, `Entities[0]=903`). Both real fixtures were made again from the same games: every earlier field is unchanged, and the Rust parser reads the fixture and the raw game the same. New synthetic games `solo_round_two_game` and `duo_round_two_game` (`tests/bg_log_builder.py`) cover each new field; the Python prototype does not read them (the parity tests drop them).

## Sources

- The user's own logs (6 session folders, 2026-10-09 to 2026-10-10).
- [HearthstoneJSON](https://hearthstonejson.com/) card data (`cards.json`, latest), for database ids, card types and keywords.
- [log-inventory.md](log-inventory.md), [parser-hslog.md](parser-hslog.md), [parser-data-round.md](parser-data-round.md).

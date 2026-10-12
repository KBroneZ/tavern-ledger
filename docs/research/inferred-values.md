# What the app infers, and what memory could read instead (T-535)

Session 091, 2026-10-12. Build 253216. **Research only:** no code changed, no memory tool run, nothing read from HearthMirror, HDT, HSTracker, Firestone, BobsBuddy or unity-spy (rule 1). Tier R3 for the decision it prepares (D-104, for the user).

The user's ask (2026-10-12): "there are many 'inferred' things in the app that we could now get from memory; fix it." Today D-063 allows exactly two memory values (the lobby's five tribes, the player's own rating), read-only, off by default, never uploaded. Anything more needs the user's decision, and rule 3 still holds: never information the player could not see on their own screen. So each value is checked one by one below.

## Answers

1. **Most "inferred" values are not memory problems.** Of the 15 values the app or the website marks as inferred, unknown or not recorded, **3 can be fixed from the log or the card data** (better than memory, always preferred), **5 come from old saved games whose logs are gone** (nothing can fix them, memory included), **3 are worth leaving as they are**, **1 is not on the player's screen** (never), and **1 is already read from memory** (D-063). (Sections 2 and 3.)
2. **Two values are real memory candidates**, both shown on the player's screen and missing from the log: **each opponent's most common minion type and count** (the game's leaderboard hover) and, in Duos, **the teammate's current board** (the game's portal). Neither is a small, safe read like D-063's; each needs a new path found on the user's client, and the teammate's board means reading game entities, which D-063 ruled out. (Section 4.)
3. **One gap needs no new memory value:** the tribes D-063 already reads are used live only; the recap and the stats still show the tavern-inferred tribes after the game, because the read is not kept with the game. Keeping it on the PC needs a small amendment to how D-063 stores values, not a new read. (Section 4.)
4. **One labelling bug** turned up on the way: the overlay marks an opponent's record "inferred" even when every result came from the log. (Section 5.)
5. **Questions for D-104** are in section 6, one at a time, each with a recommendation; a draft of D-104 follows.

## 1. Method

- **Code:** `crates/tracker/src` (the `Source` labels in `provenance.rs` and every place that sets one), `crates/bg-parser/src` (`report.rs`: `not_in_log`), `crates/desktop/ui` and its strings, `web/src` (data, game and simulator strings). Decisions D-010, D-019, D-028, D-050, D-054, D-055, D-057, D-058, D-063, D-066, D-069, D-072, D-080, D-084, D-087. Research: [lobby-tribes-exact.md](lobby-tribes-exact.md), [lobby-tribes-log.md](lobby-tribes-log.md), [memory-reader.md](memory-reader.md), [log-inventory.md](log-inventory.md), [overlay-hover.md](overlay-hover.md), [parser-data-round.md](parser-data-round.md), [combat-simulator.md](combat-simulator.md).
- **Counts:** the user's saved history (`%APPDATA%\TavernLedger\games.jsonl`, latest record per game; the backups left aside), by a scratch script kept outside the repo that prints counts only. **41 games** (2 Solo, 39 Duos; 40 finished, 1 incomplete), **468 combats**, saved by parser revisions 2 (10 games), 3 (8), 5 (6), 6 (16) and 7 (1).
- **Raw logs left on disk:** 6 sessions (2026-10-11, 02:56 to 06:21), all already saved by revision 6 or 7. The 18 games saved by revisions 2 and 3 (2026-10-04 to 10-08) have no log any more, so `tavern-watch --reparse` cannot fill them.
- **Card data:** HearthstoneJSON's `enUS` `cards.json` (the same source as D-038), fetched once into the scratch folder for one check (section 3, skins).
- **Live-only values** (the overlay's guesses, the leaderboard's order, odds) are not saved with a game, so the history cannot count them; they say so below instead of a number.

To recheck the counts: per game key (`session`, `index`) keep the last line of `games.jsonl`; a round's result is "from the log" when it has `result`, else judge it by D-028's health rule with the team's shared health in Duos (the opponents of a Duos round are one team); a board entry with `board` null is a combat the log does not replay; a shop turn's `free_refreshes` null is unknown; a skin is linked when its id is in `skin_parents`.

## 2. Inventory

Where: **W** main window (recap, Your games, stats, curious stats), **O** overlay, **S** website. Gap: **log** = the log lacks it, **old** = saved by an older parser and the log is gone, **rule** = a rule of ours, **card** = the card data lacks it.

| # | Value | Where (code) | Label | Why it is not exact | In the user's history | How wrong it can be |
|---|---|---|---|---|---|---|
| 1 | Lobby tribes (the exact five) | O lobby block (`possible.rs:159-201`), W recap "seen in the tavern" (`recap.rs:628`), W stats (`stats.rs:389`) | from the game (switch on) · entered · inferred · unknown | log: no list (D-050, [lobby-tribes-exact.md](lobby-tribes-exact.md) 1). Inferred from single-tribe pool minions Bob offers. | 41 of 41 saved games show inferred tribes (the D-063 read is live only, not kept). 31 games have a shop record: all five confirmed in each, on shop turn 2 (3 games), 3 (3), 4 (5), 5 (6), 6 (5), 7 (6), 8 (2), 10 (1). 10 old games: `CARDRACE` counts only. | Rule held 31 of 31; missing tribes until offered (none at hero select). `CARDRACE` counts alone show tribes outside the lobby. |
| 2 | Combat result (won, lost, tie) | W recap chips and record (`recap.rs:312-345`), W stats, curious, S (`game/shape.ts`) | from the log · inferred · unknown | old: revisions 2 and 3 did not read the game's result tags (D-055), so D-028's health rule. | 468 combats: 251 from the log (every one of revision 5 and later), 176 inferred, 41 unknown, all in the 18 old games. | Unknown when a side healed or gained armor, was already out, or both lost health; a tie can hide a hit a hero power prevented (D-028). |
| 3 | Record against each opponent | W recap (`recap.rs:426-436`), O hover (`live.rs:431`) | from the log · inferred · unknown | follows #2. | Inferred in the 18 old games. | Follows #2; see section 5 for the overlay label. |
| 4 | Longest win streak | W curious (`stats/curious.rs:312-326`) | from the log · inferred | follows #2. | Inferred when the streak is in an old game. | A tie, a loss, an unknown result or a missing round ends it. |
| 5 | Damage per combat; health change per turn | W curious "most damage in one combat" (`curious.rs:327-341`, always inferred), W turns chart (`turns.js:176-186`), S damage dealt and taken (`game/shape.ts:206-228`, always inferred) | inferred | rule: health before minus after. The game's own end-of-combat hero attack is in the log, but the saved report keeps no damage number (D-055 left `DAMAGE_DEALT_TO_HERO_LAST_TURN` out; D-084 keeps attacks out of the report). | All 468 combats. | Health paid or healed in the shop turn before is counted in; the last combat has no health after it. |
| 6 | Hero grouping of skins | W stats per hero (`stats.rs:173-189, 336-340`) | from the log · inferred | old/rule: before revision 4 there is no `BACON_SKIN_PARENT_ID` link; the fallback drops `_SKIN_…` from the id (D-019). | Own hero on a skin in 26 games: 16 linked by the log, 10 by the rule. Over every lobby hero: 71 skin ids linked, 48 by the rule. | The rule disagrees with the card data's parent on 2 of 48 (and the log's link agrees with the card data 71 of 71, section 3). |
| 7 | Hero pick, choices (discovers, trinkets, Dark Gifts), Old God, keywords, triples and trinkets per hero | W recap, stats, curious; O hover (`ov.shown.none`) | not recorded | old: kept from revisions 4 and 5 on (D-055, D-058). | Recorded in 23 of 41 games; "not recorded" in the 18 old ones. | Never guessed: "not recorded". |
| 8 | Shop record (gold, rolls, buys, APM) | W shop, stats, curious | not recorded · "N of M turns" | old: kept from revision 3 on (D-046). | 31 of 41 games; 2 of 345 turns without gold numbers. | Never guessed (D-066 sums known turns). |
| 9 | Free rolls from the roll button | W shop extras, curious | unknown | log: `BACON_FREE_REFRESH_COUNT` is only written in games where free rolls exist ([log-inventory.md](log-inventory.md): seen in 10 games, turn 8 on), so a game without it cannot tell 0 from unknown (D-055). | Unknown on 183 of 345 shop turns: 94 in 8 old games, 89 in 9 later games whose log never sets the count. | The total of free rolls (no gold spent) is exact; only the "why" is missing. |
| 10 | Combat boards the log does not replay | W turns ("board not in the log"), S viewer | not visible | not shown: Duos fights of the teammate's board that the game does not play back (D-010, D-054; T-213 left out by D-058). | 221 of 1 876 board entries (200 on the player's side, 21 on the opponents'). | Not shown in game; the hidden `SETASIDE` copies mix boards (D-058). |
| 11 | Possible minions of an opponent not met yet | O hover (`possible.rs:203-267`) | possible | log: the opponent's board is not shown before a combat (rule 3); the game's per-player tribe hint is not in the log ([lobby-tribes-log.md](lobby-tribes-log.md)). | Live only. | A list of 30 to 45 pool minions of their tier and the one below; they may have none of them. |
| 12 | Opponent's tavern tier "Tier ?" and leaderboard order | O hover (`live.rs:233-275`, `ov.board.tierUnknown`, `ov.opponents.hintNoOrder`) | unknown | log: `PLAYER_TECH_LEVEL` and `PLAYER_LEADERBOARD_PLACE` are not set yet early in a game. | Live only, first turns. | Unknown until the log sets them, then exact. |
| 13 | Which Duos hero is the top portrait | O hover (D-080) | rule (no mark) | log: not in it; the rule "higher player id on top" came from the user's screenshots. | Live only. | The rule fit all of the user's checks (D-080). |
| 14 | Combat odds: approximate cards, Duos teammate board | O odds (`odds.rs`, `odds_duos.rs`), S simulator | approximate · no number | rule: cards with known gaps (D-084, D-097). Duos: the teammate's current shop board is not in the log, so their last board seen in a combat is used (D-087). | Live only; the simulator's accuracy is on `/accuracy/` (D-091). | Approximate cards are marked; the teammate's board may be a turn or more old. |
| 15 | When a game was played | W recap subtitle (`win.recap.subTip`) | (note) | rule: the window shows the log session's start; the log does give per-game clock times (`game_clock.rs`, the `played` span). | 31 of 41 games have a `played` span saved. | A session with several games shows one start time. |
| — | Own rating and its change | O lobby panel, W footer | from the game · unknown | log: no MMR ([mmr-sources.md](mmr-sources.md)); read from memory under D-063 and D-069. | Not saved per game (D-063). | Exact when read; the change only when one game lies between two reads (D-069). |
| — | Other players' ratings, names, hands and current boards | nowhere | — | Not on the player's screen, or other people's data. | — | Never (D-004, D-063, rule 6). |

`not_in_log` in every saved report (`bg-parser/src/report.rs:16`), verbatim: `MMR`, `available tribes (exact list)`.

## 3. Each value: on screen? log? memory?

(a) does the game show it on the player's screen, and how we know; (b) can the log or the card data give it (always preferred); (c) could a read-only memory read through public Mono structures, as D-063, reach it, and how stable it would be. Clean room for (c): only our own `crates/game-memory` design and [memory-reader.md](memory-reader.md); where the location is not known to us, it says so rather than guessing.

| # | (a) On screen | (b) Log or card data | (c) Memory | Verdict |
|---|---|---|---|---|
| 1 Lobby tribes | Yes, at hero select and in the tavern guide ([lobby-tribes-exact.md](lobby-tribes-exact.md) 2). | No ([lobby-tribes-exact.md](lobby-tribes-exact.md) 1). | **Already read** (D-063), live only. | Keep D-063; **keep the read with the game on the PC** so the recap and stats use it (question 1). |
| 2 Combat result | Yes (the combat plays out). | **Yes from revision 4 on**; the 18 old games' logs are gone. | Not needed. | Leave as inferred for old games. |
| 3 Record | Yes. | Follows #2. | Not needed. | Leave; fix the overlay label (section 5). |
| 4 Win streak | Yes. | Follows #2. | Not needed. | Leave. |
| 5 Damage per combat | Yes: the winner's hero attacks the loser at the end of every combat, with the damage shown. | **Yes:** the end-of-combat hero attack is in the log, and D-087 matched the game's attack on 81 of 81 Duos combats (36 at the cap); the parser already reads attacks (D-084) but does not save them. | Not needed. | **Fix by log**: save each combat's damage (one number per combat) and use it instead of the health difference. Needs a parser revision and an upload key, so the hub's call (D-084 left the report unchanged for that reason). |
| 6 Skin grouping | Yes (the hero portrait). | **Yes, card data:** HearthstoneJSON's `battlegroundsSkinParentId` is on all 653 Battlegrounds skin heroes; it equals the log's own link on 71 of 71 skins in the history, and differs from the D-019 rule on 2 of 48. The website already uses it (`tools/hero_names.py`, D-092); the desktop app does not. | Not needed. | **Fix by card data**: in the app, the card data's parent before the D-019 rule; the rule stays only for a card the data does not know. Fixes the 10 old games without a reparse. |
| 7 Not recorded (pick, choices, Old God, keywords, triples, trinkets) | Yes. | Yes from revisions 4 and 5; old logs gone. | Not needed, and memory cannot read past games. | Leave. |
| 8 Shop record | Yes. | Yes from revision 3; old logs gone. | Not needed. | Leave. |
| 9 Free rolls from the button | Partly: the roll button shows a free roll when there is one; a count of 0 is the button's usual price. | Maybe: a game whose log never sets the tag and whose rolls all cost gold had none; untested on the 6 logs left. | Not worth a read. | Leave; a log check later if anyone asks. |
| 10 Hidden Duos combats | **No:** the game does not play them (D-054). | The `SETASIDE` copies, after the game only, mixed (D-058). | — | **Never** from memory (rule 3). |
| 11 Possible minions | The board: **no** (not shown before a combat). The opponent's **most common minion type and count: yes**, on the game's leaderboard hover: the user's own screenshot (session 064, [overlay-hover.md](overlay-hover.md): "the hero's tribe count and the combat history") and players' guides ([lobby-tribes-log.md](lobby-tribes-log.md), sources). | No: no tag on the lobby heroes holds a tribe or a count ([lobby-tribes-log.md](lobby-tribes-log.md)). | **Plausible, not mapped.** A public HDT issue (#4039, 2020; text only) says it sits in the game's leaderboard object next to "some information that shouldn't be exposed". The object, the field and how to tie a leaderboard entry to a hero are not known to us; finding them means a new explorer session on the user's client, as for D-063 (names and small numbers only). Stability: lower than D-063's two paths; a UI object changes with the leaderboard's look, and exists only while it is on screen. | **Candidate for memory** (question 2): the hint, never the board. |
| 12 Tier and order early | Yes (the leaderboard). | Exact once the log sets them, usually within the first turns. | Plausible, not mapped. | **Leave**: a few turns of "Tier ?" is not worth a read path. |
| 13 Duos top portrait | Yes. | No, but D-080's rule fits every check. | Plausible, not mapped. | **Leave.** |
| 14 Duos teammate's board for odds | **Yes:** in Duos a portal shows the partner's board, tavern and hand at any time ([Hearthstone Wiki, Duos](https://hearthstone.wiki.gg/wiki/Duos); [esports.gg](https://esports.gg/news/hearthstone/hearthstone-battlegrounds-duos/)). Approximate cards: a simulator matter, not data. | No (D-087). **To recheck first:** whether the log gets the partner's board while the player looks through the portal (cheap, on one game). | **Plausible but wide:** the boards live among the game's entities, so the read would walk an entity collection, the kind of read D-063 kept out ("no boards or hands") because it sits next to hidden state. Location not known to us. | **Not now** (question 3): check the log first; memory only if the user wants better Duos odds enough to widen D-063 to entities. |
| 15 Play time | — (not a game value). | **Yes:** the log's clock gives each game's span; 31 of 41 saved games have it. | Not needed. | **Fix by log**: show the game's own time when the record has one. |
| Rating | Yes (D-063). | No. | Already read (D-063, D-069). | Nothing new. |
| Others' ratings, names, hands, boards | **No** (or other people's data). | — | — | **Never.** |

## 4. Recommendation, by value to the player and risk

| Rank | Value | Action | Value to the player | Risk |
|---|---|---|---|---|
| 1 | Damage per combat (#5) | **Fix by log** (save the end-of-combat damage; parser revision and upload key, hub's call) | High: "most damage", the damage columns and the health chart on the site become exact for every game from now on. | None (log). |
| 2 | Skin grouping (#6) | **Fix by card data** in the app | Medium: per-hero stats right for the 10 old games, no reparse. | None. |
| 3 | Lobby tribes kept with the game (#1) | **Keep D-063's read on the PC per game** (question 1) | Medium: the recap and stats show the exact tribes for every game played with the switch on. | Low: no new read; changes what D-063 keeps on the PC. |
| 4 | Opponent's main minion type and count (#11) | **Candidate for memory** (question 2) | Medium: narrows the "possible" list for opponents not met yet from 30-45 names to their actual main tribe. | Medium: a third value and a new path in a UI object that also holds hidden data; one more path to recheck every patch. |
| 5 | Play time per game (#15) | **Fix by log** (display only) | Low. | None. |
| 6 | Overlay record label (section 5) | **Fix the label** | Low, but it is a false mark today. | None. |
| 7 | Duos teammate's board (#14) | **Check the log first**; memory **not now** (question 3) | Medium for Duos odds. | High: entity reads, D-063's own line. |
| 8 | Old games (#2, #3, #4, #7, #8) | **Leave as inferred / not recorded** | — | Their logs are gone; memory cannot read the past. |
| 9 | Free rolls (#9), tier and order early (#12), Duos top portrait (#13) | **Leave** | Low. | — |
| — | Hidden Duos combats (#10), others' ratings, names, hands, boards | **Drop: never** | — | Not shown to the player (rule 3). |

## 5. Labels found on the way

- **Bug: overlay record always "inferred".** `live.rs:431` sets `record_source` to inferred whenever a record exists, even when every result in it came from the log (since D-055); the doc comment at `live.rs:60-61` still says "always inferred". The recap does it right (`recap.rs:426-436`, per-combat sources). Fix: the same rule as the recap.
- **Note: website damage always "inferred".** `web/src/lib/game/render.ts:85-86` and `:137` mark damage dealt and taken inferred for every game. That is right today (it comes from health, `game/shape.ts:206-228`, #5); it should become "from the log" once #5 is saved.

Both are code changes for a later task, not this session.

## 6. Questions for the user (D-104)

For the hub to ask one at a time. Each amends D-063 only for the named value; injection, client changes, automation and anything not on the player's screen stay out (D-006, rule 3).

1. **Keep the tribes read from the game with each game, on this PC?** Today the exact tribes from the game show only during the game; the recap and stats afterwards show the tavern-inferred ones. Keeping them in the local game record (never uploaded, deleted with the switch's data) makes the recap and stats exact. No new memory read. **Recommendation: yes.**
2. **Read each opponent's most common minion type and count from the game?** The game shows it when you hover a hero on its leaderboard; it is not in the log. It would narrow the overlay's "possible minions" for opponents you have not met. Same rules as D-063 (read-only, off by default, one fixed path, checked per build, never uploaded), only that hint per hero, never a board; the path must first be found on your client in one research session. Risk: a third value and a second object that also holds things the game hides, rechecked on every patch. **Recommendation: yes, as a research step first** (find and check the path, no build), then a build only if the path is narrow and stable.
3. **The Duos teammate's current board for Duos odds?** The game shows it through the portal; the log does not have it during the shop. First we check whether the log gets it while you look through the portal (no risk). From memory it would mean reading the game's entities, which D-063 kept out. **Recommendation: log check only; no memory read for now.**

The log and card-data fixes (#5, #6, #15 and the overlay label) need no decision beyond the hub's usual call on parser revisions and upload keys.

## 7. Done in session 095 (T-540, T-539, D-108)

The user's answers are D-104; what was built and the counts are D-108.

| # | Value | Now | Before → after on the 41 saved games |
|---|---|---|---|
| 1 | Lobby tribes | The D-063 read is saved per game in `game_tribes.json` (local only) once it passes the overlay's checks; recap and stats show it "from the game" | Exact 0 → 0 (no read was saved before); every game from now on while the switch is on |
| 5 | Damage per combat | The game's end-of-combat hit from the saved replay (`hero_damage`), or 0 after a recorded tie; the health rule only as fallback. No parser revision: the replay already carries it | From the log 0 → 98 of 468 (81 hits, 17 ties); inferred 467 → 370; unknown 1 → 0 |
| 6 | Skin grouping | The game's link, else the card data's `battlegroundsSkinParentId`, else D-019 | Own hero by the rule 10 → 0; lobby skin ids by the rule 48 → 0 |
| 15 | Play time | The game's own start and length when the record has its span | 0 → 31 of 41 games |
| — | Overlay record label | Inferred only when a result came from health | Live only |
| 14 | Duos teammate's board (T-539) | **Not in the log.** 7 Duos games still in the logs on the PC, 73 shop turns: no minion of the teammate's board created or shown during a shop turn, no change of the player's hero entity outside combat; the only portal traces are cards passed to the hand (`ReuseFX_Generic_SpawnToHand_FromBGPortal_Super`). "Portal" in `Power.log` is a card's name. The log does not mark when the portal is opened, so this assumes the user looked through it in those games. | Duos odds keep the teammate's last board seen in a combat (D-087) |

Left for later: the website and the server's summary still work damage out from health (T-545).

### Draft D-104 (to fill in after the answers; not in `DECISIONS.md`)

> D-104 | 2026-10-1x | Amends D-063 (T-535). **(1)** The lobby tribes read from the game are kept with each game in the local game record, marked "from the game", never uploaded, removed when the switch's data is deleted; the recap and stats prefer them over the tavern-inferred ones. **(2)** [If yes] Research step: find, on the user's client, a read-only path to each leaderboard hero's most common minion type and count as the game's hover shows it; nothing else from that object; no build until the path is checked on real games and written down per build. **(3)** The Duos teammate's board is checked in the log only; no memory read. Every other D-063 rule stays. | The user's answers to [inferred-values.md](../research/inferred-values.md) section 6. |

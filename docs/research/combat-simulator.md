# Combat simulator (F6, T-601 to T-603)

Sessions 068, 071 and 074, 2026-10-11. The user asked for (1) the odds to win, tie or lose a combat, and the damage, before each combat in the app, from the boards the player has seen, and (2) an interactive simulator on the website that can load old boards or pairings. This page is the design (T-601), what the engine core does today (T-602), how it is checked against the user's own combats (T-603) and replayed attack by attack (T-603b), the card coverage (T-604, three rounds) and the odds in the overlay (T-605, section 8). Session 074 added Duos (T-608): the rules in section 3, the numbers in sections 7.4 and 7.5, the overlay in section 8.

Code: [`crates/combat-sim`](../../crates/combat-sim), [`crates/tracker/src/odds.rs`](../../crates/tracker/src/odds.rs), [`crates/desktop/src/odds.rs`](../../crates/desktop/src/odds.rs), Duos: [`crates/tracker/src/odds_duos.rs`](../../crates/tracker/src/odds_duos.rs), [`crates/combat-sim/src/harness_duos.rs`](../../crates/combat-sim/src/harness_duos.rs). Decisions: D-081, D-084, D-087.

## 1. Clean room (CLAUDE.md rule 1)

- **Not read, not used for ideas:** the code of HDT, Bob's Buddy, Firestone and `@firestone-hs/simulate-bgs-battle`, HearthMirror, Nomi's Kitchen.
- **Searched** (2026-10-11, one web search for open-source Battlegrounds combat simulators and their licences): twanvl's `hearthstone-battlegrounds-simulator` (MIT per the session prompt; 2020, old rules), `yossielimelech/BGSimulator` ("All Rights Reserved": off limits), `pruderior/Ghastcoiler` (licence not stated in what was seen: off limits until checked), full-game simulators Fireplace and RosettaStone (AGPL-3.0: incompatible). **No simulator code was read in this session**, MIT or not; nothing is reused, so `PROVENANCE.md` gets no code entry.
- **Sources of the rules:** card texts and card facts from HearthstoneJSON (D-038; downloaded once to a scratch folder for research, never committed; the crate holds only card ids, numbers and token ids, no card text or art), general game knowledge of Battlegrounds combat, and the user's own logged combats (section 7), which decide every rule marked "to check".

## 2. What the engine is given

The parser (D-055, D-058) saves each combat's two boards: card id, Attack, Health (minus damage), position, golden, and the keywords Taunt, Divine Shield, Reborn, Venomous, Windfury and Stealth (T-208). The board is taken **at the first attack** (`snapshot_open_legs` on the first `ATTACK` block), so start-of-combat effects have already happened. The game's own result is saved too (T-206).

The engine adds, from the card data (HearthstoneJSON in the harness; `crates/cards` in the app, see section 8): tavern tier, tribes, and the card's own Attack, Health and keywords (for Reborn).

**Not in the input, so not played:**

| Missing | Effect on odds |
|---|---|
| Hero powers, trinkets, quest rewards, anomalies, the Deity that act in combat | None of them is on the board. Results are "board only". |
| The hand (cards that summon "from your hand") | Those cards are not modelled. |
| Game-long buffs to tokens ("your Beetles have +2/+1 this game", Blood Gem upgrades, "your Undead have +1 Attack") | Tokens are summoned with their printed stats: those cards are "approximate". |
| `POISONOUS`, `MEGA_WINDFURY` tags | Read by the parser since session 071 (`Minion::sim_keywords`), but kept out of the saved report: upload-game checks every saved keyword name. The overlay and `combat-validate --logs` use them; `--history` still takes them from the card's own keywords. |
| Stats a card's text grows with the game (Tasty Lobster, Hopebringer) | Printed values are used: "approximate". |
| The player's tavern tier | Damage needs it; unknown means damage unknown, never 0. |
| Duos: the teammate's board during the shop | Not in the local log at all (only the player's and the shop's minions are; checked on a whole Duos game of 2026-10-11). The overlay uses the teammate's last board seen in a combat the game played on screen (section 8). |
| Who attacked first | Not in the saved report. Since session 071 the parser keeps each attack (attacker, defender, both boards before it) in `Round::attacks`, Rust-only; the harness reads the logs for it (section 7.2). |

## 3. Combat rules

Status: **done** = in the engine and tested; **to check** = in the engine, to be confirmed against the attack order in the logs (T-603b); **not yet** = not in the engine.

| Rule | How the engine plays it | Status |
|---|---|---|
| Who attacks first | The side with more minions; equal counts, a coin flip. Counted before start of combat. | done; whether the count is taken before or after start of combat: to check (replay: 0 differences in 9 Solo combats, 2 in 78 Duos first legs, both in one game) |
| Attack order | Each side keeps a pointer; its next attacker is the left-most minion from the pointer with Attack above 0, wrapping to the left-most. Sides alternate; a side with no attacker passes; neither: tie. | done |
| Pointer after deaths and summons | The pointer is an index that moves with removals and summons to its left. A minion summoned in the attacker's place (its Deathrattle) attacks next. | to check (replay: 1 difference in 78 Duos first legs: the log's next attacker was one place right of the engine's, after a Reborn minion came back in that place) |
| Target | Random among Taunt minions that are not Stealthed; else among minions that are not Stealthed; else any. Stealth is lost on attacking. | done (Stealth hides Taunt: found on the user's combat, game 19 round 6) |
| Damage | Attacker and target hit each other at once. Divine Shield takes a whole hit and is lost. | done |
| Venomous / Poisonous | Any damage that gets through kills. Venomous is lost after it kills; Poisonous is not. | done |
| Windfury / Mega-Windfury | Two / four attacks in a row while it lives and has Attack. | done |
| Cleave ("also damages the minions next to") | Same damage to the target's neighbours. | done |
| Deaths | After each attack, all dead are removed at once; Deathrattles resolve attacker's side first, each side left to right; then Avenge counts; then Reborn. Repeats until no one dies. | order: to check |
| Deathrattle | Summons go where the minion died. Titus adds one extra trigger (golden: two), counted from living Tituses. | done |
| Reborn | A fresh copy of the card with 1 Health, its own keywords without Reborn, where it died, after the Deathrattles. | done |
| Board limit | 7 minions; extra summons are lost. | done |
| Avenge (N) | Counts friendly deaths from the start of combat; triggers every N. | done |
| Rally | Before the damage of each attack it makes. | to check (before or after the hit) |
| "Whenever another friendly minion attacks" | Before the damage, left to right. | done |
| "After this attacks" | After the damage, only if it survived. | done |
| Start of combat | Off for logged boards (already applied). On for boards taken in the shop: modelled effects play (first attacker's side first, left to right); unmodelled ones make the result "not modelled". The overlay runs with it off and marks the odds approximate when a card on either board acts at the start of combat (section 8). | partly: few cards |
| "Start of Combat: attack immediately" (`BG36_849`) | Not played: the logged board is taken at that first attack, so the replay sees it as an attack out of order. | not yet |
| Effects that follow a friendly attack, damage dealt or a Reborn (round 2) | Before the damage for "whenever a friendly minion attacks" (the attacker included); after the damage, before deaths, for "after a friend deals damage" and "after a Rally minion attacks"; right after the Reborn copy is placed for "after a friendly minion is Reborn". A summon that "attacks immediately" swings at once, inside the death step. | done; timings: to check against more replays |
| Damage to the hero | Winner's tavern tier plus the tiers of its surviving minions; a tie deals none. The game's damage cap (D-072 shows it) is applied by the caller. | done (cap: the app) |
| Duos: two boards per team | Each team's first board fights; when a team's board is gone, its teammate's board comes in, and the combat goes on against what is left of the other side. A team with no board left loses; both at once is a tie. A team plays one board when the other is not known to it. | done (replay: 68 teammate boards came in on the user's 79 Duos combats before any difference) |
| Duos: who attacks after a board comes in | Decided again as at the start: the side with more minions, else a coin flip. The board that comes in starts from its left-most minion; the other side keeps its place in its order. | done (0 differences in those 68; found on the user's logs: 6 of 6 switches in one game fit the count, none fit "the other side" or "the same side") |
| Duos: start of combat of the board that comes in | Its start-of-combat effects (and its hero's) happen when it comes in: a hero power attack by the incoming board's right-most minion was seen right after a switch. The engine plays them when start of combat is on; logged boards are taken after them. | partly, as at the start |
| Duos: which board fights first | The game sets `BACON_DUO_PLAYER_FIGHTS_FIRST_NEXT_COMBAT` on every lobby hero each shop turn from the second one, and it alternates within each team. The overlay reads the player's own flag for their team and takes the opponent the game marks as next (D-061) as the opposing first board. | done (on the 6 Duos sessions of 2026-10-11, rounds 2 and later: the player's flag named their team's first board in 74 of 77 combats and the marked opponent the opposing one in 73 of 77; the misses had an empty first board) |
| Duos: damage | The winning team deals the tier of the hero whose board is on the field at the end plus the tiers of every surviving minion of the team, the teammate's board that never came in counted whole; both heroes of the losing team lose that much (shared health). The cap applies as in Solo. | done (the game's end-of-combat attack on 81 Duos combats: 45 exactly this sum, 36 lower, 35 of them at the cap of 15) |
| Duos: what carries over | Minions keep their damage and buffs; a board that comes in has fresh Avenge counts (each minion counts deaths from when it is on the field). | Avenge across boards: to check |
| Duos: teammate effects in combat | Only two Duos cards act on the teammate's board in combat: `BGDUO_105` (Deathrattle: copy a minion of the teammate's warband) and `BGDUO_125` (start of combat: become a copy of the teammate's minion), both not modelled yet, neither seen on the user's boards. The others act on the hand or the shop (no combat effect). | not yet (unseen) |
| Hero powers, trinkets, Deity, spells cast in combat | Not on the board. | not yet |

## 4. Cards

- **Registry keyed by exact card id** (`cards.rs`, with the cards of each round in `cards_round1.rs` to `cards_round4.rs`). Golden cards have their own ids (`BG25_354_G`, `TB_BaconUps_045`), so a golden effect is written out, never "double the normal one" by guess.
- **Two lists:** `NO_COMBAT_EFFECT` (text never changes a combat: shop, hand, Tavern, teammate, or keywords the board already shows; checked card by card) and `EFFECTS` (built from a few primitives: summon, buff all, buff random, gain stats or a keyword, give a keyword, damage, strip keywords, destroy the killer; triggers Deathrattle, Rally, Avenge, start of combat, friendly attacks, after attack, plus static ones such as Cleave, Taunt while shielded, extra Deathrattles, shield at 6 Attack).
- **Fidelity:** each effect is `Exact` or `Approximate(reason)`. A card in neither list is **not modelled**: `simulate` returns `Verdict::NotModelled` with the card ids, never a number. The harness alone may play unknown cards as plain minions, and says so.
- **Tokens** (`TOKENS`): id, printed stats, keywords, tribe, tier.

## 5. Randomness and trials

- SplitMix64, seeded (`rng.rs`): the same seed gives the same odds on every platform, WebAssembly included, so a bug report can be replayed.
- Default 10 000 trials: the standard error of a share is at most 0.5 percentage point.
- `Odds`: won, tied, lost; damage dealt and taken as counts per value (when every tier is known); trials with unknown damage; trials stopped at the turn limit (1 000 attack turns, counted as ties).

## 6. Speed

Release build on the user's PC, the 19 logged Solo combats, 10 000 trials each: **32 ms on average, 73 ms at most** (target: under 200 ms). One thread, no allocation-heavy work per attack beyond small vectors; fast enough to run on every board change in the overlay.

Duos (session 074), the same 10 000 trials on each Duos combat's boards: **74 ms on average, 308 ms at most** on the 84 combats of the logs, 105 and 389 ms on the 232 of the history: four boards, and late games with boards in the thousands make long combats. The overlay keeps its 150 ms budget: such a run stops after fewer trials (at least 1 000) and says how many in Advanced.

After card round 4 (session 075), on the history: Solo 38 ms on average (36 before), Duos 118 ms on average and 569 ms at most (108 and 432 before): the newly modelled cards make some combats longer (Beetles that grow, Golems that keep the card's stats).

## 7. Validation on the user's combats (T-603, T-603b)

`combat-validate --history %APPDATA%\TavernLedger\games.jsonl --cards cards.json` plays every logged Solo combat and prints counts, shares and card ids only (no names, player ids or heroes). The user's logs stay on the PC; the repo has synthetic fixtures only (`crates/combat-sim/tests/data`).

Run of 2026-10-11 (41 games: 2 Solo, 39 Duos):

| Combats | n | Mean p(real result) | Brier | Most likely = real |
|---|---|---|---|---|
| all | 19 | 96% | 0.058 | 18 of 19 |
| exact | 13 | 94% | 0.085 | 12 of 13 |
| approximate | 6 | 100% | 0.000 | 6 of 6 |

- Calibration: 18 of 19 combats were predicted at 80% or more for the real result, and all 18 happened. The one other: a tie the engine gave 26% (won 74%), every card exact. Its opponent had a Stealthed Taunt minion; that led to the Stealth fix above but did not change the odds, so another rule differs. Finding it needs the combat's attack order (T-603b).
- **Too few combats to call the engine accurate:** 19 Solo combats, most one-sided. Duos rounds (449, 119 with every board and a result) are not scored until Duos is modelled.
- The run of session 071 (after card round 2) gives the same table: the 19 Solo combats hold few of the round 2 cards.

### 7.2 Attack order replay (T-603b)

The parser keeps every attack of a combat in order: the attacker (side, position, card), the defender (the game's `PROPOSED_DEFENDER`, or in a log trimmed of it the first enemy minion hit) and both boards just before the attack. This stays out of the saved report and the upload; `combat-validate --logs <Logs folder or Power.log>` reads the game's own logs where they are and replays each combat: before every attack it compares the engine's boards with the log's, checks that the engine's rules pick the same attacker and allow the log's target, then plays the log's attack (the defender is forced; other random choices are still the engine's). The first step where they part names the rule. Duos combats are replayed for their first leg only, while the player's own board fights (T-608).

Run of 2026-10-11 on what is on the PC: the Solo game of 2026-10-09 (the one with the miss, kept as the real test fixture `b253216_solo.log`) and the 6 Duos sessions still in the game's Logs folder (older logs are gone).

| Input | Combats | Replayed with no difference | Attacks matched before the first difference |
|---|---|---|---|
| Solo (fixture) | 9 | 8 | 48 of 56 |
| Duos first legs (session 071) | 78 | 43 | 313 of 1 306 |
| Duos, both boards of each team (session 074) | 79 | 32 | 500 of 1 308 |

First differences of session 071 (the session 074 run, whole Duos combats: stats 25, deaths and summons 16, attack order 4, who attacks first 2, who attacks after a teammate's board comes in 0):

| First difference | Solo | Duos first legs |
|---|---|---|
| stats or keywords after an attack | 1 | 18 |
| deaths, summons or their places | 0 | 11 |
| attack order | 0 | 4 |
| who attacks first | 0 | 2 |

- **The Solo miss (game 19 round 6, a tie the engine gave 26%) replays move for move to the end**: no rule differs. With the log's targets the engine reaches the same tie, so the 74% win was the dice, not a wrong rule.
- The Solo difference (round 8, attack 3): `BG33_883` has 54/42 in the log and 46/34 in the engine. Its Rally plays Blood Gems, which the engine plays as +1/+1 (approximate, its Blood Gems were already upgraded).
- Most Duos differences are cards: stats that grow (game-long buffs, Blood Gems), Deities (`BGFYM_011`, `BGFYM_000`) and a hero power's minion (`BG22_HERO_305t`) appearing on the board, and cards round 2 still leaves out. The run prints each one with its card ids.
- Attack order: two first attacks out of order are a start-of-combat attack (`BG36_849`, "Start of Combat: attack immediately") and a minion at position 7 (`BG36_360t5`) that no card on the board explains (a hero power or trinket, not on the board). The pointer case is in the rule table above.
- Too few Solo combats to change a rule on this alone; the replay is now the tool to check each "to check" rule as more Solo games are logged.
- Known limits of the replay (session 071 review): it decides the first attacker from the boards at the first attack, after start-of-combat effects, where the engine counts before them, so a start-of-combat summon or kill can give a false "who attacks first"; an attack made inside another one ("attacks immediately") is a step of its own in the log, so the next comparison names another rule; in a log without `PROPOSED_DEFENDER` the defender is the first enemy hit, which an effect before the swing can get wrong.

- Session 074 replays whole Duos combats: when a board is gone in the log, the team's next board comes in in the engine too, from the log's board as it came in. Fewer combats run to the end than first legs did (a longer combat meets more of the card gaps above), but no difference is about the boards coming in.

### 7.3 The overlay's inputs (T-605)

The overlay does not have the board the opponent brings to the combat: it has the last one the player saw, often several turns old, and the player's own board in the shop. `combat-validate` scores that too: for each Solo combat, the player's board against the board of the same opponent from the last earlier combat with them, cards not modelled refused as in the app.

| Solo combats | No board of that opponent seen before | Not modelled | Scored | Mean p(real result) | Most likely = real | Board age (mean) |
|---|---|---|---|---|---|---|
| 19 | 14 | 0 | 5 | 40% | 2 of 5 | 7 rounds |

- **The live odds are far weaker than the engine**: on these 5 combats the board the player last saw was 7 rounds old on average, and the most likely result was right 2 times in 5. The overlay says how old the board is ("their board seen turn 6") on every number.
- The proxy is generous on one side and strict on the other: it uses the player's board as it entered the combat (after start of combat), where the app has the shop board (before it).
- 5 combats is far too few to give a number to the user as a promise; the harness gives it again as more Solo games are logged.

### 7.4 Duos combats (T-608)

`combat-validate` builds each Duos combat from the boards as they came in, in order, one per seat (`harness_duos.rs`); a board the log has no attack after is left out (an empty board replaced at once, the hero switching back after the combat, or a combat the game does not play back, whose copies are never read: D-054). A team whose second board never came in plays its first board alone. Run of 2026-10-11, unknown cards played as plain minions:

| Source | Combats with a result | Mean p(real result) | Brier | Most likely = real |
|---|---|---|---|---|
| Logs on the PC (8 Duos games) | 84 | 83% | 0.260 | 71 of 84 |
| … both teams' two boards fought | 51 | 81% | 0.290 | 42 of 51 |
| Saved history (39 Duos games) | 232 | 80% | 0.294 | 188 of 232 |
| … both teams' two boards fought | 153 | 80% | 0.299 | 123 of 153 |

- About as good as on Solo boards with the cards the engine knows; the misses are the card gaps the replay lists (game-long buffs, Deities and hero powers' minions on the board, hand summons).
- The history has 449 Duos combats with a board on each side; 232 of them have the game's result in the saved record.

### 7.5 The overlay's inputs in Duos

As 7.3, for Duos: the player's board before the combat (the shop board in the logs, read since session 074; the history has none, so there the board as it came into the combat stands in for it), the teammate's last board seen in an earlier combat, both opponents' last boards seen in earlier combats, each team in the order it fought. Cards not modelled are refused, as in the app.

| Source | Combats with a result | No board of someone seen before | Not modelled | Scored | Mean p(real result) | Most likely = real | Board age (mean) |
|---|---|---|---|---|---|---|---|
| Logs | 84 | 30 (teammate 12, opponent 18) | 18 | 36 | 72% | 26 of 36 | 2.4 rounds |
| History | 232 | 68 (teammate 20, opponent 48) | 31 | 104 | 49% | 51 of 104 | 2.4 rounds |

- **Much better than Solo's 2 of 5** on the logs: in Duos every board fights often, so the boards are only 2.4 rounds old. The history's older games do worse (51 of 104), so the number is a hint, as in Solo.
- With boards from the same combat instead of earlier ones, the same pipeline gives 79% (history) and 85% (logs): what is lost is the boards' age, not the Duos rules.
- The order of each team is taken from the combat itself; the app reads the game's flag and its marked opponent, which named the first boards in 74 and 73 of 77 combats (section 3), so these scores are a little kinder than the app's.
- In 29 history combats the player's own board did not fight, so the history has no board for it; the app has the shop board then.

### 7.6 What the app would show, and the accuracy export (T-604, T-607)

Since session 075 `combat-validate` also plays every combat with a result as the app would: on its own boards (as they came in), cards not modelled refused, start of combat off. It counts the combats a card not modelled blocks, per card, and scores the rest per mode. Before and after card round 4 (section 9):

| Source | Mode | Combats with a result | Blocked | With a hand summoner | Blocked by the hand summoners only | Scored | Mean p(real result) | Brier | Most likely = real |
|---|---|---|---|---|---|---|---|---|---|
| History | Solo | 19 | 0 → 0 | 0 | 0 | 19 → 19 | 96% → 96% | 0.058 → 0.058 | 18 of 19 → 18 of 19 |
| History | Duos | 232 | 50 (22%) → 30 (13%) | 25 | 19 → 21 | 182 → 202 | 80% → 81% | 0.292 → 0.274 | 147 of 182 → 166 of 202 |
| Logs | Duos | 84 | 23 (27%) → 18 (21%) | 15 | 12 → 13 | 61 → 66 | 85% → 84% | 0.197 → 0.212 | 53 of 61 → 57 of 66 |

- Cards that still block, by combats (history): `BG34_140` 11, `BG31_999` 7, `BG31_835` 5, `BG21_HERO_030t` 4, `BG34_140_G` 3, `BG34_Giant_017` 3, `BGS_008` 1, `TB_BaconShop_HERO_33_Buddy_G` 1. The hand summoners (with `BG34_Giant_017`, which summons from the hand too) alone now block 21 of the 30.
- The overlay's inputs (section 7.5) gain too: "a card not modelled" 31 → 16 (history) and 18 → 14 (logs); scored 104 → 119 (58 right, 49%) and 36 → 40 (29 right, 72%).
- The engine on every board that fought (unknown cards as plain minions, section 7.4): 188 → 189 of 232 right on the history, 71 of 84 on the logs (unchanged).

**Card round 5 (session 083, D-097).** No game was played since round 4 (the same 41 games in the history and 8 Duos games in the logs), so the gain comes from the rule for the hand summoners and the last cards the board can play (section 9). Before the rule changed, `combat-validate` played the blocked combats anyway (the cards not modelled as plain minions, which for a hand summoner is "the hand summon left out") and scored them apart:

| Source | Blocked combats, played anyway | n | Mean p(real result) | Brier | Most likely = real |
|---|---|---|---|---|---|
| History, Duos | by the hand summoners only | 21 | 82% | 0.324 | 17 of 21 |
| History, Duos | by other cards (Stitched Salvager in 7 of 9) | 9 | 68% | 0.574 | 6 of 9 |
| Logs, Duos | by the hand summoners only | 13 | 85% | 0.308 | 11 of 13 |
| Logs, Duos | by other cards | 5 | 60% | 0.773 | 3 of 5 |

- **With the hand summon left out, the odds are about as good as on the combats already scored** (17 of 21 right against 166 of 202; the Brier score a little worse), so the summon the hand hides is not as decisive as feared in section 10, question 4. The hand summoners now play as approximate, named with the reason ("summons a minion from the hand, which is not seen"), never as exact.
- **Stitched Salvager's combats played anyway are far worse** (6 of 9, Brier 0.574): it stays blocking, as do Ghastcoiler and Timewarped Festergut (random minions from a pool the engine does not have).

Before and after round 5:

| Source | Mode | Combats with a result | Blocked | With a hand summoner | Scored | Mean p(real result) | Brier | Most likely = real |
|---|---|---|---|---|---|---|---|---|
| History | Solo | 19 | 0 → 0 | 0 | 19 → 19 | 96% → 96% | 0.058 → 0.058 | 18 of 19 → 18 of 19 |
| History | Duos | 232 | 30 (13%) → 8 (3%) | 25 | 202 → 224 | 81% → 81% | 0.274 → 0.278 | 166 of 202 → 184 of 224 |
| Logs | Duos | 84 | 18 (21%) → 4 (5%) | 15 | 66 → 80 | 84% → 85% | 0.212 → 0.225 | 57 of 66 → 69 of 80 |

- Of the scored combats, those with a hand summoner on a board: 22 on the history (83%, Brier 0.309, 18 right) and 14 on the logs (86%, 0.286, 12 right). The harness prints both parts on every run (the scored combats with a hand summoner, the blocked ones played anyway), so the rule can be checked again as games come in; neither goes in the export.
- Cards that still block (history): `BG31_999` 7, `BGS_008` 1. The blocked combats played anyway now: 8 (64%, 5 right).
- The overlay's inputs (section 7.5) gain too: "a card not modelled" 16 → 3 (history) and 14 → 4 (logs); scored 119 → 132 (68 right, 52%) and 40 → 50 (36 right, 72%).
- The engine on every board that fought (section 7.4): 189 of 232 (history) and 71 of 84 (logs), unchanged; the export and the accuracy page show the engine version `0.5.0`.

**Export for the website (T-607).** `combat-validate (--history … | --logs …) --cards cards.json --export accuracy.json` also writes these numbers as a small JSON file. It holds counts, shares, the date, the engine version and card ids only: no names, BattleTags, account ids, seats, heroes, games or rounds. The user's own file stays on the PC until the user chooses to publish it; the repo has a synthetic example only, `crates/combat-sim/tests/data/accuracy_example.json` (a test checks its keys against the export).

| Key | Meaning |
|---|---|
| `format` | Shape version, now `1`; a reader checks it first |
| `date` | The day the file was written (UTC, `YYYY-MM-DD`) |
| `engine_version` | The `combat-sim` crate version; its minor number is the card round (`0.4.0`: round 4) |
| `trials` | Trials per combat |
| `modes.solo`, `modes.duos` | One block per mode, below |
| `…combats_with_result` | Logged combats with the game's result |
| `…blocked`, `…blocked_share` | Combats with a card not modelled on a board (no number in the app), and their share; `null` share with no combat |
| `…refused` | Combats the engine's board check refused |
| `…scored` | Combats played (with a result, not blocked, not refused) |
| `…mean_p_real` | Mean probability given to the real result; `null` when nothing was scored |
| `…brier` | Multi-class Brier score (0 best, 2 worst); `null` when nothing was scored |
| `…most_likely_right` | Scored combats whose most likely result was the real one |
| `…calibration` | Five buckets of predicted shares (`from`, `to`, `predictions`, `mean_said`, `happened`; the last two `null` for an empty bucket): of the times the engine said 0-20%, how often it happened, and so on |
| `blocking_cards` | Up to 10 card ids that block most combats, with the count (both modes) |

A page shows the scored combats with their accuracy and, beside them, the blocked share ("no odds in 13% of Duos combats: a card is not modelled"), never one without the other.

**The page (T-607, session 077, D-091).** `/accuracy/` and `/es/accuracy/`, public and indexable, built at build time from `web/src/data/accuracy.json` (the export of the user's history, published under D-089) and `web/src/data/accuracy-cards.json` (the blocking cards' names, below); no fetch. Both files are read strictly (`web/src/lib/accuracy/data.ts`): format 1 first, then exactly the export's keys, so a bad or wider file fails the tests and the build. Per mode: "most likely result was right" (with the counts), the mean probability of the real result and, in the same row, the blocked share; Advanced adds the Brier score, the refused count and the card ids; the calibration buckets as a "said / happened" chart with its table; the blocking cards by name; one paragraph on what is measured and its limits.

**Refreshing it after a card round** (one command, then a PR). The card data (HearthstoneJSON's `cards.json` in English and Spanish) is fetched into a temporary folder, never committed:

```
cargo run --release --manifest-path crates/combat-sim/Cargo.toml --features harness --bin combat-validate -- --history %APPDATA%\TavernLedger\games.jsonl --cards <tmp>\cards.json --export web/src/data/accuracy.json && python tools/accuracy_card_names.py --export web/src/data/accuracy.json --cards-en <tmp>\cards.json --cards-es <tmp>\cards.es.json --out web/src/data/accuracy-cards.json
```

Then check the diff holds only the export's keys and counts (the web tests do too: `npm test` and `npm run build && npm run test:dist` in `web/`), and open a PR; merging deploys the page. Card names (D-091): only the up to 10 blocking card ids, their English and Spanish names and a golden flag, nothing else from the card data.

## 8. One engine for the app and the website

**App (T-605, done in session 071, D-084).** During a shop turn the overlay runs the player's board as it is now against the next opponent's last board seen (D-004: only boards the player saw on screen; the opponent is the one the game marks as next, D-061). How:

- **Inputs.** The parser gives the own board in a shop turn (`own_board_now`, with Poisonous and Mega-Windfury); the opponent's board is the newest that entered play in a combat the game has shown (the live view follows the screen, D-067). Tavern tiers come from the leaderboard (the opponent's as it is now). The card facts the board does not show (tier, types, the card's own stats and keywords for Reborn) come from the card data: `crates/cards` keeps them for every Battlegrounds minion (saved format 5).
- **Running.** On its own thread, never the window's: the tracker's loop says what the odds should be worked out from; the thread waits until it has not changed for 300 ms, then runs 10 000 trials in batches of 1 000, stopping early after 150 ms (about 30 ms is usual). The same boards give the same odds (fixed seed). During the combat on screen the shop's odds stay.
- **Duos (T-608, session 074, D-087).** The player's shop board and the teammate's last board seen in a combat the game played on screen, in the order the game set (the player's own `BACON_DUO_PLAYER_FIGHTS_FIRST_NEXT_COMBAT`, read live, Rust-only), against the last boards seen of the opponent the game marks as next (first) and of their teammate (second, from the lobby's teams). Never a `SETASIDE` copy of a combat the game did not play back (D-054): the parser gives those no board. Tiers from the leaderboard. Simple shows the oldest board's age ("oldest board seen turn 4"); Advanced each board's ("Your teammate's board: seen turn 7", "Their first board…", "Their second board…"). When a board was never seen, the overlay names it instead of a number ("not seen yet: your teammate's board and their second board"); when the order or the teams are not known yet (the first shop turn), "who fights first, or who is on which team, is not known yet". English and Spanish.
- **No number when it cannot be trusted.** No board of that opponent seen yet (in Duos, any of the three boards), a card the engine does not know ("a card is not modelled", Advanced names it), the card data missing for a card, boards it cannot read. No next opponent marked: nothing shown.
- **Approximate.** Start of combat is off: the opponent's board was taken after theirs, the player's shop board has not had them yet. When a card on either board acts at the start of combat, or a card is approximate, the age line starts with "≈" and Advanced names the cards.
- **Shown.** Simple, under the next opponent's line: Win, Tie and Lose with their shares, and "their board seen turn N". Advanced, in the next opponent's block: the same, the damage both ways (10th to 90th percentile and the median, after the damage cap; "unknown" when a tier is unknown), the approximate and missing cards and the trials. English and Spanish. A switch in Settings turns it off (on by default; `odds_off` in `overlay.json`).
- **How good it is:** section 7.3; the public numbers per mode are on the site's accuracy page, `/accuracy/` (section 7.6).

**Website (T-606), three ways:**

| Option | For | Against |
|---|---|---|
| **Rust → WebAssembly** (recommended) | One engine: the website's odds are the app's odds by construction; fast (tens of ms); runs in the browser, so no server cost and no upload of boards | The site's CSP needs `'wasm-unsafe-eval'` in `script-src` (today `'self'` or `'none'`, `web/src/lib/config.ts`); a build step for the `.wasm` file (no new crate needed: the engine has no dependencies; `wasm-bindgen` or a hand-written C ABI would be one new build dependency, R2) |
| TypeScript port | No CSP change, no build step | Two engines to keep in step: every rule and card twice; differences would show as different odds in the app and on the site |
| Server function (Supabase Edge, Deno with WebAssembly) | No CSP change on the site | Server cost and rate limits; every board sent to the server; still needs the WebAssembly build |

Recommendation: WebAssembly in the browser, with `'wasm-unsafe-eval'` added only on the simulator page if the site can set the CSP per page (else site-wide). This is the user's decision (CSP change).

**Website, built (T-606, session 076, D-090).** The user chose WebAssembly (D-089). How it is built:

- **Wrapper.** `crates/sim-wasm`: a hand-written C ABI over the engine, no `wasm-bindgen` and no new crate (`serde_json` was already in `Cargo.lock`). The page's worker calls `sim_alloc(len)`, writes the JSON request into the module's memory, calls `sim_run(ptr, len)` (which frees the request and returns the answer's length) and reads the answer at `sim_answer()`. The module imports nothing from the page. A bad request is an answer (`bad_input` with a reason), never a panic.
- **Request.** `{"player": [board, board?], "opponent": [board, board?], "damage_cap": n | null}`; a board is `{"tier": 1..7 | null, "minions": [...]}`; a minion is `{"card_id", "golden", "attack", "health", "keywords": [saved names], "facts": {"tier", "tribes", "attack", "health", "keywords"} | null}`. The facts are the card data's, as `tracker::odds::Facts::from_names` reads them. At most 64 KB, two boards a side, 16 minions a board (the engine refuses more than 7).
- **Answer.** `status`: `odds` (counts of won, tied and lost trials, the damage spreads, approximate cards with the engine's reason, a start-of-combat flag), `not_modelled` (the card ids), `no_card_data` (the card ids without facts), `bad_board` or `bad_input`; every answer carries the engine's version.
- **Same odds as the app.** The same trials, batches and seeds as the overlay (10 batches of 1 000, seed `0x0DD5 ^ batch`, no time budget in the browser) and the same spread (10th, 50th and 90th percentile after the cap). `crates/sim-wasm/tests/parity.rs` plays seven fixed requests (Solo, Duos, Deathrattles, keywords, an approximate card, a start-of-combat card, an unknown tier, a card not modelled, a card without facts) with this crate and with `tracker::odds::run_odds` and checks they agree; it keeps the answers in `tests/data/parity.json`, and `web/test/unit/simEngine.test.ts` runs the committed `.wasm` on the same requests and needs the same answers.
- **The `.wasm` is committed** (`web/src/lib/sim/engine.wasm`, 166 KB since round 5): Cloudflare Pages builds the site with npm only. `node tools/build-sim-wasm.ts` (in `web/`, needs `rustup target add wasm32-unknown-unknown`) builds it with the `sim-wasm` profile (size, LTO, abort on panic, stripped), remaps this PC's paths and refuses to copy a file that still holds one, and writes `engine.stamp.json`: the file's SHA-256 and a SHA-256 over its sources (the engine's files without tests or harness, both `Cargo.toml`, the workspace's, and the `Cargo.lock` entries of its crates; line endings normalised). A unit test fails when the sources change without a rebuild, so CI needs no WebAssembly toolchain. After an engine change: `SIM_PARITY_WRITE=1 cargo test -p sim-wasm --test parity`, then the build tool, then commit both files.
- **Card data.** Not in the repo (D-038): `tools/prepare-sim.ts` fetches HearthstoneJSON's `cards.json` (English and Spanish) at build time (2-minute timeout, 64 MB cap, its own host only) and keeps, per Battlegrounds minion, only id, name, tier, types, Attack, Health, combat keywords, whether the tavern offers it and its golden card, with the checks of `crates/cards` (a card that fails one is left out). The files are written next to the page's code (git-ignored) and ship as script chunks from the site's own origin, so the CSP needs no new connection; a failed fetch stops the build and the site stays as it was. CI builds with synthetic files (`SIM_CARDS_DIR`).
- **CSP (D-090).** Per page: `/simulator/`, `/es/simulator/` and the worker's script get `'wasm-unsafe-eval'` in `script-src`, in the page's meta policy and in `_headers`; nothing else changes. Cloudflare Pages applies every matching `_headers` rule in file order and joins a repeated header with a comma (two policies, both enforced), so each of these rules first removes the site-wide policy (`! Content-Security-Policy`) and then sets its own; Pages runs a rule's removals before its values (read in Cloudflare's own asset server code, `workers-sdk`, `pages-shared/asset-server/handler.ts`). The worker carries the engine as base64, so it fetches nothing.
- **Page.** `noindex`. Signed in: the player's own games only (the user id from their session, never from the address), read with the game viewer's reader (D-079: caps, untrusted file, a file naming another game refused). A round gives the boards that fought in it, each side in the order they came in (Duos, D-087); the saved report holds no board of a combat the game did not play back (T-213 left out, D-058), so no `SETASIDE` board can come in. The player's own tier is the shop's for that turn; the others are unknown until set (the damage is unknown, never 0). The editor removes, moves, sets Attack and Health, golden (the golden card's id, as the engine keys golden effects), the eight keywords the engine tracks, adds a minion by name and sets each board's tier and, in Advanced, the damage cap. 300 ms after the last edit the boards play in a Web Worker. A card not modelled or missing from the card data blocks the run and is named; approximate cards are marked "≈" with the engine's reason (Spanish from a table a test keeps complete). Simple: Win, Tie, Lose and the likely damage; Advanced: the spreads, trials, engine version, card data source and the cap. Signed out the editor works and nothing is read.

## 9. Card coverage (T-604)

Every minion seen entering a combat in the user's history (Solo and Duos): **534 card ids, 7 907 appearances**.

| Class | Cards (round 1) | Appearances (round 1) | Cards (round 2) | Appearances (round 2) |
|---|---|---|---|---|
| No combat effect (checked) | 399 | 5 920 (75%) | 399 | 5 920 (75%) |
| Modelled, exact | 51 | 971 (12%) | 57 | 1 017 (13%) |
| Modelled, approximate | 33 | 639 (8%) | 43 | 807 (10%) |
| Not modelled | 51 | 377 (5%) | 35 | 163 (2%) |

**Round 2 (session 071)**, most seen first, each with its golden card: `BG36_241` (Rally: Mighty Dragonbreath, approximate: spell values can grow), `BG33_155` (gains when another Demon deals damage; approximate: attack damage only), `BG36_208` (left-most Deathrattle after a Rally attack; approximate: which one and when, to check), `BG28_583` (approximate: Blood Gems played on it in combat are not passed on), `BG36_515` and `BG36_514` (after a Reborn), `BG34_403` (Avenge: an Eternal Knight that attacks at once; approximate: the Knights' game-long growth), `BG34_320` (Rally: one minion of each type), `BG36_211` (whenever a Beast attacks). New primitives: an attack trigger that includes the attacker, "after a friend deals damage", "after a Rally minion attacks", "after a Reborn", a summon that attacks at once, a buff for minions with a keyword and one per type.

Left for round 3, most seen first: `BG34_140` and `BG21_HERO_030t` (summon from the hand: not known), `BG31_999` (start of combat destroy, copy on death), `BG31_835` (summons from the hand), `BG36_360t9` (Golem with its stats), `TB_BaconShop_HP_105t` (gains Deathrattles), `BG35_342` (grows with Deathrattles this game), `BG36_209`, `BG26_802`, `BG34_690`, `BG34_Giant_017`, `BG34_Giant_331`, `BG35_602`.

**Round 3 (session 074, Duos)**, the most seen of the Duos history that the board alone can play, each with its golden card: `BG35_342` (gains for each friendly Deathrattle, exact), `BG34_690` (Deathrattle: Undead +2 Attack; approximate: Undead summoned later miss it), `BG26_802` (doubles a summoned Beast's Attack; approximate: order with other summon effects), `BG35_602` (+3 Attack to a summoned Beast; approximate: the bonus grows during the game). New primitives: after a friendly Deathrattle, after a friendly summon (a Reborn copy counts). Not modelled appearances: 163 → 139 (35 → 27 cards). Left, most seen first, and why: `BG34_140`, `BG31_835`, `BG21_HERO_030t` (summon from the hand, which is not seen), `BG31_999` (copies the minion it destroyed at the start of combat, gone from a logged board), `BG36_360t9` (its Golem's card id is not in the card data), `TB_BaconShop_HP_105t` (gains other minions' Deathrattles).

**Round 4 (session 075)**, ordered by the combats each card blocked (section 7.6), each with its golden card: `BG36_360t9` (Dark Paradox: Deathrattle, a Golem with its stats; approximate: the Golem's card id `BG30_MagicItem_442t` is the one the log shows for Jailbird Juggernaut, the card data has none), `TB_BaconShop_HP_105t` and its skin (Fish of N'Zoth: gains the Deathrattle of each friendly Deathrattle minion that dies; approximate: those gained in earlier combats are not on the board), `BG36_209` (Ravaging Scorpid: Beetles +4/+4 after each friendly attack, a Beetle on death; approximate: Beetles' game-long buffs), `BG34_Giant_331` (Timewarped Warghoul: triggers a neighbour's Deathrattle; approximate: which one, and neighbours that die with it), `BG36_333` (Jailbird Juggernaut: Rally, a Golem with its stats attacks the target first; approximate: what it does when its Golem kills the target), `BG31_HERO_802pt1` and `BG31_HERO_802pt` (Carrier and Colossus, Protoss minions of a hero power; approximate: they improve during the game), `BGDUO31_208` (San'layn Scribe; approximate: Scribes dying in this combat), `TB_BaconShop_HERO_76_Buddy` (Spirit of Air; approximate: picks only minions that miss a keyword), and the golden cards of `BG26_354`, `BG27_556` (start of combat, as their normal cards), `BGS_126` (excess damage to both neighbours, exact) and `BG33_430`. New primitives: a token with the card's Attack and full Health (one that attacks first), a buff for card ids, damage to the target's neighbours, a neighbour's Deathrattle, after a friendly attack, Deathrattles gained in combat; minions now keep their full Health apart from the damage. Not modelled appearances: 139 → 83 (27 → 12 cards); blocked Duos combats 50 → 30 of 232.

The replay (section 7.2) checks the cards it meets: one first difference moved, game 3 round 8: the Beetle the Scorpid summons is now there, and the first difference is its stats (2/2 against the log's 14/14: the game-long Beetle buffs, the stated approximation). The other first differences are unchanged (32 of 79 combats to the end, 500 of 1 308 attacks).

Left blocking after round 4, and why: the hand summoners (question 4, and `BG34_Giant_017`, which summons from the hand too); `BG31_999` Stitched Salvager (its Deathrattle copies the minion it destroyed at the start of combat, which no board the app has still shows, so like the hand summoners the missing minion is often the biggest of the combat); `BGS_008` and `BG34_Giant_590` (random minions from a pool); `BG34_Giant_599` (copies Deathrattles at the start of combat); `TB_BaconShop_HERO_33_Buddy` (follows the Amalgam); the golden Fish skin (gains each Deathrattle twice); `BG26_RLK_833` (seen in combats without a result only).

**Round 5 (session 083, D-097)**, each with its golden card where there is one: the hand summoners `BG34_140` Expert Aviator, `BG31_835` Deathly Striker, `BG21_HERO_030t` Sneed's New Shredder and `BG34_Giant_017` Timewarped Scourfin (approximate: the summon from the hand is left out; why in section 7.6), `BG26_RLK_833` Foul Egg (a 3/3 Undead Chicken, `BG_RLK_833t`; exact, no golden card), the golden Fish of N'Zoth `TB_BaconUps_307` and its golden skin (each Deathrattle twice; approximate as the normal one), `TB_BaconShop_HERO_33_Buddy` Mishmash (gains what the Amalgam gains, golden twice; exact), `BG34_Giant_599` Timewarped Whirl-O-Tron (start of combat: copies the two left-most friendly Deathrattles, other Whirl-O-Trons left out, golden twice; approximate: on a board taken after the start of combat, which is every board the app has, the copies are not on the board). New primitives: a minion that gains what the Amalgam gains, and Deathrattles copied from the left-most friends; Deathrattles gained in combat can be gained twice. Not modelled appearances: 83 → 22 (12 → 3 cards). No game was played since round 4, so the history and the logs are the same as in round 4.

The replay (section 7.2) checks the cards it meets: one first difference moved, game 1 round 9: Mishmash now gains what the Amalgam gains, and the first difference is another minion's Health six attacks later (505 of 1 308 attacks before the first difference, from 500). The hand summoners' combats keep their first difference at the summon (game 4 round 5: Expert Aviator summons a minion from the hand), the stated approximation.

Left blocking, and why: `BG31_999` Stitched Salvager (its Deathrattle copies the minion it destroyed at the start of combat, gone from every board the app has; its blocked combats played anyway score far worse, section 7.6); `BGS_008` Ghastcoiler and `BG34_Giant_590` Timewarped Festergut (random minions from a pool the engine does not have: every Deathrattle minion, and the Undead Creations). Only Stitched Salvager and Ghastcoiler block combats with a result (7 and 1 of 232).

## 10. Questions for the user

1. **How the website runs the engine.** WebAssembly (needs `'wasm-unsafe-eval'` in the CSP), a TypeScript port, or a server function. Recommendation: WebAssembly, the CSP change limited to the simulator page if possible.
2. **Which boards the website's simulator may load.** Only your own games, or also other public players' boards. Recommendation: your own games only at first (your boards and the opponents' boards you met, which you saw on screen); other players' boards only from public profiles and only after T-509's privacy texts cover it.
3. **Duos odds (T-608, done in session 074).** Please check in your next Duos game that "their first board" is the one that fights your first board (the overlay takes the opponent the game marks as next).
4. **Cards that summon from the hand** (`BG34_140` Expert Aviator, the most seen card not modelled, and `BG31_835`, `BG21_HERO_030t`): today they block the odds ("a card is not modelled"). They could play as approximate, with the hand summon left out and the card named. Recommendation: keep them blocking; the summon is often the biggest minion of the combat. **Applied** (the hub's call, session 075): after round 4, counting `BG34_Giant_017` (also from the hand), they are on a board in 25 of the 232 Duos combats of the history (15 of 84 in the logs) and alone block 21 of them (13 in the logs), 9% of the Duos combats. **Changed in round 5 (session 083, D-097):** the 21 combats played with the hand summon left out gave the real result as often as the combats already scored (17 of 21 right; section 7.6), so the hand summoners now play as approximate and are named with the reason; the harness keeps scoring their combats apart to check this as games come in.

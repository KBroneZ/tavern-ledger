# Combat simulator (F6, T-601 to T-603)

Sessions 068 and 071, 2026-10-11. The user asked for (1) the odds to win, tie or lose a combat, and the damage, before each combat in the app, from the boards the player has seen, and (2) an interactive simulator on the website that can load old boards or pairings. This page is the design (T-601), what the engine core does today (T-602), how it is checked against the user's own combats (T-603) and replayed attack by attack (T-603b), the card coverage (T-604, two rounds) and the odds in the overlay (T-605, section 8).

Code: [`crates/combat-sim`](../../crates/combat-sim), [`crates/tracker/src/odds.rs`](../../crates/tracker/src/odds.rs), [`crates/desktop/src/odds.rs`](../../crates/desktop/src/odds.rs). Decisions: D-081, D-084.

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
| Duos | Each team's two boards fight in turn; the teammate's board comes in when the first is gone. | not yet: the saved Duos legs (section 7) do not show the starting boards of the second legs clearly; needs the T-213 data |
| Hero powers, trinkets, Deity, spells cast in combat | Not on the board. | not yet |

## 4. Cards

- **Registry keyed by exact card id** (`cards.rs`). Golden cards have their own ids (`BG25_354_G`, `TB_BaconUps_045`), so a golden effect is written out, never "double the normal one" by guess.
- **Two lists:** `NO_COMBAT_EFFECT` (text never changes a combat: shop, hand, Tavern, teammate, or keywords the board already shows; checked card by card) and `EFFECTS` (built from a few primitives: summon, buff all, buff random, gain stats or a keyword, give a keyword, damage, strip keywords, destroy the killer; triggers Deathrattle, Rally, Avenge, start of combat, friendly attacks, after attack, plus static ones such as Cleave, Taunt while shielded, extra Deathrattles, shield at 6 Attack).
- **Fidelity:** each effect is `Exact` or `Approximate(reason)`. A card in neither list is **not modelled**: `simulate` returns `Verdict::NotModelled` with the card ids, never a number. The harness alone may play unknown cards as plain minions, and says so.
- **Tokens** (`TOKENS`): id, printed stats, keywords, tribe, tier.

## 5. Randomness and trials

- SplitMix64, seeded (`rng.rs`): the same seed gives the same odds on every platform, WebAssembly included, so a bug report can be replayed.
- Default 10 000 trials: the standard error of a share is at most 0.5 percentage point.
- `Odds`: won, tied, lost; damage dealt and taken as counts per value (when every tier is known); trials with unknown damage; trials stopped at the turn limit (1 000 attack turns, counted as ties).

## 6. Speed

Release build on the user's PC, the 19 logged Solo combats, 10 000 trials each: **32 ms on average, 73 ms at most** (target: under 200 ms). One thread, no allocation-heavy work per attack beyond small vectors; fast enough to run on every board change in the overlay.

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
| Duos first legs | 78 | 43 | 313 of 1 306 |

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

### 7.3 The overlay's inputs (T-605)

The overlay does not have the board the opponent brings to the combat: it has the last one the player saw, often several turns old, and the player's own board in the shop. `combat-validate` scores that too: for each Solo combat, the player's board against the board of the same opponent from the last earlier combat with them, cards not modelled refused as in the app.

| Solo combats | No board of that opponent seen before | Not modelled | Scored | Mean p(real result) | Most likely = real | Board age (mean) |
|---|---|---|---|---|---|---|
| 19 | 14 | 0 | 5 | 40% | 2 of 5 | 7 rounds |

- **The live odds are far weaker than the engine**: on these 5 combats the board the player last saw was 7 rounds old on average, and the most likely result was right 2 times in 5. The overlay says how old the board is ("their board seen turn 6") on every number.
- The proxy is generous on one side and strict on the other: it uses the player's board as it entered the combat (after start of combat), where the app has the shop board (before it).
- 5 combats is far too few to give a number to the user as a promise; the harness gives it again as more Solo games are logged.

## 8. One engine for the app and the website

**App (T-605, done in session 071, D-084).** During a shop turn the overlay runs the player's board as it is now against the next opponent's last board seen (D-004: only boards the player saw on screen; the opponent is the one the game marks as next, D-061). How:

- **Inputs.** The parser gives the own board in a shop turn (`own_board_now`, with Poisonous and Mega-Windfury); the opponent's board is the newest that entered play in a combat the game has shown (the live view follows the screen, D-067). Tavern tiers come from the leaderboard (the opponent's as it is now). The card facts the board does not show (tier, types, the card's own stats and keywords for Reborn) come from the card data: `crates/cards` keeps them for every Battlegrounds minion (saved format 5).
- **Running.** On its own thread, never the window's: the tracker's loop says what the odds should be worked out from; the thread waits until it has not changed for 300 ms, then runs 10 000 trials in batches of 1 000, stopping early after 150 ms (about 30 ms is usual). The same boards give the same odds (fixed seed). During the combat on screen the shop's odds stay.
- **No number when it cannot be trusted.** Duos ("not in Duos yet", T-608), no board of that opponent seen yet, a card the engine does not know ("a card is not modelled", Advanced names it), the card data missing for a card, boards it cannot read. No next opponent marked: nothing shown.
- **Approximate.** Start of combat is off: the opponent's board was taken after theirs, the player's shop board has not had them yet. When a card on either board acts at the start of combat, or a card is approximate, the age line starts with "≈" and Advanced names the cards.
- **Shown.** Simple, under the next opponent's line: Win, Tie and Lose with their shares, and "their board seen turn N". Advanced, in the next opponent's block: the same, the damage both ways (10th to 90th percentile and the median, after the damage cap; "unknown" when a tier is unknown), the approximate and missing cards and the trials. English and Spanish. A switch in Settings turns it off (on by default; `odds_off` in `overlay.json`).
- **How good it is:** section 7.3.

**Website (T-606), three ways:**

| Option | For | Against |
|---|---|---|
| **Rust → WebAssembly** (recommended) | One engine: the website's odds are the app's odds by construction; fast (tens of ms); runs in the browser, so no server cost and no upload of boards | The site's CSP needs `'wasm-unsafe-eval'` in `script-src` (today `'self'` or `'none'`, `web/src/lib/config.ts`); a build step for the `.wasm` file (no new crate needed: the engine has no dependencies; `wasm-bindgen` or a hand-written C ABI would be one new build dependency, R2) |
| TypeScript port | No CSP change, no build step | Two engines to keep in step: every rule and card twice; differences would show as different odds in the app and on the site |
| Server function (Supabase Edge, Deno with WebAssembly) | No CSP change on the site | Server cost and rate limits; every board sent to the server; still needs the WebAssembly build |

Recommendation: WebAssembly in the browser, with `'wasm-unsafe-eval'` added only on the simulator page if the site can set the CSP per page (else site-wide). This is the user's decision (CSP change).

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

## 10. Questions for the user

1. **How the website runs the engine.** WebAssembly (needs `'wasm-unsafe-eval'` in the CSP), a TypeScript port, or a server function. Recommendation: WebAssembly, the CSP change limited to the simulator page if possible.
2. **Which boards the website's simulator may load.** Only your own games, or also other public players' boards. Recommendation: your own games only at first (your boards and the opponents' boards you met, which you saw on screen); other players' boards only from public profiles and only after T-509's privacy texts cover it.
3. **Duos odds (T-608).** You mostly play Duos, and the overlay says "not in Duos yet" there. The replay now checks Duos first legs; the engine needs the second legs (the teammate's board coming in) to give Duos odds. Recommendation: T-608 next.

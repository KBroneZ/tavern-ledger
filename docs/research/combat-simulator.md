# Combat simulator (F6, T-601 to T-603)

Session 068, 2026-10-11. The user asked for (1) the odds to win, tie or lose a combat, and the damage, before each combat in the app, from the boards the player has seen, and (2) an interactive simulator on the website that can load old boards or pairings. This page is the design (T-601), what the engine core does today (T-602), how it is checked against the user's own combats (T-603) and the card coverage (first round of T-604).

Code: [`crates/combat-sim`](../../crates/combat-sim). Decision: D-081.

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
| `POISONOUS`, `MEGA_WINDFURY` tags | The parser does not read them; both last all game, so the harness takes them from the card's own keywords. A parser change (T-604) would be better. |
| Stats a card's text grows with the game (Tasty Lobster, Hopebringer) | Printed values are used: "approximate". |
| The player's tavern tier | Damage needs it; unknown means damage unknown, never 0. |
| Who attacked first | Not saved; the engine plays the rule (section 3). |

## 3. Combat rules

Status: **done** = in the engine and tested; **to check** = in the engine, to be confirmed against the attack order in the logs (T-603b); **not yet** = not in the engine.

| Rule | How the engine plays it | Status |
|---|---|---|
| Who attacks first | The side with more minions; equal counts, a coin flip. Counted before start of combat. | done; whether the count is taken before or after start of combat: to check |
| Attack order | Each side keeps a pointer; its next attacker is the left-most minion from the pointer with Attack above 0, wrapping to the left-most. Sides alternate; a side with no attacker passes; neither: tie. | done |
| Pointer after deaths and summons | The pointer is an index that moves with removals and summons to its left. A minion summoned in the attacker's place (its Deathrattle) attacks next. | to check |
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
| Start of combat | Off for logged boards (already applied). On for boards taken in the shop: modelled effects play (first attacker's side first, left to right); unmodelled ones make the result "not modelled". | partly: few cards |
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

## 7. Validation on the user's combats (T-603)

`combat-validate --history %APPDATA%\TavernLedger\games.jsonl --cards cards.json` plays every logged Solo combat and prints counts, shares and card ids only (no names, player ids or heroes). The user's logs stay on the PC; the repo has synthetic fixtures only (`crates/combat-sim/tests/data`).

Run of 2026-10-11 (41 games: 2 Solo, 39 Duos):

| Combats | n | Mean p(real result) | Brier | Most likely = real |
|---|---|---|---|---|
| all | 19 | 96% | 0.058 | 18 of 19 |
| exact | 13 | 94% | 0.085 | 12 of 13 |
| approximate | 6 | 100% | 0.000 | 6 of 6 |

- Calibration: 18 of 19 combats were predicted at 80% or more for the real result, and all 18 happened. The one other: a tie the engine gave 26% (won 74%), every card exact. Its opponent had a Stealthed Taunt minion; that led to the Stealth fix above but did not change the odds, so another rule differs. Finding it needs the combat's attack order (T-603b).
- **Too few combats to call the engine accurate:** 19 Solo combats, most one-sided. Duos rounds (449, 119 with every board and a result) are not scored until Duos is modelled.
- **T-603b, the attack log:** to "replay the logged attack order and find the first rule that differs", the parser needs to save each combat's attacks (attacker and target by side and position, in order) as the game plays them. That is a small export in `crates/bg-parser` (tell the hub first, per the session rules); then the harness can replay them step by step.

## 8. One engine for the app and the website

**App (T-605).** `crates/desktop` uses the crate directly. During the recruit phase the overlay runs the player's latest board against the next opponent's last board seen (D-004 allows odds from boards the player saw; `GAME_SEED` is no use, see the plan's T-111 notes). Needs: (1) the card facts in `crates/cards` (today's catalog keeps names, and tier and tribes for pool minions only; it needs Attack, Health, keywords and the tokens' facts, a catalog format bump); (2) the opponent's last board was taken after its start-of-combat effects (temporary buffs included), and the player's own board is taken in the shop (before them), so start of combat is on for one side and off for the other: the first app version shows odds as "approximate" whenever either board has a start-of-combat card; (3) "not modelled" shows as a short line, never a number; (4) the damage cap the overlay already shows (D-072) is applied to the damage.

**Website (T-606), three ways:**

| Option | For | Against |
|---|---|---|
| **Rust → WebAssembly** (recommended) | One engine: the website's odds are the app's odds by construction; fast (tens of ms); runs in the browser, so no server cost and no upload of boards | The site's CSP needs `'wasm-unsafe-eval'` in `script-src` (today `'self'` or `'none'`, `web/src/lib/config.ts`); a build step for the `.wasm` file (no new crate needed: the engine has no dependencies; `wasm-bindgen` or a hand-written C ABI would be one new build dependency, R2) |
| TypeScript port | No CSP change, no build step | Two engines to keep in step: every rule and card twice; differences would show as different odds in the app and on the site |
| Server function (Supabase Edge, Deno with WebAssembly) | No CSP change on the site | Server cost and rate limits; every board sent to the server; still needs the WebAssembly build |

Recommendation: WebAssembly in the browser, with `'wasm-unsafe-eval'` added only on the simulator page if the site can set the CSP per page (else site-wide). This is the user's decision (CSP change).

## 9. Card coverage (T-604, round 1)

Every minion seen entering a combat in the user's history (Solo and Duos): **534 card ids, 7 907 appearances**.

| Class | Cards | Appearances |
|---|---|---|
| No combat effect (checked) | 399 | 5 920 (75%) |
| Modelled, exact | 51 | 971 (12%) |
| Modelled, approximate | 33 | 639 (8%) |
| Not modelled | 51 | 377 (5%) |

The full list with classes is printed by `combat-validate`. Next round, most seen first: `BG36_241` (Rally casts a spell), `BG33_155` (gains when another Demon deals damage), `BG34_140` (summons from the hand), `BG36_208` (triggers a Deathrattle after a Rally attack), `BG28_583` (Blood Gem chain), `BG31_999` (start of combat destroy, copy on death), `BG36_515`, `BG31_835`, `BG36_360t9`, `TB_BaconShop_HP_105t`, `BG34_403`.

## 10. Questions for the user

1. **How the website runs the engine.** WebAssembly (needs `'wasm-unsafe-eval'` in the CSP), a TypeScript port, or a server function. Recommendation: WebAssembly, the CSP change limited to the simulator page if possible.
2. **Which boards the website's simulator may load.** Only your own games, or also other public players' boards. Recommendation: your own games only at first (your boards and the opponents' boards you met, which you saw on screen); other players' boards only from public profiles and only after T-509's privacy texts cover it.

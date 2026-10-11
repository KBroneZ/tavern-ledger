# Session 071: Combat odds in the overlay, attack-order replay, card coverage round 2 (T-605, T-603b, T-604)

Model: opus
Effort: high
Subagents: haiku for searches; sonnet for code; opus for the review

Work in your worktree **`C:\Users\andia\tavern-ledger\.claude\worktrees\071-overlay-odds`** (branch `071-overlay-odds`, created by the hub) and follow its `CLAUDE.md`. Chat in caveman mode; the user writes in Spanish, answer in English. Everything in the repo is in English except the Spanish strings. **Tier R1**: TDD + `/code-review`.

**Other sessions at the same time:** 069 (leaderboards: `supabase/`, `web/`) and 070 (heroes and curious stats pages, `web/`). Your area: `crates/combat-sim/`, `crates/bg-parser/` (T-603b and the two keyword tags only), `crates/cards/` (card facts), `crates/tracker/` and `crates/desktop/` (the overlay odds). Nothing in `web/`, `supabase/`. If a parser change touches the uploaded report's keys, stop and tell the hub first (the upload contract test is `crates/bg-parser/tests/upload_contract.rs`; the data API reads those keys). Shared files: only your rows and notes; decision number if needed **D-084**. Every changed text file LF (`git ls-files --eol`, index side). To catch up: `git fetch origin && git merge origin/main`; no rebase, never force-push.

- **Never show a test build's overlay over the user's game.** Keep the D-068 guard on, never pass `--allow-over-game`, check `Get-Process Hearthstone` before every visual test, prefer headless renders with stubbed Tauri. Never move the user's mouse.
- Never stop or restart the user's desktop app (the hub rebuilds it). After a build, `git checkout -- crates/desktop/gen/schemas`. CI does not build `crates/desktop` nor run its Node UI tests: run them yourself and paste counts. If you bump `PARSER_REVISION`, say so (the hub reparses).
- The user is asleep: do not wait on them; questions go in your hand-over. `gh` is not available: push the branch and message the hub (`tavern-ledger-2b`, `ListAgents`, use its `[ref]`) with the branch, PR title and a short PR body.

## What the user asked (2026-10-11)

"A combat simulator like Deck Tracker's" in the app: before each combat, the odds to win, tie or lose and the damage, from the boards the player has seen. Session 068 (#125, D-081) built the engine (`crates/combat-sim`), the harness (`combat-validate`) and the design (**read `docs/research/combat-simulator.md` and D-081 first**): 18 of 19 logged Solo combats called right, 30 ms for 10 000 trials, 95% of seen minion appearances modelled.

## Clean room (absolute rule 1)

Never read or copy the code of HDT, Bob's Buddy, Firestone, `@firestone-hs/simulate-bgs-battle`, HearthMirror or Nomi's Kitchen. Their public docs and articles are fine; MIT or Apache code only with credit in `PROVENANCE.md`. Card rules from card texts, public notes and the user's own logs.

## Task

1. **T-603b attack order:** export each combat's attack order from `bg-parser` (a small, tested addition; not in the uploaded report unless the hub agrees) and make the harness replay it to name the first rule that differs for each miss.
2. **T-604 round 2:** the parser reads `POISONOUS` and `MEGA_WINDFURY`; model the next cards on the design doc's list, most seen first; rerun the harness and report (counts and card ids only).
3. **T-605 overlay odds:** during the recruit phase, run the engine on the player's current board against the next opponent's last seen board (both from the log, D-004: only boards the player saw; the next opponent from 052's logic) and show win / tie / lose percentages and the likely damage both ways in the overlay, small, near the next-opponent line; "not modelled" or "no board seen yet" instead of a number when it cannot be trusted; the age of the opponent's board ("seen turn 6"). Run off the UI thread, debounced on board changes, within a time budget. Simple: the three percentages; Advanced: damage spread and the cards that are approximate or missing. English and Spanish. A Settings switch to turn it off (on by default).
4. Accuracy check for the overlay path: the harness numbers for the same inputs the overlay would use (the opponent's board as last seen, not at combat), so the user knows how good the live odds are; put them in the design doc.
5. Plan rows T-603b, T-604, T-605 done or what is left; D-084 if needed; README.

## Acceptance criteria

- [ ] Odds in the overlay from seen boards only, honest fallbacks, EN and ES, off switch; tests.
- [ ] Attack-order replay in the harness; round 2 coverage with the new harness numbers.
- [ ] `cargo test --workspace`, desktop Node UI tests and a release build pass locally (paste counts); headless renders of the odds in Simple and Advanced at 3840x2160@150% and 1920x1080@100%.
- [ ] `/code-review` with no open CRITICAL or HIGH; branch handed to the hub.

## Red line

D-004 (as amended by D-063), D-006, D-012, D-022, D-054 apply: odds only from boards the player saw, never from hidden information; no memory reading beyond D-063. Never print BattleTags, `GameAccountId`, player names or the user's email.

# Session 074: Duos combats in the simulator and Duos odds in the overlay (T-608)

Model: opus
Effort: high
Subagents: haiku for searches; sonnet for code; opus for the review

Work in your worktree **`C:\Users\andia\tavern-ledger\.claude\worktrees\074-duos-odds`** (branch `074-duos-odds`, created by the hub) and follow its `CLAUDE.md`. Chat in caveman mode; the user writes in Spanish, answer in English. Everything in the repo is in English except the Spanish strings. **Tier R1**: TDD + `/code-review`.

**Other sessions at the same time:** 072 (community hero stats: `supabase/`, `web/`). Your area: `crates/combat-sim/`, `crates/bg-parser/` (Rust-only additions the engine needs), `crates/cards/`, `crates/tracker/`, `crates/desktop/` (the overlay odds). Nothing in `web/`, `supabase/`. If a parser change touches the uploaded report's keys, stop and tell the hub first (`crates/bg-parser/tests/upload_contract.rs`). Shared files: only your rows and notes; decision number if needed **D-087** (072 has D-085, 073 has D-086). Every changed text file LF (`git ls-files --eol`, index side). To catch up: `git fetch origin && git merge origin/main`; no rebase, never force-push.

- **Before you hand over, run the tests from a clean clone of your pushed branch** (`git clone --branch 074-duos-odds <origin> <scratch dir>` then `cargo test --locked --workspace --exclude desktop`). Two sessions in a row left a fixture out because `.gitignore` ignores `bin/` and `*.log`; check `git ls-files` for every new file.
- **Never show a test build's overlay over the user's game.** Keep the D-068 guard on, never pass `--allow-over-game`, check `Get-Process Hearthstone` before every visual test, prefer headless renders with stubbed Tauri. Never move the user's mouse.
- Never stop or restart the user's desktop app (the hub rebuilds it). After a build, `git checkout -- crates/desktop/gen/schemas`. CI does not build `crates/desktop` nor run its Node UI tests: run them yourself and paste counts. If you bump `PARSER_REVISION`, say so (the hub reparses).
- The user may be busy: do not wait on them; questions go in your hand-over. `gh` is not available: push the branch and message the hub (`tavern-ledger-2b`, `ListAgents`, use its `[ref]`) with the branch, PR title and a short PR body.

## Why

The user plays mostly Duos, and the overlay odds (T-605, #130, D-084) show "not in Duos yet". Read **`docs/research/combat-simulator.md`** (sections 7 to 9), D-081, D-084, and the Duos rules in the plan: T-213 and **D-054** (the teammate's combats the game does not play back may be used **after the game only**, never in the overlay or as the live "last board seen"), D-068, D-080 (team order in Duos).

## Clean room (absolute rule 1)

Never read or copy the code of HDT, Bob's Buddy, Firestone, `@firestone-hs/simulate-bgs-battle`, HearthMirror or Nomi's Kitchen. Their public docs and articles are fine; MIT or Apache code only with credit in `PROVENANCE.md`. Rules from card texts, public notes and the user's own logs.

## Task

1. **Duos combat rules in the engine:** how a Duos combat runs (which board of each team fights first, when the second board joins, how damage and the shared health work, what carries over between the two legs, the "pass" and teammate effects that act in combat), from public descriptions and checked against the user's logged Duos combats; written up in the design doc.
2. **Harness for Duos:** score the user's logged Duos combats (78 first legs today in the logs on the PC, more in `games.jsonl`) with both legs; replay attack by attack and name the first differing rule; report counts and card ids only. Model the next cards the Duos data needs, most seen first.
3. **Duos odds in the overlay:** the same line as Solo (Win / Tie / Lose, the board's age, Advanced with damage and approximate cards) for the next opposing team, from boards the player saw on screen only: the player's own board, the teammate's board as the game shows it to the player, and the opposing team's boards from combats the game played on screen (D-054: never a hidden `SETASIDE` copy). When a needed board was never seen, say which one is missing instead of a number. EN and ES.
4. Accuracy of the Duos overlay inputs (as 071 did for Solo) in the design doc.
5. Plan row T-608 done or what is left; D-087 if needed; README.

## Acceptance criteria

- [ ] Duos combats in the engine and the harness, with numbers on the user's logged Duos combats.
- [ ] Duos odds in the overlay from on-screen boards only (D-054), honest fallbacks, EN and ES; tests.
- [ ] Tests green from a clean clone; desktop Node UI tests and a release build pass locally (paste counts); headless renders at 3840x2160@150% and 1920x1080@100%.
- [ ] `/code-review` with no open CRITICAL or HIGH; branch handed to the hub.

## Red line

D-004 (as amended by D-063), D-006, D-012, D-022, **D-054** apply: odds only from boards the player saw on screen. Never print BattleTags, `GameAccountId`, player names or the user's email.

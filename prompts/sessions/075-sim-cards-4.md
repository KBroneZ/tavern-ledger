# Session 075: Simulator card coverage round 4, combat.rs split, accuracy export (T-604, T-607 data)

Model: opus
Effort: medium
Subagents: haiku for searches; sonnet for code and the review

Work in your worktree **`C:\Users\andia\tavern-ledger\.claude\worktrees\075-sim-cards-4`** (branch `075-sim-cards-4`, created by the hub) and follow its `CLAUDE.md`. Chat in caveman mode; the user writes in Spanish, answer in English. Everything in the repo is in English. **Tier R1**: TDD + `/code-review`.

**Other sessions at the same time:** 073 (the website in Spanish, `web/`). Your area: `crates/combat-sim/` and `crates/cards/` only (read `crates/bg-parser/`, `crates/tracker/`, `crates/desktop/`; do not change them). Nothing in `web/`, `supabase/`. Shared files: only your rows and notes; decision number if needed **D-088**. Every changed text file LF (`git ls-files --eol`, index side). To catch up: `git fetch origin && git merge origin/main`; no rebase, never force-push.

- **Before you hand over, run the tests from a clean clone of your pushed branch** (`cargo test --locked --workspace --exclude desktop`); `.gitignore` ignores `bin/` and `*.log`, so check `git ls-files` for every new file.
- Do not build or run the user's desktop app (the hub does).
- The user may be busy: do not wait on them. `gh` is not available: push the branch and message the hub (`tavern-ledger-2b`, `ListAgents`, use its `[ref]`) with the branch, PR title and a short PR body.

## Clean room (absolute rule 1)

Never read or copy the code of HDT, Bob's Buddy, Firestone, `@firestone-hs/simulate-bgs-battle`, HearthMirror or Nomi's Kitchen. Their public docs and articles are fine; MIT or Apache code only with credit in `PROVENANCE.md`. Rules from card texts, public notes and the user's own logs.

## Why

The overlay odds work in Solo (#130, D-084) and Duos (#133, D-087). Every card the engine does not know blocks the odds for that combat. Read **`docs/research/combat-simulator.md`** (sections 7 to 10), D-081, D-084, D-087.

## Task

1. **Card round 4:** with `combat-validate` on the user's history and logs, list the cards still not modelled by how often they block a combat, and model the most frequent ones (with golden versions), each with a test and its replay checked. **Keep the hand summoners blocking** (`BG34_140`, `BG31_835`, `BG21_HERO_030t`; the hub's call on section 10 question 4, the recommendation); say how many combats they still block. Rerun the harness: blocked share and accuracy before and after (counts and card ids only).
2. **Split `combat.rs`** (1 024 lines; the rule is under 800) into cohesive modules with no behaviour change; tests prove it.
3. **Accuracy export for T-607:** a `combat-validate` option that writes the harness numbers (per mode: combats scored, blocked share, mean p(real result), most-likely-right count, calibration buckets, the date and engine version) as a small JSON with **no** card-by-player detail, no names, no ids beyond card ids; documented so a later web page can show it. Do not commit the user's numbers as a fixture; a synthetic example only.
4. Plan rows T-604 (round 4), T-607 (data part), D-088 if needed; design doc numbers; README.

## Acceptance criteria

- [ ] Round 4 cards with tests; blocked share and accuracy before and after.
- [ ] `combat.rs` split, every file under 800 lines; behaviour unchanged.
- [ ] Accuracy JSON export documented, with a synthetic example.
- [ ] Clean-clone tests green (paste counts); clippy, fmt; `/code-review` with no open CRITICAL or HIGH; branch handed to the hub.

## Red line

Clean room above. D-004 (as amended by D-063), D-006, D-054 apply. Never print BattleTags, `GameAccountId`, player names or the user's email.

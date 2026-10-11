# Session 067: Duos hover shows the wrong teammate's board for opposing teams (T-343)

Model: opus
Effort: medium
Subagents: haiku for searches; sonnet for code and the review

Work in your worktree **`C:\Users\andia\tavern-ledger\.claude\worktrees\067-duos-hover-order`** (branch `067-duos-hover-order`, created by the hub) and follow its `CLAUDE.md`. Chat in caveman mode; the user writes in Spanish, answer in English. Everything in the repo is in English except the Spanish strings. **Tier R1**: TDD + `/code-review`.

**Other sessions at the same time:** 065 and 066 (website, `web/` only). Your area: the overlay's hover (`crates/desktop/src/hover.rs`, `leaderboard.rs`, `overlay*.rs`, `crates/desktop/ui/overlay*`) and `crates/tracker/` if the team order comes from there. Nothing in `web/`, `supabase/`. Shared files: only your task's row and notes; decision number if needed **D-080**. Every changed text file LF (`git ls-files --eol`, index side).

- **Never show a test build's overlay over the user's game.** Keep the D-068 guard on, never pass `--allow-over-game`, check `Get-Process Hearthstone` before any visual test, prefer headless renders with stubbed Tauri. Never move the user's mouse.
- Never stop or restart the user's desktop app (the hub rebuilds it). After a build, `git checkout -- crates/desktop/gen/schemas`. CI does not build `crates/desktop` nor run its Node UI tests: run them yourself and paste counts.
- To catch up with `main`: `git fetch origin && git merge origin/main`. No rebase, never force-push.
- The user is asleep: do not wait on them. `gh` is not available: push the branch and message the hub (`tavern-ledger-2b`, `ListAgents`, use its `[ref]`) with the branch, PR title and a short PR body.

## What the user said (2026-10-11, after session 064)

Asked: "hover the top portrait of an opposing team; does our board across the top show that same hero?" Answer: **"in an enemy team they appear the other way round: one hero's board is shown for the other"** (the minions). So for opposing teams the code's "first seat = top" rule (session 059, T-340; see the T-343 row and notes, D-068, D-072) is backwards. Earlier answer: in the user's own team their own hero is the top portrait, though they hold the higher player id in 33 of 39 Duos games.

## Task

1. Read the T-340 and T-343 rows and notes, D-068, D-072, `docs/research/overlay-hover.md` (sessions 058, 059, 064) and how the hover picks the hero for a portrait.
2. Work out the rule the game uses for a team's two portraits that fits both answers (own team: own hero on top; opposing teams: the reverse of today's choice), using the user's Duos logs (tag names, ids and counts only; never player names). If one rule explains both, use it; if the data only supports "swap for opposing teams", do that and say so.
3. Fix the hover so hovering a portrait shows that hero's board; tests with synthetic Duos fixtures for both orders.
4. Plan row T-343 done (or what is left), notes, D-080 if a rule is decided; one PR.

## Acceptance criteria

- [ ] Opposing team's top portrait shows its own hero's board; tests.
- [ ] `cargo test --workspace`, desktop Node UI tests and a release build pass locally (paste counts); headless render of a Duos hover.
- [ ] `/code-review` with no open CRITICAL or HIGH; branch handed to the hub; ask the hub to have the user confirm in their next Duos game.

## Red line

D-004 (as amended by D-063), D-006, D-012, D-022, D-054 apply. Never print BattleTags, `GameAccountId`, player names or the user's email.

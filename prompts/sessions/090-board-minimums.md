# Session 090: Leaderboards with no minimum games (T-534)

Model: opus
Effort: medium
Subagents: haiku for searches; sonnet for code and the review

Work in your worktree **`C:\Users\andia\tavern-ledger\.claude\worktrees\090-board-minimums`** (branch `090-board-minimums`, created by the hub) and follow its `CLAUDE.md`. Chat in caveman mode; the user writes in Spanish, answer in English. Everything in the repo is in English. **Tier R2** (production database, public data): TDD + `/code-review` + `/security-review`.

**Other sessions at the same time:** 086 (combat replay, app crates), 087 (Dark Gifts, simulator crates and the wasm), 089 (wording review of `web/`, told to leave the leaderboards to you). **Your area:** the leaderboards only: `supabase/migrations/` (a new migration), `supabase/tests/`, `web/src/lib/leaderboards/` and the leaderboards pages. Not the community hero stats (`community_heroes`, `/heroes/`). Decision **D-103**. Every changed text file LF. To catch up: `git fetch origin && git merge origin/main`; no rebase, never force-push.

- Migrations deploy on merge (D-031). Never run anything against production; local stack only.
- The user may be busy: do not wait on them. Push the branch and message the hub (`tavern-ledger-2b`, `ListAgents`, use its `[ref]`) with the branch, PR title and a short PR body.

## Why

The user (2026-10-12), looking at `/es/leaderboards/` where every board says "Aún no hay nadie: ningún jugador público tiene 20 partidas terminadas en los últimos 30 días": "remove the minimum from all the rankings; we have no users yet, show whatever there is." D-082 set at least 20 finished games per board (10 with the hero on the per-hero board).

## Task

1. A migration that drops the minimum on every leaderboard board (any public profile with at least one finished game in the mode and period shows), keeping every other D-082 rule: public profiles only, the display-name filter, the impossible-value exclusions, the one switch for profile and leaderboards (D-089), `noindex`. If the minimum lives as a single constant, make it one value that can be raised again later.
2. The web: each row shows its number of games next to the value, and rows below the old minimum are marked "few games" (do not hide them), so a 1-game 100% win rate is read for what it is; ranking order stays by the board's value, ties by more games. Empty boards say so plainly in EN and ES ("No public players yet" style; no mention of a minimum).
3. pgTAP and web tests for the new rule, the mark and the empty state.
4. **Leave the community hero stats (`/heroes/`, D-083's 50 games from 5 players) unchanged:** that minimum protects the privacy of players who are not public. Say so in D-103.
5. D-103 (amends D-082's minimum), plan row T-534, README.

## Acceptance criteria

- [ ] Every board shows public players with one or more finished games; few-game rows marked; games shown per row.
- [ ] Other D-082 rules and the community stats unchanged; no new grants; RLS unchanged.
- [ ] pgTAP and web tests green from a clean clone of the pushed branch (paste counts); `/code-review` and `/security-review` with no open CRITICAL or HIGH; branch handed to the hub.

## Red line

D-004 (as amended by D-063), D-006. Never print BattleTags, `GameAccountId`, display names of real players, keys or the user's email. Nothing run against production.

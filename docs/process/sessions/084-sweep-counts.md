# Session 084: The daily sweep records its stats backfill (T-524)

Model: opus
Effort: medium
Subagents: haiku for searches; sonnet for code and the review

Work in your worktree **`C:\Users\andia\tavern-ledger\.claude\worktrees\084-sweep-counts`** (branch `084-sweep-counts`, created by the hub) and follow its `CLAUDE.md`. Chat in caveman mode; the user writes in Spanish, answer in English. Everything in the repo is in English. **Tier R2** (production database): TDD + `/code-review` + `/security-review`.

**Other sessions at the same time:** 082 (brand assets: `docs/brand/`, `web/`, `crates/desktop/icons/`) and 083 (simulator cards: `crates/combat-sim/`, `crates/cards/`, the wasm and accuracy data in `web/`). Your area: `supabase/` only. Shared files: only your rows and notes; decision number if needed **D-096**. Every changed text file LF (`git ls-files --eol`, index side). To catch up: `git fetch origin && git merge origin/main`; no rebase, never force-push.

- Migrations deploy on merge (D-031). Never run anything against production; use the local stack only (`npx supabase start`, `db reset`, `test db`).
- The user may be busy: do not wait on them. Push the branch and message the hub (`tavern-ledger-2b`, `ListAgents`, use its `[ref]`) with the branch, PR title and a short PR body.

## Why

Most of the user's games showed no rounds, combats, triples or APM on the website (T-512) because the stats backfill runs only in the daily sweep, and it had not run since T-502 deployed. The hub ran the sweep by hand on 2026-10-11 and it filled 41 of 41 games. But `private.sweep_runs` records only the housekeeping counts (`auth_events`, `upload_events`, `ip_failures`), so nobody can tell later whether a run backfilled anything or failed part-way. Read `supabase/migrations/20261010090000_sweep_schedule.sql`, the later migrations that touch the sweep or the backfill (T-502), D-031 and the deploy notes on the sweep.

## Task

1. Every sweep run records in its `sweep_runs` row how many games the backfill looked at, filled and skipped (skips as counts by reason), next to the housekeeping counts. A step that fails leaves a row that says so, not silence (empty ≠ zero). The table stays private (no grants); the 90-day pruning stays.
2. If the backfill can run on its own (`run_sweep` or similar), its result reports the same counts.
3. pgTAP tests in `supabase/tests/` for the new counts and the failure row; all database tests green on the local stack.
4. Document how to read the last runs (one read-only SQL line for the SQL editor). Plan rows: T-524 new; T-512 from "explained" to done (the manual run filled 41 of 41). README.

## Acceptance criteria

- [ ] Migration with the counts and the failure record; no grants added; RLS unchanged.
- [ ] pgTAP tests green locally (paste counts); `/code-review` and `/security-review` with no open CRITICAL or HIGH; branch handed to the hub.

## Red line

D-004 (as amended by D-063), D-006. Never print BattleTags, `GameAccountId`, keys or the user's email. No dashboard changes; nothing run against production.

# Session 098: Storage alert in the daily sweep (T-543)

Model: opus
Effort: high
Subagents: haiku for searches; sonnet for code and the review

Work in your worktree **`C:\Users\andia\tavern-ledger\.claude\worktrees\098-storage-alert`** (branch `098-storage-alert`, created by the hub) and follow its `CLAUDE.md`. Chat in caveman mode; the user writes in Spanish, answer in English. Everything in the repo is in English. **Tier R2** (migration on the hosted database, deployed on merge by D-031): TDD, `/code-review` and `/security-review`.

**Other sessions at the same time:** 092 (website home page, `web/`) and 099 (combat replay on the website's game viewer, `web/src/lib/game/`). Your area: `supabase/migrations/` (one new migration), `supabase/functions/sweep/`, `supabase/tests/`, `docs/research/deploy.md` 10.5, `docs/research/data-capacity.md` (a short "built" note). Decision **D-110** if a rule needs one. Every changed text file LF. To catch up: `git fetch origin && git merge origin/main`; no rebase, never force-push.

- The user may be busy: do not wait on them. Push the branch and message the hub (`tavern-ledger-c7`, `ListAgents`, use its `[ref]`) with the branch, PR title, a short PR body and the post-merge checks.
- No dashboard, plan or billing change; no new secret.

## Why

D-109 (the user, 2026-10-12): "the daily sweep records how full the hosted project is (file bytes, file count, database size) and flags 50 % and 80 % of the Free plan's limits." Session 097's numbers and limits are in `docs/research/data-capacity.md` (sections 2, 7 and 7.1: today about 13 MB of 500 MB database and 0.3 MB of 1 GB files). D-109 also says Pro comes at 80 % of any Free limit: the flag is what tells the hub.

## Task

1. Each sweep run records, next to its current counts in `private.sweep_runs`: database size (`pg_database_size`), the `games` bucket's file bytes and file count (from `storage.objects` metadata, as section 7's query), and the level reached against each limit: none, 50 % or 80 %. The limits live in one place (one SQL function or a small table), with the date and source they were read from, so a plan change is a one-line migration. Never block or slow the sweep's real work if the measurement fails: record the failure, as the other steps do.
2. The sweep's response carries the same numbers and level.
3. A read-only query in `deploy.md` 10.5 that the hub runs after the daily sweep to see the latest level; say there what the hub does at 50 % (tell the user) and at 80 % (tell the user it is time for Pro, D-109). Propose (do not build) whether a mail to the owner at 80 % is worth it later.
4. Tests: pgTAP for the migration and grants (nothing new readable by `anon`, `authenticated` or `service_role` beyond what the sweep already uses), Deno for the handler. Parity with the local stack (`supabase db reset`).
5. Plan row T-543, README line, D-110 only if you set a rule D-109 does not cover.

## Acceptance criteria

- [ ] Sweep records size, files and level; a failed measurement is recorded, never fatal.
- [ ] Limits in one place with source and date; grants unchanged for client roles.
- [ ] pgTAP and Deno green from a clean clone (paste counts); `/code-review` and `/security-review` with no open CRITICAL or HIGH; branch handed to the hub with the post-merge checks.

## Red line

Rules 6 and 7. Never print BattleTags, `GameAccountId`, keys or the user's email; no account, plan or billing change.

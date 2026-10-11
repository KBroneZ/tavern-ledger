# Session 063: Data API for the website (T-502)

Model: opus
Effort: high (schema and RLS), medium for the rest
Subagents: haiku for searches; sonnet for code; opus for the reviews (`database-reviewer`, `security-reviewer`)

Work in your worktree **`C:\Users\andia\tavern-ledger\.claude\worktrees\063-web-data-api`** (branch `063-web-data-api`, created by the hub) and follow its `CLAUDE.md`. Chat in caveman mode; the user writes in Spanish, answer in English. Everything in the repo is in English. **Tier R2 plus a second AI** (new tables, views and RLS that decide what is public): TDD + `/code-review` + `/security-review` + a second AI review (`/santa-loop` or `ask-chatgpt`).

**Another session runs at the same time:** 062 (web design system, `web/` only). To stay out of its way:
- Work only in your worktree. Never touch the main checkout or other worktrees.
- Your area: `supabase/` (migrations, database tests, the `upload-game` function if you fill summaries at upload), plus docs. Nothing in `web/`. Nothing in `crates/` unless the hub agrees first (the upload contract test lives in `crates/bg-parser/tests/upload_contract.rs`; if the report keys change, tell the hub).
- Shared files (`plan.md`, `README.md`, `DECISIONS.md`, `PROVENANCE.md`): only your task's row and notes. Your decision number is **D-071**, only if needed. Keep every changed text file LF (`git ls-files --eol`; check the index side, the checkout uses `core.autocrlf=true`).
- You may start Docker and the local Supabase stack for tests (062 does not use it). Stop it when you are done.
- To catch up with `main`: `git fetch origin && git merge origin/main`. No rebase, never force-push.

Working rules (user's decisions):
- Decide technical matters yourself. Ask the user, **through the hub** as well as in your tab: new dependencies, anything that makes more data public than today, production dashboard changes.
- Never touch production by hand (no SQL editor writes, no dashboard saves). Production changes only through merged migrations (D-031).
- `gh` is not available in task sessions. When your branch is ready (tests green, all reviews done, no open CRITICAL or HIGH), push it and message the hub (`tavern-ledger-d9`, find it with `ListAgents`, use its `[ref]`) with the branch, the PR title and a short PR body (summary, review results, test plan). The hub opens the PR, waits for CI and merges it.
- **Merging to `main` deploys the migrations and functions.** Afterwards check, and tell the hub: `/auth/v1/settings` still says `disable_signup: true`, functions answer `401` without a token, the validator answers its version (`POST …/functions/v1/upload-game` with the publishable key as `apikey` and bearer gives `405 … "validator": N`), and the new views answer as expected for an anonymous caller (nothing private).

## Why

Round 5 puts everything the app knows on the website (read **`docs/plan/web-master-plan.md`** first: "What we have", "Rules that shape the website", the information architecture). Today `public.games` holds one summary row per game and the full report is a gzipped file in Storage. The pages need stats computed on the server, never by downloading every report into the browser.

## At the start

1. Conversation title: **`#063 Web data API`**.
2. `git fetch origin && git merge origin/main`.
3. Read D-031, D-036, D-042, D-047, D-053, the migrations in `supabase/migrations/`, the database tests in `supabase/tests/database/`, `supabase/functions/upload-game/` (validator 6, `report_keys.json`), and a report's shape (`crates/bg-parser`, the report types; synthetic fixtures only).

## Task

1. **Per-game summary** with the numbers the pages need (place, hero, mode, date, rounds, damage dealt and taken, gold spent and beyond income, rolls, triples, buys and sells, APM, top tribes on the final board, Duos teammate hero; whatever the report carries, each with "known" or not, never a zero for unknown). Choose and record in D-071 how it is filled: at upload by `upload-game` (validated report → summary columns or a `game_stats` table), plus a **backfill** for the games already stored (a one-off path through the function or the sweep, service role only; nothing typed by hand in production).
2. **Per-user SQL views or functions** for the pages: overview (games, average place, top-4 / top-2, win rate, per mode, with game counts), heroes (per hero: games, average place, top-4 or top-2 rate, best and worst place), games list (filters: mode, hero, date range, place; paged), records (best streaks, biggest combat, the curious stats the data backs). Every value carries how many games back it.
3. **Privacy (absolute rule 6):** everything readable only by the owner, or by everyone when the profile `is_public`, exactly like `games` today. No new column with names, BattleTags, ratings or account ids. `security_invoker` views (or `security definer` functions with a fixed `search_path` and explicit checks), and tests that an anonymous caller and another user see nothing of a private profile. Leaderboards are **not** in this task (T-506 needs user decisions); design so they can be built on these summaries.
4. **Export and delete:** the new data is in the account export and goes away with account deletion and with the sweep's orphan clean-up; tests.
5. **Tests:** database tests in the existing style for every view and policy; function tests for the summary at upload; the CI jobs stay green. Docs: a short `docs/research/` or `supabase/` note on the API (what each view returns, how to call it from the site with the publishable key). Plan row **T-502** done with notes.

## Out of scope

Pages and the site's code (`web/`, T-503), leaderboards (T-506), the rating (never uploaded, D-063), opening sign-ups, the CSP.

## Acceptance criteria

- [ ] Summary filled at upload and backfilled for stored games; unknown kept as unknown.
- [ ] Per-user views with game counts; private profiles invisible to others; export and delete cover the new data; all tested.
- [ ] `/code-review`, `/security-review` and a second AI with no open CRITICAL or HIGH; branch handed to the hub; post-merge checks reported.

## Red line

Never print keys, tokens, the user's email or BattleTags. No production writes by hand. Nothing more public than today without the user's OK.

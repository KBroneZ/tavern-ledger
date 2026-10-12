# Session 043: Remove the functions' fallback to the legacy service key (T-104j)

Model: opus
Effort: medium
Subagents: haiku for searches; sonnet for code and reviews

Work in your worktree **`C:\Users\andia\tavern-ledger\.claude\worktrees\043-legacy-key-fallback`** (branch `043-legacy-key-fallback`, created by the hub) and follow its `CLAUDE.md`. Chat in caveman mode; the user writes in Spanish, answer in English. Everything in the repo is in English. **Tier R2** (production backend, keys): TDD + `/code-review` + `/security-review`.

**Two other sessions run at the same time:** 040 (log inventory: new files in `docs/research/`, `tools/`, `tests/`) and 041 (redesign: `.claude/skills/`, `crates/desktop/ui/`, `docs/research/redesign.md`). To stay out of their way:
- Work only in your worktree. Never touch the main checkout or other worktrees.
- Your area: `supabase/functions/` (the shared key helper and its callers), `supabase/migrations/` and `supabase/tests/` only if the sweep's caller needs a change, and `docs/research/deploy.md` sections 10.6 and 10.11. Nothing else.
- Shared files (`plan.md`, `README.md`, `DECISIONS.md`, `PROVENANCE.md`): only your task's row and notes. Your decision number is **D-053** (D-051 and D-052 are reserved by 040 and 041). Keep every changed text file LF (`git ls-files --eol` before committing).
- You may start Docker and the local Supabase stack this round (no other session uses it). Stop it when you are done.
- To catch up with `main`: `git fetch origin && git merge origin/main`. No rebase, never force-push.
- **Merging to `main` deploys the functions** (D-031, D-036). After the merge, check: `/auth/v1/settings` still says `disable_signup: true`; each function answers `401` without a token; each function's newest log line says it uses the new secret key; the next upload from the user's app is accepted (or, if no game is played, a test of the sweep's next scheduled run in `private.sweep_runs`).

Working rules (user's decisions):
- Decide technical matters yourself. Ask the user: accounts, accepting terms, paying, granting access between services, and **turning off legacy keys in Supabase** (that stays their call, after this PR; tell them exactly where, they may be on their phone).
- Never type, print or copy keys or secrets: not in chat, logs, files, commits or test fixtures (use made-up values that gitleaks does not flag; #72 needed a fix for that). Dashboard pages are read only for you; if a change is needed there, the user makes it or presses Save.
- **`gh` is not logged in inside task sessions.** When your branch is ready (tests green, both reviews done, no open CRITICAL or HIGH), push it and send the hub (`Hub prompts session 036`, find it with `ListAgents`, use its `[ref]`) a message with the branch, the PR title, a short PR body (summary, review results, test plan) and anything still waiting on the user. The hub opens the PR, waits for CI and merges it; then you run the post-merge checks above and report them the same way.

## Why

D-042 moved the Edge Functions to the new secret key (`SUPABASE_SECRET_KEYS`, `default`) and kept `SUPABASE_SERVICE_ROLE_KEY` (the legacy service key) as a fallback "until production logs show the new key, then the fallback goes". The user agreed (2026-10-10) to remove it, so that legacy keys can later be turned off and one less master key exists.

## Effort and models per phase

| Phase | Effort | Model |
|-------|--------|-------|
| Check production uses the new key | medium | Opus |
| Code and tests | medium | Sonnet |
| Docs, reviews | medium | `code-reviewer`, `security-reviewer` |

## At the start

1. Conversation title: **`#043 Legacy key fallback`**.
2. `git fetch origin && git merge origin/main`.
3. Read D-031, D-036, D-042, `docs/research/deploy.md` 10.5 and 10.6, `supabase/functions/_shared/service_key.ts` and its tests, every caller of it, the sweep schedule migrations (how `pg_cron` calls `sweep` and with which credential from Vault), and `supabase/config.toml`.

## Task

1. **Evidence first.** In the Supabase dashboard (read only; Edge Functions → each function → Logs), confirm each function's start line says `service key from SUPABASE_SECRET_KEYS (new secret key)`, with no `legacy fallback` lines since D-042 deployed. Record what you saw (counts and dates, no values) in `deploy.md` 10.6. If any function still falls back, stop and tell the hub and the user why before changing code.
2. **Everything else that still uses a legacy key.** Find every place the backend still sends or accepts a legacy JWT-style key: the `pg_cron` → `sweep` call and what Vault holds for it, `sweep`'s accepted credentials, any `apikey` header, the site's and the app's publishable key (it must already be the new `sb_publishable_…`), CI, `config.toml`. List them in D-053. Anything that would break when legacy keys are turned off gets fixed here, or, if it needs a dashboard change (for example a new Vault entry), the user is told exactly where.
3. **Remove the fallback.** `service_key.ts` reads only `SUPABASE_SECRET_KEYS` (`default`); a missing or broken value fails loudly at start (5xx with a clear log line, never a silent pass), so the app keeps games waiting instead of losing them. Update the tests (Deno), the log wording, and the comments.
4. Record in `deploy.md` 10.11 that DMARC aggregate reports are live (done by the hub on 2026-10-10: `dmarc@tavernledger.net` forwards like `contact@`, and the `_dmarc` record carries `rua=mailto:dmarc@tavernledger.net`; public DNS checked).
5. Plan row **T-104j** "Remove the functions' fallback to the legacy service key"; D-053 (what still used legacy keys, what was changed, what the user does to turn legacy keys off and how to roll back); README; same PR.

## Out of scope

Turning legacy keys off (the user's call, after the merge), any auth setting, the site, the desktop app, opening sign-ups.

## Acceptance criteria

- [ ] Production evidence that every function already uses the new key, recorded without values.
- [ ] No code path reads `SUPABASE_SERVICE_ROLE_KEY`; nothing in the backend needs a legacy key, or the user knows exactly what to change first.
- [ ] Deno and pgTAP tests green locally and in CI; gitleaks green.
- [ ] `/code-review` and `/security-review` with no open CRITICAL or HIGH; PR merged through the hub; post-merge checks done and reported.
- [ ] The user has the exact steps to turn legacy keys off, and to turn them back on if something breaks.

## Red line

D-004, D-006, D-012, D-022 apply. Never print keys, the user's email, BattleTags or `GameAccountId`. No accounts, terms, payments or publishing without the user.

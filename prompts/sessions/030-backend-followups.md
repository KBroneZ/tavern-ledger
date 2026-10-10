# Session 030: Backend follow-ups from 022 (T-104g)

Model: opus
Effort: high
Subagents: haiku for searches; sonnet for code and reviews

Work in your worktree **`C:\Users\andia\tavern-ledger\.claude\worktrees\030-backend-followups`** (branch `030-backend-followups`, created by the hub) and follow its `CLAUDE.md`. Chat in caveman mode; the user writes in Spanish, answer in English. Everything in the repo is in English. **Tier R2** (production backend, user data, keys): TDD + `/code-review` + `/security-review`.

**Other sessions may run at the same time** (026 card images, 029 email sender). To stay out of their way:
- Work only in your worktree. Never touch the main checkout or other worktrees.
- Your area: `supabase/` (functions, migrations, database tests, `config.toml`), `docs/research/deploy.md` sections 10.5 and 10.6 (029 owns section 10.11 and the mail parts of section 10), and the rows of `docs/legal/data-inventory.md` and `docs/legal/privacy.md` about refused uploads and the IP hash only (029 edits the sender parts of those files: keep your changes to those rows so the merge is clean). Do not touch `crates/` or `web/`.
- Shared files (`plan.md`, `README.md`, `DECISIONS.md`, `PROVENANCE.md`): only your task's row and notes. Your decision number is **D-042**. Keep them LF (`git ls-files --eol` on every changed file before committing): a CRLF file shows as a whole-file rewrite and conflicts with the other sessions.
- Only this session may start Docker and the local Supabase stack this round.
- To catch up with `main`: `git fetch origin && git merge origin/main`. No rebase, never force-push. **Merging to `main` applies migrations and deploys the functions** (D-031, D-036): check after the merge (10.8 step 3's list, at least: `/auth/v1/settings` still says `disable_signup: true`, functions answer `401` without a token, the sweep still runs).

Working rules (user's decisions):
- Decide technical matters yourself. Ask the user: accounts, accepting terms, paying, granting access between services, making something public, new dependencies.
- Never type, print or copy keys or secrets. A secret the functions need is set by the user from the Supabase dashboard (tell them exactly where to tap; they may be on their phone), never through chat, logs, files or commits. If your own auto-mode refuses an action (function secrets, Vault, dashboard changes), say so in your tab and let the user decide; never ask another session to do it for you.
- **`gh` is not logged in inside task sessions.** When your branch is ready (tests green, both reviews done, no open CRITICAL or HIGH), push it and send the hub (`Hub prompts/sessions/025`, find it with `ListAgents`) a message with the branch, the PR title, a short PR body (summary, review results, test plan) and anything still waiting on the user. The hub opens the PR, waits for CI and merges it; then you run the post-merge checks above and report them to the hub the same way.

## Why

Session 022 put the backend live (D-031) and left three follow-ups in `docs/research/deploy.md` (10.5, 10.6) and its security review:
1. `upload-game` keeps a plain SHA-256 of the client IP for an hour (`private.upload_ip_failures.ip_key`). The IPv4 space is small, so an unsalted hash can be reversed by brute force; it should not be.
2. The Free plan pauses a project after a week without activity. The daily sweep (`pg_cron` → `sweep` function) may or may not count as activity. Nobody has checked.
3. The functions read `SUPABASE_SERVICE_ROLE_KEY`, the legacy service key. Before legacy keys are ever turned off, they must use the new secret key (`sb_secret_…`).

## Effort and models per phase

| Phase | Effort | Model |
|-------|--------|-------|
| Read and check state | low | Haiku |
| Salt design, migration, tests | high | Opus, Sonnet for code |
| Pause check and secret key (primary docs) | medium | Opus, Haiku for search |
| Docs, review | medium | `code-reviewer`, `security-reviewer` |

## At the start

1. Conversation title: **`#030 Backend follow-ups`**.
2. `git fetch origin && git merge origin/main`.
3. Read `docs/research/deploy.md` section 10 (all of it), `DECISIONS.md` (D-020, D-021, D-025, D-031, D-036), `supabase/functions/upload-game/handler.ts`, `supabase/migrations/20261009150000_upload.sql`, `supabase/migrations/20261010090000_sweep_schedule.sql`, `supabase/config.toml`, and the IP rows in `docs/legal/data-inventory.md` and `docs/legal/privacy.md`.

## Task

1. **Salted IP hash.** Make the stored key unlinkable to an IP without a secret, with the 1 h retention kept. Prefer a design with no function secret for the user to set: for example a random salt made in the database (`gen_random_bytes`), kept in a private table or Vault, rotated at least daily by the existing sweep or `pg_cron`, with the old salt dropped once its rows have expired; the hashing (HMAC-SHA-256 or SHA-256 of salt and IP) may happen in SQL so the function never sees the salt. Keep blocking working across a rotation (accept that a rotation may reset a counter; say so). Migration plus pgTAP tests (format check, rotation, no unsalted key left, old rows gone) and Deno tests for the handler. Existing unsalted rows: delete them in the migration (they are at most an hour old).
2. **Pause check.** From Supabase's own docs and the dashboard (read only), find what counts as activity for the Free plan's pause, whether a scheduled `pg_cron` job or a function call counts, and whether the project shows a pause warning. Look at `private.sweep_runs` (count and last run only, via the SQL editor or `supabase` CLI linked read-only; ask the user if access needs granting). Write the finding into `deploy.md` 10.5; if the sweep does not count, propose the smallest thing that does and ask the user before adding any external pinger.
3. **New secret key.** From Supabase's docs, find how Edge Functions get the new secret key (a platform-provided variable, or a function secret the user sets). Change the three functions to read it, with the legacy variable as a fallback only until the user confirms the new key is set, and a clear error if neither is present. The handlers already send new-style keys in `apikey` only: keep that tested. If a function secret is needed, tell the user exactly where to set it; do not turn off legacy keys (that is the user's call, later).
4. Data inventory and privacy draft rows for refused uploads updated (salted hash, retention); `deploy.md` 10.5 and 10.6; plan row **T-104g** "Backend follow-ups from 022 (salted IP hash, pause check, new secret key)"; README; D-042; same PR.

## Out of scope

Email sender (029), opening sign-ups, Pro plan, backups, any change to auth settings, `crates/`, `web/`.

## Acceptance criteria

- [ ] No unsalted IP hash stored; salt rotates; blocking still works; pgTAP and Deno tests green locally and in CI.
- [ ] Pause behaviour checked against Supabase's own docs and recorded, with the sweep's live evidence.
- [ ] Functions read the new secret key (legacy only as a fallback), tested; the user knows if anything needs setting.
- [ ] `/code-review` and `/security-review` with no open CRITICAL or HIGH; PR merged through the hub; post-merge checks done and reported.

## Red line

D-004, D-006, D-012, D-022 apply. Never print keys, BattleTags or `GameAccountId`. No accounts, terms, payments or publishing without the user.

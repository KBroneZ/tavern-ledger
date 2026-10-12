# Session 012: Supabase project and schema (T-104a)

Model: opus for the schema, row-level security and deletion design (user data, R2); sonnet for code
Effort: high for the design and security review; medium for the rest
Subagents: haiku for doc lookups; sonnet for code and reviews; `security-reviewer` before the PR

Work in `C:\Users\andia\tavern-ledger` and follow its `CLAUDE.md`. Chat in caveman mode; the user writes in Spanish, answer in English. Everything in the repo is in English. **Tier R2** (user data, auth, new dependencies): TDD + `/code-review` + `/security-review`, and a `PROVENANCE.md` row per dependency.

Working rules (user's decisions, 2026-10-09):
- Decide technical matters yourself. Ask only what is the user's call: creating accounts, accepting terms or a DPA, paying, new dependencies, data with unclear licences.
- **You merge your own PRs, always**, once CI is green and no review has open CRITICAL or HIGH findings: `gh pr merge <n> --squash --delete-branch`.
- The user often works remotely without access to the PC: never ask them to do something on it.
- `gh` has no login of its own: export `GH_TOKEN` from the credential Git keeps (`git credential fill`) and never print it.
- Anything long-running (local Supabase stack, the app) goes in a separate window (`Start-Process`), not in the conversation.
- Secrets (Supabase keys, service role, database password) never in code, fixtures, logs, commits or chat: `.env` or `.local/` only.

**Red line (D-004, D-006, D-012, D-017):** the server never crawls the leaderboard, never stores other players' rows and never stores a rating, MMR or BattleTag. Never print BattleTags or `GameAccountId` in chat, docs, tests or tool output.

## Effort and models per phase

| Phase | Effort | Subagents (if needed) |
|-------|--------|-----------------------|
| Read and check state | low | Haiku |
| Ask the user (account, CLI dependency) | low | — |
| Schema, RLS, Edge Function design | high | — |
| Code (tests first) | medium | Sonnet |
| Review | high | `code-reviewer` + `security-reviewer` |

## At the start

1. Conversation title: **`#012 Supabase schema`**.
2. `git switch main && git pull`; branch `012-supabase-schema`.
3. Read `README.md`, `docs/plan/plan.md` (T-104a to T-104d), `docs/decisions/DECISIONS.md` (D-003, D-012, D-015, D-017, D-020) and `docs/research/web-stack.md` (requirements, option A, recommendation, upload design).
4. Green before touching anything: `cargo test --workspace --exclude desktop` and `python -m unittest discover -s tests` (with `.venv`).
5. T-101: check `%APPDATA%\TavernLedger\games.jsonl` for new games (session 011 left 23 games from 5 sessions), counting per session without names. If a new game appeared on its own while the app was open, mark T-101 done.

## Ask the user first (one question each, with a recommendation)

- **Supabase CLI** (MIT) as a dev dependency, and whether Docker Desktop is installed or may be installed, to run the stack locally. Recommended: yes; everything in this session can be built and tested locally without an account.
- **Hosted project:** creating the Supabase account and organisation, accepting the terms and the DPA, region Frankfurt (`eu-central-1`), Free plan. Only with the user's explicit OK, and the user does it or says how. If there is no answer, keep everything local and leave the hosted project for later.

## Task

1. **Schema** in `supabase/migrations/` (SQL, reviewed line by line):
   - `profiles` (user id, display name chosen by the user, `is_public` default false, created and updated dates, string lengths capped);
   - `games` summary rows keyed by `(user_id, session, index)`: mode, hero card id, place, build, date, tribes offered, `saved_at` revision, SHA-256 of the uploaded bytes, storage path. No names, no rating, no MMR field;
   - storage bucket `games` (private), objects under `<user_id>/`.
2. **Row-level security:** clients read only their own rows, plus rows of public profiles; **no write policy** for clients on `games` or the bucket. Tests that prove it (pgTAP or SQL tests run against the local stack): another user cannot read a private profile, nobody can write a game row directly.
3. **Export and deletion** as SQL functions or an Edge Function stub, following the order in `web-stack.md` (storage objects, rows, profile, auth user). Test that nothing of the user remains.
4. **Backups until Pro:** a script (PowerShell, run on the developer's machine) for `supabase db dump` plus the bucket, written to `.local/backups/`, never to the repo or CI.
5. Upload Edge Function only if time allows; otherwise it is T-104d's start. The server checks from the upload design apply.
6. Plan (T-104a), README, `DECISIONS.md` (if a new decision appears) and `PROVENANCE.md` in the same PR.

## Out of scope

- Website pages (T-104c), privacy policy text (T-104b), desktop sign-in and upload (T-104d).
- Paying for Pro; email provider choice (T-104b).
- Battle.net login (ruled out, D-020).

## Acceptance criteria

- [ ] User's answers recorded (CLI, Docker, hosted project); nothing created without them.
- [ ] Migrations apply cleanly on a fresh local stack.
- [ ] RLS tests: private data unreadable by others; no client writes on games or the bucket.
- [ ] Export returns everything held about a user; deletion leaves nothing (tested).
- [ ] No secret in the diff (CI secret scan green).
- [ ] `/code-review` and `/security-review`: no open CRITICAL or HIGH.
- [ ] CI green; PR merged by you.
- [ ] Ask the user whether they want the next prompt.

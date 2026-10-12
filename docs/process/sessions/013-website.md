# Session 013: Website: sign-in, account, export, deletion, public profile (T-104c)

Model: opus for auth, deletion and security design (user data, R2); sonnet for code
Effort: high for design and security review; medium for the rest
Subagents: haiku for doc lookups; sonnet for code and reviews; `security-reviewer` before the PR

Work in **`C:\Users\andia\tl-013`** (a git worktree on branch `013-website`, already created) and follow its `CLAUDE.md`. Chat in caveman mode; the user writes in Spanish, answer in English. Everything in the repo is in English. **Tier R2** (auth, user data, new dependencies): TDD + `/code-review` + `/security-review`, and a `PROVENANCE.md` row per dependency.

**Another session runs at the same time** (#014, desktop robustness, worktree `C:\Users\andia\tl-014`). To stay out of each other's way:
- Work only in `C:\Users\andia\tl-013`. Never touch `C:\Users\andia\tavern-ledger` (it holds the running desktop app) or `tl-014`.
- Your area: `web/` (new), `supabase/` (only what the site needs: CORS and the recent sign-in check in `delete-account`, auth redirect URLs in `config.toml`), `package.json`/lock, CI for the site. Do not touch `crates/`, `tools/check_logs.py` or `tools/dev/`: they belong to #014.
- Shared files (`docs/plan/plan.md`, `README.md`, `DECISIONS.md`, `PROVENANCE.md`): only your task's row and notes. If you need a decision, it is **D-023** (D-024 is reserved for #014).
- To catch up with `main`: `git fetch origin && git merge origin/main`. No rebase, never force-push.
- The local Supabase stack (Docker) is yours; #014 never starts it.

Working rules (user's decisions, 2026-10-09):
- Decide technical matters yourself. Ask only what is the user's call: creating accounts, accepting terms, paying, new dependencies not already approved, data with unclear licences.
- **You merge your own PRs, always**, once CI is green and no review has open CRITICAL or HIGH findings: `gh pr merge <n> --squash --delete-branch`.
- The user often works remotely without access to the PC: never ask them to do something on it.
- `gh` has no login of its own: export `GH_TOKEN` from the credential Git keeps (`git credential fill`) and never print it.
- Anything long-running (local Supabase stack, dev server) goes in a separate window (`Start-Process`), not in the conversation.
- Secrets (service role key, database password) never in code, fixtures, logs, commits or chat: `.env` or `.local/` only. The site only ever holds the anon (publishable) key.
- Python: use `C:\Users\andia\tavern-ledger\.venv\Scripts\python` (the worktree has no `.venv`).

**Red line (D-004, D-012, D-017, D-021):** the site never shows or stores a rating, MMR or BattleTag, and clients never write game rows or files (D-021). Never print BattleTags or `GameAccountId` in chat, docs, tests or tool output.

## Effort and models per phase

| Phase | Effort | Subagents (if needed) |
|-------|--------|-----------------------|
| Read and check state | low | Haiku |
| Page and auth design | high | — |
| Code (tests first) | medium | Sonnet |
| Review | high | `code-reviewer` + `security-reviewer` |

## At the start

1. Conversation title: **`#013 Website`**.
2. `cd C:\Users\andia\tl-013 && git fetch origin && git merge origin/main`; `npm ci`.
3. Read `README.md`, `docs/plan/plan.md` (T-104a to T-104d and the T-104a notes), `docs/decisions/DECISIONS.md` (D-003, D-012, D-017, D-020, D-021), `docs/research/web-stack.md` (requirements, option A, upload design) and `supabase/` (migrations, `delete-account`, tests).
4. Green before touching anything: `cargo test --workspace --exclude desktop`, the Python tests, and the Supabase tests against the local stack (`npx supabase start` in a separate window, `npx supabase test db`, the Deno tests, `TAVERN_SUPABASE_LOCAL=1` end-to-end tests).

## Task

1. **Astro static site** in `web/` (D-020 already chose Astro and `@supabase/supabase-js`; pin exact versions, lockfile, `PROVENANCE.md` rows). Any other dependency (test runner, UI kit) needs the user's OK: prefer none (Node's built-in test runner).
2. **Pages:** home (what the app does, the "Unofficial fan project…" notice, rule 5), sign-in and sign-up by email (password length 10 as in `config.toml`, email confirmation), account page, public profile page.
3. **Account page:** display name and "public profile" switch (private by default); **export** (calls `export_my_data()`, downloads the JSON); **delete account** (typed confirmation, then `delete-account`). Add to `delete-account` the two items left by T-104a: CORS for the site's origin only, and a recent sign-in check (refuse if the last sign-in is older than a few minutes; tests first).
4. **Public profile:** only when `is_public`; shows display name and the game summaries the schema allows (hero, place, mode, date). Never ratings or names of other players. A private or missing profile shows the same "not found" (no way to probe which accounts exist).
5. **Security:** strict Content-Security-Policy (no inline scripts; `connect-src` only Supabase), no `innerHTML` with data, `textContent` only; the session in memory or Supabase's default storage, nothing custom. Check sign-up and sign-in error messages don't reveal whether an email exists.
6. **Tests:** unit tests for the page logic; an end-to-end script against the local stack (sign up, confirm via the local mail catcher, set profile, export, delete, then check nothing remains), skipped unless `TAVERN_SUPABASE_LOCAL=1` like the existing ones.
7. CI: build the site and run its unit tests (no Docker in CI).
8. Plan (T-104c), README and, if needed, D-023, in the same PR.

## Out of scope

- Deploying (Cloudflare Pages account = the user's call), the hosted Supabase project, a domain.
- Privacy policy text and terms (T-104b): the pages link to a placeholder that says the site is not open to the public yet.
- Desktop sign-in and upload (T-104d); OAuth providers (Discord, GitHub) unless trivial and approved.

## Acceptance criteria

- [ ] Tests before code; all green (Rust, Python, pgTAP, Deno, site unit tests; local end-to-end run once with output).
- [ ] Export downloads every row the user owns; deletion leaves nothing (checked end to end).
- [ ] `delete-account` refuses other origins and stale sign-ins (tests).
- [ ] CSP in place; no `innerHTML` with data; no service key in `web/`.
- [ ] `/code-review` and `/security-review` with no open CRITICAL or HIGH; CI green; PR merged by you.
- [ ] Plan, README (and D-023 if needed) updated in the same PR.
- [ ] Ask the user whether they want the next prompt (T-104d, desktop upload, comes after this and #014).

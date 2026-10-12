# Session 016: Desktop sign-in and game upload (T-104d)

Model: opus for auth, upload and security design (user data, network, R2); sonnet for code
Effort: high for design and security review; medium for the rest
Subagents: haiku for doc lookups; sonnet for code and reviews; `security-reviewer` before the PR

Work in your worktree **`C:\Users\andia\tavern-ledger\.claude\worktrees\016-desktop-upload`** (branch `016-desktop-upload`, created by the hub) and follow its `CLAUDE.md`. Chat in caveman mode; the user writes in Spanish, answer in English. Everything in the repo is in English. **Tier R2** (auth, network, user data, maybe dependencies): TDD + `/code-review` + `/security-review`, a `PROVENANCE.md` row per new dependency.

**Other sessions run at the same time** (#017 provenance and report bundle; #018 privacy and terms). To stay out of each other's way:
- Work only in your worktree. Never touch the main checkout or other worktrees.
- Your area: `crates/uploader` (new) or a new module in `tracker`, `supabase/` (the upload Edge Function, quotas, the daily sweep, migrations they need), and in `crates/desktop` only the sign-in/upload settings, tray menu entries and commands they need (keep `main.rs` changes small: #017 also adds commands there). Do not touch `crates/desktop/ui/` beyond one settings panel for upload, `tracker::stats`, `web/` or the privacy pages.
- Shared files (`plan.md`, `README.md`, `DECISIONS.md`, `PROVENANCE.md`): only your task's rows and notes. Your decision number, if needed, is **D-025**.
- To catch up with `main`: `git fetch origin && git merge origin/main`. No rebase, never force-push.
- You own Docker and the local Supabase stack; no other session starts them. You may not stop or restart the desktop app the user has running (#017 may); test your build with `--data-dir` in a temporary folder.

Working rules (user's decisions, 2026-10-09):
- Decide technical matters yourself. Ask only what is the user's call: creating accounts, accepting terms, paying, new dependencies, data with unclear licences.
- **You merge your own PRs, always**, once CI is green and no review has open CRITICAL or HIGH findings: `gh pr merge <n> --squash --delete-branch`.
- The user often works remotely without access to the PC: never ask them to do something on it.
- `gh` has no login of its own: export `GH_TOKEN` from the credential Git keeps (`git credential fill`) and never print it.
- Anything long-running (local Supabase stack, the app) goes in a separate window (`Start-Process`).
- Secrets never in code, fixtures, logs, commits or chat. Tokens on the desktop: Windows Credential Manager or DPAPI-protected file, never plain text.
- Python: use `C:\Users\andia\tavern-ledger\.venv\Scripts\python`.

**Red line (D-004, D-012, D-017, D-021):** the upload carries the local report only (card ids, places, boards): never player names, BattleTags, `GameAccountId`, ratings or MMR; clients never write rows or files directly, only through the Edge Function (D-021). Upload is **off until the user turns it on**.

## Effort and models per phase

| Phase | Effort | Subagents (if needed) |
|-------|--------|-----------------------|
| Read and check state | low | Haiku |
| Design (auth flow, queue, function, quotas) | high | — |
| Code (tests first) | medium | Sonnet |
| Review | high | `code-reviewer` + `security-reviewer` |

## At the start

1. Conversation title: **`#016 Desktop upload`**.
2. `git fetch origin && git merge origin/main`; `npm ci` (Supabase CLI).
3. Read `README.md`, `docs/plan/plan.md` (T-104a to T-104d and their notes, T-107), `docs/decisions/DECISIONS.md` (D-012, D-015, D-020, D-021, D-023, D-024), `docs/research/web-stack.md` ("Upload design") and `supabase/`.
4. Green before touching anything: Rust (workspace and `desktop`), clippy, fmt, Python, and the Supabase tests on the local stack (`npx supabase start` in a separate window, `npx supabase test db`, Deno tests, `TAVERN_SUPABASE_LOCAL=1`).

## Task

Implement the "Upload design" in `docs/research/web-stack.md`:

1. **Edge Function `upload-game`**: `PUT /v1/games/{session}/{index}` with strict patterns; gzip body, at most 64 KiB read, streamed inflate capped at 512 KiB; validates the report shape; checks it holds no player names (no BattleTag-shaped strings, no fields outside the allowed set); writes the file then the row with the service role; SHA-256 of the inflated bytes; same key replaces (idempotent). Quotas per user (hour, day, total games and bytes) and per IP; clear error codes.
2. **Daily sweep** of files with no row (and the leftover audit entry case from T-104a), runnable locally, tested.
3. **Schema:** add the parser version (`parser.version`, `parser.revision`, T-107 / D-024) to the games row if the design needs it; migration + pgTAP tests.
4. **Desktop sign-in**: system browser, authorization code + PKCE, loopback redirect on `127.0.0.1` only, random `state`, one-time code; check what Supabase accepts for loopback ports and document it. Sign-out removes the tokens.
5. **Upload queue**: an "uploaded" mark per game next to the history; after each game and at start-up, send unmarked games oldest first, one at a time; backoff with jitter on network errors, timeouts (15 s) and `5xx`; stop and show the reason on `4xx`. A re-imported game (last record wins) uploads again.
6. **App:** an "Upload games" switch (off by default), sign-in state, last upload result and the number of games waiting. Never on by itself, never switched on by an update.
7. **Tests:** unit tests for the queue and the PKCE flow (fake server), Deno tests for the function, end-to-end against the local stack (sign in, upload, re-upload, quota, sign out) skipped unless `TAVERN_SUPABASE_LOCAL=1`.
8. Plan (T-104d), README, `PROVENANCE.md` and D-025 if needed, in the same PR.

## Out of scope

- The hosted Supabase project and deploying the function (needs the user's account).
- Website changes, privacy text (#018), the report bundle (#017).

## Acceptance criteria

- [ ] Tests before code; all green (Rust, clippy, fmt, Python, pgTAP, Deno; local end-to-end once with output).
- [ ] A forged upload with a BattleTag-shaped string or extra fields is refused (test).
- [ ] Upload is off by default; the switch, sign-in and queue state are visible in the app.
- [ ] No token in plain text on disk (checked).
- [ ] `/code-review` and `/security-review` with no open CRITICAL or HIGH; CI green; PR merged by you.
- [ ] Plan, README (and D-025 if needed) updated in the same PR.
- [ ] Tell the hub conversation's user what is left for the hosted deploy.

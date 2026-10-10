# Session 037: Follow-ups before opening sign-ups (T-104i)

Model: opus
Effort: high
Subagents: haiku for searches; sonnet for code and reviews

Work in your worktree **`C:\Users\andia\tavern-ledger\.claude\worktrees\037-signup-followups`** (branch `037-signup-followups`, created by the hub) and follow its `CLAUDE.md`. Chat in caveman mode; the user writes in Spanish, answer in English. Everything in the repo is in English. **Tier R2** (auth, user data, production backend): TDD + `/code-review` + `/security-review`.

**Another session may run at the same time** (a desktop overlay fix after the user's real-game test). To stay out of its way:
- Work only in your worktree. Never touch the main checkout or other worktrees.
- Your area: `web/`, `supabase/`, `docs/legal/`, `docs/research/deploy.md` (section 10), and in `crates/` **only** `crates/uploader/src/loopback.rs` and `crates/uploader/src/auth.rs` (plus the one place in `crates/desktop/` that turns a sign-in error into text, if the hint needs it; keep that change to a few lines). Nothing else in `crates/`.
- Shared files (`plan.md`, `README.md`, `DECISIONS.md`, `PROVENANCE.md`): only your task's row and notes. Your decision number is **D-048**. Keep every changed text file LF (`git ls-files --eol` before committing): a CRLF file rewrites the whole file and conflicts with others.
- Only this session may start Docker and the local Supabase stack this round. Never stop or restart the user's desktop app (it runs from the main checkout); test with your own build and a scratch data folder.
- To catch up with `main`: `git fetch origin && git merge origin/main`. No rebase, never force-push.
- **Merging to `main` deploys** the site (Cloudflare Pages) and the Supabase migrations and functions (D-031, D-036). After the merge check: `/auth/v1/settings` still says `disable_signup: true`, functions answer `401` without a token, the site's new pages answer.

Working rules (user's decisions):
- Decide technical matters yourself. Ask the user: accounts, accepting terms, paying, granting access between services, making something public, new dependencies, DNS changes on the live domain.
- Never type, print or copy keys, secrets, the user's email, BattleTags or `GameAccountId`. Dashboard changes (Supabase auth settings, email templates, Cloudflare DNS) are done by the user or, if you fill a form in the browser, the user presses Save; tell them exactly where to tap (they may be on their phone). If your own auto-mode refuses an action, say so in your tab and let the user decide; never ask another session to do it.
- **Sign-ups stay closed.** Opening them is the user's call, after this session.
- **`gh` is not logged in inside task sessions.** When your branch is ready (tests green, both reviews done, no open CRITICAL or HIGH), push it and send the hub (`Hub prompts session 036`, find it with `ListAgents`, use its `[ref]`) a message with the branch, the PR title, a short PR body (summary, review results, test plan) and anything still waiting on the user. The hub opens the PR, waits for CI and merges it; then you run the post-merge checks above and report them the same way.

## Why

`docs/plan/plan.md`, "Follow-ups before opening sign-ups (hub, 2026-10-10)", lists six things to do before anyone else can make an account. They are all small and all touch sign-in, so one session does them together.

## Effort and models per phase

| Phase | Effort | Model |
|-------|--------|-------|
| Read and check state | low | Haiku |
| Confirmation links and password reset (site) | high | Opus, Sonnet for code |
| Empty `auth_events` in the export | medium | Opus |
| DMARC `rua`, decision notes, app hint | low | Sonnet |
| Privacy policy and terms out of draft | high | Opus |
| Review | medium | `code-reviewer`, `security-reviewer` |

## At the start

1. Conversation title: **`#037 Sign-up follow-ups`**.
2. `git fetch origin && git merge origin/main`.
3. Read the plan's follow-ups paragraph, `docs/research/deploy.md` section 10 (all), `DECISIONS.md` (D-021, D-025, D-031, D-034, D-037, D-041, D-042), `web/src/pages/account.astro`, `signin.astro`, the site's auth script, `supabase/config.toml` (`[auth]`), the export function and the migration that writes `auth_events`, `crates/uploader/src/loopback.rs`, and `docs/legal/` (all three files).

## Task

1. **Every confirmation link says something, and password reset exists.** Today `/account/` reads only PKCE `?code=` links; a confirmation, invite, magic-link or recovery link made another way (implicit `#access_token=…`, `?token_hash=…&type=…`, or an `?error=…&error_code=…` answer) lands there with nothing said. Handle each kind Supabase sends for this project's settings (check its auth docs for the exact shapes), show "Email confirmed, now sign in" or the right error in words, and never leave a token in the address bar or history afterwards. Add a "Forgot password?" flow: request page (same answer whether the address exists or not), the recovery link landing on a "set a new password" form, rate-limit errors shown in words. Allow only the redirect URLs already configured (`https://tavernledger.net/account/`, the desktop loopback); if a new one is needed, ask the user and tell them where to add it. Keep the site's Content-Security-Policy as it is (no new external origin). Tests for the link parsing and the flows.
2. **Empty `auth_events` in the export.** Session 029 found the account export's `auth_events` empty on the hosted project. Find why (nothing written, the trigger or hook not installed in production, a filter, RLS) from the migrations, the functions and read-only checks; fix it with a migration and tests. If it needs a dashboard setting (an auth hook), tell the user where.
3. **DMARC aggregate reports.** Propose a `rua` address (on the domain's own forwarding, e.g. `dmarc@tavernledger.net` forwarded like `contact@`) and the exact DNS record; the user changes DNS. Record it in `deploy.md` 10.11.
4. **Decision notes.** Add "replaced by D-041" to D-034 and D-037 (a note in their last column, nothing else changed).
5. **App hint for an old link.** When the loopback receives `otp_expired` (a link from an older sign-in attempt), the app says "That link is from an earlier attempt or expired: use the newest email, or ask for a new link." Unit test in `loopback.rs` or `auth.rs`.
6. **T-104b out of draft.** Go through `docs/legal/privacy.md` and `terms.md` (and the site's `privacy.astro` and `terms.astro`, which must say the same): resolve every part marked to be checked against the live setup (Supabase EU region, Lettermint D-041, Cloudflare Pages, salted IP hash D-042, retention, export and deletion, contact addresses), remove the draft markers, and list for the user, in plain words, what each document now promises. **The user approves the final text before the PR is handed over**; ask in your tab and wait.
7. Plan row **T-104i** "Follow-ups before opening sign-ups" (and mark the follow-ups paragraph done item by item), T-104b status, README, D-048 (what you decided on link handling and reset), `deploy.md`; same PR.

## Out of scope

Opening sign-ups, Pro plan, any paid change, turning off legacy keys, the overlay, card names in other languages, anything in `crates/` beyond the hint.

## Acceptance criteria

- [ ] Every link kind the project can send lands on a page that says what happened; no token left in the URL; password reset works end to end on the local stack.
- [ ] `auth_events` filled and exported, with tests; the production cause named.
- [ ] DMARC `rua` record proposed to the user; D-034 and D-037 annotated; app hint tested.
- [ ] Privacy policy and terms out of draft, matching the site, approved by the user.
- [ ] `/code-review` and `/security-review` with no open CRITICAL or HIGH; PR merged through the hub; post-merge checks done and reported.

## Red line

D-004, D-006, D-012, D-022 apply. Never print keys, the user's email, BattleTags or `GameAccountId`. No accounts, terms, payments, DNS changes or publishing without the user.

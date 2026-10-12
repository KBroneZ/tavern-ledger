# Session 046: Website form accessibility follow-ups (T-318)

Model: sonnet
Effort: medium
Subagents: haiku for searches; sonnet for reviews (`code-reviewer`, `a11y-architect`)

Work in your worktree **`C:\Users\andia\tavern-ledger\.claude\worktrees\046-site-forms-a11y`** (branch `046-site-forms-a11y`, created by the hub) and follow its `CLAUDE.md`. Chat in caveman mode; the user writes in Spanish, answer in English. Everything in the repo is in English. **Tier R2** (the sign-in, reset and account scripts handle user input and auth): TDD + `/code-review` + `/security-review`.

**Other sessions run at the same time:** 044 (parser data round: `crates/bg-parser/`, `crates/tracker/`, `tests/`, `tools/`, `supabase/functions/upload-game/`) and 045 (app accessibility: `crates/desktop/`). To stay out of their way:
- Work only in your worktree. Never touch the main checkout or other worktrees.
- Your area: `web/` only. Nothing in `crates/`, `supabase/`, `tools/`, `tests/`.
- Shared files (`plan.md`, `README.md`, `DECISIONS.md`, `PROVENANCE.md`): only your task's row and notes. Your decision number is **D-058**, only if needed. Keep every changed text file LF (`git ls-files --eol`; check the index side, the checkout uses `core.autocrlf=true`).
- You may start Docker and the local Supabase stack for the e2e tests (no other session uses it this round; tell the hub if 044 asks). Stop it when you are done.
- To catch up with `main`: `git fetch origin && git merge origin/main`. No rebase, never force-push.
- **Merging to `main` deploys the site.** After the merge, check: every page 200, the CSP header unchanged, `/auth/v1/settings` says `disable_signup: true`, the auth pages render and the #72 link messages still show.

Working rules (user's decisions):
- Decide technical matters yourself. Ask the user: new dependencies, any change to the CSP.
- Never type real credentials on the live site; e2e runs on the local stack with test accounts made there.
- **`gh` is not logged in inside task sessions.** When your branch is ready (tests green, both reviews done, no open CRITICAL or HIGH), push it and send the hub (`Hub prompts session 036`, find it with `ListAgents`, use its `[ref]`) a message with the branch, the PR title and a short PR body (summary, review results, test plan). The hub opens the PR, waits for CI and merges it; then you run the post-merge checks above.

## Why

Session 042's accessibility review (plan, notes T-316, "Left for later") found two things in the form scripts from #72:
1. Error messages are not tied to their fields: add `aria-invalid` and `aria-describedby`, and move focus to the first field in error.
2. Focus is lost when a button turns busy or a form is swapped (sign-in to reset, request to "check your email", set-password form); keep it or move it to the new heading or message, announced politely.

## At the start

1. Conversation title: **`#046 Site forms accessibility`**.
2. `git fetch origin && git merge origin/main`.
3. Read D-048, D-052, D-056, the T-316 notes, and `web/src/scripts/` (sign-in, forgot, account) with their tests.

## Task

1. Errors tied to fields on every form (sign-in, sign-up when open, forgot password, set new password, profile, delete account), with the same wording as today; tests.
2. Focus management on busy buttons and swapped forms, with a polite live region where a message replaces a form; tests.
3. No change to what the scripts send, the auth flow, the CSP or the routes. The three builds (closed, open, open with sign-ups) still pass `test:dist`; the local e2e (`test:local`) passes.
4. Run the `accessibility` skill and the `a11y-architect` agent; axe clean on every page.
5. Plan row **T-318** "Website form accessibility follow-ups"; same PR.

## Out of scope

The look (D-052, D-056), opening sign-ups, auth or data changes, anything outside `web/`.

## Acceptance criteria

- [ ] Every form error tied to its field; focus never lost on busy or swap; tested.
- [ ] Auth behaviour unchanged; unit, dist and local e2e tests green.
- [ ] `/code-review` and `/security-review` with no open CRITICAL or HIGH; PR merged through the hub; post-merge checks reported.

## Red line

Never print keys, the user's email or passwords. No change to what is sent to the server.

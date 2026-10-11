# Session 065: Personal dashboard and games list on the website (T-503)

Model: opus
Effort: high (how the static site renders live data), medium for the rest
Subagents: haiku for searches; sonnet for code and reviews (`code-reviewer`, `a11y-architect`); opus for `security-reviewer`

Work in your worktree **`C:\Users\andia\tavern-ledger\.claude\worktrees\065-web-dashboard`** (branch `065-web-dashboard`, created by the hub) and follow its `CLAUDE.md`. Chat in caveman mode; the user writes in Spanish, answer in English. Everything in the repo is in English. **Tier R2** (pages that read user data): TDD + `/code-review` + `/security-review`.

**Another session runs at the same time:** 064 (app follow-ups: `crates/desktop`, `crates/tracker`, `crates/bg-parser`). To stay out of its way:
- Work only in your worktree. Never touch the main checkout or other worktrees.
- Your area: `web/` only, plus docs. Nothing in `crates/`, `supabase/` (if the data API lacks something, tell the hub; do not change it yourself), `tools/`.
- Shared files (`plan.md`, `README.md`, `DECISIONS.md`): only your task's row and notes. Your decision number is **D-073**, only if needed (064 has D-072). Keep every changed text file LF (`git ls-files --eol`, index side; the checkout uses `core.autocrlf=true`).
- To catch up with `main`: `git fetch origin && git merge origin/main`. No rebase, never force-push.
- Stop any dev server you start before you hand over.

Working rules (user's decisions):
- Decide technical and design matters yourself (Lamplight glass D-052, the design system D-070). Ask the user, **through the hub** as well as in your tab: new dependencies (an Astro adapter or server rendering counts), any change to the CSP, anything that shows more data than the data API's rules allow.
- `gh` is not available in task sessions. When your branch is ready (tests green, reviews done, no open CRITICAL or HIGH), push it and message the hub (`tavern-ledger-2b`, find it with `ListAgents`, use its `[ref]`) with the branch, the PR title and a short PR body (summary, review results, test plan). The hub opens the PR, waits for CI and merges it. **Merging to `main` deploys the site:** afterwards check every page answers 200 and the CSP header is as expected, and tell the hub.

## Why

Round 5 puts everything the app knows on the website (read **`docs/plan/web-master-plan.md`**). T-501 (D-070, #115) built the pieces and `/style/`; T-502 (D-071, #116) built the data API: `game_summaries`, `user_overview`, `user_heroes`, `user_records`, all `security_invoker` with the same rules as `games` (read **`docs/research/web-data-api.md`**). This task builds the first real pages on them.

The user's taste: compact, visual, key numbers first and details folded, Simple by default with an Advanced mode, no walls of text, every number says how many games back it.

## At the start

1. Conversation title: **`#065 Web dashboard`**.
2. `git fetch origin && git merge origin/main`.
3. Read the master plan, D-030, D-052, D-070, D-071, `docs/research/web-data-api.md`, `web/src/components/data/`, `web/src/lib/data/`, `web/src/pages/profile.astro`, `account.astro` and the CSP set-up.

## Task

1. **How live data renders.** The site is static Astro on Cloudflare Pages and the T-501 charts are drawn at build time; the pages here read data in the browser with the publishable key. Choose the simplest way that keeps the design system's look, the CSP (no inline script, `script-src 'self'`) and no layout shift: for example a small client renderer that builds the same markup from the same `lib/data` helpers, with reserved space and skeletons. Server rendering or a new dependency needs the user's OK through the hub. Record the choice in D-073.
2. **Personal dashboard `/me/`** (signed in): overview per mode (games, average place, top-4 or top-2, wins, with game counts), place distribution and place-over-time charts, last games (top N), top heroes, records the data backs; Simple shows the key numbers, Advanced the rest (economy, combats, APM, triples). Signed out: a short message and the sign-in link.
3. **Public profile:** the same overview, read only, for a public profile. The site has `/profile/?u=…` today; keep a working URL (pretty `/u/<name>/` paths need server routing on a static site; decide, and never break an existing link).
4. **Games list:** filters (mode, hero, date range, place), compact rows, paging, a row per game with hero picture placeholder, place badge, date, rounds and the key numbers; unknown shown as unknown. The game viewer (T-504) comes next: link each row to where it will live, or leave the link out until it exists.
5. **Privacy (absolute rule 6):** only what the views return; no other player's name anywhere; a private profile shows "This profile is private" to others, never an error that hints at its data. Pictures stay the local placeholders (the CSP decision for card art is pending with the user).
6. **States:** loading, empty (no games yet), unknown values, network error with a retry; no silent failures.
7. Accessibility WCAG 2.2 AA (axe clean on each page), keyboard, reduced motion; phone and desktop; within the web performance budget.
8. Tests: unit tests for the data shaping and filters; the build tests (`test`, `test:dist`) green in every build mode; an end-to-end check against the local Supabase stack if you can run it (Docker), with synthetic data only. Plan row **T-503** done with notes.

## Out of scope

The game viewer (T-504), heroes and curious-stats pages (T-505), leaderboards (T-506), Spanish strings (T-508; keep every string in `lib/data/strings.ts` or the existing string place), changes to `supabase/`, the CSP.

## Acceptance criteria

- [ ] `/me/`, the public profile and the games list work on live data for the owner and for a public profile; private profiles stay private; all states shown.
- [ ] axe clean, keyboard works, CLS 0; `test` and `test:dist` green (paste counts).
- [ ] `/code-review` and `/security-review` with no open CRITICAL or HIGH; branch handed to the hub; post-merge checks reported.

## Red line

No real player data in fixtures or screenshots, no Blizzard art in the repo, no CSP change and no new dependency without the user's OK, never print keys, tokens, the user's email or BattleTags.

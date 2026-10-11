# Session 069: Public leaderboards, name filter and the privacy texts for public stats (T-506, T-509)

Model: opus
Effort: high (what becomes public, RLS, anti-abuse), medium for the pages
Subagents: haiku for searches; sonnet for code; opus for the reviews (`database-reviewer`, `security-reviewer`)

Work in your worktree **`C:\Users\andia\tavern-ledger\.claude\worktrees\069-web-leaderboards`** (branch `069-web-leaderboards`, created by the hub) and follow its `CLAUDE.md`. Chat in caveman mode; the user writes in Spanish, answer in English. Everything in the repo is in English. **Tier R2 plus a second AI** (new public views, RLS, legal texts): TDD + `/code-review` + `/security-review` + a second AI review (`/santa-loop` or `ask-chatgpt`).

**Other sessions at the same time:** 066 (game viewer: `web/src/pages/game*`, `web/src/components/game/`, `web/src/lib/game/`, `pictures.ts`, the CSP) and 068 (combat simulator: `crates/combat-sim/`). To stay out of their way:
- Work only in your worktree. Never touch the main checkout or other worktrees.
- Your area: `supabase/` (migrations, database tests, functions if needed), new files under `web/src/pages/leaderboards*`, `web/src/lib/leaderboards/`, the legal pages and texts (`web/src/pages/privacy.astro`, `terms.astro`, `docs/legal/`), the account page only for the display-name check, plus docs. Do not touch 066's files or the CSP. Shared design-system files (`web/src/components/data/`, `web/src/lib/view/`, `web/src/lib/data/`, `data.css`): add, do not rewrite.
- Shared files (`plan.md`, `README.md`, `DECISIONS.md`): only your rows and notes. Your decision number is **D-082**, only if needed (D-079 is 066's, D-081 is 068's). Every changed text file LF (`git ls-files --eol`, index side; the checkout uses `core.autocrlf=true`).
- The local Supabase stack: 066 may be using it (same ports). Ask the hub before starting it; stop it when you are done.
- To catch up: `git fetch origin && git merge origin/main`. No rebase, never force-push. Stop any dev server you start.

Working rules:
- The user is asleep: do not wait on them. Their decisions for this task are already made (below). Anything beyond them goes in your hand-over as a question with a recommendation.
- Never touch production by hand; production changes only through merged migrations (D-031). **Do not open sign-ups** (D-078: the hub does it after this task is merged and checked).
- `gh` is not available. When your branch is ready (tests green, all reviews done, no open CRITICAL or HIGH), push it and message the hub (`tavern-ledger-2b`, `ListAgents`, use its `[ref]`) with the branch, PR title and a short PR body. The hub opens the PR, waits for CI and merges. **Merging deploys the site, the migrations and the functions:** afterwards check `/auth/v1/settings` still says `disable_signup: true`, functions answer `401` without a token, the validator answers its version, every page answers 200, the CSP is unchanged, and the leaderboard views return only public profiles for an anonymous caller; tell the hub.

## The user's decisions (2026-10-11)

- **D-075 scope:** only players whose profile is public, by their display name, and only with a minimum number of games in the period (you set the number per board and explain it; the master plan's example is 20 games). No opponents' names, no BattleTags.
- **D-076:** no rating on the website; boards rank stats from uploaded games only.
- **D-077 names:** today's rule (1 to 32 letters, digits, space, `_.-`) plus a basic offensive-word filter, checked on the server when a name is set and before a profile shows on a public board.
- **D-078:** sign-ups open after this task (T-506) and the texts (T-509) are merged and checked live. The texts must be right before anyone else signs up.

## Task

1. **Leaderboards (T-506)**, built on `game_stats` and the T-502 views (read `docs/research/web-data-api.md`, D-071): per mode (Solo, Duos) and period (last 7 days, last 30 days, all time): average place, top-4 / top-2 rate, win rate, games played, longest top-half streak, best average place per hero (minimum games per hero), and the economy and fun boards the data backs (APM, gold beyond income, rolls, triples per game, biggest board, most golden minions). Each row: rank, display name, value, games behind it. SQL views or materialized views refreshed on a schedule (like the sweep), never computed in the browser; only finished games that passed the validator; outlier flags or exclusion for impossible values (explain them); ties ordered stably.
2. **Privacy:** only `is_public` profiles; a profile turned private disappears at the next refresh at the latest (say how soon); no new personal data; tests for anon and another user, a private profile, a name that fails the filter, a player under the minimum.
3. **Name filter (D-077):** server-side check (database constraint or function used on every write path) with a small, documented word list kept in the repo (no third-party list with an unclear licence; if you reuse one, `PROVENANCE.md`); the account page shows a clear message when a name is refused; existing names that fail are hidden from boards until changed.
4. **Page `/leaderboards/`** on the design system (D-070) and the client renderers (`web/src/lib/view/`, D-073): mode and period switches, one compact board per stat with top N and "Show all", an explanation fold per board ("how this is counted": minimum games, period, self-reported, validator), Simple (the main boards) and Advanced (all); links to public profiles (`/profile/?id=`). Linked from the site menu.
5. **Texts (T-509):** privacy policy and terms updated for public profiles and leaderboards: what becomes public and where, that it is self-reported, how to opt out (make the profile private) and how fast it takes effect, retention, the name filter. Plain English; mark what changed and the date. `docs/legal/` and the site pages in step.
6. Accessibility WCAG 2.2 AA (axe clean), keyboard, phone and desktop, CLS 0. Tests: database tests for every view and policy, unit tests for the page's data shaping, the build tests in every build mode, a local end-to-end check with synthetic users only. Plan rows **T-506** and **T-509** done with notes; master plan notes.

## Out of scope

Opening sign-ups (the hub, D-078), community hero stats (T-507), rating, Spanish strings (T-508), the CSP, 066's and 068's areas.

## Acceptance criteria

- [ ] Boards per mode and period with thresholds, game counts and explanations; only public profiles; refresh tested.
- [ ] Name filter on every write path; refused names hidden; tested.
- [ ] Privacy policy and terms cover public stats and leaderboards.
- [ ] `/code-review`, `/security-review` and a second AI with no open CRITICAL or HIGH; branch handed to the hub; post-merge checks reported.

## Red line

Nothing becomes public beyond D-075. Never print keys, tokens, the user's email or BattleTags. No production writes by hand. No CSP change, no new dependency without the user's OK.

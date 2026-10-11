# Session 072: Community hero stats on the website (T-507, T-401)

Model: opus
Effort: high (consent, aggregation, what becomes public), medium for the page
Subagents: haiku for searches; sonnet for code; opus for the reviews (`database-reviewer`, `security-reviewer`)

Work in your worktree **`C:\Users\andia\tavern-ledger\.claude\worktrees\072-web-community-heroes`** (branch `072-web-community-heroes`, created by the hub) and follow its `CLAUDE.md`. Chat in caveman mode; the user writes in Spanish, answer in English. Everything in the repo is in English. **Tier R2 plus a second AI** (new aggregate data shown to everyone, a consent switch, legal text): TDD + `/code-review` + `/security-review` + a second AI review (`/santa-loop` or `ask-chatgpt`).

**Other sessions at the same time:** 071 (combat odds in the app, `crates/`). Your area: `supabase/`, new files under `web/src/pages/heroes*` and `web/src/lib/community/` (or similar), the account page (the consent switch), the privacy policy and terms (`web/src/pages/privacy.astro`, `terms.astro`, `docs/legal/`), plus docs. Shared design-system files (`web/src/components/data/`, `web/src/lib/view/`, `web/src/lib/data/`, `data.css`): add, do not rewrite. No CSP change, no new dependency without the user's OK.
- Shared files (`plan.md`, `README.md`, `DECISIONS.md`): only your rows and notes; decision number if needed **D-085** (071 has D-084). Every changed text file LF (`git ls-files --eol`, index side; the checkout uses `core.autocrlf=true`).
- The local Supabase stack is free; start it from your worktree and stop it when done.
- To catch up: `git fetch origin && git merge origin/main`. No rebase, never force-push. Stop any dev server you start.

Working rules: the user is asleep or busy; do not wait on them. Questions go in your hand-over with a recommendation. Never touch production by hand (D-031); never open sign-ups. `gh` is not available: push the branch and message the hub (`tavern-ledger-2b`, `ListAgents`, use its `[ref]`) with the branch, PR title and a short PR body. **Merging deploys the site, migrations and functions:** afterwards check `/auth/v1/settings` still says `disable_signup: true`, functions answer `401` without a token, the validator answers its version, every page answers 200, the CSP is unchanged, and the community view shows nothing that breaks the thresholds; tell the hub.

## The rules that apply

- **Plan rule (T-401, "Rules and limits"):** community stats are **opt-in and off by default, never switched on by an update**. So a separate switch on the account page, "Include my games in community hero stats", off by default for everyone, existing accounts included. A public profile does not imply it (and a private profile can still opt in: the aggregate shows no player).
- **D-075's spirit:** honest numbers with volume thresholds; never a 1-game 100%.
- Absolute rule 6 (GDPR): the privacy policy says what is aggregated, from whom (only those who opted in), that no player is shown, how to opt out and how fast it takes effect; the export includes the switch's state.
- No rating (D-076). No opponents' names, no BattleTags.

## Task

1. **Consent:** a `profiles` column (or a separate table) for the opt-in, off by default, settable only by the owner, in the export, removed with the account; the account page switch with a plain one-line explanation and a link to the policy.
2. **Aggregates:** per mode (Solo, Duos), patch or period (choose: the current game build range, last 30 days, all time), per hero: games, pick rate when offered (from the hero choice data), average place, top-4 / top-2 rate, win rate, with the number of games and of distinct players behind each. Thresholds: a hero shows only with enough games **and** enough distinct players (for example 50 games from at least 5 players; choose and explain); below that, "Not enough games yet". Only finished games with a server summary; the same impossible-value exclusions as the leaderboards (D-082). Built on a schedule (like the leaderboard snapshot), read through a function that returns only rows above the thresholds; no per-player rows ever leave the database.
3. **Page `/heroes/`** on the design system and client renderers: mode and period switches, a sortable compact table (hero picture, games, players, pick rate, average place, top-half rate), Simple and Advanced, "How this is counted" fold (opt-in only, thresholds, self-reported, validator), the empty state while there is too little data (today there is one account). In the menu; `noindex` like the leaderboards.
4. **Texts:** privacy policy and terms sections for community stats, dated, with a "What changed" line, matching `docs/legal/`.
5. Tests: database tests (opt-in off by default, thresholds, a private and a public profile, opt-out takes effect, no per-player data reachable, export and delete), unit tests for the page shaping, build tests in every mode, a local end-to-end check with synthetic users. Plan rows **T-507** and **T-401** done with notes; master plan notes.

## Out of scope

Opening sign-ups (the hub, D-078), Spanish strings (T-508), the CSP, `crates/`.

## Acceptance criteria

- [ ] Opt-in switch, off by default; aggregates with thresholds by games and players; nothing per player exposed; all tested.
- [ ] `/heroes/` page with honest empty state; texts updated.
- [ ] `/code-review`, `/security-review` and a second AI with no open CRITICAL or HIGH; branch handed to the hub; post-merge checks reported.

## Red line

Nothing becomes public beyond the thresholds and the opt-in. Never print keys, tokens, the user's email or BattleTags. No production writes by hand. No CSP change, no new dependency without the user's OK.

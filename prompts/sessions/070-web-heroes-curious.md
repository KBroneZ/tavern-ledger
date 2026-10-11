# Session 070: Heroes and curious stats pages on the website (T-505)

Model: opus
Effort: medium
Subagents: haiku for searches; sonnet for code and reviews (`code-reviewer`, `a11y-architect`)

Work in your worktree **`C:\Users\andia\tavern-ledger\.claude\worktrees\070-web-heroes-curious`** (branch `070-web-heroes-curious`, created by the hub) and follow its `CLAUDE.md`. Chat in caveman mode; the user writes in Spanish, answer in English. Everything in the repo is in English. **Tier R1** (pages on the existing data API, no new data exposed): TDD + `/code-review`.

**Other sessions at the same time:** 069 (public leaderboards: `supabase/`, `web/src/pages/leaderboards*`, `web/src/lib/leaderboards/`, privacy and terms pages, the account page's name check) and 068 (combat simulator, `crates/combat-sim/`). To stay out of their way:
- Work only in your worktree. Never touch the main checkout or other worktrees.
- Your area: new files under `web/src/pages/` for your two pages, `web/src/lib/heroes/` (or similar), `web/src/scripts/` for their scripts, plus links to them from the dashboard (`/me/`, the public profile). Nothing in `supabase/`, `crates/`, the CSP, the legal pages. Shared design-system files (`web/src/components/data/`, `web/src/lib/view/`, `web/src/lib/data/`, `data.css`): add, do not rewrite.
- `/heroes/` is reserved for the community hero stats (T-507); pick other paths for the per-player pages (for example `/my-heroes/?id=` and `/curious/?id=`, in the `?id=` style of `/profile/` and `/games/`). Record the choice in the plan notes.
- If the data API lacks something a page needs, do not change `supabase/`: show what exists, and list the gaps in your hand-over for a later summary version.
- Shared files (`plan.md`, `README.md`, `DECISIONS.md`): only your rows and notes; decision number if needed **D-083** (069 has D-082). Every changed text file LF (`git ls-files --eol`, index side; the checkout uses `core.autocrlf=true`).
- To catch up: `git fetch origin && git merge origin/main`. No rebase, never force-push. Stop any dev server you start. The local Supabase stack belongs to 069 right now: ask the hub before starting it.

Working rules: decide technical and design matters yourself (D-052, D-070, D-073, D-079). The user is asleep: do not wait on them; questions go in your hand-over. `gh` is not available: push the branch and message the hub (`tavern-ledger-2b`, `ListAgents`, use its `[ref]`) with the branch, PR title and a short PR body (summary, review results, test plan). **Merging deploys the site:** afterwards check every page answers 200 and the CSP header is unchanged, and tell the hub.

## Why

Round 5 (read **`docs/plan/web-master-plan.md`**, the information architecture rows for heroes and curious stats). T-503 (#122, D-073) built `/me/`, the public profile and `/games/` with the client renderers in `web/src/lib/view/`; T-504 (#124, D-079) built the game viewer with pictures from HearthstoneJSON. The data: `user_heroes` and `user_records` views and `game_summaries` (read **`docs/research/web-data-api.md`**).

## Task

1. **Heroes page** (owner and public profiles): per mode, one row per hero with picture, games, average place, top-4 or top-2 rate, wins, best and worst place, pick rate from hero select (`picked_games / offered_games`, with both counts); sort by games or average place; top N with "Show all"; a minimum-games note where a rate rests on few games. Simple: the main columns; Advanced: everything. Each hero links to the games list filtered by that hero.
2. **Curious stats page** (owner and public profiles): the records and fun numbers the data backs (`user_records`, `game_stats`: longest win streak, biggest combat damage dealt and taken, biggest board, most golden minions, most rolls, most gold beyond income, highest APM, most triples…), each with its value, the game it came from (link to `/game/`), and how many games back it; inferred values marked. Compare with the app's curious stats (T-216, `crates/tracker`, D-058): list the ones the website cannot show yet and why.
3. Links from `/me/` and the public profile; same privacy as those pages (a private profile reads "This profile is private" to others).
4. States: loading with reserved space (CLS 0), empty, unknown, network error with retry.
5. Accessibility WCAG 2.2 AA (axe clean), keyboard, phone and desktop. Tests: unit tests for the data shaping and sorting; the build tests in every build mode; a local end-to-end check only if 069 has released the stack (ask the hub). Plan row **T-505** done with notes.

## Out of scope

Community hero stats (T-507), leaderboards, Spanish strings (T-508; keep strings in one place), `supabase/` changes, the CSP.

## Acceptance criteria

- [ ] Heroes and curious stats pages on live data, owner and public; private stays private; all states.
- [ ] axe clean, keyboard, CLS 0; `test` and `test:dist` green (paste counts).
- [ ] `/code-review` with no open CRITICAL or HIGH; branch handed to the hub; post-merge checks reported.

## Red line

No real player data in fixtures or screenshots, no Blizzard art in the repo, no CSP change, no new dependency without the user's OK; never print keys, tokens, the user's email or BattleTags.

# Session 062: Design system for the web data pages (T-501)

Model: opus
Effort: medium
Subagents: haiku for searches; sonnet for code and reviews (`code-reviewer`, `a11y-architect`)

Work in your worktree **`C:\Users\andia\tavern-ledger\.claude\worktrees\062-web-design-system`** (branch `062-web-design-system`, created by the hub) and follow its `CLAUDE.md`. Chat in caveman mode; the user writes in Spanish, answer in English. Everything in the repo is in English. **Tier R1**: TDD where there is logic + `/code-review`.

**Another session runs at the same time:** 063 (data API, `supabase/` only). To stay out of its way:
- Work only in your worktree. Never touch the main checkout or other worktrees.
- Your area: `web/` only. Nothing in `crates/`, `supabase/`, `tools/`, `tests/`.
- Shared files (`plan.md`, `README.md`, `DECISIONS.md`, `PROVENANCE.md`): only your task's row and notes. Your decision number is **D-070**, only if needed. Keep every changed text file LF (`git ls-files --eol`; check the index side, the checkout uses `core.autocrlf=true`).
- To catch up with `main`: `git fetch origin && git merge origin/main`. No rebase, never force-push.
- Stop any dev server you start (`astro dev`, `astro preview`) before you hand over.

Working rules (user's decisions):
- Decide technical matters yourself. Ask the user, **through the hub** as well as in your tab: new dependencies, any change to the CSP.
- `gh` is not available in task sessions. When your branch is ready (tests green, reviews done, no open CRITICAL or HIGH), push it and message the hub (`tavern-ledger-2b`, find it with `ListAgents`, use its `[ref]`) with the branch, the PR title and a short PR body (summary, review results, test plan). The hub opens the PR, waits for CI and merges it. **Merging to `main` deploys the site:** afterwards check every page answers 200 and the CSP header is unchanged, and tell the hub.

## Why

Round 5 puts everything the app knows on the website (read **`docs/plan/web-master-plan.md`** first, and D-030, D-052, D-056, D-062, D-065). The data pages (dashboard, games list, game viewer, heroes, curious stats, leaderboards) need one shared set of pieces first, so every later task (T-503 to T-507) builds from the same parts and looks the same.

The user's taste, from the app rounds: compact, visual, key numbers first and details folded, **Simple by default with an Advanced mode**, no walls of text, no white-edged pictures, no native scrollbars that look cheap, English and Spanish later (T-508: keep every string in one place so it can be translated).

## At the start

1. Conversation title: **`#062 Web design system`**.
2. `git fetch origin && git merge origin/main`.
3. Read the master plan, the site's current look (`web/src/styles/`, `web/src/layouts/Base.astro`), the design skills in `.claude/skills/` and the `dataviz` skill.

## Task

1. **Tokens** for the data pages on top of the Lamplight glass tokens (D-052): surfaces, text levels, the semantic colours for places (1st, top-4/top-2, bottom), win / loss / tie, "inferred" and "unknown", spacing and type scale for dense data. Light theme not needed (the site is dark).
2. **Components** (Astro, no framework unless you ask the user first): stat tile (value, label, "in 28 of 31 games" backing line, optional trend), compact table with top N and "show all", fold (details/summary styled, keyboard and screen reader friendly), Simple/Advanced switch (remembered per viewer), mode switch (Solo / Duos), place badge, hero and card picture with the crop the app uses (053, D-065: no white edges), empty, unknown and loading states, a thin styled scrollbar.
3. **Charts** following the `dataviz` skill: place distribution bar, place-over-time sparkline, per-turn bars for the game viewer. Inline SVG rendered at build time or a tiny local script; no chart library unless the user agrees.
4. **Pictures:** card art is **not** allowed by the CSP yet (decision pending, the hub asks the user when T-504 comes). Build the picture component so the source is one setting, and use local, obviously synthetic placeholders here. Never bundle Blizzard art.
5. **Style page** (`/style/`, not linked from the menu, `noindex`): every component with invented data (no real player data), all states, at phone and desktop widths.
6. Accessibility: WCAG 2.2 AA, axe clean on the style page, keyboard for folds and switches, reduced motion respected. Performance: no layout shift, within the budget in the web rules.
7. Tests: unit tests for any logic (formatting, top N, thresholds text); the existing `test` and `test:dist` stay green. Plan row **T-501** done with notes; short notes in the master plan if a piece changed.

## Out of scope

Real data and Supabase calls (T-502, T-503), new pages beyond `/style/`, the CSP, Spanish strings, anything outside `web/`.

## Acceptance criteria

- [ ] Tokens and components above, used on `/style/` with invented data, phone and desktop.
- [ ] axe clean, keyboard works, no layout shift; `test` and `test:dist` green (paste counts).
- [ ] `/code-review` with no open CRITICAL or HIGH; branch handed to the hub; post-merge checks reported.

## Red line

No real player data, no Blizzard art in the repo, no CSP change, no new dependency without the user's OK.

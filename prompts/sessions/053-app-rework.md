# Session 053: Main window rework, compact and visual (T-320 to T-323)

Model: opus
Effort: high (layout decisions), medium for the rest
Subagents: haiku for searches; sonnet for code and reviews

Work in your worktree **`C:\Users\andia\tavern-ledger\.claude\worktrees\053-app-rework`** (branch `053-app-rework`, created by the hub) and follow its `CLAUDE.md`. Chat in caveman mode; the user writes in Spanish, answer in English. Everything in the repo is in English. **Tier R1** (UI only; nothing new is uploaded or parsed): TDD + `/code-review`.

**Other sessions at the same time:** 052 (overlay data: overlay files, `crates/tracker/src/live.rs`, the parser, the validator; it also adds one field to `Status` in `main.rs` and one line to `renderStatus` in `ui/app.js`) and 054 (tribes research: docs only). To stay out of their way:
- Work only in your worktree. Never touch the main checkout or other worktrees.
- Your area: the **main window** of `crates/desktop/` (`ui/index.html`, `app.js`, `cards.js`, `curious.*`, `extras.js`, `round-two.*`, `shop.*`, `style.css`, `tabs.js`, `upload.*`, `focus-keeper.js`, new files of yours; `src/main.rs` and `src/card_data.rs` only where the window needs it) and, if a view needs it, small data changes in `crates/tracker/src/recap.rs`, `stats.rs`, `stats/`. **Do not touch** the overlay files (`ui/overlay*`, `src/overlay*.rs`, `hover.rs`, `leaderboard.rs`), `crates/bg-parser/`, `supabase/`, `web/`. Leave `renderStatus` and the `Status` struct alone (052 changes them).
- Shared files (`plan.md`, `README.md`, `DECISIONS.md`): only your tasks' rows and notes. Your decision number, if you need one, is **D-062**. Keep every changed text file LF (`git ls-files --eol`; the checkout uses `core.autocrlf=true`, so check the index side).
- Never stop or restart the user's desktop app; test with your own build and a scratch copy of the user's history (`%APPDATA%\TavernLedger\games.jsonl`, read only). After a build, `git checkout -- crates/desktop/gen/schemas`. CI does not build `crates/desktop` nor run its Node UI tests: run them yourself and paste the counts. Note: two overlay checks in `ui_checks.rs` fail on a CRLF working copy and pass in CI (LF); fix the check to accept both line endings if it is trivial.
- To catch up with `main`: `git fetch origin && git merge origin/main`. No rebase, never force-push.

Working rules (user's decisions):
- Decide technical and design matters yourself (use the design skills in `.claude/skills/`, Lamplight glass, D-052). Ask the user only about new dependencies.
- **Ask your questions through the hub** (`tavern-ledger-47`, find it with `ListAgents` and use its `[ref]`) as well as in your tab.
- **`gh` is not available in task sessions.** When your branch is ready (tests green, review done, no open CRITICAL or HIGH), push it and message the hub with the branch, the PR title, a short PR body (summary, review results, test plan) and anything still waiting on the user. The hub opens the PR, merges it and rebuilds the user's app.

## Why: the user's words (2026-10-11), after using the app

- "There is still too much." The heroes table should not list every hero played: only the most played, ordered (ties broken sensibly), with a fold or "show all" for the rest. **The same for hero picks and for everything else**: compact but easy to use.
- Opening a recap still opens a dropdown inside the list. It should **fill the whole app window**, with a **close button (X) at the top right** that goes back to where you were (T-322). Escape closes it too; focus returns to the game you opened.
- Some hero and card pictures show **white at the sides**, as if the circle crop were not fitted to the image (T-320). Also check pictures that do not load at all.
- Turns: a clearer, more visual look per turn; when a turn is opened, its shops sit behind a "See shops" toggle (T-321).
- Fewer details at once across the app; secondary things behind toggles (T-323).
- "I don't like the scrollbar showing at the side, it feels like a low-quality app." Hide the native scrollbar or replace it with a thin one in the Lamplight style that only shows while scrolling or on hover; scrolling must still work by wheel, touchpad and keyboard, and a visible cue of more content must remain (fade or shadow at the edge). Best of all: a layout where the main views fit without a page scrollbar.

## At the start

1. Conversation title: **`#053 App rework`**.
2. `git fetch origin && git merge origin/main`.
3. Read the plan section "App rework" (T-320 to T-326), D-052, D-057, D-060, and the main window UI. Run your own build on the scratch history and look at every view first (screenshots for yourself; never share names or BattleTags).

## Task

1. **Compact lists:** every long table (heroes, hero picks, trinkets and Dark Gifts, opponents, tribes, …) shows the top 5 to 8 by games (ties: better average place, then name), with "Show all (N)" to expand; sortable where it helps. Same rule everywhere (one shared helper).
2. **Recap view (T-322):** the recap replaces the main content (not a fold, not a new window), with a header (hero, place, date), an X at the top right and Escape to close, focus handled (WCAG), the list's scroll position and selection kept on return.
3. **Turns (T-321):** each turn as a compact visual row or card (round, result, damage, board summary); opening it shows the board and the rest; shops behind "See shops".
4. **Pictures (T-320):** fix the white edges (object-fit and crop sizing, background, the circle mask fitted to the art) for heroes and cards everywhere in the main window; find ids that fail to load and show a clean placeholder; tests where possible.
5. **Less at once (T-323):** go view by view; keep the key number visible, move the rest behind toggles. Keep 045's accessibility (focus kept, keyboard, labels) and 051's folds.
6. Node UI tests and markup checks; plan rows T-320 to T-323 with notes; README; same PR.

## Out of scope

The overlay (052), settings and languages (T-324, T-325, a later session), tribes (054), uploads, the website.

## Acceptance criteria

- [ ] Long lists show top N with "Show all", one shared helper, tested.
- [ ] Recap fills the window, X and Escape close it, focus and scroll restored (tested).
- [ ] No white edges on hero or card pictures; failed pictures show a placeholder.
- [ ] No native side scrollbar; scrolling still works by wheel, touchpad and keyboard, with a cue of more content.
- [ ] `cargo test --workspace`, the desktop Node UI tests and a release build of `desktop` pass locally (paste the counts); before/after screenshots described in the hand-over.
- [ ] `/code-review` with no open CRITICAL or HIGH; branch handed to the hub.

## Red line

D-004, D-006, D-012, D-022 apply. Never print BattleTags, `GameAccountId`, player names or the user's email.

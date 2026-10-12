# Session 050: Overlay accessibility follow-up (T-319)

Model: opus
Effort: medium
Subagents: haiku for searches; sonnet for code and reviews

Work in your worktree **`C:\Users\andia\tavern-ledger\.claude\worktrees\050-overlay-pointer-a11y`** (branch `050-overlay-pointer-a11y`, created by the hub) and follow its `CLAUDE.md`. Chat in caveman mode; the user writes in Spanish, answer in English. Everything in the repo is in English. **Tier R1** (UI only; nothing new is uploaded): TDD + `/code-review`.

**Other sessions run at the same time:** 049 (parser round 2: `crates/bg-parser/`, `crates/tracker/`, the validator; it does not touch `crates/desktop/`). To stay out of its way:
- Work only in your worktree. Never touch the main checkout or other worktrees.
- Your area: `crates/desktop/` (overlay UI and the window code it needs). Do not touch `crates/tracker/`, `crates/bg-parser/`, `supabase/` or `web/`.
- Shared files (`plan.md`, `README.md`, `DECISIONS.md`): only your task's row and notes. Add the plan row **T-319** next to T-317. Your decision number, if you need one, is **D-059**. Keep every changed text file LF (`git ls-files --eol` before committing; the checkout uses `core.autocrlf=true`, so check the index side).
- Never stop or restart the user's desktop app; test with your own build, a scratch data folder and `--overlay-unlock`. After a build, `git checkout -- crates/desktop/gen/schemas`. CI does not build `crates/desktop` nor run its Node UI tests: run them yourself and paste the counts in the hand-over.
- To catch up with `main`: `git fetch origin && git merge origin/main`. No rebase, never force-push.

Working rules (user's decisions):
- Decide technical matters yourself. Ask the user only about new dependencies.
- **Ask your questions through the hub** (`tavern-ledger-47`, find it with `ListAgents` and use its `[ref]`) as well as in your tab: the user follows the hub from their phone, not your tab.
- **`gh` is not available in task sessions.** When your branch is ready (tests green, review done, no open CRITICAL or HIGH), push it and message the hub with the branch, the PR title, a short PR body (summary, review results, test plan) and anything still waiting on the user. The hub opens the PR, waits for CI, merges it and rebuilds the user's app.

## Why

Session 045 (T-317) added keyboard arranging for the overlay and left three items open (plan notes T-317, "Left open"): a single-pointer alternative to dragging (WCAG 2.2 SC 2.5.7 Dragging Movements), the toolbar or hint covering a panel at the screen edge, and a second contrasting ring for the focus outline.

## At the start

1. Conversation title: **`#050 Overlay pointer a11y`**.
2. `git fetch origin && git merge origin/main`.
3. Read the plan notes T-311 and T-317, D-052, `crates/desktop/ui/overlay*.js`, `overlay.css`, `focus-keeper.js`, `crates/desktop/src/overlay*.rs` and the UI tests in `crates/desktop/ui-tests/`.

## Task

1. **Single-pointer move and size.** While unlocked, the selected panel (and the leaderboard box) gets on-screen buttons to move it (four directions) and to make it bigger or smaller, using the same steps and limits as the keys in `overlay-keys.js` and the same delayed save into `overlay.json`. Buttons of at least 24 px (2.5.8), labelled, reachable by keyboard, hidden when locked.
2. **Edge overlap.** The toolbar and the hint never cover a panel at the screen edge: place them on the side with room (or flip), tested with panels at each edge.
3. **Focus ring.** A second contrasting ring (outline plus inner or outer ring) so focus is visible on both light and dark game backgrounds; the contrast test stays green, Glass and High contrast themes both.
4. Node UI tests and markup checks for the above.
5. Plan row T-319 with notes, README, same PR.

## Out of scope

The overlay data items (T-312 to T-315, next session), the main window, the website, uploads.

## Acceptance criteria

- [ ] Every panel can be moved and resized with single clicks only, saved like a drag.
- [ ] Toolbar and hint never cover a panel at any screen edge (tested).
- [ ] Two-colour focus ring; contrast test green.
- [ ] `cargo test --workspace`, the desktop Node UI tests and a release build of `desktop` pass locally (paste the counts).
- [ ] `/code-review` with no open CRITICAL or HIGH; branch handed to the hub.

## Red line

D-004, D-006, D-012, D-022 apply: the overlay shows only what the player could see. Never print BattleTags, `GameAccountId`, player names or the user's email.

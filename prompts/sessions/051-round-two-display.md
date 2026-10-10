# Session 051: Show parser round 2 in the main window (T-207, T-208, T-211, T-212 display)

Model: opus
Effort: medium
Subagents: haiku for searches; sonnet for code and reviews

Work in your worktree **`C:\Users\andia\tavern-ledger\.claude\worktrees\051-round-two-display`** (branch `051-round-two-display`, created by the hub) and follow its `CLAUDE.md`. Chat in caveman mode; the user writes in Spanish, answer in English. Everything in the repo is in English. **Tier R1** (display only; nothing new is uploaded): TDD + `/code-review`. If you end up changing what is uploaded or parsed, it becomes R2: stop and tell the hub first.

**Other sessions run at the same time:** 050 (overlay accessibility: `crates/desktop/ui/overlay*.js`, `overlay.css`, `crates/desktop/src/overlay*.rs`). To stay out of its way:
- Work only in your worktree. Never touch the main checkout or other worktrees.
- Your area: the **main window** of `crates/desktop/` (`ui/index.html`, `app.js`, `extras.js`, `shop.js`, `curious.js`, `style.css` and new files of yours, `src/main.rs` and `src/card_data.rs` if needed) and, in `crates/tracker/`, `recap.rs`, `stats.rs`, `stats/` and `choices.rs` if the window needs a small data change. **Do not touch the overlay files** listed above, `crates/bg-parser/`, `supabase/` or `web/`.
- Shared files (`plan.md`, `README.md`, `DECISIONS.md`): only your tasks' rows and notes. Your decision number, if you need one, is **D-060**. Keep every changed text file LF (`git ls-files --eol` before committing; the checkout uses `core.autocrlf=true`, so check the index side).
- Never stop or restart the user's desktop app; test with your own build, a scratch data folder (copy of the user's history is fine, read only) and the log replay (T-D02). After a build, `git checkout -- crates/desktop/gen/schemas`. CI does not build `crates/desktop` nor run its Node UI tests: run them yourself and paste the counts in the hand-over.
- To catch up with `main`: `git fetch origin && git merge origin/main`. No rebase, never force-push.

Working rules (user's decisions):
- Decide technical matters yourself. Ask the user only about new dependencies or anything about Blizzard's rules you are unsure of.
- **Ask your questions through the hub** (`tavern-ledger-47`, find it with `ListAgents` and use its `[ref]`) as well as in your tab: the user follows the hub from their phone, not your tab.
- **`gh` is not available in task sessions.** When your branch is ready (tests green, review done, no open CRITICAL or HIGH), push it and message the hub with the branch, the PR title, a short PR body (summary, review results, test plan) and anything still waiting on the user. The hub opens the PR, waits for CI, merges it and rebuilds the user's app.

## Why

Session 049 (D-058, parser revision 5) added data the window does not show yet: each opponent's triples and trinkets per turn as the leaderboard shows them, keywords on every board minion, the choices offered to the player (discovers, trinket offers, Dark Gifts, quests, triple rewards) with the one taken, the Old God, and `tracker::choices` stats (per trinket and Dark Gift: offered, taken, pick rate, average place when taken). The user's history was reparsed with revision 5.

## At the start

1. Conversation title: **`#051 Round two display`**.
2. `git fetch origin && git merge origin/main`.
3. Read D-055, D-057, D-058, `docs/research/parser-round-2.md`, the plan rows T-207, T-208, T-211, T-212, `crates/tracker/src/choices.rs`, `recap.rs`, `stats.rs`, and the main window UI (Lamplight glass, D-052; keep 045's accessibility work: focus kept, keyboard, labels).

## Task

1. **Recap:** per opponent, triples and trinkets as of each turn met (and at the end); keywords on the boards seen (short, readable marks with text alternatives, not colour only); the player's choices per turn (offered cards named by the card data, the taken one marked); trinkets taken and the Old God. Older records say "not recorded", never empty or zero.
2. **Stats:** the choices table per mode (per trinket and per Dark Gift: offered, taken, pick rate, average place when taken), with the number of games behind it, like 048's hero picks.
3. **Curious stats:** add any stat the new data backs honestly (for example triples, now that `lobby[].triples` exists for the player too if it does; check), each with source and game count. Drop what the data cannot back.
4. Node UI tests and markup checks; `tracker` tests for any data change.
5. Plan rows (the "in the window" part of T-207, T-208, T-211, T-212), README, D-060 only if a real choice was made; same PR.

## Out of scope

The overlay and its hover card (session 052), golden by `PREMIUM` (052), the parser, uploads, the website.

## Acceptance criteria

- [ ] 049's data visible in recap and stats; old records say "not recorded".
- [ ] Keyword marks readable without colour and announced to screen readers.
- [ ] `cargo test --workspace`, the desktop Node UI tests and a release build of `desktop` pass locally (paste the counts).
- [ ] `/code-review` with no open CRITICAL or HIGH; branch handed to the hub.

## Red line

D-004, D-006, D-012, D-022, D-054 apply. Never print BattleTags, `GameAccountId`, player names or the user's email.

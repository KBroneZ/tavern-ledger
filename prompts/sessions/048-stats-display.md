# Session 048: Show 044's data in the app, and the "Curious stats" tab (T-216)

Model: opus
Effort: medium (high for the stats definitions)
Subagents: haiku for searches; sonnet for code and reviews

Work in your worktree **`C:\Users\andia\tavern-ledger\.claude\worktrees\048-stats-display`** (branch `048-stats-display`, created by the hub) and follow its `CLAUDE.md`. Chat in caveman mode; the user writes in Spanish, answer in English. Everything in the repo is in English. **Tier R1** (display and local stats; nothing new is uploaded): TDD + `/code-review`. If you end up changing what is uploaded, it becomes R2: stop and tell the hub first.

**Other sessions run at the same time:** 049 (parser round 2: `crates/bg-parser/`, the validator, new tracker modules). To stay out of each other's way:
- Work only in your worktree. Never touch the main checkout or other worktrees.
- Your area: `crates/desktop/` (src and ui) and, in `crates/tracker/`, `stats.rs` and `recap.rs` (you own these two files this round; 049 waits for your merge before it touches them). Do not touch `crates/bg-parser/`, `supabase/` or `web/`.
- Shared files (`plan.md`, `README.md`, `DECISIONS.md`, `PROVENANCE.md`): only your tasks' rows and notes. Your decision number, if you need one, is **D-057**. Keep every changed text file LF (`git ls-files --eol` before committing; the checkout uses `core.autocrlf=true`, so check the index side).
- Never stop or restart the user's desktop app; test with your own build, a scratch data folder and the log replay (T-D02). After a build, `git checkout -- crates/desktop/gen/schemas`. CI does not build `crates/desktop` nor run its Node UI tests: run them yourself and say so in the hand-over.
- To catch up with `main`: `git fetch origin && git merge origin/main`. No rebase, never force-push.

Working rules (user's decisions):
- Decide technical matters yourself. Ask the user only about new dependencies or anything about Blizzard's rules you are unsure of.
- **Ask your questions through the hub** (`tavern-ledger-47`, find it with `ListAgents` and use its `[ref]`) as well as in your tab: the user follows the hub from their phone, not your tab.
- **`gh` is not available in task sessions.** When your branch is ready (tests green, review done, no open CRITICAL or HIGH), push it and message the hub with the branch, the PR title, a short PR body (summary, review results, test plan) and anything still waiting on the user. The hub opens the PR, waits for CI and merges it; after the merge it rebuilds the user's app.

## Why

Session 044 (D-055) added round results "from the log", the hero select record, shop extras and skin links, but only as data in `tracker`. The window does not show them yet, and `crates/desktop/src/main.rs` still calls `stats::compute_with_entered` instead of `compute_with_links`, so skins are not grouped by the game's link. The user also asked for a "Curious stats" tab (T-216): fun totals across all games, each honest about its source.

## At the start

1. Conversation title: **`#048 Stats display`**.
2. `git fetch origin && git merge origin/main`.
3. Read D-019, D-028, D-046, D-055, the plan rows T-206, T-209, T-210, T-214, T-216, `crates/tracker/src/stats.rs` and `recap.rs`, `crates/desktop/src/main.rs` (around `compute_with_entered`), `crates/desktop/src/card_data.rs` (`hero_links`), and the UI in `crates/desktop/ui/` (Lamplight glass, D-052; keep 045's accessibility work: focus, keyboard, labels).

## Task

1. **Skins by link.** Pass `cards.hero_links()` into `stats::compute_with_links` in `main.rs`; keep the D-019 name rule only as the labelled fallback when the link is missing. Test it.
2. **Show 044's data** in the recap and stats, in the Lamplight look:
   - Round results: mark which came from the log and which were inferred from health (D-055), and fewer "unknown".
   - Hero select: heroes offered, rerolls, pick; in the stats per hero: offered, picked, pick rate, average place when picked.
   - Shop extras: extra gold, free rolls and why, buy and sell prices, Duos passes.
   - Older records say "not recorded" (re-read with `--reparse`), never zero.
3. **T-216 "Curious stats" tab.** A new tab in the main window. Start with: gold spent, gold beyond the fixed income (extra gold, `TEMP_RESOURCES`), golden Brann or Titus sold, rolls (free ones apart), triples, the most bought minion, the longest win streak (combats), the most damage taken in one combat. Each stat shows its source (log, inferred) and how many games it covers ("in 14 of 29 games"); a stat with no data says so, never a guess. Computed in `tracker::stats` with tests (synthetic reports, plus old reports without the fields). Drop any stat the data cannot back and say why in the plan note.
4. Node UI tests for the new tab and the new recap parts; keyboard and screen-reader labels as 045 did.
5. Plan rows (T-216 and the "in the window" follow-ups of T-206, T-209, T-210, T-214), README, D-057 only if a real choice was made; same PR.

## Out of scope

The parser and new log fields (049), the overlay (T-312 to T-315, later session), the web profile version of the curious stats, uploads.

## Acceptance criteria

- [ ] `compute_with_links` used; skins grouped by link, tested.
- [ ] 044's data visible in recap and stats; old records say "not recorded".
- [ ] Curious stats tab with source and game count per stat, tested in `tracker` and the UI.
- [ ] `cargo test --workspace`, the desktop Node UI tests and a release build of `desktop` pass locally (paste the counts).
- [ ] `/code-review` with no open CRITICAL or HIGH; branch handed to the hub.

## Red line

D-004, D-006, D-012, D-022 apply. Never print BattleTags, `GameAccountId`, player names or the user's email.

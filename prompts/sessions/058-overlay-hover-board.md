# Session 058: Overlay hover board across the top, readable size, wrong-board bug (T-333 to T-335)

Model: opus
Effort: high (the wrong-board bug and the geometry), medium for the rest
Subagents: haiku for searches; sonnet for code and reviews

Work in your worktree **`C:\Users\andia\tavern-ledger\.claude\worktrees\058-overlay-hover-board`** (branch `058-overlay-hover-board`, created by the hub) and follow its `CLAUDE.md`. Chat in caveman mode; the user writes in Spanish, answer in English. Everything in the repo is in English except the Spanish strings. **Tier R1** (overlay display; nothing uploaded): TDD + `/code-review`.

**Other sessions at the same time:** 055 (memory reader: a new crate, `crates/tracker/` wiring for tribes and rating, a config flag, Tauri commands and the switch in the Settings tab, texts in `web/` and `docs/legal/`). To stay out of its way:
- Work only in your worktree. Never touch the main checkout or other worktrees.
- Your area: the overlay (`crates/desktop/ui/overlay*`, `overlay.css`, `overlay.html`, `crates/desktop/src/overlay*.rs`, `hover.rs`, `leaderboard.rs`, `screen_fit.rs`), overlay strings in `ui/strings/overlay.js`, and in `crates/tracker/` only `live.rs` and `live_tests.rs` if the bug lives there. Do not touch the Settings tab, the reader crate, `web/`, `docs/legal/`, `supabase/`, `crates/bg-parser/`.
- Shared files: only your tasks' rows and notes; decision number if needed **D-067**. Every changed text file LF (`git ls-files --eol`, index side).
- Never stop or restart the user's desktop app. **Never show your test build's overlay over the user's game** (session 056 did for 7 s; it may take clicks). Test with your own build, the log replay (T-D02), a scratch data folder and `--overlay-unlock`, only while Hearthstone is closed or with your overlay click-through. After a build, `git checkout -- crates/desktop/gen/schemas`. CI does not build `crates/desktop` nor run its Node UI tests: run them yourself and paste counts.
- To catch up with `main`: `git fetch origin && git merge origin/main`. No rebase, never force-push.

Working rules: decide technical and design matters yourself (Lamplight glass D-052). Ask your questions through the hub (`tavern-ledger-47`, `ListAgents`, use its `[ref]`) as well as in your tab. `gh` is not available in task sessions: push the branch and message the hub with the branch, PR title and a short body.

## What the user said (2026-10-11, after two live Duos games)

1. "The overlay looks like it has small letters." Their screen: 3840x2160 at 150% scaling, game fullscreen. Text must be readable at a glance during a game: scale with the screen (DPI and resolution), not fixed pixel sizes; check the minimum sizes against the game's own leaderboard text.
2. "When you hover over the enemies, sometimes it shows another player's board, and it looks bad." **Bug:** the hover card is matched to the wrong opponent at times. Find why (leaderboard slot geometry vs the live order of heroes, Duos teams, a hero that moved slots after a combat, dead heroes, the "Next" block from 052, stale state). The leaderboard order in the game changes with health; check how `hover.rs` and `leaderboard.rs` map a slot to a hero and what the log gives (`PLAYER_LEADERBOARD_PLACE` or similar). Reproduce with the replay of a real Duos game (fixtures or the user's logs, shared read). Tests that fail before the fix.
3. "I want the board to be shown at the top, like in Deck Tracker; look for info and orient yourself with it." On hover over a leaderboard hero, the opponent's last board seen appears **across the top of the screen**, laid out like the game's own board (seven slots, minion pictures with attack and health, golden and keyword marks), with the hero, tier, triples and trinkets and the turn it was seen. Research **only from public material**: HDT's screenshots, release notes, wiki and videos of the feature (absolute rule 1: never its code, never HearthMirror, Firestone or BobsBuddy code; Nomi's Kitchen not at all). Write what you learned in `docs/research/overlay-hover.md` (a short section with sources).

## Rules (D-004, D-054)

Only what the player saw: the last board met in a combat the game played back, labelled with its turn; nothing from D-054's hidden Duos combats; opponents not met yet show the "possible" guess labelled as such (T-307), never as seen.

## Task

1. Fix the wrong-board bug, with tests.
2. Hover board across the top (T-333), sized to the screen, never covering the player's own board or the shop buttons in a way that blocks play (the overlay is click-through when locked).
3. Readable text sizes across the overlay (T-334), scaled to the screen; the contrast test stays green.
4. Pictures as in 053/056 (no white edges).
5. Plan rows T-333 to T-335 (T-335: the bug) with notes, README; same PR.

## Acceptance criteria

- [ ] Wrong-board bug reproduced in a test, then fixed.
- [ ] Board across the top on hover, game-like layout, labelled turn seen.
- [ ] Text readable at 3840x2160 @150% and at 1920x1080 @100% (tested sizes).
- [ ] `cargo test --workspace`, desktop Node UI tests, release build pass locally (paste counts); `/code-review` with no open CRITICAL or HIGH; branch handed to the hub with before/after notes.

## Red line

D-004 (as amended by D-063), D-006, D-012, D-022, D-054 apply. Never print BattleTags, `GameAccountId`, player names or the user's email.

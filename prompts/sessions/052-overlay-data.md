# Session 052: Overlay data items (T-312 to T-315) and golden by PREMIUM

Model: opus
Effort: high (log checks), medium for the rest
Subagents: haiku for searches; sonnet for code and reviews

Work in your worktree **`C:\Users\andia\tavern-ledger\.claude\worktrees\052-overlay-data`** (branch `052-overlay-data`, created by the hub) and follow its `CLAUDE.md`. Chat in caveman mode; the user writes in Spanish, answer in English. Everything in the repo is in English. **Tier R2** if golden by `PREMIUM` changes the saved report (it does: the upload changes): TDD + `/code-review` + `/security-review`.

**Other sessions at the same time:** possibly 051 (the main window's recap and stats: `crates/desktop/ui/` main window files, `tracker` `recap.rs`, `stats.rs`, `stats/`, `choices.rs`). To stay out of its way:
- Work only in your worktree. Never touch the main checkout or other worktrees.
- Your area: the overlay (`crates/desktop/ui/overlay*.js`, `overlay.css`, `overlay.html`, `crates/desktop/src/overlay*.rs`, `hover.rs`, `leaderboard.rs`), `crates/tracker/src/live.rs`, `live_tests.rs`, `tail.rs` and new tracker modules, `crates/bg-parser/` (for `PREMIUM` and `LoadingScreen.log`), and in `supabase/` only `report_keys.json`, `validate.ts` and their tests (D-047). **Do not touch** 051's files listed above or `web/`. If you must change `crates/desktop/src/main.rs`, keep it small and tell the hub.
- Shared files (`plan.md`, `README.md`, `DECISIONS.md`, `PROVENANCE.md`): only your tasks' rows and notes. Your decision number is **D-061**. Keep every changed text file LF (`git ls-files --eol` before committing; the checkout uses `core.autocrlf=true`, so check the index side).
- Never stop or restart the user's desktop app; test with your own build, a scratch data folder, `--overlay-unlock` and the log replay (T-D02). After a build, `git checkout -- crates/desktop/gen/schemas`. CI does not build `crates/desktop` nor run its Node UI tests: run them yourself and paste the counts.
- To catch up with `main`: `git fetch origin && git merge origin/main`. No rebase, never force-push.
- **Upload contract (D-047):** any new or changed report key goes into `report_keys.json` and `validate.ts`, `upload_contract.rs` stays green, `PARSER_REVISION` and `VALIDATOR_VERSION` are bumped. After the merge, check `POST …/functions/v1/upload-game` answers `405 … "validator": <new>` and tell the hub.
- **gitleaks scans the full history:** use obviously fake, low-entropy test values.

Working rules (user's decisions):
- Decide technical matters yourself. Ask the user only about new dependencies or anything about Blizzard's rules you are unsure of.
- **Ask your questions through the hub** (`tavern-ledger-47`, find it with `ListAgents` and use its `[ref]`) as well as in your tab: the user follows the hub from their phone, not your tab.
- **`gh` is not available in task sessions.** When your branch is ready (tests green, both reviews done, no open CRITICAL or HIGH), push it and message the hub with the branch, the PR title, a short PR body (summary, review results, test plan) and anything still waiting on the user.
- Fixtures: only the user's own logs through `tools/make_fixture.py`, or synthetic ones. Read raw logs with shared access; print counts, tag names and card ids only, never names, BattleTags or `GameAccountId`.

## Why

The overlay items from the log inventory (plan section "From the log inventory (T-111)"), now that session 049 (D-058) parses triples, trinkets and keywords, plus 049's follow-up: golden minions read from `PREMIUM`.

## At the start

1. Conversation title: **`#052 Overlay data`**.
2. `git fetch origin && git merge origin/main`.
3. Read D-004, D-006, D-054, D-058, `docs/research/log-inventory.md` and `parser-round-2.md`, the plan rows T-208 and T-312 to T-315 and notes T-317 and T-319, the overlay code and `crates/tracker/src/live.rs`.

## Rules (D-004, D-006, D-054)

- The overlay shows only what the player could see on their screen at that moment. Leaderboard values as the tooltip shows them; nothing from combat copies the game does not play back; never anything from D-054's hidden Duos combats.
- Check each tag against the raw logs first (counts anyone can re-check); leave out anything unreliable and say so.

## Task

1. **Golden by `PREMIUM`** on board minions (049's follow-up): checked on raw logs; parser revision and validator bumped; earlier fields unchanged in the real fixtures.
2. **T-312** Opponents' triples and trinkets in the hover card.
3. **T-313** Next opponent marked in the overlay with their last board seen (Duos: the next opposing team), from `NEXT_OPPONENT_PLAYER_ID`; check that the game itself shows the next opponent at that moment.
4. **T-314** "In the Battlegrounds lobby" and "game loading" from the scene changes in `LoadingScreen.log`, for the app's status and the overlay's start (the app may need `LoadingScreen.log` enabled in `log.config`: check how the setup handles it, D-012).
5. **T-315** The combat damage cap, **only after checking that the game shows it** to the player; if it does not, leave it out and say why.
6. Keywords on boards in the hover card (T-208's overlay side): short marks with text alternatives, not colour only; fits the card's size limits.
7. Tests (parser, tracker, Node UI), plan rows and notes, D-061, README; same PR.

## Out of scope

The main window recap and stats (051), T-213 (left out, D-058), T-215 combat replay, the website.

## Acceptance criteria

- [ ] Each tag checked on raw logs with counts; unreliable ones left out and said so.
- [ ] Nothing in the overlay the player could not see at that moment (tested where it can be).
- [ ] Upload contract green, revisions bumped, validator bounds tested (Deno).
- [ ] `cargo test --workspace`, the desktop Node UI tests and a release build of `desktop` pass locally (paste the counts).
- [ ] `/code-review` and `/security-review` with no open CRITICAL or HIGH; branch handed to the hub; post-merge validator check reported.

## Red line

D-004, D-006, D-012, D-022, D-054 apply. Never print BattleTags, `GameAccountId`, player names or the user's email.

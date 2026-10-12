# Session 049: Parser round 2 (T-207, T-208, T-211, T-212, then T-213)

Model: opus
Effort: high (log research and the parser), medium for the rest
Subagents: haiku for searches; sonnet for code and reviews

Work in your worktree **`C:\Users\andia\tavern-ledger\.claude\worktrees\049-parser-round-2`** (branch `049-parser-round-2`, created by the hub) and follow its `CLAUDE.md`. Chat in caveman mode; the user writes in Spanish, answer in English. Everything in the repo is in English. **Tier R2** (the saved report gains keys, so the upload changes): TDD + `/code-review` + `/security-review`.

**Other sessions run at the same time:** 048 (the app's display of 044's data and the "Curious stats" tab: `crates/desktop/`, and `crates/tracker/src/stats.rs` and `recap.rs`). To stay out of each other's way:
- Work only in your worktree. Never touch the main checkout or other worktrees.
- Your area: `crates/bg-parser/`, `crates/tracker/` **except `stats.rs` and `recap.rs`** (048 owns them until it merges; the hub tells you when, then `git merge origin/main` and add your recap and stats data there), `tests/` (Python parity and `bg_log_builder.py`), `tools/make_fixture.py` and the real fixtures, and in `supabase/` only `supabase/functions/upload-game/report_keys.json`, the `KEYS` table and checks in `validate.ts`, and their tests (D-047). **Do not touch `crates/desktop/` or `web/`.** Do the parser and validator work first; by the time it is done, 048 has likely merged.
- Shared files (`plan.md`, `README.md`, `DECISIONS.md`, `PROVENANCE.md`): only your tasks' rows and notes. Your decision number is **D-058**. Keep every changed text file LF (`git ls-files --eol` before committing; the checkout uses `core.autocrlf=true`, so check the index side).
- Never stop or restart the user's desktop app; test with your own build, a scratch data folder and the log replay (T-D02). After a build, `git checkout -- crates/desktop/gen/schemas`. No Docker needed; if you need the local Supabase stack, tell the hub first.
- To catch up with `main`: `git fetch origin && git merge origin/main`. No rebase, never force-push.
- **Upload contract (D-047):** every new report key goes into `report_keys.json` and `validate.ts` (bounds, shapes), `upload_contract.rs` stays green, `PARSER_REVISION` and `VALIDATOR_VERSION` are bumped. Merging to `main` deploys the function: after the merge, check that `POST …/functions/v1/upload-game` (publishable key as `apikey` and bearer) answers `405 … "validator": <new>` and tell the hub, who reparses the user's history and restarts the app.
- **gitleaks scans the full history:** a made-up test value that looks random fails CI. Use obviously fake, low-entropy values.

Working rules (user's decisions):
- Decide technical matters yourself. Ask the user only about new dependencies or anything about Blizzard's rules you are unsure of.
- **Ask your questions through the hub** (`tavern-ledger-47`, find it with `ListAgents` and use its `[ref]`) as well as in your tab: the user follows the hub from their phone, not your tab.
- **`gh` is not available in task sessions.** When your branch is ready (tests green, both reviews done, no open CRITICAL or HIGH), push it and message the hub with the branch, the PR title, a short PR body (summary, review results, test plan) and anything still waiting on the user. The hub opens the PR, waits for CI and merges it.
- Fixtures: only the user's own logs through `tools/make_fixture.py`, or synthetic ones. Read raw logs with shared access; print counts, tag names and card ids only, never names, BattleTags or `GameAccountId`.

## Why

The rest of the log inventory's parser items (`docs/research/log-inventory.md`, "Worth adding next"; plan section "From the log inventory (T-111)"). They feed the recap, the stats, later the overlay (T-312) and the web viewer.

## At the start

1. Conversation title: **`#049 Parser round 2`**.
2. `git fetch origin && git merge origin/main`.
3. Read the inventory rows for each tag below, D-019, D-028, D-046, D-047, D-054, D-055, `docs/research/parser-hslog.md` (the `SETASIDE` copies), `crates/bg-parser/src/` and `supabase/functions/upload-game/report_keys.json`.

## Rules (D-004, D-006, D-054, absolute rule 1)

- Only the log. Only what the player saw, or their own choices. Opponents' triples and trinkets only as the game's leaderboard tooltip shows them; the local player's side only for offers.
- **T-213 (D-054):** the Duos combats the game does not play back appear **after the game only**, labelled "not shown in game": never in the overlay, never during the game, never as the live "last board seen". Keep them out of anything `live.rs` exposes; test that.
- Unknown is never zero: an older record says "not recorded" until `--reparse`, a value the log skipped says "no data".
- Clean-room: no code of HDT, Firestone, their simulators, HearthMirror or BobsBuddy; Nomi's Kitchen: nothing.

## Task

Check each tag against the raw lines of every real game first (counts anyone can re-check); leave out anything unreliable and say so.

1. **T-207** Opponents' triples and trinkets (`PLAYER_TRIPLES`, `BACON_*_TRINKET_*`) per opponent and turn, as the leaderboard tooltip shows them.
2. **T-208** Keywords on the boards seen (taunt, divine shield, reborn, venomous, windfury, stealth, deathrattle, golden), in the board snapshots.
3. **T-211** Season choices: own trinkets and trinket offers to the player, quests and rewards, Dark Gifts, the Old God, Timewarped buys, whichever the logs hold.
4. **T-212** Discover picks: what the player was offered and what they took.
5. **T-213** Hidden Duos combats, after the game only (D-054).
6. Recap and stats data in `tracker` (after 048 merges, in `recap.rs` and `stats.rs`), with tests. The window display is a later session.
7. Fixtures: synthetic Solo and Duos games in `bg_log_builder.py` with each new tag; the real fixtures made again with `make_fixture.py` if they must keep new lines (privacy scan again; the earlier fields must not change). Parity tests drop the new fields, as before.
8. Plan rows T-207, T-208, T-211, T-212, T-213 with notes; D-058; README; same PR.

## Out of scope

`crates/desktop/` and its UI, the overlay (T-312 to T-315), T-215 combat replay, T-216, the website.

## Acceptance criteria

- [ ] Each tag's meaning checked against raw lines, with counts; unreliable ones left out and said so.
- [ ] New report fields tested with synthetic and real fixtures; older records load as "not recorded".
- [ ] T-213 data never reaches `live.rs` or the overlay (tested).
- [ ] Upload contract green, `PARSER_REVISION` and `VALIDATOR_VERSION` bumped, validator bounds tested (Deno tests).
- [ ] `/code-review` and `/security-review` with no open CRITICAL or HIGH; PR merged through the hub; post-merge validator check reported.

## Red line

D-004, D-006, D-012, D-022, D-054 apply. Never print BattleTags, `GameAccountId`, player names or the user's email.

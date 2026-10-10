# Session 034: Shop and economy record, actions per minute (T-204, T-205)

Model: opus for the log research, sonnet for the code
Effort: high for the research, medium for the code
Subagents: haiku for searches; sonnet for code and reviews

**Start only after session 033 (overlay hover) has merged**: both change `crates/bg-parser` and `crates/tracker`.

Work in your worktree **`C:\Users\andia\tavern-ledger\.claude\worktrees\034-shop-stats`** (branch `034-shop-stats`, created by the hub) and follow its `CLAUDE.md`. Chat in caveman mode; the user writes in Spanish, answer in English. Everything in the repo is in English. **Tier R1** (local log reading and UI); anything that would upload the new data is out of scope.

To stay tidy:
- Work only in your worktree. Never touch the main checkout or other worktrees.
- Shared files (`plan.md`, `README.md`, `DECISIONS.md`, `PROVENANCE.md`): only your tasks' rows and notes. Your decision number is **D-046**. Keep every changed text file LF (`git ls-files --eol`).
- Never stop or restart the user's desktop app; test with your own build, your own data folder and the log replay (T-D02). The hub restarts the user's app after your merge.
- To catch up with `main`: `git fetch origin && git merge origin/main`. No rebase, never force-push.

Working rules (user's decisions):
- Decide technical matters yourself. Ask the user: new dependencies.
- **`gh` is not logged in inside task sessions.** When your branch is ready (tests green, `/code-review` done, no open CRITICAL or HIGH), push it and send the hub (`Hub prompts/sessions/025`, find it with `ListAgents`) a message with the branch, the PR title, a short PR body (summary, review results, test plan) and anything still waiting on the user. The hub opens the PR, waits for CI and merges it.

## Why (user's first live test, 2026-10-10)

The user wants every game to keep its economy: the minions offered in each shop and on which turn, how many rolls in total, gold spent, when each tavern tier-up happened, and similar. And an APM count like other tools show, without injecting anything. The plan's live-test notes list the tags and blocks the user's own log already holds (counted, nothing printed).

## Rules

- **Log only** (D-004): no input hooks, no memory, no injection. APM counts the player's own actions **as the log records them** (so it is "logged actions per minute"; say so in the UI). If the log turns out to miss something the user cares about (for example moving minions on the board), write it down and ask the user; a global input hook is a separate decision for the user, not part of this task.
- Clean-room (absolute rule 1): the APM idea comes from other tools; **Nomi's Kitchen's code and pages that show its code are never read or given to any AI**; HDT and Firestone code neither.
- Only the player's own shop: never another player's shop or hand (the log does not have them, and if something looks like it, do not show it).
- Unknown is not zero: a game the log cut short says so; a turn with no shop event shows "no data", not 0.

## Effort and models per phase

| Phase | Effort | Model |
|-------|--------|-------|
| Read and check state | low | Haiku |
| Log research on the user's own logs (counts and synthetic examples only) | high | Opus |
| Parser, tracker, stats | medium | Sonnet |
| Recap and stats UI | medium | Sonnet |
| Review | medium | `code-reviewer` |

## At the start

1. Conversation title: **`#034 Shop stats`**.
2. `git fetch origin && git merge origin/main`.
3. Read the plan (F2 rows, the live-test notes, T-107 on parser revisions), `DECISIONS.md` (D-004, D-010, D-015, D-017, D-028, D-043, D-044), `crates/bg-parser/src/lib.rs`, `crates/tracker/src/recap.rs`, `stats.rs`, `tools/make_fixture.py` and the real fixtures under `crates/bg-parser/tests/data/real/`.

## Task

1. **Research** on the user's own logs (read only; print counts, tag names and card ids, never names): how a shop offer appears (minions entering Bob's side for the local player each turn, `IS_BACON_POOL_MINION`, zones), rolls (`TB_BaconShop_8p_Reroll_Button` blocks, `BACON_FREE_REFRESH_COUNT`), buys and sells (`DragBuy`, `DragBuy_Spell`, `DragSell`), freezes (`LockAll`, `FROZEN`), tier-ups (`TechUpNN` blocks, `PLAYER_TECH_LEVEL` on the local hero), gold (`RESOURCES`, `RESOURCES_USED`, `NUM_RESOURCES_SPENT_THIS_GAME`), turns and their times. Write it into `docs/research/` with what is certain and what is guessed. Duos: only the local player's shop.
2. **Parser** (new revision, T-107): per game and per turn, the shop offers (card ids, turn, whether frozen in), rolls (free and paid), buys, sells, freezes, gold available and spent, tier-ups with the turn, and the list of logged player actions with their log time. Tests on synthetic logs (extend `tests/bg_log_builder.py`) and on the real fixtures (expected reports regenerated and checked by hand). Old saved games keep working and say "not recorded" until `--reparse`.
3. **APM**: logged actions per minute of game (from the first shop turn to the end), per turn and per game, with the definition shown in the UI.
4. **Recap**: a "Shop" section: turn by turn (tier, gold, rolls, buys, sells, offers with card pictures from the card data, frozen ones marked), tier-up turns, totals and APM. **Stats**: averages per game (rolls, gold spent, turn of each tier-up, APM), per hero too, apart for Solo and Duos.
5. Plan rows T-204 and T-205 and notes, README, D-046, same PR.

## Out of scope

Uploading the new data, the overlay (033), input hooks, other players' shops.

## Acceptance criteria

- [ ] Research note with each value's source tag or block, checked on the user's logs (counts only).
- [ ] Parser records shop offers, rolls, buys, sells, freezes, gold and tier-ups per turn; new revision; tests on synthetic and real fixtures.
- [ ] APM from logged actions, defined in the UI.
- [ ] Recap and stats show it; unknown is never shown as zero.
- [ ] `/code-review` with no open CRITICAL or HIGH; PR merged through the hub.

## Red line

D-004, D-006, D-012, D-022 apply. Never print BattleTags or `GameAccountId`. No accounts, terms, payments or publishing without the user.

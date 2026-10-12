# Session 044: Parser data round from the log inventory (T-206, T-209, T-210, T-214)

Model: opus
Effort: high (log research and the parser), medium for the rest
Subagents: haiku for searches; sonnet for code and reviews

Work in your worktree **`C:\Users\andia\tavern-ledger\.claude\worktrees\044-parser-data-round`** (branch `044-parser-data-round`, created by the hub) and follow its `CLAUDE.md`. Chat in caveman mode; the user writes in Spanish, answer in English. Everything in the repo is in English. **Tier R2** (the saved report gains keys, so the upload changes): TDD + `/code-review` + `/security-review`.

**Other sessions run at the same time:** 041 (redesign of the app and the overlay: `crates/desktop/ui/`, `crates/desktop/src/` for UI, `.claude/skills/`) and soon 042 (the website: `web/`). To stay out of their way:
- Work only in your worktree. Never touch the main checkout or other worktrees.
- Your area: `crates/bg-parser/`, `crates/tracker/`, `tests/` (Python parity and `bg_log_builder.py`), `tools/make_fixture.py` and the real fixtures, and in `supabase/` only `supabase/functions/upload-game/report_keys.json`, the `KEYS` table and checks in `validate.ts`, and their tests (D-047). **Do not touch `crates/desktop/` or `web/`** while 041 and 042 run. The new values reach the recap and stats as data from `tracker` (its recap and stats structs, with tests); how they look in the window comes after 041 merges. If 041 merges while you run, merge `origin/main` and you may then add the display in `crates/desktop/ui/` in the new look.
- Shared files (`plan.md`, `README.md`, `DECISIONS.md`, `PROVENANCE.md`): only your tasks' rows and notes. Your decision number is **D-055**. Keep every changed text file LF (`git ls-files --eol` before committing; the checkout uses `core.autocrlf=true`, so check the index side).
- Never stop or restart the user's desktop app; test with your own build, a scratch data folder and the log replay (T-D02). After a build, `git checkout -- crates/desktop/gen/schemas`. No Docker needed; if you need the local Supabase stack, tell the hub first.
- To catch up with `main`: `git fetch origin && git merge origin/main`. No rebase, never force-push.
- **Upload contract (D-047):** every new report key goes into `report_keys.json` and `validate.ts` (bounds, shapes), `upload_contract.rs` stays green, `PARSER_REVISION` and `VALIDATOR_VERSION` are bumped. Merging to `main` deploys the function: after the merge, check that `POST …/functions/v1/upload-game` (publishable key as `apikey` and bearer) answers `405 … "validator": <new>` and tell the hub, who reparses the user's history (`tavern-watch --reparse`) and restarts the app.

Working rules (user's decisions):
- Decide technical matters yourself. Ask the user: new dependencies, anything about Blizzard's rules you are unsure of.
- **`gh` is not logged in inside task sessions.** When your branch is ready (tests green, both reviews done, no open CRITICAL or HIGH), push it and send the hub (`Hub prompts session 036`, find it with `ListAgents`, use its `[ref]`) a message with the branch, the PR title, a short PR body (summary, review results, test plan) and anything still waiting on the user. The hub opens the PR, waits for CI and merges it.
- Fixtures: only the user's own logs through `tools/make_fixture.py`, or synthetic ones. Read raw logs with shared access; print counts, tag names and card ids only, never names, BattleTags or `GameAccountId`.

## Why

The user wants every item of the log inventory (`docs/research/log-inventory.md`, "Worth adding next"; plan section "From the log inventory (T-111)"), each in its time, and as many in parallel as can safely run. This round takes the four parser and data items that need no new UI first; they also feed the "Curious stats" tab later (T-216).

## Effort and models per phase

| Phase | Effort | Model |
|-------|--------|-------|
| Read state, D-019, D-028, D-046, D-047, the inventory | low | Haiku |
| Check each tag against the raw logs | high | Opus |
| Parser, tracker, validator, fixtures | medium | Sonnet |
| Docs, reviews | medium | `code-reviewer`, `security-reviewer` |

## At the start

1. Conversation title: **`#044 Parser data round`**.
2. `git fetch origin && git merge origin/main`.
3. Read the inventory rows for each tag below, D-019, D-028, D-044, D-046, D-047, D-054, `crates/bg-parser/src/` (collector, shop, state), `crates/tracker/src/recap.rs` and `stats.rs`, and `supabase/functions/upload-game/report_keys.json`.

## Rules (D-004, D-006, absolute rule 1)

- Only the log. Only what the player saw, or their own choices. The local player's side only for offers (never other players' trinket or hero offers).
- Unknown is never zero: an older record says "not recorded" until `--reparse`, a value the log skipped says "no data".
- Clean-room: no code of HDT, Firestone, their simulators, HearthMirror or BobsBuddy; Nomi's Kitchen: nothing.

## Task

1. **T-206 Combat result as the game records it.** Check `BACON_WON_LAST_COMBAT` and `DAMAGE_DEALT_TO_HERO_LAST_TURN` (and anything else the inventory names) against the raw lines of every real game: what they mean for wins, losses and ties, in Solo and Duos. Where they are reliable, the result comes from them (source `log`) and the health rule (D-028) stays as the fallback (source `inferred`); fewer "unknown" results. Record the rule in D-055.
2. **T-209 Hero select record.** Heroes offered (card ids), rerolls used (`BACON_NUM_MULLIGAN_REFRESH_USED`, …), the hero picked; in the stats: per hero, times offered, times picked, pick rate, average place when picked. Only the local player's offers.
3. **T-210 Shop record extras.** Extra gold (`TEMP_RESOURCES`), why a roll was free (`BACON_FREE_REFRESH_COUNT` and the like), buy and sell prices where the log gives them (`BACON_OVERRIDE_BG_COST`, `BACON_SELL_VALUE`), the turn timer (`TIMEOUT`) if it means that, Duos passes (`IS_USING_PASS_OPTION`, `DECK_ACTION`). Totals per game and in the stats, so T-216 can show "gold beyond the fixed income".
4. **T-214 Skins grouped by the game's own link.** `BACON_SKIN` and `BACON_SKIN_PARENT_ID` (a database id) to the base hero, through the card data already cached (D-038; the database-id to card-id map). The D-019 name rule stays only as the fallback when the link is missing, labelled; say so in D-055.
5. Fixtures: a synthetic Duos and Solo game in `bg_log_builder.py` with each new tag; the real fixtures made again with `make_fixture.py` if they must keep new lines (privacy scan again; the earlier fields must not change). Parity: the Python prototype need not read the new fields (drop them in the parity tests, as before).
6. Plan rows T-206, T-209, T-210, T-214 with notes; D-055; README; same PR.

## Out of scope

UI changes while 041 runs, the overlay (T-312 to T-315), T-207, T-208, T-211, T-212, T-213, T-215, T-216 (later sessions), the website.

## Acceptance criteria

- [ ] Each tag's meaning checked against the raw lines, with counts anyone can re-check; anything unreliable left out and said so.
- [ ] New report fields tested with synthetic and real fixtures; older records load as "not recorded".
- [ ] Upload contract green, `PARSER_REVISION` and `VALIDATOR_VERSION` bumped, the validator bounds tested (Deno tests).
- [ ] Recap and stats data in `tracker` for the four tasks, tested.
- [ ] `/code-review` and `/security-review` with no open CRITICAL or HIGH; PR merged through the hub; post-merge validator check reported.

## Red line

D-004, D-006, D-012, D-022 apply. Never print BattleTags, `GameAccountId`, player names or the user's email.

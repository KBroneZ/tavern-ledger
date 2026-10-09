# Session 005: Solo parser (T-002)

Model: sonnet
Effort: medium
Subagents: haiku

Work in `C:\Users\andia\tavern-ledger` and follow its `CLAUDE.md`: caveman in chat; documents in Spanish; public README and product in English. **Tier R1** (normal code): TDD + `/code-review`. Branch and PR; never push directly to `main`. The user wants you to move on your own: decide what you can, ask only what is theirs (new dependencies, committing real logs, merge).

**Red line (D-004, D-006):** local logs only. No memory reading, injection or game automation. Never print BattleTags or `GameAccountId` in chat, docs, tests or tool output.

## Effort and models per phase

| Phase | Effort | Subagents (if needed) |
|-------|--------|-----------------------|
| Reading and diagnosis | medium | Haiku (searches in large logs) |
| Tests and fix | medium | Sonnet if code needs to be delegated |
| Review and docs | medium | — |

## At the start

1. Title **`#005 Solo parser`**.
2. Check that `main` is up to date and that there are no open PRs from session 004 left unmerged (the measurement 10 one may stay open; do not touch it).
3. Read: `README.md`, `docs/plan/plan.md` (T-002, session 003 notes), `docs/research/parser-hslog.md`, `tools/parse_bg.py` and `tests/test_parse_bg.py` + `tests/bg_log_builder.py`.
4. Run `python tools/check_logs.py` and note in the plan if there are new games (especially Solo, `GT_BATTLEGROUNDS`).
5. This prompt file goes into this session's PR if it is not yet in `main`.

## Context

The first Solo game (session `Hearthstone_2026_10_09_00_26_56`, build 253216) comes out `ok` in `tools/parse_bg.py` but with two failures (plan, T-002 notes of 2026-10-09):

1. **Final health 30** although the player finished 7th (eliminated), and health goes from 12 to 30 in the last round. Suspicion: when dying in Solo the leaderboard hero changes or resets (ghost, new `HERO_ENTITY`, `DAMAGE` to 0…) and `hero_health()` / `rounds_of()` read the wrong entity at the end. It does not happen in Duos because the teammate is still alive.
2. **Another lobby player also with placement 7.** Suspicion: `PLAYER_LEADERBOARD_PLACE` is read at the end of the game, when it is no longer the final placement of the eliminated players, or it is read from a ghost hero.

## Task

1. **Diagnosis** with the real log (read-only, without printing names): which tags change on the local hero and on the eliminated ones' heroes when dying in Solo (`HEALTH`, `DAMAGE`, `HERO_ENTITY`, `PLAYER_LEADERBOARD_PLACE`, zone…). Use `hslog` or bounded searches; the log is 22 MB. Note in `docs/research/parser-hslog.md` what you find, with approximate line number and without names.
2. **Tests first** with synthetic logs (`tests/bg_log_builder.py`): a Solo game where the local player dies in round N with the real tag sequence, and another with two eliminated players in different rounds. They must fail with the current code.
3. **Minimal fix** in `tools/parse_bg.py`: final health = the one at the moment of elimination (or 0 if the game marks it that way; decide with evidence), placements without repeats. If a datum cannot be known with certainty, it comes out as unknown, never an invented value.
4. Check that the 22 Duos games are still `ok` with the same results (compare `--json` output before and after, without saving it in the repo).
5. `GT_BATTLEGROUNDS` moves into `TESTED_GAME_TYPES` only if the real game comes out right.

## Out of scope

- Trimmed fixtures from real logs (they need the user's OK before being committed).
- Choosing the stack (P-002), app, web or overlay.
- The leaderboard client (T-006, done).

## Acceptance criteria

- [ ] Cause of the two failures documented with log evidence (no names).
- [ ] New synthetic tests that fail before and pass after; whole suite green.
- [ ] The real Solo game gives coherent final health and placement; the 22 Duos, unchanged.
- [ ] `/code-review` with no open CRITICAL or HIGH findings.
- [ ] Plan, `parser-hslog.md` and README up to date in the same PR; CI green.

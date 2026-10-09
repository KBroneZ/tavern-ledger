# Session 003: MMR sources

Model: sonnet
Effort: medium
Subagents: haiku

Work in `C:\Users\andia\tavern-ledger` and follow its `CLAUDE.md`: caveman in the chat; documents in Spanish; public README and product in English. **Tier R2 if it touches the network** (queries to the public leaderboard: `/code-review` + `/security-review`); R0 if it is only research and docs. Branch and PR; never push directly to `main`.

**Red line (D-004, D-006):** local log files only. No memory reading, injection, BepInEx, DLLs, modifying game files or automating actions. If an MMR source would need any of that, it is discarded and noted.

## Effort and models per phase

| Phase | Effort | Subagents (if needed) |
|-------|--------|-----------------------|
| Read and plan | low | Haiku (searches) |
| Local logs: inventory of files and `log.config` sections | medium | Haiku to sweep files |
| Public leaderboard: endpoint, data, limits | medium | Haiku to read docs; ChatGPT (`ask-chatgpt`) as a second opinion if in doubt |
| Report and docs | low | — |

**Autonomy:** Claude decides how to search and the structure of the report. **The user approves** any change to `log.config` (even if it only enables log sections), any new dependency and the PR merge.

## At the start

1. Title **`#003 MMR sources`**.
2. Check that PR KBroneZ/tavern-ledger#2 is merged and `main` is up to date. If not, warn.
3. Read: `README.md`, `docs/plan/plan.md` (T-002, T-004, T-006), `docs/decisions/DECISIONS.md` (D-005, P-007), `docs/research/feasibility.md` and `docs/research/parser-hslog.md`.
4. Run `python tools/check_logs.py` (there may be new Solo games: the user was going to play some).
5. This prompt file goes into this session's PR.

## Context

- `Power.log` does **not** expose MMR (T-003: search for `MMR`/`Rating` with no results). See `docs/research/parser-hslog.md`.
- D-005: MMR comes from Blizzard's public leaderboard; outside the leaderboard, "no data", never an invented value. The leaderboard only covers the top (~8000 MMR).
- Logs in `C:\Battle.net\Battle.net\Hearthstone\Logs\Hearthstone_<date>\`; `log.config` in `%LOCALAPPDATA%\Blizzard\Hearthstone\log.config`.
- Logs may contain BattleTags and account ids (`GameAccountId`): **never** dump them to the chat or to docs. When using `grep`, print only line numbers, tag names or counts.
- Rule 4 of `CLAUDE.md`: the leaderboard is queried with cache, few requests, timeout and an identifiable User-Agent; only public endpoints that the web itself uses.

## Task

1. **Local logs:** inventory of the files in each session folder and of the `log.config` sections. Look for rating/MMR and its change (before/after a game) in Solo and Duos. If some section that is not enabled could expose it, propose to the user to enable it (with the exact text of the change and how to undo it) and touch nothing without their OK.
2. **Public leaderboard:** identify the endpoint the official web uses, its parameters (region, Solo/Duos mode, season, page), the response format, what identifies a player (visible name, no full BattleTag), how many rows it covers and which limits or terms apply. At most a few manual requests with an identifiable User-Agent; save a sample response only if it carries no player data beyond what is public, and record it in `PROVENANCE.md`.
3. **Matching:** how to match the local user with their leaderboard row (name, region, mode) without inventing, and what happens if names are repeated or the player does not appear.
4. **Report:** `docs/research/mmr-sources.md` with a table of data → source → how → evidence → limitations, and a proposal to close P-007 (options: enter it by hand, infer deltas, do not show it) with a recommendation.
5. Update the plan (T-004 done or in progress; T-002 if there are Solo games), `DECISIONS.md` (close P-007 only if the user decides) and the README, in the same PR.

## Out of scope

- Leaderboard client with cache and tests (T-006).
- Changes to `tools/parse_bg.py` unless an MMR value shows up in a log it already reads (then, with a test).
- Choosing the stack (P-002), app, web or overlay.
- Any spending, new account or acceptance of terms.

## Acceptance criteria

- [ ] Inventory of local logs with evidence (file, section, approximate line), without player names.
- [ ] Leaderboard endpoint documented with a sample request and known limits.
- [ ] Report `docs/research/mmr-sources.md` with a table and a recommendation for P-007.
- [ ] No change to `log.config` without the user's explicit OK.
- [ ] Plan, decisions and README up to date in the same PR; CI green.

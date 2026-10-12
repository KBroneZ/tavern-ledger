# Session 002: hslog parser prototype

Model: sonnet
Effort: medium
Subagents: haiku

Work in `C:\Users\andia\tavern-ledger` and follow its `CLAUDE.md`: caveman in the chat; documents in Spanish; public README and product in English. **Tier R2** (the first external dependency comes in: `/code-review` + `/security-review`). Branch and PR; never push directly to `main`.

**Red line (D-004, D-006):** local log files only. No injection, BepInEx, DLLs, memory reading, modifying game files or automating actions. If a task seems to need any of that, stop and warn.

## Effort and models per phase

| Phase | Effort | Subagents (if needed) |
|-------|--------|-----------------------|
| Read and plan | low | Haiku (searches) |
| Research on `hslog` and `hearthstone` (version, license, BG and Duos support) | medium | Haiku to read docs and HearthSim's MIT code |
| Prototype implementation | medium | Sonnet |
| Review | medium | — (`/code-review` + `/security-review`) |
| Report and docs | low | — |

**Autonomy:** Claude decides the structure of the prototype and the report. **The user approves** adding dependencies (with pinned version and a row in `PROVENANCE.md`) and the PR merge.

## At the start

1. Title **`#002 hslog parser prototype`**.
2. Check that PR KBroneZ/tavern-ledger#1 is merged and `main` is up to date. If not, warn.
3. Read: `README.md`, `docs/plan/plan.md` (T-002 and T-003, with their notes), `docs/decisions/DECISIONS.md`, `docs/research/feasibility.md` and `PROVENANCE.md`.
4. Run `python tools/check_logs.py` to see which logs exist (there may be new Solo games).

## Context

- Logs in `C:\Battle.net\Battle.net\Hearthstone\Logs\Hearthstone_<date>\Power_old.log` (120–563 MB per session). As of 2026-10-08: 23 games, all `GT_BATTLEGROUNDS_DUO`, build 253216; none of Solo.
- Logs are **not copied to the repo** in this session. The prototype reads them from their local path and its sample output must not include other players' names (BattleTags) unless they are public.
- `python-hslog` and `python-hearthstone` (HearthSim) are MIT. HDT, Firestone, their simulators, HearthMirror, BobsBuddy and Nomi's Kitchen are **not read or ported** (see `CLAUDE.md`).

## Task

1. **Dependencies:** check PyPI and GitHub for the latest version of `hslog` and `hearthstone`, their license and whether they keep up with patches (date of the last release, BG/Duos enums). Propose to the user adding them with a pinned version (`requirements.txt` or `pyproject.toml`) and record them in `PROVENANCE.md`. If `hslog` does not support the current build, say so with evidence and propose an alternative before continuing.
2. **Prototype (T-003):** script in `tools/` that, for each game in a `Power*.log`, tries to extract: own hero (and the partner's in Duos), lobby tribes, own board and each opponent's board per round, opponents faced, health and final placement. Whatever does not come out of the log is marked "not available", never invented. If the parser fails on a new build, the script says "unsupported version" instead of giving false data.
3. **Tests:** with a synthetic or trimmed fragment of an own log, without third-party BattleTags. If a real log is trimmed into a fixture, record it in `PROVENANCE.md` (origin: the user's own logs) and ask for OK before committing it.
4. **Report:** `docs/research/parser-hslog.md` with a table of data → does it come from the log? → how (tag, entity, event) → evidence (game and approximate line), plus what is missing and what that implies for the product (overlay, replays, MMR).
5. **CI:** if dependencies are added, install the pinned versions in the existing job without creating new jobs (watch the Actions minutes).
6. Update the plan (T-003 done or in progress; T-002 depending on whether there are fixtures), `DECISIONS.md` if there are new decisions, and the README, in the same PR.

## Out of scope

- MMR and leaderboard (T-004, T-006).
- Choosing the stack (P-002), although the report can provide data to decide it.
- Desktop app, web, overlay.
- Any spending, domain or code signing.

## Acceptance criteria

- [ ] Dependencies with pinned version, verified license and a row in `PROVENANCE.md`, approved by the user.
- [ ] Prototype run against at least 3 real games, with its output (without third-party BattleTags) as evidence.
- [ ] Report `docs/research/parser-hslog.md` with what comes out, what does not, and the evidence.
- [ ] Tests green locally and in CI (link to the run).
- [ ] Plan, decisions and README up to date in the same PR.

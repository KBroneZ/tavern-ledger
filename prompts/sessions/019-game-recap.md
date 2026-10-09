# Session 019: Recap after each game, record against each opponent (T-202, T-203)

Model: sonnet
Effort: medium
Subagents: haiku for searches; sonnet for code and reviews

Work in your worktree **`C:\Users\andia\tavern-ledger\.claude\worktrees\019-game-recap`** (branch `019-game-recap`, created by the hub) and follow its `CLAUDE.md`. Chat in caveman mode; the user writes in Spanish, answer in English. Everything in the repo is in English. **Tier R1** (local code, no network): TDD + `/code-review`. A new dependency makes it R2 (ask the user first).

**Another session runs at the same time** (#016 desktop upload). To stay out of its way:
- Work only in your worktree. Never touch the main checkout or other worktrees.
- Your area: `crates/desktop/ui/` (`app.js`, `index.html`, `style.css`), `tracker::stats` or a new `tracker` module for the recap, `bg-parser` only if a value the recap needs is not in the report yet. In `crates/desktop/src/main.rs` keep changes small (one or two commands): #016 also edits it. Do not touch `supabase/`, `web/`, upload code or the upload settings panel.
- Shared files (`plan.md`, `README.md`, `DECISIONS.md`): only your tasks' rows and notes. Your decision number, if needed, is **D-028**.
- To catch up with `main`: `git fetch origin && git merge origin/main`. No rebase, never force-push.
- Never start Docker or the local Supabase stack. **You are the only session that may stop and restart the desktop app** the user has running: leave a working app running at the end, built from your merged `main` in the main checkout (`cargo build --release -p desktop` there, stop the old one, `Start-Process` the new one). That is the only thing you may do in the main checkout, after your PR is merged.

Working rules (user's decisions, 2026-10-09):
- Decide technical matters yourself. Ask only what is the user's call: new dependencies, data with unclear licences, committing real logs.
- **You merge your own PRs, always**, once CI is green and no review has open CRITICAL or HIGH findings: `gh pr merge <n> --squash --delete-branch`.
- The user often works remotely without access to the PC: never ask them to do something on it.
- `gh` is already logged in through `GH_TOKEN` in the environment; never print it.
- Anything long-running goes in a separate window (`Start-Process`).
- Python: use `C:\Users\andia\tavern-ledger\.venv\Scripts\python`.

**Red line (D-004, D-006, D-012):** local log files only. Opponents are identified **only by hero and by their place in this game**, never by player name, BattleTag or `GameAccountId`, and never across games (no per-person history of other players). Show only what the player could see on their own screen during the game.

## Effort and models per phase

| Phase | Effort | Subagents (if needed) |
|-------|--------|-----------------------|
| Read and check state | low | Haiku |
| Design (recap contents, how a combat result is read) | medium | — |
| Code (tests first) | medium | Sonnet |
| Review | medium | `code-reviewer` |

## At the start

1. Conversation title: **`#019 Game recap`**.
2. `git fetch origin && git merge origin/main`.
3. Read `README.md`, `docs/plan/plan.md` (T-101, T-102, T-109, T-202, T-203), `docs/decisions/DECISIONS.md` (D-005, D-011, D-012, D-017, D-019, D-024, D-026 if present), `docs/research/implementation-notes.md` and `crates/` (`bg-parser` `report.rs`: `Round`, `CombatEntry`, `LobbyPlayer`; `tracker`; `desktop`).
4. Green before touching anything: Rust (workspace and `desktop`), clippy, fmt, Python.

## Task

1. **T-203 record against each opponent, within one game.** From `rounds`: for each opponent (by player id inside the game, shown by hero), the combats fought and the result of each: *won*, *lost*, *tie* or *unknown*. Read the result from what the log gives (damage taken, `own_health_after`, the opponent's health if the log has it); where it is inferred, label it so with the T-109 source labels; when the log can't tell, say *unknown*, never guess. Duos: say clearly how combats are counted for the team. Tests with synthetic reports, including a game with missing rounds.
2. **T-202 recap after each game.** When a game ends (and from any row in the history): final place, hero (and teammate's hero in Duos), rounds played, health over the rounds, the record from (1), the tribes seen in the tavern (inferred, labelled), and the parser's warnings if any. Shown in the app window; every value with its source label (T-109). `textContent` only.
3. Plan (T-202, T-203), README and D-028 if needed, in the same PR.
4. After merging: move the running desktop app to the new `main` build (see above) and check the recap with the real history.

## Out of scope

- Upload (#016), the web viewer (T-201), the overlay (T-301, T-302), entering tribes by hand (T-303), any history of other players across games.

## Acceptance criteria

- [ ] Tests before code; green: Rust (workspace and `desktop`), clippy, fmt, Python.
- [ ] Record per opponent within a game, by hero only; unknown results shown as unknown; inferred ones labelled.
- [ ] Recap shown at game end and from the history; every value has a source label.
- [ ] No names, BattleTags or `GameAccountId` anywhere (code, tests, docs, output).
- [ ] App tried with the real history; app left running from `main`.
- [ ] `/code-review` with no open CRITICAL or HIGH; CI green; PR merged by you.
- [ ] Plan, README (and D-028 if needed) updated in the same PR.

# Session 014: Desktop robustness: log setup check, parser version per game, log replay (T-106, T-107, T-D02)

Model: sonnet
Effort: medium
Subagents: haiku for searches; sonnet for code and reviews

Work in **`C:\Users\andia\tl-014`** (a git worktree on branch `014-desktop-robustness`, already created) and follow its `CLAUDE.md`. Chat in caveman mode; the user writes in Spanish, answer in English. Everything in the repo is in English. **Tier R1** (local code, no network): TDD + `/code-review`. A new dependency makes it R2 (ask the user first, `/security-review`, `PROVENANCE.md` row).

**Another session runs at the same time** (#013, website, worktree `C:\Users\andia\tl-013`). To stay out of each other's way:
- Work only in `C:\Users\andia\tl-014`. Never edit files in `C:\Users\andia\tavern-ledger` or `tl-013`.
- Your area: `crates/`, `tools/check_logs.py`, `tools/dev/` (new), `tests/`. Do not touch `web/`, `supabase/` or `package.json`: they belong to #013. Never start the local Supabase stack or Docker.
- Shared files (`docs/plan/plan.md`, `README.md`, `DECISIONS.md`): only your tasks' rows and notes. If you need a decision, it is **D-024** (D-023 is reserved for #013).
- To catch up with `main`: `git fetch origin && git merge origin/main`. No rebase, never force-push.
- **The desktop app running now** comes from `C:\Users\andia\tavern-ledger\target\release\desktop.exe` and is waiting for the T-101 live test. You may stop it to try your build (only one copy can write the history), but leave a working app running at the end: yours if merged, or relaunch that one.

Working rules (user's decisions, 2026-10-09):
- Decide technical matters yourself. Ask only what is the user's call: new dependencies, data with unclear licences, committing real logs.
- **You merge your own PRs, always**, once CI is green and no review has open CRITICAL or HIGH findings: `gh pr merge <n> --squash --delete-branch`.
- The user often works remotely without access to the PC: never ask them to do something on it.
- `gh` has no login of its own: export `GH_TOKEN` from the credential Git keeps (`git credential fill`) and never print it.
- Anything long-running (the app, a replay) goes in a separate window (`Start-Process`), not in the conversation.
- Python: use `C:\Users\andia\tavern-ledger\.venv\Scripts\python` (the worktree has no `.venv`).

**Red line (D-004, D-006):** local log files only. Never edit the game's files for the user (`log.config`, `client.config`): show what to change. Never print BattleTags or `GameAccountId` in chat, docs, tests, history or tool output.

## Effort and models per phase

| Phase | Effort | Subagents (if needed) |
|-------|--------|-----------------------|
| Read and check state | low | Haiku |
| Design (record format, replay) | medium | — |
| Code (tests first) | medium | Sonnet |
| Review | medium | `code-reviewer` |

## At the start

1. Conversation title: **`#014 Desktop robustness`**.
2. `cd C:\Users\andia\tl-014 && git fetch origin && git merge origin/main`.
3. Read `README.md`, `docs/plan/plan.md` (T-101, T-106, T-107, T-D02, "Future ideas"), `docs/decisions/DECISIONS.md` (D-004, D-010, D-015 to D-019, D-022), `docs/research/implementation-notes.md` and `crates/` (`tracker`, `desktop`, `bg-parser`).
4. Green before touching anything: `cargo test --workspace --exclude desktop`, `cargo test -p desktop`, clippy `-D warnings`, `cargo fmt --check` and the Python tests.
5. T-101: count games per session in `%APPDATA%\TavernLedger\games.jsonl` without names (23 games from 5 sessions so far). If a new game appeared on its own while the app was open, mark T-101 done.

## Task

1. **T-106 log setup check.** `check_logs.py` and the app (status line, plus a short "how to fix" text) report: `log.config` `[Power]` settings (already in `check_logs.py`) and `client.config` `[Log] FileSizeLimit.Int=-1` next to `Hearthstone.exe` (without it the game stops writing a log at about 10 MB). Missing file, missing setting and unreadable file are three different states. Read-only: never write either file. The Rust check lives in `tracker::discover` (or a new module) with unit tests on synthetic files.
2. **T-107 parser version and build per game.** Each saved record gets the parser version (crate version plus a `PARSER_REVISION` constant bumped when parsing changes) and the game build (already in the report). Old records without it load as "unknown version", never as an error. Add `tavern-watch --reparse`: re-read the sessions whose logs are still on disk, save only games whose report changed (last record wins, D-015), and list the games that could not be re-read because their logs are gone. Tests first. Note for T-104d: the server schema has a `build` column but no parser version; write that down in the plan, do not change `supabase/`.
3. **T-D02 log replay** (dev tool, never shipped): `tools/dev/` script or a `tracker` example that copies a saved `Power.log` into a temporary `Logs\Hearthstone_<date>\` folder line by line, at a chosen speed, so `tavern-watch --logs-dir <tmp> --data-dir <tmp>` and the app can be tested without playing. Output never prints names. Use it once on one real log (locally, nothing committed) and on a synthetic one in a test.
4. Plan (T-106, T-107, T-D02), README and, if needed, D-024, in the same PR.

## Out of scope

- T-D01 (reconnect dev tool): needs the user's OK first.
- Uploading, the website, `supabase/` (#013 and T-104d).
- Writing game config files.

## Acceptance criteria

- [ ] Tests before code; green: Rust (workspace and `desktop`), clippy, fmt, Python.
- [ ] Log setup check: three states per setting, tested; the app shows it.
- [ ] Old history loads; `--reparse` tested on synthetic sessions (changed, unchanged, logs gone).
- [ ] Replay tool tried on one real log; the tracker saved the same games as the original (counts per session, no names).
- [ ] `/code-review` with no open CRITICAL or HIGH; CI green; PR merged by you; an app left running.
- [ ] Plan, README (and D-024 if needed) updated in the same PR.
- [ ] Ask the user whether they want the next prompt.

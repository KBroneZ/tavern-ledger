# Session 031: Reconnect/unplug dev tool (T-D01)

Model: opus
Effort: high
Subagents: haiku for searches; sonnet for code and reviews

Work in your worktree **`C:\Users\andia\tavern-ledger\.claude\worktrees\031-dev-reconnect`** (branch `031-dev-reconnect`, created by the hub) and follow its `CLAUDE.md`. Chat in caveman mode; the user writes in Spanish, answer in English. Everything in the repo is in English. **Tier R2** (it changes how the game's connection behaves on the user's PC, needs admin rights, carries account risk): TDD + `/code-review` + `/security-review`.

**Other sessions may run at the same time** (029 email sender, 032 multi-build fixtures). To stay out of their way:
- Work only in your worktree. Never touch the main checkout or other worktrees.
- Your area: `tools/dev/` (new tool), and in `crates/tracker` and `crates/desktop` only the code that marks and excludes dev-tool games. Do not touch `crates/bg-parser/tests/data/`, `tests/bg_log_builder.py`, `tools/gen_parser_fixtures.py` (032), `supabase/`, `web/`, `docs/legal/`.
- Shared files (`plan.md`, `README.md`, `DECISIONS.md`, `PROVENANCE.md`): only your task's row and notes. Your decision number is **D-043**. Keep every changed text file LF (`git ls-files --eol`): a CRLF file shows as a whole-file rewrite and conflicts with the other sessions.
- Never stop or restart the user's desktop app (it runs from the main checkout's `target\release`); test with your own build and your own data folder. The hub restarts the user's app after your merge.
- To catch up with `main`: `git fetch origin && git merge origin/main`. No rebase, never force-push.

Working rules (user's decisions):
- Decide technical matters yourself. Ask the user: accounts, accepting terms, paying, new dependencies, anything that runs with admin rights on their PC for the first time.
- **`gh` is not logged in inside task sessions.** When your branch is ready (tests green, both reviews done, no open CRITICAL or HIGH), push it and send the hub (`Hub prompts/sessions/025`, find it with `ListAgents`) a message with the branch, the PR title, a short PR body (summary, review results, test plan) and anything still waiting on the user. The hub opens the PR, waits for CI and merges it.
- The user is at home these days; they can try the tool in a real game if you ask, but never press them to.

## Why (D-022, D-035)

When collecting test games, skipping combat animations by dropping the game's connection (the client reconnects and jumps ahead) saves a lot of time. D-022 allows it **only** as a dev tool: under `tools/dev/`, never in the app, the installer, releases or user docs; run by hand, on the user's own account, with admin rights, knowing it can carry account risk; and games played with it are marked so they never become fixtures or count as normal games. The user said yes on 2026-10-10 (D-035).

## Effort and models per phase

| Phase | Effort | Model |
|-------|--------|-------|
| Read and check state | low | Haiku |
| Method research (Microsoft docs only) | high | Opus, Haiku for search |
| Tool, marking, tests | medium | Sonnet |
| Review | medium | `code-reviewer`, `security-reviewer` |

## At the start

1. Conversation title: **`#031 Dev reconnect tool`**.
2. `git fetch origin && git merge origin/main`.
3. Read `DECISIONS.md` (D-004, D-006, D-022, D-035), `docs/research/github-landscape.md` (the reconnect part), `tools/dev/replay_log.py` (style of an existing dev tool), and how the tracker writes the history (`crates/tracker`, D-015, D-016).

## Clean-room (absolute rule 1)

Other trackers ship this feature. **Do not read or port their code** (HDT, Firestone, Nomi's Kitchen, anything without a compatible licence). Design from Microsoft's own documentation only (for example `SetTcpEntry` with `MIB_TCP_STATE_DELETE_TCB` in `iphlpapi`, or a short Windows Firewall block rule), and record the source pages in `docs/research/`.

## Task

1. **Method.** Pick the simplest way to drop only Hearthstone's game-server TCP connection for a moment, from Microsoft's docs: affect only `Hearthstone.exe`'s connections (by process id), never other programs; restore at once (no rule or state left behind, also after a crash or Ctrl+C); no packet capture, no memory reading, no injection, no change to game files (D-004, D-006). Standard library and `ctypes` only (no new dependency without the user's OK).
2. **Tool** `tools/dev/reconnect.py`: refuses to run without admin rights (clear message), shows a banner that it is a dev tool with account risk, runs only while started by hand, a global hotkey (`RegisterHotKey` via `ctypes`; pick a key the game does not use and make it configurable) drops the connection once per press, with a cooldown. Every use is appended to `%APPDATA%\TavernLedger\dev-reconnects.jsonl` (UTC time, nothing else: no names, no ids).
3. **Marking.** The tracker reads that file and marks any game whose time span holds a reconnect as `dev_reconnect`; the history shows a clear mark on those games; stats, hero stats, MMR follow-up and uploads leave them out (say which in the PR); `tools/gen_parser_fixtures.py`-style fixture tools refuse a log with a marked game. Missing or unreadable file = no marks, with a logged warning (never a crash). Tests for the marking with synthetic data.
4. **Docs.** `tools/dev/README.md` (what it does, the risk, admin, how to undo), `docs/research/` notes with the Microsoft pages, plan T-D01 row and notes, README (only under dev tools, never in user docs), D-043.
5. **Try it** only if the user wants to (a real game on their account, their call): otherwise test the drop on a harmless local TCP connection you open yourself.

## Out of scope

Anything that ships to users, automatic use (no timers, no triggers from the log), other games or programs, network capture.

## Acceptance criteria

- [ ] Method from Microsoft docs only, recorded; no third-party tracker code read.
- [ ] Tool needs admin, affects only Hearthstone's connections, leaves nothing behind; tested on a local connection.
- [ ] Marked games are flagged in the history and left out of stats, uploads and fixtures; tests green.
- [ ] Nothing of the tool in the app, installer or user docs.
- [ ] `/code-review` and `/security-review` with no open CRITICAL or HIGH; PR merged through the hub.

## Red line

D-004, D-006, D-012, D-022 apply. Never print BattleTags or `GameAccountId`. No accounts, terms, payments or publishing without the user.

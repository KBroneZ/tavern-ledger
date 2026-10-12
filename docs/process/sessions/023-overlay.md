# Session 023: Minimal overlay (T-301)

Model: sonnet
Effort: high for the overlay window and live updates; medium for the rest
Subagents: haiku for searches; sonnet for code and reviews

Work in your worktree **`C:\Users\andia\tavern-ledger\.claude\worktrees\023-overlay`** (branch `023-overlay`, created by the hub) and follow its `CLAUDE.md`. Chat in caveman mode; the user writes in Spanish, answer in English. Everything in the repo is in English. **Tier R1** (local code, no network): TDD + `/code-review`. A new dependency or Tauri plugin makes it R2 (ask the user first).

**Another session runs at the same time** (#022 hosted backend). To stay out of its way:
- Work only in your worktree. Never touch the main checkout or other worktrees.
- Your area: `crates/desktop` (a new overlay window, its UI under `crates/desktop/ui/` or a new `ui-overlay/`, the commands and events it needs), `tracker` (a live-game state the overlay reads), `bg-parser` only if a live value is missing. Do not touch `supabase/`, `web/`, the upload code or the server config default (#022 may add one in `crates/`: if you both touch the same file, keep your change small and merge `main` first).
- Shared files (`plan.md`, `README.md`, `DECISIONS.md`): only your task's row and notes. Your decision number, if needed, is **D-032**.
- To catch up with `main`: `git fetch origin && git merge origin/main`. No rebase, never force-push.
- Never start Docker or the local Supabase stack. **You are the only session that may stop and restart the desktop app** the user has running: leave a working app running at the end, built from your merged `main` in the main checkout (`cargo build --release -p desktop` there, stop the old one, `Start-Process` the new one). That is the only thing you may do in the main checkout, after your PR is merged.

Working rules (user's decisions, 2026-10-09):
- Decide technical matters yourself. Ask only what is the user's call: new dependencies, data with unclear licences, committing real logs.
- **You merge your own PRs, always**, once CI is green and no review has open CRITICAL or HIGH findings: `gh pr merge <n> --squash --delete-branch`.
- The user often works remotely without access to the PC: never ask them to do something on it.
- `gh` is already logged in through `GH_TOKEN` in the environment; never print it.
- Anything long-running goes in a separate window (`Start-Process`).
- Python: use `C:\Users\andia\tavern-ledger\.venv\Scripts\python`.

**Red line (D-004, D-006, D-010):** the overlay is a separate transparent window on top of the game: it never reads the game's memory, injects anything or changes game files, and it never clicks or types for the player. It shows **only what the player could see on their own screen**: boards that entered play (D-010), tribes seen in the tavern (inferred, labelled) or entered by the user (T-303, labelled), the record from health (D-028, labelled inferred). Opponents by hero only; no names, BattleTags or `GameAccountId`.

## Effort and models per phase

| Phase | Effort | Subagents (if needed) |
|-------|--------|-----------------------|
| Read and check state | low | Haiku |
| Design (live state, window behaviour, what each panel shows) | high | — |
| Code (tests first) | high | Sonnet |
| Review | medium | `code-reviewer` |

## At the start

1. Conversation title: **`#023 Overlay`**.
2. `git fetch origin && git merge origin/main`.
3. Read `README.md`, `docs/plan/plan.md` (T-005, T-101, T-109, T-202, T-203, T-301 to T-303, T-D02 and their notes), `docs/decisions/DECISIONS.md` (D-004, D-006, D-010, D-019, D-026, D-028, D-029), `docs/research/desktop-stack.md`, `spikes/overlay-tauri/` (the tested transparent, always-on-top, click-through window) and `crates/`.
4. Green before touching anything: Rust (workspace and `desktop`), clippy, fmt, Python.

## Task

1. **Live game state.** While a game runs, the tracker keeps a small state the overlay reads: lobby heroes, tribes (seen in the tavern and entered by hand, apart), each opponent's last board that entered play with the round it was seen, and the record so far against each. Built from the same parser as the history, so live and saved agree (a test replays a fixture through both and compares). Unknown stays unknown.
2. **Overlay window.** A second Tauri window, as in the spike: transparent, always on top, click-through, no taskbar entry, hidden when no game runs and when Hearthstone is not the foreground window if that can be read without touching the game (window title or process only). Three compact panels: tribes, opponents (hero, last board, record), and a one-line game status. Every value with its source label (T-109). A tray menu entry to show or hide it. Say in the app and README that exclusive fullscreen hides any overlay (use windowed or borderless).
3. **Test without playing.** Use the log replay (T-D02) to drive the overlay through a saved game at game speed; check it by screenshot against the replay at a few rounds. No real logs committed.
4. Plan (T-301), README and D-032 if needed, in the same PR.
5. After merging: move the running desktop app to the new `main` build (see above) and leave the overlay off by default.

## Out of scope

- Moving panels, choosing panels and themes (T-302), upload and hosted backend (#022), anything the player could not see on screen.

## Acceptance criteria

- [ ] Tests before code; green: Rust (workspace and `desktop`), clippy, fmt, Python.
- [ ] Live state equals the saved report for the same log (test).
- [ ] Overlay transparent, on top, click-through, hidden outside games; every value labelled with its source.
- [ ] Checked with the log replay; screenshots described in the PR (no real names in them).
- [ ] App left running from `main`, overlay off by default.
- [ ] `/code-review` with no open CRITICAL or HIGH; CI green; PR merged by you.
- [ ] Plan, README (and D-032 if needed) updated in the same PR.

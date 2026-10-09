# Session 024: Customizable overlay (T-302)

Model: sonnet
Effort: medium (high for the lock/unlock window behaviour)
Subagents: haiku for searches; sonnet for code and reviews

Work in your worktree **`C:\Users\andia\tavern-ledger\.claude\worktrees\024-overlay-custom`** (branch `024-overlay-custom`, created by the hub) and follow its `CLAUDE.md`. Chat in caveman mode; the user writes in Spanish, answer in English. Everything in the repo is in English. **Tier R1** (local code, no network): TDD + `/code-review`. A new dependency or Tauri plugin makes it R2 (ask the user first).

**Another session runs at the same time** (#022 hosted backend). To stay out of its way:
- Work only in your worktree. Never touch the main checkout or other worktrees.
- Your area: the overlay in `crates/desktop` (its window, its UI, `overlay.json`, its tray entries and commands) and `tracker::live` only if a panel needs a value it does not have. Do not touch `supabase/`, `web/`, the upload code or the server config default (#022 may add one in `crates/`; merge `main` first if you both touch a file).
- Shared files (`plan.md`, `README.md`, `DECISIONS.md`): only your task's row and notes. Your decision number, if needed, is **D-033**.
- To catch up with `main`: `git fetch origin && git merge origin/main`. No rebase, never force-push.
- Never start Docker or the local Supabase stack. **You are the only session that may stop and restart the desktop app** the user has running: leave a working app running at the end, built from your merged `main` in the main checkout (`cargo build --release -p desktop` there, stop the old one, `Start-Process` the new one). That is the only thing you may do in the main checkout, after your PR is merged.

Working rules (user's decisions, 2026-10-09):
- Decide technical matters yourself. Ask only what is the user's call: new dependencies, data with unclear licences, committing real logs.
- **You merge your own PRs, always**, once CI is green and no review has open CRITICAL or HIGH findings: `gh pr merge <n> --squash --delete-branch`.
- The user often works remotely without access to the PC: never ask them to do something on it.
- `gh` is already logged in through `GH_TOKEN` in the environment; never print it.
- Anything long-running goes in a separate window (`Start-Process`).
- Python: use `C:\Users\andia\tavern-ledger\.venv\Scripts\python`.

**Red line (D-004, D-006, D-032):** the overlay stays a separate window: no memory reading, injection or game file changes, never clicks or types for the player, and shows only what the player could see on screen. Opponents by hero only; no names, BattleTags or `GameAccountId`. Source labels (T-109) stay on every value in every theme.

## Effort and models per phase

| Phase | Effort | Subagents (if needed) |
|-------|--------|-----------------------|
| Read and check state | low | Haiku |
| Design (locked vs unlocked, saved layout, themes) | medium | — |
| Code (tests first) | high | Sonnet |
| Review | medium | `code-reviewer` |

## At the start

1. Conversation title: **`#024 Overlay custom`**.
2. `git fetch origin && git merge origin/main`.
3. Read `README.md`, `docs/plan/plan.md` (T-109, T-301, T-302, T-D02 and their notes), `docs/decisions/DECISIONS.md` (D-004, D-006, D-010, D-028, D-029, D-032) and the overlay code in `crates/desktop`.
4. Green before touching anything: Rust (workspace and `desktop`), clippy, fmt, Python.

## Task

1. **Lock and unlock.** Locked (the default) = today's overlay: click-through, never takes focus. Unlocked = the panels can be dragged and resized with the mouse; switching is done from the tray menu and from a small button on the overlay that is only clickable while unlocked, plus a hotkey only if it needs no new dependency and does not reach into the game. The overlay goes back to locked when a game starts if the user left it unlocked, so it never steals clicks mid-game unless asked.
2. **Layout saved.** Each panel's position and size, and which panels are shown, saved in `overlay.json` (per screen resolution if simple), with a "reset layout" entry. Positions are kept on screen when the resolution changes. Tests for loading, saving, clamping and bad files (a broken `overlay.json` falls back to defaults and says so, never crashes).
3. **Themes.** At least three (for example dark, light, high-contrast), each readable over the game and passing a contrast check in a test; opacity setting. No meaning by colour alone.
4. Check it with the log replay (T-D02) in locked and unlocked mode; describe screenshots in the PR (no real names).
5. Plan (T-302), README and D-033 if needed, in the same PR.
6. After merging: move the running desktop app to the new `main` build (see above), overlay still off by default.

## Out of scope

- New data in the panels, upload and hosted backend (#022), the website.

## Acceptance criteria

- [ ] Tests before code; green: Rust (workspace and `desktop`), clippy, fmt, Python.
- [ ] Locked = click-through as before; unlocked = panels move and resize; back to locked when a game starts.
- [ ] Layout and panel choice saved and restored; bad file falls back to defaults with a message.
- [ ] Three themes with a contrast test; source labels in all of them.
- [ ] Checked with the log replay; app left running from `main`.
- [ ] `/code-review` with no open CRITICAL or HIGH; CI green; PR merged by you.
- [ ] Plan, README (and D-033 if needed) updated in the same PR.

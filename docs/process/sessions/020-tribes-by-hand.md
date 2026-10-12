# Session 020: Lobby tribes entered by hand (T-303)

Model: sonnet
Effort: medium
Subagents: haiku for searches; sonnet for code and reviews

Work in your worktree **`C:\Users\andia\tavern-ledger\.claude\worktrees\020-tribes-by-hand`** (branch `020-tribes-by-hand`, created by the hub) and follow its `CLAUDE.md`. Chat in caveman mode; the user writes in Spanish, answer in English. Everything in the repo is in English. **Tier R1** (local code, no network): TDD + `/code-review`. A new dependency makes it R2 (ask the user first).

**Another session runs at the same time** (#016 desktop upload). To stay out of its way:
- Work only in your worktree. Never touch the main checkout or other worktrees.
- Your area: `crates/desktop/ui/` (`app.js`, `index.html`, `style.css`), `tracker` (`store` for the saved entry, `stats` for showing it), `bg-parser` only if the tribe list or game id you need is not exposed yet. In `crates/desktop/src/main.rs` keep changes small (one or two commands): #016 also edits it. Do not touch `supabase/`, `web/`, upload code or the upload settings panel.
- Shared files (`plan.md`, `README.md`, `DECISIONS.md`): only your task's row and notes. Your decision number, if needed, is **D-029**.
- To catch up with `main`: `git fetch origin && git merge origin/main`. No rebase, never force-push.
- Never start Docker or the local Supabase stack. **You are the only session that may stop and restart the desktop app** the user has running: leave a working app running at the end, built from your merged `main` in the main checkout (`cargo build --release -p desktop` there, stop the old one, `Start-Process` the new one). That is the only thing you may do in the main checkout, after your PR is merged.

Working rules (user's decisions, 2026-10-09):
- Decide technical matters yourself. Ask only what is the user's call: new dependencies, data with unclear licences, committing real logs.
- **You merge your own PRs, always**, once CI is green and no review has open CRITICAL or HIGH findings: `gh pr merge <n> --squash --delete-branch`.
- The user often works remotely without access to the PC: never ask them to do something on it.
- `gh` is already logged in through `GH_TOKEN` in the environment; never print it.
- Anything long-running goes in a separate window (`Start-Process`).
- Python: use `C:\Users\andia\tavern-ledger\.venv\Scripts\python`.

**Red line (D-004, D-006):** local log files only. The app never reads the game's memory or screen to find the tribes: the user types or picks what they see. No names, BattleTags or `GameAccountId` anywhere.

## Effort and models per phase

| Phase | Effort | Subagents (if needed) |
|-------|--------|-----------------------|
| Read and check state | low | Haiku |
| Design (where the entry lives, how it attaches to a game) | medium | — |
| Code (tests first) | medium | Sonnet |
| Review | medium | `code-reviewer` |

## At the start

1. Conversation title: **`#020 Tribes by hand`**.
2. `git fetch origin && git merge origin/main`.
3. Read `README.md`, `docs/plan/plan.md` (T-101, T-109, T-202, T-203, T-301, T-303 and their notes), `docs/decisions/DECISIONS.md` (D-017, D-019, D-024, D-026, D-028), `docs/research/implementation-notes.md` and `crates/` (`bg-parser` report `shop_tribes`, `tracker` `store` and `stats`, `desktop`).
4. Green before touching anything: Rust (workspace and `desktop`), clippy, fmt, Python.

## Task

1. **T-303 tribes by hand.** In the app window, while a game is starting (hero select) or at any time after from the history row: the user picks the lobby's 5 tribes from a fixed list of the current Battlegrounds tribes (no free text). Saved locally next to the game's record, **outside the parser's report** (like `parser` in D-024), so re-reading a game (T-107) never loses it. Attach the entry to the game in progress; if no game is in progress, it waits and attaches to the next one, and the user can move or clear it from the history.
2. **Never mixed.** Shown with the *entered by you* source label (T-109), next to the *inferred* tribes seen in the tavern, never merged into them. If both exist and differ, show both; no automatic "correction" either way. Stats that use tribes say which source they used.
3. Tests first: saving and loading, attaching to the right game, re-reading a game keeps the entry, wrong input refused (not 5 tribes, unknown tribe, duplicates). `textContent` only in the UI.
4. Plan (T-303), README and D-029 if needed, in the same PR.
5. After merging: move the running desktop app to the new `main` build (see above) and check the history still shows.

## Out of scope

- Upload (#016), the overlay (T-301, T-302: the overlay will later show this entry), reading tribes from anything but the log and the user's own input.

## Acceptance criteria

- [ ] Tests before code; green: Rust (workspace and `desktop`), clippy, fmt, Python.
- [ ] Tribes entered by hand are saved outside the report, survive a re-read, and attach to the right game.
- [ ] Always labelled *entered by you*; never merged with inferred tribes.
- [ ] App tried with the real history; app left running from `main`.
- [ ] `/code-review` with no open CRITICAL or HIGH; CI green; PR merged by you.
- [ ] Plan, README (and D-029 if needed) updated in the same PR.

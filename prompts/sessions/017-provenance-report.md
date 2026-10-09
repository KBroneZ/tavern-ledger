# Session 017: Where values come from, and "report a problem" bundle (T-109, T-110)

Model: sonnet
Effort: medium
Subagents: haiku for searches; sonnet for code and reviews

Work in your worktree **`C:\Users\andia\tavern-ledger\.claude\worktrees\017-provenance-report`** (branch `017-provenance-report`, created by the hub) and follow its `CLAUDE.md`. Chat in caveman mode; the user writes in Spanish, answer in English. Everything in the repo is in English. **Tier R1** (local code, no network): TDD + `/code-review`. A new dependency makes it R2 (ask the user first).

**Other sessions run at the same time** (#016 desktop upload; #018 privacy and terms). To stay out of each other's way:
- Work only in your worktree. Never touch the main checkout or other worktrees.
- Your area: `crates/desktop/ui/` (`app.js`, `index.html`, `style.css`), `tracker::stats`, a new `tracker` module for the bundle, `bg-parser` only if a value needs a source flag. In `crates/desktop/src/main.rs` keep changes small (one or two commands): #016 also edits it. Do not touch `supabase/`, `web/`, upload code or the upload settings panel.
- Shared files (`plan.md`, `README.md`, `DECISIONS.md`): only your tasks' rows and notes. Your decision number, if needed, is **D-026**.
- To catch up with `main`: `git fetch origin && git merge origin/main`. No rebase, never force-push.
- Never start Docker or the local Supabase stack. **You are the only session that may stop and restart the desktop app** the user has running (it waits for the T-101 live test): leave a working app running at the end, built from your merged `main` in the main checkout (`cargo build --release -p desktop` there, stop the old one, `Start-Process` the new one). That is the only thing you may do in the main checkout, after your PR is merged.

Working rules (user's decisions, 2026-10-09):
- Decide technical matters yourself. Ask only what is the user's call: new dependencies, data with unclear licences, committing real logs.
- **You merge your own PRs, always**, once CI is green and no review has open CRITICAL or HIGH findings: `gh pr merge <n> --squash --delete-branch`.
- The user often works remotely without access to the PC: never ask them to do something on it.
- `gh` has no login of its own: export `GH_TOKEN` from the credential Git keeps (`git credential fill`) and never print it.
- Anything long-running goes in a separate window (`Start-Process`).
- Python: use `C:\Users\andia\tavern-ledger\.venv\Scripts\python`.

**Red line (D-004, D-006):** local log files only. Never print BattleTags or `GameAccountId` in chat, docs, tests, history, bundles or tool output.

## Effort and models per phase

| Phase | Effort | Subagents (if needed) |
|-------|--------|-----------------------|
| Read and check state | low | Haiku |
| Design (source labels, bundle contents) | medium | — |
| Code (tests first) | medium | Sonnet |
| Review | medium | `code-reviewer` |

## At the start

1. Conversation title: **`#017 Provenance and report`**.
2. `git fetch origin && git merge origin/main`.
3. Read `README.md`, `docs/plan/plan.md` (T-101, T-102, T-106, T-107, T-109, T-110), `docs/decisions/DECISIONS.md` (D-005, D-011, D-017, D-019, D-024), `docs/research/implementation-notes.md` and `crates/` (`tracker`, `desktop`, `bg-parser`).
4. Green before touching anything: Rust (workspace and `desktop`), clippy, fmt, Python.
5. T-101: count games per session in `%APPDATA%\TavernLedger\games.jsonl` without names (23 games from 5 sessions so far). If a new game appeared on its own while the app was open, mark T-101 done.

## Task

1. **T-109 where each value comes from.** Every number or label the app shows says its source: *from the log*, *inferred* (tribes seen in the tavern, skin grouping D-019), *entered by you* (nothing yet; keep the label ready for T-303), *from the leaderboard* (not in the app yet), *unknown*. Design it as data from Rust (a small enum serialized with the stats and rows), rendered in `app.js` with a short legend and a tooltip, never with colour alone. Tests in Rust for the labels; `textContent` only.
2. **T-110 "Report a problem" bundle.** From a game row: build one JSON file with that game's report, the parser version and revision (D-024), the game build, the app version, the log setup check (T-106) and the parser's warnings and problems. **Names removed**: run the report through a check that refuses anything BattleTag-shaped or any field outside the allowed set (tests with synthetic names). The app shows the file's content before saving it to a folder the user picks (or Downloads); it never sends it anywhere.
3. Plan (T-109, T-110), README and D-026 if needed, in the same PR.
4. After merging: move the running desktop app to the new `main` build (see above) and check it shows the history.

## Out of scope

- Upload (#016), website and privacy (#018), entering tribes by hand (T-303).

## Acceptance criteria

- [ ] Tests before code; green: Rust (workspace and `desktop`), clippy, fmt, Python.
- [ ] Every value in the window has a source label; legend present; no colour-only meaning.
- [ ] Bundle refuses names (tests) and is shown before saving; nothing is sent.
- [ ] App tried with the real history; app left running from `main`.
- [ ] `/code-review` with no open CRITICAL or HIGH; CI green; PR merged by you.
- [ ] Plan, README (and D-026 if needed) updated in the same PR.

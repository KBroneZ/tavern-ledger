# Session 008: T-101 wrap-up (desktop app)

Model: sonnet
Effort: medium
Subagents: haiku for searches; sonnet for code

Work in `C:\Users\andia\tavern-ledger` and follow its `CLAUDE.md`: caveman in chat; documents in Spanish; public README, product and UI in English. **Tier R2** (new dependencies, user files, possible network for card data): TDD + `/code-review` + `/security-review`. Branch and PR; never push directly to `main`. The user wants you to move on your own: decide what you can and ask only what is theirs (new dependencies, data sources with doubtful licenses, committing real logs, merge).

**Red line (D-004, D-006):** local logs only. No memory reading, injection or game automation; do not send clicks or keys to Hearthstone. Never print BattleTags or `GameAccountId` in chat, docs, tests, history or tool output.

## Effort and models per phase

| Phase | Effort | Subagents (if needed) |
|-------|--------|-----------------------|
| Reading and state | low | Haiku |
| Card data license | medium | Haiku (search); decision with the user if in doubt |
| Code (tests first) | medium | Sonnet |
| Review | medium | `code-reviewer` and `security-reviewer` in parallel |

## At the start

1. Title **`#008 T-101 wrap-up`**.
2. Check that the PR from sessions 006/007 (branch `006-007-tauri-app`: Tauri stack, Rust parser, tracker and app) is merged into `main`. If not, stop and warn.
3. Read: `README.md`, `docs/plan/plan.md` (T-101 notes), `docs/decisions/DECISIONS.md` (D-014, D-015), `docs/research/desktop-stack.md` and the code in `crates/` (`bg-parser`, `tracker`, `desktop`).
4. Run `cargo test --workspace --exclude desktop` and `python -m unittest discover -s tests` (with `.venv`). Everything must be green before touching anything.
5. Ask the user whether they have played games with the app open since session 007 and what they saw (did it show up by itself at the end?). If there are new games, compare them with `tools/compare_parsers.py` (results only, no names) and note it in the plan.

## State (2026-10-09)

- `bg-parser`: port of the Python prototype, identical on the 23 real games and on 13 synthetic cases (CI). Lines over 1 MiB are skipped and mark the game as `unsupported`.
- `tracker` / `tavern-watch`: follows `Power.log` (rotation, new sessions, partial lines) and saves to `%APPDATA%\TavernLedger\games.jsonl` (D-015). The user's real history already has their 23 games imported.
- `desktop`: Tauri app without npm, stats per mode (Solo 1–8, Duos 1–4 per team). CI does not build it (Linux needs system libraries).
- Session 007 reviews with no CRITICAL or HIGH; the MEDIUM ones are fixed.

## Task (in order; stop if a piece needs a user decision)

1. **Live test.** If the user has played with the app open, confirm from their account and from the history that the game showed up when it ended. If not, leave it as pending: do not launch Hearthstone to play.
2. **Hero names** instead of card ids. Look for a card data source with a compatible license (for example, the data used by HearthSim's `hearthstone` library, MIT, or HearthstoneJSON) and **verify the license of the data, not only of the code**. If it is not clear, stop and ask (rule 1 and Fan Content Policy, rule 5). No card images in this task. The data is bundled or downloaded with cache, timeout and pinned version; note it in `PROVENANCE.md`. An id with no known name comes out as the id, never invented.
3. **Single instance** of the app (the review flagged it LOW: two processes write the same history). Options: `tauri-plugin-single-instance` (new dependency, needs OK) or a lock file of our own. Recommend and ask.
4. **Tray and start with Windows** (optional, only if time allows): the app follows the log while minimized. Autostart is optional and off by default.
5. Close T-101 in the plan if 1–3 are done; if not, make clear what is missing.

## Out of scope

- Advanced stats (T-102), web and accounts (T-103/T-104), signing and installer (T-105), overlay (T-301/T-302).
- npm/TypeScript in the UI, unless the user asks.
- Trimmed fixtures from real logs (they need the user's OK before being committed).

## Acceptance criteria

- [ ] New tests before the code; `cargo test --workspace --exclude desktop`, `cargo clippy -- -D warnings`, `cargo fmt --check` and the Python tests green.
- [ ] Hero names with source and license noted in `PROVENANCE.md` (or the user's decision to postpone it, in `DECISIONS.md`).
- [ ] No player name in the history, the UI or the tests (checked against the names in the log without printing them, as in session 007).
- [ ] `/code-review` and `/security-review` with no open CRITICAL or HIGH findings.
- [ ] Plan, `DECISIONS.md`, `PROVENANCE.md` and README up to date in the same PR; CI green.

# Session 010: Personal stats (T-102)

Model: sonnet
Effort: medium
Subagents: haiku for searches; sonnet for code and reviews

Work in `C:\Users\andia\tavern-ledger` and follow its `CLAUDE.md`. Chat in caveman mode, in Spanish. Docs, code, commits, branches, PRs and UI are in English. **Tier R1** (normal code over local data): TDD + `/code-review`. A new dependency or any network access makes it R2 (`/security-review` and a `PROVENANCE.md` row).

Working rules (user's decisions, 2026-10-09):
- Decide technical matters yourself. Ask only what is the user's call: new dependencies, data with unclear licences, committing real logs.
- **You merge your own PRs, always**, once CI is green and no review has open CRITICAL or HIGH findings: `gh pr merge <n> --squash --delete-branch`.
- The user often works remotely without access to the PC: never ask them to do something on it.
- `gh` has no login of its own: export `GH_TOKEN` from the credential Git keeps (`git credential fill`) and never print it.
- Anything long-running (the app, live processes) goes in a separate window on the main PC (`Start-Process`), not in the conversation.

**Red line (D-004, D-006):** local log files only. No memory reading, injection or game automation. Never print BattleTags or `GameAccountId` in chat, docs, tests, history or tool output.

## Effort and models per phase

| Phase | Effort | Subagents (if needed) |
|-------|--------|-----------------------|
| Read and check state | low | Haiku |
| Stats design | medium | — |
| Code (tests first) | medium | Sonnet |
| Review | medium | `code-reviewer` (plus `security-reviewer` if it becomes R2) |

## At the start

1. Conversation title: **`#010 Personal stats`**.
2. `git switch main && git pull`; branch `010-personal-stats`.
3. Read `README.md`, the plan (T-101 and T-102 notes), the decisions (D-005, D-015 to D-018) and `crates/`: `bg-parser/src/report.rs`, `tracker/src/store.rs`, `desktop/src/main.rs` and `desktop/ui/app.js`. Session 009 moved the docs to English and may have renamed paths.
4. Green before touching anything: `cargo test --workspace --exclude desktop`, `cargo test -p desktop`, `python -m unittest discover -s tests` (with `.venv`).
5. If T-101 is still open (live test), check the history for new games, counting per session without names. Ask the user what they saw.

## State (2026-10-09, after session 008)

- `main` has the Rust parser, `tavern-watch` and the Tauri app. The app has a tray icon, optional start with Windows and a single history writer (`games.lock`).
- Each report in the history has these fields:
  - `hero`, `teammate_hero`, `final_place`, `final_health`;
  - `lobby`: 8 heroes, each with place and Duos team;
  - `shop_tribes`: tribes of the tavern offers, not the exact lobby tribes;
  - `rounds`: health and boards;
  - `card_names`: hero names read from the log, in the client's language.
- The app already shows, per mode: games, average place, top 4 % (Solo) or top 2 % (Duos) and wins. That logic lives in `app.js` and has no tests.
- The real history holds 23 games: 22 Duos, 1 Solo.

## Task

1. **Per-hero stats**, per mode (Solo and Duos apart): games, average place, top-half share and wins, sorted by games played. Hero name from `card_names`, the card id when missing.
2. **Skin grouping.** Ids carry skins and variants (`TB_BaconShop_HERO_102_SKIN_G`, `BG20_HERO_102pe`). Decide from the data whether to group them by base hero. Compare ids and hero names in the real history (hero names may be shown; player names never). If it is unclear, do not group and note why. Never invent a mapping.
3. **Tribes:** show `shop_tribes` as "tribes seen in the tavern", never as "lobby tribes". The log does not give the lobby tribes exactly; see the parser research doc.
4. **Logic in Rust, with tests:**
   - a stats module (for example `tracker::stats`) with unit tests;
   - the app gets the result through a Tauri command;
   - `app.js` only renders, with `textContent`;
   - move the totals that `app.js` computes today into the module, so there is only one calculation.
5. **Empty ≠ unknown ≠ zero:**
   - `incomplete` and `unsupported` games do not count for places, but say how many there are;
   - with 0 valid games, show "—", never 0.
6. Plan (T-102), README and, if there are decisions, `DECISIONS.md`, in the same PR.

## Out of scope

- The website, accounts and game upload (T-103/T-104); MMR (D-011 to D-013, another task); the overlay; the installer (T-105).
- npm/TypeScript in the UI unless the user asks for it.
- Card images.

## Acceptance criteria

- [ ] New tests before the code. Green: `cargo test --workspace --exclude desktop`, `cargo test -p desktop`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --check` and the Python tests.
- [ ] Per-hero stats and totals computed in Rust, with tests for incomplete games, an empty history and Solo vs Duos.
- [ ] No player names in the UI, tests or output (checked against the log's `PlayerName` values without printing them).
- [ ] App tried with the real history, launched in a separate window.
- [ ] `/code-review` with no open CRITICAL or HIGH; CI green; PR merged by you.
- [ ] Plan, README (and `DECISIONS.md` if needed) updated in the same PR.

# Session 055: Memory reader for the lobby tribes and own rating (T-327, T-328, D-063)

Model: opus
Effort: high
Subagents: haiku for searches; sonnet for code; opus for the reviews

Work in your worktree **`C:\Users\andia\tavern-ledger\.claude\worktrees\055-memory-reader`** (branch `055-memory-reader`, created by the hub) and follow its `CLAUDE.md`. Chat in caveman mode; the user writes in Spanish, answer in English. Everything in the repo is in English. **Tier R2 plus a second AI** (it touches the Blizzard limits and the user's account): TDD + `/code-review` + `/security-review` + `/santa-loop` (or `ask-chatgpt` for an independent review) before the hand-over.

**Other sessions at the same time:** 056 (Settings page, Spanish, overlay picture crop: `crates/desktop/ui/`, `crates/desktop/src/main.rs` settings parts). To stay out of its way:
- Work only in your worktree. Never touch the main checkout or other worktrees.
- Your area: a **new crate** (for example `crates/game-memory/`) for the reader, its wiring in `crates/tracker/` (the tribes source and the rating) and the smallest possible change in `crates/desktop/src/` (a config flag, off by default, and Tauri commands to read and set it, plus the status of the reader). **No UI**: 056 builds the Settings page and adds the switch there after you merge; give it the command names and the risk paragraph text in your hand-over. T-328 text changes: `CLAUDE.md` rule 3, `README.md`, `web/src/pages/index.astro`, `docs/legal/terms.md`, `docs/legal/privacy.md`, code comments, plan "Out of scope".
- Shared files (`plan.md`, `README.md`, `DECISIONS.md`, `PROVENANCE.md`): only your tasks' rows and notes. Decision number if you need one: **D-064**. Every changed text file LF (`git ls-files --eol`, index side).
- **New dependencies** (for example the `windows` crate features for `ReadProcessMemory`): allowed only with a compatible licence, pinned, recorded in `PROVENANCE.md`; tell the hub which ones in the hand-over (dependencies are R2).
- Never stop or restart the user's desktop app. After a build, `git checkout -- crates/desktop/gen/schemas`. CI does not build `crates/desktop`: run its tests yourself.
- To catch up with `main`: `git fetch origin && git merge origin/main`. No rebase, never force-push.

Working rules (user's decisions):
- **Ask your questions through the hub** (`tavern-ledger-47`, find it with `ListAgents` and use its `[ref]`) as well as in your tab. **You need the user's game open** (hero select, the Battlegrounds screen) to find and check the fields on their client: ask through the hub when, and say exactly what screen you need. The user may be on their phone; never ask them to do something they cannot.
- **`gh` is not available in task sessions.** Push the branch and message the hub with the branch, PR title, a short PR body (summary, reviews, test plan), the new dependencies and anything still waiting on the user.

## Why

The log has no lobby tribes (054, `docs/research/lobby-tribes-exact.md`). The user approved **D-063** on 2026-10-11: read, read-only, two values the game shows the player: the lobby's five tribes at hero select and the player's own Battlegrounds rating (Solo, Duos). Read the research doc in full first (sections 4 and 6 are the design and the decision) and the plan rows T-327 and T-328.

## Hard limits (D-004 as amended by D-063, D-006, absolute rules 1, 3, 6, 7)

- **Read-only:** open the process with `PROCESS_QUERY_LIMITED_INFORMATION` and `PROCESS_VM_READ` only. Never `PROCESS_VM_WRITE`, `PROCESS_VM_OPERATION`, `CreateRemoteThread`, DLLs, hooks, debuggers attached, or anything that changes the game (D-006, no exceptions).
- **Only the two values D-063 names.** Nothing else from memory: not opponents' names, BattleTags, ratings, account ids, collection, or anything the player cannot see. Tests guard the scope.
- **Clean room:** no code, offsets or structure layouts from HearthMirror, HDT, HSTracker, Firestone/MindVision, BobsBuddy or unity-spy (not read, not ported; Nomi's Kitchen not read at all). Find the data yourself through Mono's public runtime structures and the user's own client; write down how in the research doc, per game build.
- **Fail safe:** on any doubt (unknown build, checks fail: five distinct tribes, consistent with the shop rule of D-050), the app says "unsupported game version" and falls back to the inferred tribes. Never show a value that failed its checks.
- **Switch off by default.** With it off, the game process is never opened.
- Values are labelled "from the game", kept on the PC, **not uploaded** (no change to the report or the validator).
- Never print the user's BattleTag, account ids or other players' data while exploring memory; print offsets, type names, counts and tribe ids only.

## Task

1. Reader crate with a narrow API (`lobby_tribes() -> Result<[Tribe; 5]>`, `own_rating() -> Result<Rating>`), unit tests with synthetic memory images, and a check mode the app can run.
2. Find and verify the fields on the user's client (with the user at the right screen), across at least two games; record the build and how they were found.
3. Wire into `tracker`: tribes source `game` beats inferred and entered (labelled), the rating shown where the leaderboard rating is today; fallback paths tested.
4. Config flag off by default, Tauri commands, status (`off`, `ok`, `unsupported version`, `not running`).
5. T-328 texts in the same PR.
6. Plan rows T-327 and T-328, D-064 if needed, README, PROVENANCE; one PR through the hub.

## Out of scope

The Settings page and the switch UI (056), uploads, anything beyond D-063's two values.

## Acceptance criteria

- [ ] Read-only proven by the code (only the two access rights) and by a test that checks the flags.
- [ ] Both values found and checked on the user's client in two or more games, build recorded.
- [ ] Off by default; with it off, the game is never opened (tested).
- [ ] Fallback to inferred tribes on any failure, with "unsupported game version" (tested).
- [ ] T-328 texts changed; no claim left that the app only reads logs.
- [ ] `cargo test --workspace` and desktop tests pass locally (paste counts); `/code-review`, `/security-review` and a second AI review with no open CRITICAL or HIGH.

## Red line

D-004 (as amended by D-063), D-006, D-012, D-022, D-054 apply. Never print BattleTags, `GameAccountId`, player names or the user's email.

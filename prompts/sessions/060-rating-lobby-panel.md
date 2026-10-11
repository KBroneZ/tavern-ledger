# Session 060: Own rating from the game, the lobby panel, footer text (T-337, T-338)

Model: opus
Effort: high (memory mapping), medium for the panel
Subagents: haiku for searches; sonnet for code; opus for the reviews

Work in your worktree **`C:\Users\andia\tavern-ledger\.claude\worktrees\060-rating-lobby-panel`** (branch `060-rating-lobby-panel`, created by the hub) and follow its `CLAUDE.md`. Chat in caveman mode; the user writes in Spanish, answer in English. Everything in the repo is in English except the Spanish strings. **Tier R2 plus a second AI** for the memory part (D-063 scope): TDD + `/code-review` + `/security-review` + a second AI review (`/santa-loop` or `ask-chatgpt`).

No other task session runs now. Work only in your worktree; never touch the main checkout or other worktrees (055's worktree stays as it is: read its files, do not change them). Every changed text file LF (`git ls-files --eol`, index side). Decision number if needed **D-069**. Never stop or restart the user's desktop app; **never show a test build's overlay over the user's game** (the guard from 059 holds for debug builds; keep it on). After a build, `git checkout -- crates/desktop/gen/schemas`. CI does not build `crates/desktop` nor run its Node UI tests: run them yourself and paste counts. To catch up with `main`: `git fetch origin && git merge origin/main`; no rebase, never force-push.

Working rules: ask your questions through the hub (`tavern-ledger-47`, `ListAgents`, use its `[ref]`) as well as in your tab. `gh` is not available in task sessions: push the branch and message the hub with the branch, PR title and a short body. **Live reads:** auto mode blocks you from reading the game's memory; the user runs your commands with `!` **in your tab**. Give one short command at a time through the hub, with the exact screen it needs (the Battlegrounds lobby screen for the rating). Never print BattleTags, account ids or other players' data.

## State

- D-063 (approved): read-only reading of two values the game shows the player: the lobby's tribes (done in 055, #108) and the player's own Battlegrounds rating (Solo, Duos), off by default, never uploaded. Read `docs/research/memory-reader.md`, `docs/research/lobby-tribes-exact.md`, `crates/game-memory/`, and the plan row T-337.
- 055's scratch explorer: source in 055's scratchpad (`C:/Users/andia/AppData/Local/Temp/claude/C--Users-andia-tavern-ledger--claude-worktrees-055-memory-reader/59a0a12c-3517-4d88-bcbd-25d614e17967/scratchpad/explore`), binary at `.claude/worktrees/055-memory-reader/.local/explore.exe`. Run by the user on the Battlegrounds lobby screen on 2026-10-11, `explore.exe rating` printed `assembly+0x60 -> image; image+0x30 name = Assembly-CSharp`, `starts 2`, then **panicked at src\walk.rs:36:36: attempt to add with overflow**. Copy the explorer into your own scratchpad, fix it (checked arithmetic, bounds), and continue the mapping from there.
- The user will tell the hub the rating the game shows on screen; the hub passes it to you to check the value.

## Task

1. **T-337 own rating:** map the path to `NetCacheBaconRatingInfo` (`Rating`, `DuosRating`) through the service locator, implement `game::read_own_rating` with the same fail-safes as the tribes (checked build only, typed objects, plausible range, two agreeing reads), verified against the number on screen in at least one Solo or Duos lobby. On any doubt: "unsupported game version", never a value. Same switch as the tribes.
2. **T-338 lobby panel (the user's request, 2026-10-11):** "In the lobby before playing, a short overlay with the MMR, the MMR change, a summary of your stats, without overdoing it." Shown only on the Battlegrounds lobby screen (052's `LoadingScreen.log` scenes), gone when a game loads: own rating (Solo and Duos; "from the game" when the switch is on, else the leaderboard rating if known, else nothing), the change since the last game (from the last rating recorded after a game; never a guess), and a 1-line summary (games today, average place, top-4 share). Small, in a corner the lobby screen leaves free; Simple/Advanced like D-067; English and Spanish; can be turned off in Settings.
3. **Footer text:** the main window footer still says "Hero names come from your game's log, in your game's language. MMR is not in the game's log." Update it to match 057 (names follow the app language) and D-063 (rating from the game when the switch is on). English and Spanish.
4. Plan rows T-337 (done) and T-338 (add it), D-069 if needed, docs, README; one PR.

## Acceptance criteria

- [ ] Rating read and checked against the screen on the user's client, build recorded; fail-safe tested.
- [ ] Lobby panel only in the lobby, small, EN and ES, off switch; tests.
- [ ] Footer text updated.
- [ ] `cargo test --workspace`, desktop Node UI tests, release build pass locally (paste counts); reviews with no open CRITICAL or HIGH; branch handed to the hub.

## Red line

D-004 (as amended by D-063), D-006, D-012, D-022, D-054 apply. Read-only rights only; nothing beyond D-063's two values. Never print BattleTags, `GameAccountId`, player names or the user's email.

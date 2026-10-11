# Session 086: Combat replay, attack by attack, in the app (T-215)

Model: opus
Effort: high
Subagents: haiku for searches; sonnet for code and the review

Work in your worktree **`C:\Users\andia\tavern-ledger\.claude\worktrees\086-combat-replay`** (branch `086-combat-replay`, created by the hub) and follow its `CLAUDE.md`. Chat in caveman mode; the user writes in Spanish, answer in English. Everything in the repo is in English. **Tier R1**: TDD + `/code-review` (R2 if you touch the upload format or the validator: then also `/security-review`).

**Other sessions at the same time:** 085 (home page concepts, `docs/brand/home/` only). Your area: `crates/bg-parser/`, `crates/tracker/`, `crates/desktop/` (the main window's recap), and, only if the saved report gains a field, `supabase/functions/upload-game/` (its report key list and the validator version) plus the web viewer's types in `web/src/lib/game/` so nothing breaks. Shared files: only your rows and notes; decision number **D-099** (085 may take D-098). Every changed text file LF (`git ls-files --eol`, index side). To catch up: `git fetch origin && git merge origin/main`; no rebase, never force-push.

- **Before you hand over, run the tests from a clean clone of your pushed branch** (`cargo test --locked --workspace --exclude desktop`, `cargo check -p desktop`, plus the Deno and web tests if you touch them).
- Do not build or run the user's desktop app (the hub does, and reparses if `PARSER_REVISION` changes; it is 7 now). **Never show a test build over the user's game:** check `Hearthstone.exe` is not running before any visual test; prefer headless renders.
- The user may be busy: do not wait on them. Push the branch and message the hub (`tavern-ledger-2b`, `ListAgents`, use its `[ref]`) with the branch, PR title, a short PR body, and whether a reparse is needed.

## Why

The oldest open app item: the recap shows each combat's boards and result, but not how the fight went. The game plays back the player's own combats in the log (D-010), so they can be replayed attack by attack. The user's taste (rounds 3 and 4): compact and visual, key numbers first and details folded, Simple by default with Advanced (D-067, D-072), pictures without white edges, English and Spanish. Read D-010, D-032, D-054, D-058 (Duos copies stay out), D-067, D-072, D-087, `docs/research/parser-hslog.md` and `log-inventory.md`, and how `crates/combat-sim` names combat events (its replay format may be reusable for the view, not for the data: the replay here is what the log says happened, never a simulation).

## Task

1. **Parser:** for every combat the game plays back (Solo, and Duos fights the game shows), record the ordered events from the log: attacks (attacker, defender), damage, divine shield popped, deaths, summons, deathrattle and start-of-combat triggers where the log shows them, and the final hero damage. Entities by card id, side and board slot, golden flag; nothing about other players beyond what the player saw on screen (D-004, D-054). Combats the log does not play back say "not shown" (empty ≠ unknown). Tests on the synthetic and the user's fixtures; bump `PARSER_REVISION` if saved reports change, and say so.
2. **Recap in the main window:** a replay for each shown combat: both boards with the art the recap already uses, step forward and back, play and pause, speed, and a step list in Advanced. Simple shows the board, play button and outcome; details fold. English and Spanish strings.
3. **Upload and web:** if the report gains the events, keep uploads working (report key list, validator version, size limits checked) and keep the web viewer building; the web replay itself is a later task (note it in the plan, do not build it).
4. Plan row T-215 (app part), the web part as a new task, D-099, README.

## Acceptance criteria

- [ ] Events for shown combats, tested; "not shown" for the others; revision bump stated.
- [ ] Replay in the recap, Simple and Advanced, EN and ES; headless screenshots in the message to the hub.
- [ ] Uploads and the web build still pass; validator bumped if the report changed.
- [ ] Clean-clone tests green (paste counts); clippy, fmt; every file under 800 lines; `/code-review` with no open CRITICAL or HIGH; branch handed to the hub.

## Red line

Absolute rule 1 (clean room: never read or copy HDT, Firestone, Bob's Buddy, HearthMirror code; never read Nomi's Kitchen). D-004 (as amended by D-063), D-006, D-054. Never print BattleTags, `GameAccountId`, player names or the user's email.

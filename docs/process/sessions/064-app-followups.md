# Session 064: App follow-ups from round 4 (T-315, T-332, T-341 to T-344)

Model: opus
Effort: medium
Subagents: haiku for searches; sonnet for code and reviews

Work in your worktree **`C:\Users\andia\tavern-ledger\.claude\worktrees\064-app-followups`** (branch `064-app-followups`, created by the hub) and follow its `CLAUDE.md`. Chat in caveman mode; the user writes in Spanish, answer in English. Everything in the repo is in English except the Spanish strings. **Tier R1**: TDD + `/code-review` (the parser part included).

**Other sessions at the same time:** 062 (web design system, `web/` only) and 063 (data API, `supabase/` only). To stay out of their way:
- Work only in your worktree. Never touch the main checkout or other worktrees.
- Your area: `crates/desktop/`, `crates/tracker/`, `crates/bg-parser/` (T-332 only). Nothing in `web/`, `supabase/`, `docs/legal/`. If a parser change touches the uploaded report's keys, stop and tell the hub first (063 builds on them; the contract test is `crates/bg-parser/tests/upload_contract.rs`).
- Shared files (`plan.md`, `README.md`, `DECISIONS.md`): only your tasks' rows and notes; decision number if needed **D-072** (062 has D-070, 063 has D-071). Every changed text file LF (`git ls-files --eol`, index side; the checkout uses `core.autocrlf=true`).
- **Never show a test build's overlay over the user's game.** Hearthstone is running as this session starts. The guard from 059 (D-068) hides a debug or dev-flag build's overlay while `Hearthstone.exe` runs; keep it on, never pass `--allow-over-game`, check `Get-Process Hearthstone` before every visual test and prefer headless renders with stubbed Tauri (as 058 and 059 did). Never move the user's mouse.
- Never stop or restart the user's desktop app. After a build, `git checkout -- crates/desktop/gen/schemas`. CI does not build `crates/desktop` nor run its Node UI tests: run them yourself and paste counts.
- If you bump `PARSER_REVISION`, say so in your hand-over (the hub reparses the user's history after the merge).
- To catch up with `main`: `git fetch origin && git merge origin/main`. No rebase, never force-push.

Working rules: decide technical and design matters yourself (Lamplight glass D-052, Simple/Advanced D-067). Ask your questions through the hub (`tavern-ledger-2b`, `ListAgents`, use its `[ref]`) as well as in your tab. `gh` is not available in task sessions: push the branch and message the hub with the branch, PR title and a short PR body (summary, review results, test plan).

## At the start

1. Conversation title: **`#064 App follow-ups`**.
2. `git fetch origin && git merge origin/main`.
3. Read the plan rows and notes for T-315, T-332, T-333 to T-340 and T-338, D-061, D-067, D-068, D-069, and `docs/research/overlay-hover.md`, `docs/research/overlay-data.md`.

## Task (what the user reported on 2026-10-11, after round 4)

1. **T-315 damage cap in the overlay.** The user's screenshot shows the game's own text "Daño máximo: 15" (max damage) on screen, so the game shows it to the player and D-061's condition is met. Find where the log carries it (tag names and counts only from the user's logs; synthetic fixtures), show it next to the turn or the next opponent in the overlay, Simple and Advanced, English and Spanish. If the log does not carry it, say so in the notes and leave the row open; never compute a guessed value.
2. **T-341 top board vs the game's hover card.** The hover board across the top (T-333) covers the game's own hover card ("Playing as… / Combat history") that appears when the player hovers a hero in the leaderboard. Move or size it so it never covers that card at any supported resolution (D-067's scaling; check 3840x2160 at 150% and 1920x1080 at 100% headless).
3. **T-342 lobby panel numbers** (T-338):
   - "+N since the last game" did not show when Duos went 7197 → 7272. Find why (when the last rating is recorded, which mode, which game) and fix it; tests.
   - "10 games · top 2 67%" does not add up (likely 9 games had a place). Count the games behind each share consistently and say it as "6 of 9", the rule "every number says how many games back it".
4. **T-343 Duos hover: which teammate is on top.** Session 059 assumed the lower player id is the top portrait of a team's slot. Ask the user through the hub (one short question, with a description of what to look at in their next Duos game), and make the code follow the answer; until then leave the assumption and note it.
5. **T-332 turn 1 with no gold** (parser): a turn taken up by discovers has no gold tags. Per the row: record `gold_spent` 0 for such a turn when `NUM_RESOURCES_SPENT_THIS_GAME` proves it, the turn's own gold stays unknown. Tests on a synthetic fixture.
6. **T-344 Simple/Advanced in the main window** (the user's idea): the same switch as the overlay's (D-067) for the main window's lists, recap and stats: Simple shows key numbers and folds the rest; Advanced shows everything. One setting or two (overlay, main window): decide and record. English and Spanish.

Add rows T-341 to T-344 to the plan; mark each task done or say what is left.

## Acceptance criteria

- [ ] Each item done with tests, or left open with the reason in the plan notes.
- [ ] `cargo test --workspace`, the desktop Node UI tests and a release build pass locally (paste counts); headless renders of every changed overlay or window state at both resolutions.
- [ ] `/code-review` with no open CRITICAL or HIGH; branch handed to the hub.

## Red line

D-004 (as amended by D-063), D-006, D-012, D-022, D-054 apply. Local log files and D-063's two read-only values only. Never print BattleTags, `GameAccountId`, player names or the user's email.

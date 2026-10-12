# Session 059: Slim overlay side panel, single-hero hover in Duos, test-overlay guard (T-336, T-339, T-340)

Model: opus
Effort: medium
Subagents: haiku for searches; sonnet for code and reviews

Work in your worktree **`C:\Users\andia\tavern-ledger\.claude\worktrees\059-overlay-slim`** (branch `059-overlay-slim`, created by the hub) and follow its `CLAUDE.md`. Chat in caveman mode; the user writes in Spanish, answer in English. Everything in the repo is in English except the Spanish strings. **Tier R1**: TDD + `/code-review`.

**Other sessions at the same time:** 055 (memory reader: a new crate, `crates/tracker/` tribes and rating source, a config flag and Tauri commands, the memory switch in the Settings tab, texts in `web/` and `docs/legal/`). To stay out of its way:
- Work only in your worktree. Never touch the main checkout or other worktrees.
- Your area: the overlay (`crates/desktop/ui/overlay*`, `overlay.css`, `overlay.html`, `ui/strings/overlay.js`, `crates/desktop/src/overlay*.rs`, `hover.rs`, `leaderboard.rs`, `screen_fit.rs`) and a small guard in `crates/desktop/src/main.rs` (keep it small). Do not touch the Settings tab, `crates/tracker/` (except reading), the reader crate, `web/`, `docs/legal/`, `supabase/`, `crates/bg-parser/`.
- Shared files: only your tasks' rows and notes; decision number if needed **D-068**. Every changed text file LF (`git ls-files --eol`, index side).
- **Never show a test build's overlay over the user's game.** Sessions 056 and 058 did (7 s and 10 s). Check `Get-Process Hearthstone` immediately before every visual test; if it runs, test off screen only (headless render with stubbed Tauri, as 058 did). Never move the user's mouse.
- Never stop or restart the user's desktop app. After a build, `git checkout -- crates/desktop/gen/schemas`. CI does not build `crates/desktop` nor run its Node UI tests: run them yourself and paste counts.
- To catch up with `main`: `git fetch origin && git merge origin/main`. No rebase, never force-push.

Working rules: decide technical and design matters yourself (Lamplight glass D-052, Simple/Advanced D-067). Ask your questions through the hub (`tavern-ledger-47`, `ListAgents`, use its `[ref]`) as well as in your tab. `gh` is not available in task sessions: push the branch and message the hub with the branch, PR title and a short body.

## What the user said (2026-10-11, live Duos game, after 058)

Screenshot of the side panel: `C:\Users\andia\AppData\Local\Temp\claude\C--Users-andia-tavern-ledger\ed9aef9c-b64b-452b-ba1e-0c26bce2f7a2\scratchpad\overlay-side-panel.webp`.

1. "In the side overlay the names are not needed; bigger icons; it has too much information that makes no sense there. Orient yourself with Deck Tracker to understand what we want." Today the panel shows: your heroes' names, "no combat yet · ? hp", a Tribes block (seen in the tavern with counts, lobby "2 of 5 confirmed", inferred tags), an Opponents block with the next team's names, "T1+1 ? hp not fought", "no triple in the log yet" per hero, "no board seen yet unknown", a list of every team with names, a hint line and a source footer.
2. "When you hover over a pair, it shows both; ideally it shows only the one you hover, even in Duos."

## Task

1. **Slim side panel (T-339), in Simple mode (the default):** icon-first, no hero names (names on hover or in Advanced only), bigger hero portraits (readable at a glance at 3840x2160 @150%), only what helps a decision: the lobby tribes as tribe icons or short chips (the exact ones once 055's switch is on, else the inferred ones with one small mark), the next opponent marked, each opponent's tier and health as numbers (no "?" walls: hide unknowns in Simple), a triple count only when above 0. Move the source footer, the hint line and "not fought / no board seen" texts to Advanced or to a tooltip. Look at HDT's and Firestone's public screenshots for density (public material only; absolute rule 1; Nomi's Kitchen not at all). Advanced keeps today's detail.
2. **Single-hero hover in Duos (T-340):** hovering one hero of a team shows only that hero's last board (the board strip from 058), not both; the teammate appears only when you hover them.
3. **Test-overlay guard (T-336 follow-up):** debug and test builds (and any run with a dev flag) refuse to show the overlay window while `Hearthstone.exe` runs, unless an explicit `--allow-over-game` flag is given; release builds unchanged. Tests.
4. Node UI tests, markup checks; plan rows T-339, T-340 (and the guard note under T-336); README; same PR.

## Acceptance criteria

- [ ] Simple side panel: no names, bigger icons, no "?" or "unknown" texts; Advanced unchanged; tested.
- [ ] Duos hover shows only the hovered hero (tested).
- [ ] Guard: a test build does not show its overlay while Hearthstone runs (tested).
- [ ] `cargo test --workspace`, desktop Node UI tests, release build pass locally (paste counts); `/code-review` with no open CRITICAL or HIGH; branch handed to the hub with before/after notes (off-screen renders).

## Red line

D-004 (as amended by D-063), D-006, D-012, D-022, D-054 apply. Never print BattleTags, `GameAccountId`, player names or the user's email.

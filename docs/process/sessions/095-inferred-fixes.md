# Session 095: Fewer inferred values: exact tribes, combat damage, skins, play time (T-540, T-539)

Model: opus
Effort: high
Subagents: haiku for searches; sonnet for code and the review

Work in your worktree **`C:\Users\andia\tavern-ledger\.claude\worktrees\095-inferred-fixes`** (branch `095-inferred-fixes`, created by the hub) and follow its `CLAUDE.md`. Chat in caveman mode; the user writes in Spanish, answer in English. Everything in the repo is in English. **Tier R1, R2 if the upload format changes** (then also `/security-review`).

**Run after 094 (Dark Gifts parser) has merged**, since both change `crates/bg-parser/` and `crates/tracker/`. Your area: `crates/bg-parser/`, `crates/tracker/`, `crates/desktop/` (recap, stats, overlay record), `supabase/functions/upload-game/` if the report changes, the web game viewer if the report changes. Decision **D-108** if needed. Every changed text file LF. To catch up: `git fetch origin && git merge origin/main`; no rebase, never force-push.

- Do not build or run the user's desktop app; the hub rebuilds and reparses if `PARSER_REVISION` changes (say so). Never show a test build over the user's game.
- The user may be busy: do not wait on them. Push the branch and message the hub (`tavern-ledger-2b`, `ListAgents`, use its `[ref]`) with the branch, PR title, a short PR body and whether a reparse is needed.

## Why

Session 091's research (`docs/research/inferred-values.md`) and the user's answers in **D-104**. No new memory read is approved; D-063's two values stay the only ones.

## Task (T-540)

1. **Exact tribes with each game:** save the five tribes D-063 already reads at hero select with the game, on this PC only (never in the upload; check the uploader and the problem-report privacy check exclude it), and show them in the recap and the stats as exact; games without them keep the inferred tribes, marked as such.
2. **Combat damage** from the game's end-of-combat attack in the log, replacing the inferred damage (D-057) where the log has it.
3. **Skins:** group hero skins by HearthstoneJSON's `battlegroundsSkinParentId` (matches the log's own link 71 of 71 in the research); D-019's rule is wrong on 2 of 48.
4. **Each game's own play time.**
5. **Overlay bug:** an opponent's record is marked inferred even when every result came from the log (`crates/tracker/src/live.rs`, around line 431).
6. **T-539 (log only):** check in the user's logs whether the Duos teammate's current board appears while the player looks through the portal; if it does, record it for Duos odds; if not, say so. No memory read.
7. Counts before and after (how many values are still inferred) on the user's history; D-108 if a rule changes; plan rows T-540, T-539; README.

## Acceptance criteria

- [ ] Each fix tested; inferred counts before and after.
- [ ] Tribes never uploaded (tested).
- [ ] T-539 answered from the logs.
- [ ] Clean-clone tests green (paste counts); clippy, fmt; files under 800 lines; `/code-review` with no open CRITICAL or HIGH; branch handed to the hub.

## Red line

Absolute rules 1 and 3; D-004 as amended by D-063 and D-104; D-006; D-054. Never print BattleTags, `GameAccountId`, player names, keys or the user's email.

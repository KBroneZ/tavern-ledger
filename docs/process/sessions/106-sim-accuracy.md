# Session 106: Combat simulator accuracy, deep iteration (T-609)

Model: opus
Effort: high
Subagents: haiku for searches; sonnet for code and the review

Work in your worktree **`C:\Users\andia\tavern-ledger\.claude\worktrees\106-sim-accuracy`** (branch `106-sim-accuracy`, created by the hub) and follow its `CLAUDE.md`. Chat in caveman mode; the user writes in Spanish, answer in English. Everything in the repo is in English. **Tier R1**: TDD + `/code-review`.

**Other sessions at the same time:** 103 (auth emails, `supabase/templates/`). Your area: `crates/combat-sim/`, `crates/combat-validate/` (or wherever `combat-validate` lives), `crates/tracker/src/odds.rs` only if inputs change, the wasm build and `web/src/lib/sim/engine.*` when the engine version changes, the accuracy export and `/accuracy/` data, `docs/research/combat-simulator.md`. Decision **D-115** for rule changes. Every changed text file LF. To catch up: `git fetch origin && git merge origin/main`; no rebase, never force-push.

- **Budget rule (the user's):** the account's weekly usage must not pass 98 %. The hub watches it; if the hub tells you to stop, commit and push what you have within minutes, write the state and next steps in `docs/research/combat-simulator.md`, and message the hub. Commit and push after each round, so nothing is lost.
- The user may be busy (asleep tonight): do not wait on them. Message the hub (`tavern-ledger-c7`, `ListAgents`, use its `[ref]`) after each round with the numbers, and at the end with the branch, PR title and a short PR body. Do not build or run the user's desktop app; say if the hub must rebuild it.

## Clean room (absolute rule 1)

Never read, copy or port HDT, Bob's Buddy, Firestone, their simulators or HearthMirror code; never read Nomi's Kitchen. Rules come from card texts (HearthstoneJSON), patch notes, public wiki descriptions of mechanics, and the user's own logs. Write in the doc where each rule came from.

## Why (the user, 2026-10-12, night)

"Do iterations and deep research on the combat simulator to get good accuracy. Tomorrow I'll add the new members and we'll have more combat data to compare and verify." Today (D-107, session 094): Brier 0.251 on every board (71/84 right), app path 0.215 (69/80, 4 blocked), exact replays 34 of 79, on the user's 84 Duos combats.

## Task

1. **Measure first:** run `combat-validate --logs` on the user's logs (read-only) and the replays saved with games (parser revision 9) and break every miss down by cause: unmodelled or approximate card, attack order, targeting, start-of-combat effects, Deathrattle/Reborn ordering, summons when the board is full, Dark Gifts, hero powers/trinkets/quests that the engine does not see, input errors (board taken at the wrong moment). Counts per cause, the biggest first.
2. **Iterate in rounds:** each round fixes the biggest cause it can fix with a rule from public text, adds tests (unit and a replay-based regression from the user's combats), re-measures, and records before/after numbers. Stop a cause when the next fix is guesswork; mark approximations by name as today (D-088). Keep the engine deterministic for a seed and the website's results equal to the app's.
3. **Exact replay as the strongest check:** where the log has the full attack sequence (`rounds[].replay`), compare step by step and use the first divergence to find the rule that differs.
4. **Ready for more players:** tomorrow other people's games arrive. Make the validation run on any set of games the project may legally use (the user's own logs; uploaded games only as the privacy policy and D-088/D-091 allow, aggregate counts only, no names), with one command and a summary that says how many combats, how many blocked and why. Do not read other players' data tonight.
5. Update the accuracy export and `/accuracy/` page data only with numbers you measured; engine version bump and wasm rebuild if the engine changed; D-115; plan row T-609 (new: "simulator accuracy, deep iteration"); README.

## Acceptance criteria

- [ ] Miss breakdown by cause with counts, before and after each round.
- [ ] Each fix tested, sourced from public text or the user's logs; approximations named.
- [ ] App and website give equal results; clean-clone tests green (paste counts); clippy, fmt; `/code-review` with no open CRITICAL or HIGH; branch handed to the hub.

## Red line

Absolute rules 1 and 3; D-004 as amended by D-063 and D-104; D-054. Never print BattleTags, `GameAccountId`, player names, keys or the user's email.

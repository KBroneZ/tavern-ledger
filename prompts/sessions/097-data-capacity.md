# Session 097: Data capacity before real users (T-537, research)

Model: opus
Effort: medium
Subagents: haiku for searches and the size counts

Work in your worktree **`C:\Users\andia\tavern-ledger\.claude\worktrees\097-data-capacity`** (branch `097-data-capacity`, created by the hub) and follow its `CLAUDE.md`. Chat in caveman mode; the user writes in Spanish, answer in English. Everything in the repo is in English. **Research only**: no code, migration or dashboard change. The choice that follows can touch user data and money (R2/R3), so the user decides.

**Other sessions at the same time:** 092 (website home page, `web/`), 093 (app, `crates/desktop/`), 094 (Dark Gifts, `crates/bg-parser/`, `crates/tracker/`, upload-game). Your area: `docs/research/data-capacity.md` (new), plan row T-537, README line. Read everything else; change nothing else. Decision number reserved for the user's answer: **D-109** (do not write it; propose it). Every changed text file LF.

- The user may be busy: do not wait on them. Push the branch and message the hub (`tavern-ledger-c7`, `ListAgents`, use its `[ref]`) with the branch, PR title, a short PR body and the questions for the user.
- You cannot sign in to dashboards. If a number needs a read-only SQL query on the hosted project (table sizes, row counts), write the exact query and send it to the hub, which runs it in the Supabase SQL editor through Chrome.

## Why

The user (2026-10-12) marked this important: saved games grow. A Duos game with the T-215 replay (`rounds[].replay`, parser revision 8, validator 7) is about 200 KB of JSON, about 20 KB gzipped. Before sign-ups open, know how far the current plan goes and pick a policy.

## Task

1. **Measure real sizes** from the user's own history (`%APPDATA%\TavernLedger\games.jsonl`, read-only; print counts and sizes only, never names, BattleTags or ids): per game, Solo and Duos, with and without replays, raw JSON and gzipped; which parts weigh most (replay, boards, shop, turns). Then what the server stores per upload (read `supabase/functions/upload-game/`, the `games` table and its migrations: is the full report kept, compressed or not, which indexes and derived tables grow with it).
2. **Supabase's current limits** for the plan the project is on and the next one up (database size, storage, egress, row and function limits, what happens at the limit), from Supabase's own pricing and docs pages, with the date read. Also Cloudflare Pages limits that matter for the website reading games.
3. **Projection:** players × games per day × bytes per game → when each limit is hit, for a few honest scenarios (10, 100, 1,000 active players). Say which numbers are measured and which are assumed.
4. **Options**, each with cost, effort, what the player loses and how it fits D-002/D-003 (premium lives on the server) and GDPR (rule 6): replays kept on the PC only (not uploaded); a retention window on the server (full game for N days, then the summary); compressing the stored report; a storage alert from the daily sweep (`private.sweep_runs`, `deploy.md` 10.5 in `docs/research/deploy.md`); long history as a paid extra. Hub 061 recommended: replays on the PC only and the storage alert now; 90-day retention when sign-ups open; long history as a paid extra later. Check that against your numbers and say if you disagree.
5. **Questions for the user**, ready for the hub to ask one at a time, each with a recommendation, for D-109.
6. Plan row T-537 (research done; build follows the user's answer), README.

## Acceptance criteria

- [ ] Sizes measured from the user's games (counts only, nothing identifying); limits cited with source and date; projection with measured vs assumed marked.
- [ ] No code, migration or dashboard changed.
- [ ] Questions for D-109 ready; branch handed to the hub.

## Red line

Rules 4, 6 and 7. Never print BattleTags, `GameAccountId`, player names, keys or the user's email; no account, plan or billing change.

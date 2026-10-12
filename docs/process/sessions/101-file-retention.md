# Session 101: 28-day rolling retention of game files (T-544)

Model: opus
Effort: high
Subagents: haiku for searches; sonnet for code and the reviews

Work in your worktree **`C:\Users\andia\tavern-ledger\.claude\worktrees\101-file-retention`** (branch `101-file-retention`, created by the hub) and follow its `CLAUDE.md`. Chat in caveman mode; the user writes in Spanish, answer in English. Everything in the repo is in English. **Tier R2, and it deletes user data:** TDD, `/code-review`, `/security-review` and a second AI (ask the hub to run `ask-chatgpt` on your diff, or run `/santa-loop`) before the hub merges. Migrations deploy on merge (D-031).

**Other sessions at the same time:** 100 (simulator page: `web/src/pages/[...lang]/simulator.astro`, `web/src/components/sim/`, `web/src/lib/sim/`). Your area: `supabase/` (migration, sweep function, tests), `docs/legal/` (privacy EN and ES, data inventory), the website's legal pages only if they need a change, the game viewer only to say clearly when a game's full file has expired (`web/src/lib/game/`). Decision **D-113** if a rule needs one. Every changed text file LF. To catch up: `git fetch origin && git merge origin/main`; no rebase, never force-push.

- The user may be busy: do not wait on them. Push the branch and message the hub (`tavern-ledger-c7`, `ListAgents`, use its `[ref]`) with the branch, PR title, a short PR body and the exact post-merge steps.

## Why

D-109 (the user, 2026-10-12): a free account's full game files (turn-by-turn boards, replays) are kept on the server **28 days from when each game was played**, rolling per game; summaries (stats, records, placement) stay while the account exists; the app keeps everything on the PC. Paid plan and **beta testers (free for life)**: unlimited. Sign-ups are already open (D-089), so deletion may start only once the privacy policy says so. Numbers: `docs/research/data-capacity.md`.

## Task

1. **Exemption first.** There are no paid tiers yet, and the owner's account and future beta testers must never lose files. Add the smallest server-side "unlimited history" mark (set by the owner only, never writable by `anon` or `authenticated`, no client UI) and a documented SQL line for the hub to mark an account. The hub marks the owner's account after the merge, **before** the first deleting run.
2. **Retention in the daily sweep:** delete the Storage file (through the Storage API, as the orphan step does) of each game of an unmarked account whose played time is older than 28 days; keep the summary rows; record counts in `private.sweep_runs` (deleted, skipped by reason, failures), never ids. Bounded per run (count and time), safe to repeat, a failure recorded and never fatal. The 28 days live in one place.
3. **Ship it switched off:** the first deploy runs in report-only mode (counts what it would delete); the hub checks a run, marks the owner, then turns it on with a one-line migration or setting you provide. Say exactly how.
4. **Privacy policy (EN, ES) and data inventory** say it before it runs: what is kept 28 days, what stays, that the app keeps everything locally, that exempt accounts keep their files; dated, with a "What changed" line. Export and account deletion still cover everything kept (rule 6).
5. **Game viewer:** a game whose file has expired shows its summary and a plain line ("Full details are kept 28 days; this game's have expired") instead of an error.
6. Tests: pgTAP (grants, the mark, the selection: the 28-day boundary, exempt accounts, other users' files untouched), Deno for the sweep step (report-only and live), web tests for the expired view. Plan row T-544, README, D-113 if needed.

## Acceptance criteria

- [ ] Nothing deleted in report-only mode; the owner's exemption works; boundary and isolation tested.
- [ ] Privacy EN and ES and the data inventory updated and dated; export and delete unchanged in coverage.
- [ ] pgTAP, Deno and web green from a clean clone (paste counts); `/code-review`, `/security-review` and a second AI with no open CRITICAL or HIGH; branch handed to the hub with the exact post-merge steps.

## Red line

Rules 6 and 7. Never print BattleTags, `GameAccountId`, user ids, keys or the user's email; no plan, billing or dashboard change.

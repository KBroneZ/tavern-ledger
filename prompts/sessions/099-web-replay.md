# Session 099: Combat replay on the website's game viewer (T-536)

Model: opus
Effort: high
Subagents: haiku for searches; sonnet for code and the review

Work in your worktree **`C:\Users\andia\tavern-ledger\.claude\worktrees\099-web-replay`** (branch `099-web-replay`, created by the hub) and follow its `CLAUDE.md`. Chat in caveman mode; the user writes in Spanish, answer in English. Everything in the repo is in English. **Tier R1** (reads stored data as untrusted input; no upload or server change): TDD + `/code-review`.

**Other sessions at the same time:** 092 (home page: `web/src/pages/[...lang]/index.astro`, `web/src/styles/` home, scene, buttons and tavern styles, `components/ui/Button.astro`, `components/home/`) and 098 (`supabase/`). Your area: the game viewer (`web/src/lib/game/`, `web/src/pages/[...lang]/game.astro`, its styles and strings), new files for the replay. Do not edit 092's files; if you need a shared style or component, tell the hub first. Decision **D-111** if a rule needs one. Every changed text file LF. To catch up: `git fetch origin && git merge origin/main`; no rebase, never force-push.

- The user may be busy: do not wait on them. Push the branch and message the hub (`tavern-ledger-c7`, `ListAgents`, use its `[ref]`) with the branch, PR title, a short PR body and headless screenshots. Visual change: the hub may ask for a Cloudflare preview before merging.

## Why

T-215 gave the app combat replays, attack by attack (D-099, session 086): parser revision 8 keeps each shown combat's events in `rounds[].replay`, upload-game checks them (validator 7, now 8 with Dark Gifts, D-107), and the server keeps them (D-109: replays stay on the server). The website's game viewer does not play them yet. D-003 makes replays a free feature.

## Task

1. In the game viewer, each round with a `replay` gets a "Combat replay" like the app's recap: **Simple** = the two boards, play/pause and the result; **Advanced** = steps, speed and the step list. English and Spanish (voice guide `docs/brand/voice.md`). Dark Gift marks on minions as the viewer already draws them (D-107). Reduced motion: no animation, step by step only. Keyboard and screen reader usable (live region for the current step, focus kept).
2. Read the replay as **untrusted input** through the viewer's reader (D-079): validate shape and bounds, cap the steps, and show "replay not available" for anything malformed instead of guessing. A round without a replay shows nothing new.
3. Reuse the app's replay logic only where it is plain data handling with a clear port (both are ours, AGPL); the app's UI code stays where it is. Explain any rule that differs from the app.
4. Tests: unit tests for the reader and the step model (including malformed and oversized replays), build checks. Headless screenshots at 400 and 1440 px, EN and ES, Simple and Advanced, from a fixture game with a replay (synthetic or the user's own, no other players' private data).
5. Plan row T-536, README line, D-111 if needed.

## Acceptance criteria

- [ ] Replay plays in Simple and Advanced, EN and ES, keyboard and reduced motion ok.
- [ ] Malformed replays tested and shown as not available.
- [ ] Web tests and build checks green from a clean clone (paste counts); `/code-review` with no open CRITICAL or HIGH; branch and screenshots handed to the hub.

## Red line

Rules 1, 5 and 6. Never print BattleTags, `GameAccountId`, player names, keys or the user's email.

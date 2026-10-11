# Session 093: Overlay off, the new logo and better wording in the app (T-531)

Model: opus
Effort: high
Subagents: haiku for searches; sonnet for code and the review

Work in your worktree **`C:\Users\andia\tavern-ledger\.claude\worktrees\093-app-polish`** (branch `093-app-polish`, created by the hub) and follow its `CLAUDE.md`. Chat in caveman mode; the user writes in Spanish, answer in English. Everything in the repo is in English. **Tier R1**: TDD + `/code-review`.

**Other sessions at the same time:** 092 (the website home page, `web/`) and 094 (Dark Gifts on board minions: `crates/bg-parser/`, `crates/tracker/`, and the gift marks in the app's boards and the web viewer). Your area: `crates/desktop/` (settings, first start, masthead, overlay setting, strings), `crates/desktop/icons/` if needed, `docs/brand/`. Agree with 094 through the hub before touching the same UI file (094 only adds gift marks to minion pictures). Decision **D-106**. Every changed text file LF. To catch up: `git fetch origin && git merge origin/main`; no rebase, never force-push.

- Do not build or run the user's desktop app (the hub does). **Never show a test build over the user's game**: check `Hearthstone.exe` is not running before any visual test; prefer headless renders.
- The user may be busy: do not wait on them. Push the branch and message the hub (`tavern-ledger-2b`, `ListAgents`, use its `[ref]`) with the branch, PR title and a short PR body.

## Why (the user's notes, 2026-10-12)

1. **Overlay off as a first-class choice:** some players only want the stats and information, not the overlay, and "use it next to your favourite tracker to keep better stats of your games" is a selling point. Today only the tray menu has "Show overlay during a game".
2. **The logo:** the app's masthead still shows the old amber square next to "Tavern Ledger" (user's screenshot); the TL tankard logo (D-094, `docs/brand/logo/`) must be everywhere. "Check all the other logos that were not changed."
3. **Wording:** session 089 reviewed every public text and wrote a table of current → proposed app strings (EN and ES) in `docs/brand/` (voice guide, D-102), including the new name for "Curious stats". Apply it.

## Task

1. **Overlay setting:** a clear switch in the Settings page ("Overlay during games": on/off), the same value as the tray menu check (one source of truth, persisted), and a one-time question at first start: "Do you want the in-game overlay, or only stats?" (both explained in one line each; default on; changeable later). With the overlay off, nothing overlay-related runs during a game (no window, no hover hooks, no odds work), and the main window still records everything. Tests for the persisted value and the first-start flow.
2. **Logo audit:** list every place a mark or icon shows: app masthead, window title bar, tray, overlay panels, about/settings, installer images, website pages, favicon, social card, emails if any, docs. Replace every old mark with the right variant (D-094: `mark.svg`, `mark-small.svg` at small sizes, `app-tile` where a tile is expected). Report the list with before/after.
3. **App strings:** apply 089's table in English and Spanish, including the new name of the Curious stats tab; the strings tests stay green.
4. D-106, plan row T-531, README.

## Acceptance criteria

- [ ] Overlay switch in Settings, synced with the tray, asked once at first start; with it off nothing overlay-related runs.
- [ ] No old mark left anywhere; list in the hand-off.
- [ ] 089's app strings applied in EN and ES.
- [ ] Clean-clone tests green (paste counts); clippy, fmt; files under 800 lines; `/code-review` with no open CRITICAL or HIGH; headless screenshots; branch handed to the hub.

## Red line

D-004 (as amended by D-063), D-006, D-054; rule 5. Never print BattleTags, `GameAccountId`, player names, keys or the user's email.

# Session 033: Overlay v2, boards on hover over the leaderboard (T-306, T-307, T-308)

Model: opus
Effort: high (hover geometry and design), medium for the rest
Subagents: haiku for searches; sonnet for code and reviews

Work in your worktree **`C:\Users\andia\tavern-ledger\.claude\worktrees\033-overlay-hover`** (branch `033-overlay-hover`, created by the hub) and follow its `CLAUDE.md`. Chat in caveman mode; the user writes in Spanish, answer in English. Everything in the repo is in English. **Tier R1** (local UI and log reading). A new dependency makes it R2: ask the user first and record it in `PROVENANCE.md`.

**You run alone this round** (session 034, shop and economy stats, starts after you merge: it also changes `crates/bg-parser` and `crates/tracker`). Still:
- Work only in your worktree. Never touch the main checkout or other worktrees.
- Shared files (`plan.md`, `README.md`, `DECISIONS.md`, `PROVENANCE.md`): only your tasks' rows and notes. Your decision number is **D-045**. Keep every changed text file LF (`git ls-files --eol`).
- Never stop or restart the user's desktop app (it runs from the main checkout's `target\release`); test with your own build, your own data folder and the log replay (T-D02). The hub restarts the user's app after your merge.
- To catch up with `main`: `git fetch origin && git merge origin/main`. No rebase, never force-push.

Working rules (user's decisions):
- Decide technical matters yourself. Ask the user: new dependencies, anything about Blizzard's rules you are unsure of.
- **`gh` is not logged in inside task sessions.** When your branch is ready (tests green, `/code-review` done, no open CRITICAL or HIGH), push it and send the hub (`Hub prompts/sessions/025`, find it with `ListAgents`) a message with the branch, the PR title, a short PR body (summary, review results, test plan) and anything still waiting on the user. The hub opens the PR, waits for CI and merges it.
- The user is at home and plays; you may ask them to try a build in a real game (Solo and Duos) and send screenshots. Never press them to.

## Why (user's first live test, 2026-10-10)

Read the plan's "Notes from the user's first live test". In short: the overlay worked, but the user does not want a panel listing opponents' minions. They want what Hearthstone Deck Tracker does: hover a hero on **the game's own leaderboard** (the column of heroes at the left of the game screen) and see that opponent's last-seen board next to it. For an opponent at tavern tier 1 or 2, the same hover also shows the tier 1-2 minions they could have from the lobby's tribes (past tier 2 it is too hard to guess, so nothing). And the overlay must never be larger than the screen. The overlay switch is only in the tray today and the user could not find it.

## Rules for this feature (D-004, D-006, absolute rule 1)

- **Nothing read from the game** but the log: no memory, no screenshots or pixel reading of the game, no injection. Allowed: the cursor position (`GetCursorPos`), the game window's rectangle (`GetClientRect`/`ClientToScreen` or `GetWindowRect`, user32, as the overlay already does for the foreground check), the monitor's size and DPI, and the log.
- Show only what the player saw or public card data: a board the player met in combat (as today, with its round), and "possible" minions from the public card pool. Label each with its source (T-109): `seen in round N`, `possible` (new source; explain it in the legend).
- Clean-room: HDT's and Firestone's code is not read or ported. Their public pages and release notes may be read for the idea only (record the links in `docs/research/`). Nomi's Kitchen: nothing at all.
- The overlay stays click-through when locked; hover works by watching the cursor, never by taking the mouse.

## Effort and models per phase

| Phase | Effort | Model |
|-------|--------|-------|
| Read and check state | low | Haiku |
| Leaderboard geometry and hover design | high | Opus |
| Parser/tracker data (leaderboard order, opponents' tiers, card pool) | medium | Sonnet |
| Overlay UI, screen bounds, switch | medium | Sonnet |
| Review | medium | `code-reviewer` |

## At the start

1. Conversation title: **`#033 Overlay hover`**.
2. `git fetch origin && git merge origin/main`.
3. Read the plan's F3 rows and notes (T-301 to T-308 and the live-test notes), `DECISIONS.md` (D-004, D-006, D-017, D-028, D-032, D-033, D-038, D-039), `crates/desktop/src/overlay.rs`, `overlay_layout.rs`, `crates/desktop/ui/overlay*.{js,css,html}`, `crates/tracker/src/live.rs`, and `crates/cards` (what the card data keeps today: names only).

## Task

1. **Data.** From the log: each lobby hero's leaderboard place (`PLAYER_LEADERBOARD_PLACE`) and tavern tier (`PLAYER_TECH_LEVEL`, set on each hero entity) as they change, into the live state (`tracker::live`) with tests on synthetic logs. Duos: the leaderboard shows teams; map each slot to the team's heroes. From the card data: keep each Battlegrounds pool minion's tier and tribes (HearthstoneJSON fields; check which ones on its own pages) next to the names; same caps and validation as today.
2. **Hover.** Work out where each leaderboard slot is on screen from the game window's client rectangle (the game scales its layout with the window height; measure from screenshots at more than one size: ask the user for one at 3840x2160 and use the replay at 1920x1080 and 1280x720). Keep the numbers in one place, tested. Because a patch can move the leaderboard, give the unlocked overlay a way for the user to adjust the leaderboard area (drag a box over it), saved per resolution in `overlay.json`. Watch the cursor (poll at about 20 Hz while a game is on and Hearthstone is in front; stop otherwise); when it rests on a slot, show that opponent's card next to the leaderboard: hero, health, record, the last-seen board with the round, and nothing for your own hero. Hide on leave. No game data in the popup that the panel does not already show today.
3. **Tier 1-2 possible minions.** On the same hover, if that opponent's tier is 1 or 2: the pool minions of tier ≤ their tier whose tribe is in the lobby (lobby tribes entered by the user; if none, the tribes seen in the tavern, labelled inferred) plus neutral ones, labelled `possible`, as small names or icons under the board. Tier 3 or more: nothing. No tribes known: say so instead of guessing.
4. **Opponents panel.** Remove the minion lists from it; keep it as an optional compact list (hero, health, record, tier) that the user can hide. Make "not fought" rows short.
5. **Screen bounds (T-308).** The overlay window and every panel and popup stay inside the monitor that holds the game (work area), after a resolution, DPI or monitor change too, locked or unlocked; a popup flips side or shrinks rather than going off screen. Tests for the fitting.
6. **Switch (T-308).** An "Overlay" on/off switch in the main window (header or settings), in sync with the tray item, using the existing saved choice in `overlay.json`.
7. **Check** with the log replay (T-D02) at two window sizes, then ask the user for one real game (Solo, and Duos if they can) with screenshots of the hover.
8. Plan rows T-306, T-307, T-308 and notes, README, D-045, research note with the public links, same PR.

## Out of scope

Shop and economy stats and APM (session 034), combat odds, card names in the game's language (follow-up), anything that reads the game other than the log.

## Acceptance criteria

- [ ] Hovering a leaderboard hero shows that opponent's last-seen board (with its round) next to the leaderboard; nothing is taken from the game but the log, the cursor and the window rectangle.
- [ ] At tier 1-2 the hover also lists `possible` minions from the lobby's tribes; from tier 3 nothing; unknown tribes said, not guessed.
- [ ] Leaderboard area adjustable by the user and saved per resolution; geometry numbers tested.
- [ ] Overlay, panels and popups never leave the screen; tests.
- [ ] Overlay switch in the main window, in sync with the tray.
- [ ] Checked with the replay at two sizes and, if the user agrees, in a real game.
- [ ] `/code-review` with no open CRITICAL or HIGH; PR merged through the hub.

## Red line

D-004, D-006, D-012, D-022 apply. Never print BattleTags or `GameAccountId`. No accounts, terms, payments or publishing without the user.

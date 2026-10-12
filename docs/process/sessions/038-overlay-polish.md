# Session 038: Overlay polish after the first Duos game (T-309)

Model: opus
Effort: medium
Subagents: haiku for searches; sonnet for code and reviews

Work in your worktree **`C:\Users\andia\tavern-ledger\.claude\worktrees\038-overlay-polish`** (branch `038-overlay-polish`, created by the hub) and follow its `CLAUDE.md`. Chat in caveman mode; the user writes in Spanish, answer in English. Everything in the repo is in English. **Tier R1** (local UI and log reading). A new dependency makes it R2: ask the user first and record it in `PROVENANCE.md`.

**Another session runs at the same time** (037, follow-ups before opening sign-ups: `web/`, `supabase/`, `docs/legal/`, `crates/uploader/src/loopback.rs` and `auth.rs`, and a few lines in `crates/desktop/` that turn a sign-in error into text). To stay out of its way:
- Work only in your worktree. Never touch the main checkout or other worktrees.
- Your area: the overlay in `crates/desktop/` (Rust and `ui/`), and `crates/tracker/` or `crates/bg-parser/` only if the shop check below finds a parser bug. Do not touch `crates/uploader/`, `web/`, `supabase/`, `docs/legal/`, or the sign-in error text in `crates/desktop/`.
- Shared files (`plan.md`, `README.md`, `DECISIONS.md`, `PROVENANCE.md`): only your task's rows and notes. Your decision number is **D-049**. Keep every changed text file LF (`git ls-files --eol` before committing).
- Never stop or restart the user's desktop app (it runs from the main checkout's `target\release`); test with your own build, your own data folder and the log replay (T-D02). After a build, `git checkout -- crates/desktop/gen/schemas` (the build rewrites them with CRLF). The hub restarts the user's app after your merge.
- Never start Docker or the local Supabase stack (037 owns it).
- To catch up with `main`: `git fetch origin && git merge origin/main`. No rebase, never force-push.
- **If the shop check changes what the parser writes:** bump `PARSER_REVISION`, keep `supabase/functions/upload-game/report_keys.json` and `validate.ts` in step only if keys change (D-047; that is 037's area, so tell the hub first and keep it to the key list), and bump `VALIDATOR_VERSION` if the validator starts accepting something new.

Working rules (user's decisions):
- Decide technical matters yourself. Ask the user: new dependencies, anything about Blizzard's rules you are unsure of.
- **`gh` is not logged in inside task sessions.** When your branch is ready (tests green, `/code-review` done, no open CRITICAL or HIGH), push it and send the hub (`Hub prompts session 036`, find it with `ListAgents`, use its `[ref]`) a message with the branch, the PR title, a short PR body (summary, review results, test plan) and anything still waiting on the user. The hub opens the PR, waits for CI and merges it.
- The user is at home and plays; you may ask them to try a build in a real game (Solo and Duos) and send screenshots. Never press them to.

## Why (the user's first Duos game with overlay v2, 2026-10-10)

The user played one Duos game (saved in the history as the last record of session folder `Hearthstone_2026_10_10_18_11_27`, uploaded, parser revision 3, no problems) and asked for two changes, for **every mode** (Solo and Duos):

1. **Possible minions by game turn, not by tier.** Today (D-045) the hover card shows "possible" minions for an opponent at tavern tier 1 or 2. The user wants them **only on game turns 2 and 3**: nothing on turn 1, nothing from turn 4 on. The pool stays as D-045 has it (pool minions of the opponent's tier or lower, lobby tribes plus neutral and all-tribe ones, labelled `possible`). Use the turn the log gives (the same turn number the shop record and the rounds use); say in D-049 which one.
2. **The opponent's board goes at the top, not next to the leaderboard.** Today the hover card opens beside the game's leaderboard column on the left. It must open **at the top of the game window**, horizontally centred, minions in one row, like the board strip the game itself shows; still click-through, still inside the screen (shrink rather than leave it), still on the monitor that holds the game. The hover trigger stays the leaderboard (D-045 geometry and the user's drag box).

Also check, from that game's own log (`C:\Battle.net\Battle.net\Hearthstone\Logs\Hearthstone_2026_10_10_18_11_27\Power.log`, shared read access, counts and card ids only, never names): its shop record shows turn 8 with 3 buys and 10 sells and turn 9 with 2 buys and 10 sells. That looks wrong for Duos (passing a card to the teammate, or a teammate's action, may be read as a sell). Count by hand from the raw lines; if the parser is wrong, fix it with a synthetic Duos fixture (`tests/bg_log_builder.py`), keep the real fixtures' reports unchanged unless the bug is in them too, and update D-046's notes. If it is right, say why in D-049.

The Duos leaderboard geometry is still unverified (the user did not check the hover in this game). Keep it; ask the user once, in your tab, for a full-screen 4K screenshot of a Duos game with the new build.

## Effort and models per phase

| Phase | Effort | Model |
|-------|--------|-------|
| Read and check state | low | Haiku |
| Turn rule and top placement | medium | Sonnet |
| Duos shop check against the raw log | high | Opus |
| Docs, review | medium | `code-reviewer` |

## At the start

1. Conversation title: **`#038 Overlay polish`**.
2. `git fetch origin && git merge origin/main`.
3. Read D-045, D-046, the plan's notes for T-306 to T-308 and T-204, the overlay code in `crates/desktop/` (hover thread, card placement, possible minions) and `ui/`, and `crates/bg-parser/src/shop.rs`.

## Rules (D-004, D-006, absolute rule 1)

- Only the log, the cursor position, the game window's rectangle and the monitor's size and DPI. No memory, no pixel reading of the game, no injection, nothing automated.
- Show only what the player saw (boards met in combat, with their round) or public card data (`possible`), each labelled with its source.
- Clean-room: no code of HDT, Firestone or their simulators is read or ported; Nomi's Kitchen: nothing at all.

## Task

1. Possible minions on game turns 2 and 3 only, every mode; tests for turns 1, 2, 3 and 4 in Solo and Duos.
2. Hover card at the top of the game window, centred, one row; tests for the placement maths (4K, 1080p, a window not at the screen's origin, a small window where it must shrink, two monitors).
3. Duos shop check, and a fix if needed (see above).
4. Plan row **T-309** "Overlay polish after the first Duos game"; D-049; README; same PR.

## Out of scope

Sign-in, upload, the website, card names in other languages, the switch to turn pictures off, T-201.

## Acceptance criteria

- [ ] No possible minions on turn 1 or from turn 4 on; shown on turns 2 and 3 in Solo and Duos; tested.
- [ ] The board opens at the top of the game window, centred, inside the screen; tested; checked with your own build and the log replay.
- [ ] The Duos shop counts checked against the raw log, fixed if wrong.
- [ ] `/code-review` with no open CRITICAL or HIGH; PR merged through the hub.

## Red line

D-004, D-006, D-012, D-022 apply. Never print BattleTags, `GameAccountId`, player names or the user's email.

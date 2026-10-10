# Session 039: Lobby tribes from the log, and a board guess for opponents not met yet (T-310)

Model: opus
Effort: high (log research), medium for the rest
Subagents: haiku for searches; sonnet for code and reviews

Work in your worktree **`C:\Users\andia\tavern-ledger\.claude\worktrees\039-lobby-tribes-log`** (branch `039-lobby-tribes-log`, created by the hub) and follow its `CLAUDE.md`. Chat in caveman mode; the user writes in Spanish, answer in English. Everything in the repo is in English. **Tier R1** (log reading, local UI); it becomes **R2** if the saved report gains keys (the upload changes): then `/security-review` as well. A new dependency: ask the user first.

**Another session runs at the same time** (037, follow-ups before opening sign-ups: `web/`, `supabase/`, `docs/legal/`, `crates/uploader/src/loopback.rs` and `auth.rs`, and a few lines in `crates/desktop/` that turn a sign-in error into text). To stay out of its way:
- Work only in your worktree. Never touch the main checkout or other worktrees.
- Your area: `crates/bg-parser/`, `crates/tracker/`, the overlay in `crates/desktop/` (Rust and `ui/`), `tests/`, `tools/` (fixtures), `docs/research/`. Not `crates/uploader/`, `web/`, `docs/legal/`, nor the sign-in error text in `crates/desktop/`.
- **Upload contract (D-047):** prefer keeping new values in the live state only (as D-045 did). If the saved report gains keys, the only `supabase/` files you may touch are `supabase/functions/upload-game/report_keys.json` and the `KEYS` table and checks in `validate.ts`, bump `PARSER_REVISION` and `VALIDATOR_VERSION`, and message the hub before you start on them (037 works in `supabase/`).
- Shared files (`plan.md`, `README.md`, `DECISIONS.md`, `PROVENANCE.md`): only your task's rows and notes. Your decision number is **D-050**. Keep every changed text file LF (`git ls-files --eol` before committing).
- Never stop or restart the user's desktop app (it runs from the main checkout's `target\release`); test with your own build, your own data folder and the log replay (T-D02). After a build, `git checkout -- crates/desktop/gen/schemas`. The hub restarts the user's app after your merge (and runs `--reparse` if the parser revision changed).
- Never start Docker or the local Supabase stack (037 owns it).
- To catch up with `main`: `git fetch origin && git merge origin/main`. No rebase, never force-push.

Working rules (user's decisions):
- Decide technical matters yourself. Ask the user: new dependencies, anything about Blizzard's rules you are unsure of.
- **`gh` is not logged in inside task sessions.** When your branch is ready (tests green, reviews done, no open CRITICAL or HIGH), push it and send the hub (`Hub prompts session 036`, find it with `ListAgents`, use its `[ref]`) a message with the branch, the PR title, a short PR body (summary, review results, test plan) and anything still waiting on the user. The hub opens the PR, waits for CI and merges it.
- The user is at home and plays; you may ask them to try a build in a real game (Solo and Duos) and send screenshots. Never press them to.

## Why (the user, 2026-10-10)

Today the lobby's five tribes are either entered by hand (T-303) or guessed from the minions seen in the tavern (labelled inferred), and every report says `available tribes (exact list)` is not in the log (`docs/research/parser-hslog.md`). The user is sure the log has them at the start of the game. They want:

1. **The lobby's tribes read from the log** at the start of the game (exact, labelled as from the log), so the overlay and the recap no longer depend on the hand entry or the guess.
2. **A board guess for an opponent whose board has not been seen yet.** When hovering such an opponent, combine the lobby's tribes with what the game itself shows for that player (the user says the game shows **a minion type and a number of minions** for each player: find where that is in the log, for example a tag on the player or hero entity) and their tavern tier, and show the minions they likely have, labelled `possible`.

## A lead the hub found (counts of tag names only, the user's Duos log of 2026-10-10)

The log has `BACON_SUBSET_<TRIBE>` tags (ABERRATION, BEAST, DEMON, DRAGON, ELEMENTALS, MECH, MURLOC, NAGA, PIRATE, QUILLBOAR, UNDEAD) with very different counts per tribe (DRAGON 44 and NAGA 12 against PIRATE 2876 and ELEMENTALS 1548). Find which entities carry them and whether they mark the lobby's tribes (or the banned ones) at game start. Other tags to look at: `CARDRACE`, `IS_BACON_POOL_MINION`, `BACON_MAX_PLAYER_TECH_LEVEL`, and anything on the player and hero entities that changes after each combat (a tribe and a count). No tag of the form `*DOMINANT*` or `*TRIBE*` showed up, so look by value, not by name.

## Effort and models per phase

| Phase | Effort | Model |
|-------|--------|-------|
| Read and check state | low | Haiku |
| Log research: lobby tribes, per-player tribe and count | high | Opus |
| Parser, tracker, overlay | medium | Sonnet |
| Docs, review | medium | `code-reviewer` (+ `security-reviewer` if R2) |

## At the start

1. Conversation title: **`#039 Lobby tribes from the log`**.
2. `git fetch origin && git merge origin/main`.
3. Read D-017, D-029, D-045, D-046, D-049, `docs/research/parser-hslog.md`, the plan's notes for T-303 and T-306 to T-309, `crates/tracker/src/possible.rs` and `live.rs`, `crates/tracker/src/lobby_tribes.rs`, and the overlay hover code.

## Rules (D-004, D-006, absolute rule 1)

- Only the log (and, for the overlay, the cursor position, the game window's rectangle and the monitor). No memory, no pixel reading of the game, no injection, nothing automated.
- Show only what the player could see or public card data. If a value the game draws on screen is **not** in the log, it is out of reach: say so in the research and in D-050, never work around it.
- Read the user's `Power.log` files with shared access (`[IO.File]::Open(..., 'Read', 'ReadWrite')`) while the game may run. Print and record counts, tag names, entity types and card ids only, never player names, BattleTags or `GameAccountId`.
- Clean-room: no code of HDT, Firestone, their simulators, HearthMirror or BobsBuddy is read or ported; public pages may be read for ideas only (record links). Nomi's Kitchen: nothing at all.

## Task

1. **Research** (`docs/research/lobby-tribes-log.md`): across every Battlegrounds game in the user's logs (Solo and Duos), where the lobby's tribes are in the log and from when, checked against the user's hand entries in `lobby_tribes.json` and the tribes seen in the tavern; and whether the per-player minion type and count the game shows are in the log (which tag, on which entity, when it changes). Each finding with counts that anyone can re-check from the raw lines.
2. **Lobby tribes:** if found, the parser reads them (source `log`, exact) and the overlay, the recap and the stats use them first; the hand entry stays possible and is shown apart if it differs; the guess from the tavern is used only when the log has none. Tests with a synthetic fixture (`tests/bg_log_builder.py`) and the real fixtures (`tools/make_fixture.py` must keep the lines that carry them; re-check the privacy scan).
3. **Board guess:** for an opponent with no board seen yet, the hover card shows the likely minions from the lobby's tribes, the opponent's tier, and the tribe and count the game shows (if in the log), labelled `possible`, with what it is based on. Decide how this sits with D-049's turn 2-3 rule (keep it simple; say why in D-050) and show the user a screenshot from your own build with the log replay before handing over.
4. Remove `available tribes (exact list)` from `not_in_log` if it is now read. Plan row **T-310**; D-050; README; same PR.

## Out of scope

Sign-in, upload code beyond the key list, the website, card names in other languages, the switch to turn pictures off, T-201.

## Acceptance criteria

- [ ] Research written with re-checkable counts; the answer for both questions is clear (found, where; or not in the log).
- [ ] If found: lobby tribes from the log used first everywhere, tested; real and synthetic fixtures updated and privacy-scanned.
- [ ] The hover card guesses the board of an opponent not met yet, labelled `possible`, tested, seen in your own build.
- [ ] Upload contract kept (`upload_contract.rs` green); reviews with no open CRITICAL or HIGH; PR merged through the hub.

## Red line

D-004, D-006, D-012, D-022 apply. Never print BattleTags, `GameAccountId`, player names or the user's email.

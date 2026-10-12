# Session 040: Inventory of everything the game's logs give (T-111)

Model: opus
Effort: high
Subagents: haiku for scans and counts; sonnet for the script and its tests

Work in your worktree **`C:\Users\andia\tavern-ledger\.claude\worktrees\040-log-inventory`** (branch `040-log-inventory`, created by the hub) and follow its `CLAUDE.md`. Chat in caveman mode; the user writes in Spanish, answer in English. Everything in the repo is in English. **Tier R1** (read-only research and a read-only script). No new dependency (Python standard library only).

**Two other sessions run at the same time:** 037 (sign-up follow-ups: `web/`, `supabase/`, `docs/legal/`, `crates/uploader/`) and 039 (lobby tribes from the log: `crates/bg-parser/`, `crates/tracker/`, the overlay, `tests/bg_log_builder.py`, `tools/make_fixture.py`, `docs/research/lobby-tribes-log.md`). To stay out of their way:
- Work only in your worktree. Never touch the main checkout or other worktrees.
- Your area: **new** files only: `docs/research/log-inventory.md`, `tools/log_inventory.py`, `tests/test_log_inventory.py`. No change to `crates/`, `web/`, `supabase/`, or other files in `tools/` and `tests/`.
- Shared files (`plan.md`, `README.md`, `DECISIONS.md`, `PROVENANCE.md`): only your task's row and notes. Your decision number is **D-051** (only if you decide something; a pure inventory may need none). Keep every changed text file LF (`git ls-files --eol` before committing).
- 039 researches the lobby's tribes and the per-player tribe and minion count in depth. Do not duplicate it: list those tags in the inventory and point to 039's work. If you find something about them first, send it to session 039 (`ListAgents`, name starting `039`) in one short message.
- Never stop or restart the user's desktop app; never start Docker.
- To catch up with `main`: `git fetch origin && git merge origin/main`. No rebase, never force-push.

Working rules (user's decisions):
- Decide technical matters yourself. Ask the user: new dependencies, anything about Blizzard's rules you are unsure of, and any change to the game's `log.config` (the app never writes it, T-106; the user edits it by hand if they want more log sections).
- **`gh` is not logged in inside task sessions.** When your branch is ready (tests green, `/code-review` done, no open CRITICAL or HIGH), push it and send the hub (`Hub prompts session 036`, find it with `ListAgents`, use its `[ref]`) a message with the branch, the PR title, a short PR body (summary, review results, test plan). The hub opens the PR, waits for CI and merges it.

## Why (the user, 2026-10-10)

"Get as much information as the game gives us, to see what we can and cannot use: comb through all the logs, and then we decide what goes in and what does not." Today the parser reads a chosen subset of `Power.log`. Nobody has listed everything the logs hold.

## What is on disk (hub's listing, names and sizes only)

`C:\Battle.net\Battle.net\Hearthstone\Logs\` holds 6 session folders `Hearthstone_<date>\` with: `Power.log` / `Power_old.log` (about 580 MB in total), `LoadingScreen(_old).log`, `Hearthstone.log`, `GameNetLogger.log`, `Net.log`, `Achievements(_old).log`, `Gameplay.log`, `LuckyDraw.log`, `Decks.log`, `FullScreenFX.log`, `ExceptionReporter.log`, `Login.log`, `Store.log`, `Downloader.log`, `Spells.log`, `Arena.log`. The user's `log.config` (`%LOCALAPPDATA%\Blizzard\Hearthstone\log.config`) enables `[Achievements] [Arena] [FullScreenFX] [LoadingScreen] [Power] [Decks] [Net]`.

## Privacy while you read (strict)

- Open files with shared read access (`[IO.File]::Open(..., 'Read', 'ReadWrite')` or Python `open` without locking); the game may be running.
- **Print, keep and commit only:** line kinds, tag names, enum values, entity types, card ids, counts, and the shape of a line with every value replaced by a placeholder. **Never** player names, BattleTags, `GameAccountId`, account or region ids, IP addresses, hostnames, session tokens, file paths with the Windows user name, chat, or the user's email. `Net.log`, `GameNetLogger.log`, `Login.log` and `Hearthstone.log` may hold network and account values: for those, keys and counts only.
- Nothing from the logs goes into `.local/` or the repo except what the rules above allow.

## Rules (D-004, D-006, absolute rule 1)

- Local log files only. No memory, no pixel reading, no network capture, no injection.
- For each item, say whether showing it would respect "only what the player could see on their screen" (D-004): a value about an opponent that the game never shows the player is **not usable**, even if the log has it (for example a hidden card in an opponent's hand or Bob's next offers). Mark these clearly.
- Clean-room: no code of HDT, Firestone, their simulators, HearthMirror or BobsBuddy is read or ported; public pages (for example the HearthSim docs on tags) may be read for names and ideas, with links recorded. Nomi's Kitchen: nothing at all.

## Effort and models per phase

| Phase | Effort | Model |
|-------|--------|-------|
| Read state, D-004, D-017, `parser-hslog.md`, `shop-and-apm.md` | low | Haiku |
| `tools/log_inventory.py` and its tests | medium | Sonnet |
| Run it on every log, read the results, classify | high | Opus |
| Write-up, review | medium | `code-reviewer` |

## At the start

1. Conversation title: **`#040 Log inventory`**.
2. `git fetch origin && git merge origin/main`.
3. Read D-004, D-006, D-017, D-045, D-046, `docs/research/parser-hslog.md`, `docs/research/shop-and-apm.md`, and what the parser reads today (`crates/bg-parser/src/`, skim).

## Task

1. **`tools/log_inventory.py`** (read-only, stdlib): for a logs folder, per file kind: line kinds (logger and method, e.g. `GameState.DebugPrintPower`, `PowerTaskList...`), and for `Power.log` every `tag=` name with counts, the entity types and zones it appears on, its value range (numbers) or value set (enums), first and last game phase it appears in (hero select, shop, combat, end), and Battlegrounds games vs other modes. Output JSON and a short text summary; refuses to print anything outside the allow-list above (test it with synthetic lines carrying fake names, ids and IPs). Tests in `tests/test_log_inventory.py` (unittest, as the other Python tests).
2. **Run it** on all six session folders (every file kind) and on Solo and Duos games apart.
3. **`docs/research/log-inventory.md`**: one table per file kind and, for `Power.log`, per group of tags (game, player and hero, minions, shop and Bob, combat, trinkets, quests, anomalies, Duos, cosmetics and other). Columns: item, what it means (checked against the raw lines, not guessed), when it appears, counts, **used today** (where in the parser), **possible use** (overlay, recap, stats, web), **allowed** (yes / no and why, D-004), and effort (low, medium, high). End with a short ranked list "worth adding next" for the user to choose from, and a list of log sections not enabled today that might hold more (with what each is said to hold, from public pages), for the user to decide.
4. Plan row **T-111** "Inventory of everything the game's logs give"; README; D-051 only if needed; same PR.

## Out of scope

Implementing any of the findings (the user chooses first), changing `log.config`, anything in `crates/`, `web/`, `supabase/`.

## Acceptance criteria

- [ ] Script and tests green; the allow-list refuses names, ids and IPs in tests.
- [ ] Every file kind and every `Power.log` tag in the user's logs listed with counts, meaning, current use, possible use, allowed or not, effort.
- [ ] A ranked "worth adding next" list and a "log sections to consider" list for the user.
- [ ] Nothing personal in the repo (re-check the diff for names, ids, IPs, paths with the user name).
- [ ] `/code-review` with no open CRITICAL or HIGH; PR merged through the hub.

## Red line

D-004, D-006, D-012, D-022 apply. Never print BattleTags, `GameAccountId`, player names, account ids, IPs or the user's email.

# Session 054: Exact lobby tribes, research (T-326)

Model: opus
Effort: high
Subagents: haiku for searches; opus for the Blizzard-rules and clean-room assessment

Work in your worktree **`C:\Users\andia\tavern-ledger\.claude\worktrees\054-lobby-tribes-research`** (branch `054-lobby-tribes-research`, created by the hub) and follow its `CLAUDE.md`. Chat in caveman mode; the user writes in Spanish, answer in English. Everything in the repo is in English. **Research only, no product code.** Output: a research doc, plan notes and a draft decision.

**Other sessions at the same time:** 052 (overlay data) and 053 (main window rework). Your area: `docs/research/`, the T-326 row and notes in `docs/plan/plan.md`, and a **draft** row in `DECISIONS.md` (number **D-063**, status "pending the user's OK"). Read-only scripts in your scratchpad, not in the repo (unless one is worth keeping in `tools/`, read-only, with tests). Keep changed text files LF.

Working rules:
- **Ask your questions through the hub** (`tavern-ledger-47`, find it with `ListAgents` and use its `[ref]`) as well as in your tab.
- **`gh` is not available in task sessions.** When done, push the branch and message the hub with the branch, PR title and body.
- Read raw logs with shared access; print counts, tag numbers and card ids only, never names, BattleTags or `GameAccountId`. Never run, attach to or read the memory of Hearthstone in this session; never download or run other trackers or their libraries.

## Why: the user's decision (2026-10-11)

The user wants the exact lobby tribes ("all the other apps have them; I won't go without them"). Asked how, they chose: **the log and public sources first; if those cannot give them, read-only memory reading**, as a new decision with the risk written down. Injection and client changes stay ruled out (D-006). Today the app works the tribes out from the tavern (D-050, T-310) and labels them inferred.

## Task

1. **The log, to the bottom.** On every raw game in `C:\Battle.net\Battle.net\Hearthstone\Logs` (all log files, not only `Power.log`): the game entity's unnamed tag 4730 (= 5 in all 11 games, session 040), any other tag or line that changes with the lobby's tribes (compare games with different known tribes: the user's own lobbies are in `games.jsonl` as inferred or entered tribes), `log.config` options not enabled yet (more verbose sections, other log names) and what they would add. Counts anyone can re-check.
2. **Public sources.** HearthstoneJSON (data already used), Blizzard's Hearthstone Game Data API (`/hearthstone/metadata`, cards with `minionTypeId`, the battlegrounds game mode), other public APIs or datasets that other apps document. Does any give the current lobby's tribes? (Probably not: they are per game.)
3. **How other apps do it.** From their public docs, issues, READMEs and changelogs (HDT, Firestone, others): which mechanism gives them the tribes (memory reading through HearthMirror or similar, the log, a server). Absolute rule 1: you may read what they say about **where** the data comes from; **never copy, port or paraphrase their code**; HearthMirror, HDT, Firestone, BobsBuddy are not used; Nomi's Kitchen is not read at all.
4. **If no log or public route exists: a memory-reading design for the user's decision**, written, not built:
   - What exactly would be read (only the lobby's tribes, and only what the game shows the player at hero select), read-only, from our own process, no injection, no writes, no hooks (D-006 stays).
   - Blizzard's terms and how they treat HDT and Firestone in practice (sources, dates); the risk to the user's account, said plainly.
   - Clean-room plan: how we would find the data ourselves without HearthMirror's code; how it breaks on patches and how the app says "unsupported version" instead of false data.
   - What changes in the docs, the site and the privacy text (absolute rules 3 and 8: no false "only reads logs" claims).
   - Draft **D-063** (pending the user's OK) that would amend D-004 for this one read.
5. **What else other apps take from memory** (the user's request, 2026-10-11: "see what else the other apps get, to understand what they do without trouble, and use it too"). From their public docs, feature lists, changelogs and issues only: a table of every data item HDT, Firestone and similar apps get from memory (for example: the player's and opponents' MMR or rating, opponents' names and BattleTags, the lobby's tribes, anomalies, quests and rewards, hero options, the player's collection, deck info, …). For each: where they say it comes from, whether the game shows it to the player on screen (D-004: we only show what the player could see), whether it touches other players' personal data (GDPR, absolute rule 6), how Blizzard has treated it (sources), and a recommendation: **use** (shown in game, low risk), **maybe** (needs a user call), or **no**. Fold the "use" items into the D-063 draft so the user approves one scope, not item by item.
6. Research doc `docs/research/lobby-tribes-exact.md`, plan row T-326 notes, draft D-063; one PR through the hub.

## Out of scope

Building a memory reader (only after the user approves D-063, in a later session), any UI change.

## Acceptance criteria

- [ ] Every log lead checked with counts; a clear yes or no on "the log has the tribes".
- [ ] Public sources listed with what each gives.
- [ ] How other apps get them, with sources, no code read beyond what rule 1 allows.
- [ ] If needed, the memory design and draft D-063 with the account risk said plainly.
- [ ] Hand-over to the hub with a short summary the hub can pass to the user on a phone.

## Red line

D-004, D-006, D-012, D-022 apply until the user approves otherwise. Never print BattleTags, `GameAccountId`, player names or the user's email.

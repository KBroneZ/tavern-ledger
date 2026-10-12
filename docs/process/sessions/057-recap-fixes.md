# Session 057: Recap fixes from the user's first game on the new build

Model: opus
Effort: medium
Subagents: haiku for searches; sonnet for code and reviews

Work in your worktree **`C:\Users\andia\tavern-ledger\.claude\worktrees\057-recap-fixes`** (branch `057-recap-fixes`, created by the hub) and follow its `CLAUDE.md`. Chat in caveman mode; the user writes in Spanish, answer in English. Everything in the repo is in English except the Spanish strings. **Tier R1** (display and local data; nothing new uploaded; a new download of card data in a second language is the same source and terms as today, D-038): TDD + `/code-review`.

**Other sessions at the same time:** 055 (memory reader: a new crate, `crates/tracker/` wiring for tribes and rating, a config flag and Tauri commands, the memory switch in the Settings tab, texts in `web/` and `docs/legal/`). To stay out of its way:
- Work only in your worktree. Never touch the main checkout or other worktrees.
- Your area: the main window of `crates/desktop/` (recap, shop section, `ui/strings/*`), `crates/desktop/src/card_data.rs`, `crates/cards/`, and in `crates/tracker/` only `shop.rs`, `recap.rs`, `stats.rs`, `stats/`. Do not touch the Settings memory switch, the new reader crate, `web/`, `docs/legal/`, `supabase/`, `crates/bg-parser/`.
- Shared files: only your tasks' rows and notes; decision number if needed **D-066**. Every changed text file LF (`git ls-files --eol`, index side).
- Never stop or restart the user's desktop app; test with your own build on a scratch copy of `%APPDATA%\TavernLedger\games.jsonl` (read only). After a build, `git checkout -- crates/desktop/gen/schemas`. CI does not build `crates/desktop` nor run its Node UI tests: run them yourself and paste counts.
- To catch up with `main`: `git fetch origin && git merge origin/main`. No rebase, never force-push.

Working rules: decide technical and design matters yourself (Lamplight glass D-052, compact style D-062, i18n D-065). Ask your questions through the hub (`tavern-ledger-47`, `ListAgents`, use its `[ref]`) as well as in your tab. `gh` is not available in task sessions: push the branch and message the hub with the branch, PR title and a short body.

## What the user saw (2026-10-11, Duos game, 3rd, 11 turns, parser revision 6)

Screenshot of the recap's "Shop and actions": `C:\Users\andia\AppData\Local\Temp\claude\C--Users-andia-tavern-ledger\ed9aef9c-b64b-452b-ba1e-0c26bce2f7a2\scratchpad\shop-recap.webp`.

1. **Totals thrown away for one missing turn.** In the saved report, turn 1 has no gold fields (`gold_spent`, `extra_gold`, `sell_gold`, … empty) while turns 2 to 11 are complete. The recap then shows "Gold spent —" and "not every turn · unknown" for extra gold, gold from selling, gold beyond the fixed income, gold on minions, gold on tavern spells. Fix: sum the turns that have the value and say how many ("138 · 10 of 11 turns"); "no data" only when no turn has it. Never invent turn 1 (unknown is not zero). Same rule in the stats and curious stats (count games and turns honestly). Also find out **why turn 1 has no gold** in the log (first shop starts before the tags arrive? check the raw log of that game with shared access; counts and tag names only) and fix the parser side only if it is in `tracker` (the parser crate is out of your area: if the fix belongs there, describe it for a follow-up).
2. **Layout:** the section's left edge is cut off ("hop and actions", "PM (logged)"); the numbers are spread over the full window width with large gaps; the long explanation paragraph overflows to the right. Fix: proper padding, a compact grid of stat tiles with a sensible max width, explanations wrapped (and behind an "About this" fold, 053's pattern). Check every recap section for the same clipping at the window widths the user uses (their screen is 3840x2160 at 150%).
3. **Card and hero names follow the app's language.** Today names come from the game's log (`card_names`, in the client's language, D-017), so with the app in English the names stay Spanish. Use the card data (HearthstoneJSON, D-038) in the app's language (enUS, esES) for names everywhere in the window and overlay, falling back to the log's name when an id is missing. Cache both languages; switching language in Settings changes names at once.

## Acceptance criteria

- [ ] Totals over known turns with "N of M turns"; tests with a game missing turn 1 and one with no data.
- [ ] No clipped edges or overflow in the recap; compact tiles.
- [ ] Names follow the app language, with fallback; tests.
- [ ] `cargo test --workspace`, desktop Node UI tests, release build pass locally (paste counts); `/code-review` with no open CRITICAL or HIGH; branch handed to the hub.

## Red line

D-004 (as amended by D-063), D-006, D-012, D-022 apply. Never print BattleTags, `GameAccountId`, player names or the user's email.

# Session 087: Dark Gifts on board minions, research and simulator (T-527)

Model: opus
Effort: high
Subagents: haiku for searches; sonnet for code and the review

Work in your worktree **`C:\Users\andia\tavern-ledger\.claude\worktrees\087-dark-gifts`** (branch `087-dark-gifts`, created by the hub) and follow its `CLAUDE.md`. Chat in caveman mode; the user writes in Spanish, answer in English. Everything in the repo is in English. **Tier R1**: TDD + `/code-review`.

**Other sessions at the same time:** 085 (home page concepts, `docs/brand/home/`) and 086 (combat replay: `crates/bg-parser/`, `crates/tracker/`, `crates/desktop/`, maybe `supabase/functions/upload-game/` and `web/src/lib/game/`). **Your area:** `crates/combat-sim/`, `crates/cards/`, `crates/sim-wasm/` (and the rebuilt wasm and accuracy data in `web/` only through their tools, D-090, D-091), plus a research note in `docs/research/`. **Do not change `crates/bg-parser/`, `crates/tracker/` or `crates/desktop/`** while 086 runs: reading Dark Gifts from the log and showing them in the app and on the website is the next session's job (T-528), built on your research. Shared files: only your rows and notes; decision number **D-100**. Every changed text file LF (`git ls-files --eol`, index side). To catch up: `git fetch origin && git merge origin/main`; no rebase, never force-push.

- **Before you hand over, run the tests from a clean clone of your pushed branch** (`cargo test --locked --workspace --exclude desktop`, plus the web build and tests if you rebuild the wasm).
- Do not build or run the user's desktop app. The user may be busy: do not wait on them. Push the branch and message the hub (`tavern-ledger-2b`, `ListAgents`, use its `[ref]`) with the branch, PR title and a short PR body.

## Clean room (absolute rule 1)

Never read or copy the code of HDT, Bob's Buddy, Firestone, `@firestone-hs/simulate-bgs-battle`, HearthMirror or Nomi's Kitchen. Their public docs and articles are fine; MIT or Apache code only with credit in `PROVENANCE.md`. Rules from card texts (HearthstoneJSON), public patch notes and the user's own logs.

## Why

The user (2026-10-12): this season adds **Dark Gifts** ("Don siniestro" in esES): existing minions that carry an extra special effect. They must show wherever a minion shows (recap, overlay, website boards), and the simulator must play them: "it adds complexity, but we did not come here to stop halfway". Today the parser reads only the Dark Gift named by each **offer** (`DARK_GIFT_ENTITY`, D-058: 105 of 105 offers name their gift) and the stats list gifts taken (D-060, D-062); a gifted minion on the board is not marked, and the simulator does not know gifts, so a gifted minion plays as the plain card (wrong odds, silently: a silent failure is a defect).

## Task

1. **Research (`docs/research/dark-gifts.md`):** from the card data, the full list of this season's Dark Gifts: id, English and esES name, text, and what they do in combat (start of combat, deathrattle, on attack, aura, stat changes) versus recruit-phase only. From the user's logs (read-only, tag names, counts and card ids only): how a gifted minion appears on the board and in combat copies (enchantment entity attached, `DARK_GIFT_ENTITY` or other tags, on the minion or on an enchantment), whether the gift survives golden triples and combat copies, and whether opponents' gifted minions are visible in what the player saw (D-054). Say exactly what the parser (T-528) must read, with counts from the logs.
2. **Simulator:** a minion can carry a Dark Gift in the engine's board model and the JSON the wasm takes. Model every gift with a combat effect, each with a test (and golden if it differs); recruit-only gifts change nothing in combat and say so. A gift the engine does not know blocks the odds or is approximate by the same rules as cards (D-088, D-097), never ignored silently. Harness: say how many of the user's combats had a gifted minion (from your research reading; the harness can only use gifts once T-528 records them, so note it).
3. **Website simulator:** gifts selectable on a minion in the editor's data shape (the UI work can wait for T-526; at least the engine and its JSON accept them), wasm rebuilt with the tool.
4. Plan rows: T-527 (this), T-528 (parser, app and website display of gifts on minions, after 086 merges) with your research's instructions, D-100, README.

## Acceptance criteria

- [ ] Research note with the list of gifts and how the log carries them, with counts.
- [ ] Engine models every combat-relevant gift, tested; unknown gifts block or approximate, never silent.
- [ ] Wasm accepts gifts; rebuilt by the tool; web build green.
- [ ] Clean-clone tests green (paste counts); clippy, fmt; files under 800 lines; `/code-review` with no open CRITICAL or HIGH; branch handed to the hub.

## Red line

Clean room above. D-004 (as amended by D-063), D-006, D-054. Never print BattleTags, `GameAccountId`, player names or the user's email.

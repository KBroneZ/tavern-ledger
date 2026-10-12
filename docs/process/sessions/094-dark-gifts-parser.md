# Session 094: Dark Gifts on board minions, everywhere they show (T-528)

Model: opus
Effort: high
Subagents: haiku for searches; sonnet for code and the review

Work in your worktree **`C:\Users\andia\tavern-ledger\.claude\worktrees\094-dark-gifts-parser`** (branch `094-dark-gifts-parser`, created by the hub) and follow its `CLAUDE.md`. Chat in caveman mode; the user writes in Spanish, answer in English. Everything in the repo is in English. **Tier R1, R2 if the upload format changes** (then also `/security-review`).

**Other sessions at the same time:** 092 (website home page) and 093 (app settings, logo and strings in `crates/desktop/`). Your area: `crates/bg-parser/`, `crates/tracker/` (filling `gifts` in `odds.rs` and the reports), the gift marks on minion pictures in `crates/desktop/ui/` (agree with 093 through the hub before editing a file it touches), `supabase/functions/upload-game/` (report keys, validator version) and the website's game viewer and simulator data (`web/src/lib/game/`, `web/src/lib/sim/`). Decision **D-107**. Every changed text file LF. To catch up: `git fetch origin && git merge origin/main`; no rebase, never force-push.

- Do not build or run the user's desktop app; the hub rebuilds and reparses if `PARSER_REVISION` changes (say so). Never show a test build over the user's game.
- The user may be busy: do not wait on them. Push the branch and message the hub (`tavern-ledger-2b`, `ListAgents`, use its `[ref]`) with the branch, PR title, a short PR body and whether a reparse is needed.

## Clean room (absolute rule 1)

Never read or copy HDT, Bob's Buddy, Firestone, HearthMirror code; never read Nomi's Kitchen.

## Why

The user (2026-10-12): this season's Dark Gifts must show wherever a minion shows and the simulator must use them, "we did not come here to stop halfway". Session 087 (D-100) did the research and the engine: **read `docs/research/dark-gifts.md`, section 5 is your brief** (a gift on a board minion is an enchantment in PLAY attached to it; `HAS_DARK_GIFT` marks the minion; the enchantment-to-gift table, stat-holder enchantments to ignore, `EDR_100t13e` for Harpy's Talons; gifts survive combat copies, golden triples and copies; opponents' gifts are visible to the player). The engine already takes `Minion::gifts`; `crates/tracker/src/odds.rs` passes `Vec::new()` until you fill it.

## Task

1. **Parser:** every board minion (own and opponents', as the player saw them; combat boards and recruit boards) carries its gifts (gift card ids) from the log, per the research. Tests on synthetic and the user's fixtures; bump `PARSER_REVISION` if saved reports change.
2. **Odds:** fill `gifts` in `odds.rs`, so the overlay odds and the harness use them; rerun `combat-validate` on the user's history and report accuracy and blocked share before and after (counts and card ids only); regenerate the accuracy export with its tool.
3. **Show them:** a small gift mark on every minion picture that has one (app recap boards, combat replay if merged, overlay boards, website game viewer), with the gift's name and text as a tooltip, EN and ES; "few words, key first" (D-067, D-072).
4. **Upload and website:** report keys and validator version if the report changed; the website viewer and the simulator's game loader pass gifts through (the simulator's gift editing UI waits for T-526).
5. D-107, plan row T-528 done, README.

## Acceptance criteria

- [ ] Gifts on every board minion from the log, tested; revision bump stated.
- [ ] Odds use gifts; accuracy before and after.
- [ ] Gift marks in the app, overlay and website viewer, EN and ES.
- [ ] Clean-clone tests green (paste counts: cargo, Deno, web); clippy, fmt; files under 800 lines; `/code-review` with no open CRITICAL or HIGH; branch handed to the hub.

## Red line

Clean room above. D-004 (as amended by D-063), D-006, D-054. Never print BattleTags, `GameAccountId`, player names, keys or the user's email.

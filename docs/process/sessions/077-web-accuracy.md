# Session 077: Simulator accuracy page and a "what's new" line (T-607)

Model: opus
Effort: medium
Subagents: haiku for searches; sonnet for code and the review

Work in your worktree **`C:\Users\andia\tavern-ledger\.claude\worktrees\077-web-accuracy`** (branch `077-web-accuracy`, created by the hub) and follow its `CLAUDE.md`. Chat in caveman mode; the user writes in Spanish, answer in English. Everything in the repo is in English. **Tier R1** (static page, counts the user agreed to publish): TDD + `/code-review`.

**Other sessions at the same time:** 076 (the website simulator: the WebAssembly build, `/simulator/`, the CSP). Your area: a new page `web/src/pages/[...lang]/accuracy.astro` and its code (`web/src/lib/accuracy/` or similar), one static data file, a short "what's new" block on the home page, plus docs. Do **not** touch the CSP, `config.ts`, `astro.config.mjs` or `crates/` (read them). Shared design-system files (`web/src/components/data/`, `web/src/lib/view/`, `data.css`): add, do not rewrite. Shared files (`plan.md`, `README.md`, `DECISIONS.md`, the site menu): only your rows and notes; decision number if needed **D-091** (076 has D-090). Every changed text file LF (`git ls-files --eol`, index side).

- To catch up: `git fetch origin && git merge origin/main`. No rebase, never force-push. Stop any dev server or `astro preview` you start.
- **Before you hand over, run the web tests and build checks from a clean clone of your pushed branch**; check `git ls-files` for every new file (`.gitignore` ignores `bin/` and `*.log`).
- Do not build or run the user's desktop app. The user may be busy: do not wait on them; questions go in your hand-over with a recommendation. `gh` is not available: push the branch and message the hub (`tavern-ledger-2b`, `ListAgents`, use its `[ref]`) with the branch, PR title and a short PR body. **Merging deploys the site:** afterwards check every page answers 200, the CSP is unchanged and `/accuracy/` shows the numbers; tell the hub.

## Why

D-089: the accuracy page is **public** and may show the counts from the user's own combats, **only the fields of the export** (D-088, `docs/research/combat-simulator.md` section 7.6). And no email for the community hero stats (T-507, D-085): a short line on the site says what is new instead.

## Task

1. **The numbers.** Run `combat-validate --history … --cards … --export` (and `--logs` if the logs are there) locally on the user's own history (`%APPDATA%\TavernLedger\games.jsonl`, read only) with the current engine, and commit the resulting JSON as the page's data (e.g. `web/src/data/accuracy.json`). Check it holds only the export's keys (no names, BattleTags, account ids, seats, heroes, games or rounds; card ids are fine). Document in the design doc how to refresh it after a card round (one command, then a PR).
2. **The page** (`/accuracy/`, `/es/accuracy/`, indexable, English and Spanish through the D-086 tables), built at build time from the JSON (no runtime fetch): per mode, the combats scored with the most-likely-right count and mean probability of the real result, **always next to the blocked share** ("no odds in 13% of Duos combats: a card is not modelled"; never one without the other), the calibration buckets as a small chart from the design system, the Brier score in Advanced, the engine version and date, the cards that block most (names from the card data, ids in Advanced). One short paragraph on what is measured and its limits (the user's own combats only, start of combat off, boards seen at the first attack). No invented figures; a test reads the committed JSON and fails on a missing key or a wrong `format`.
3. **What's new.** A small dated block on the home page (both languages): community hero stats with the opt-in on the account page, the simulator accuracy page, and (only once 076 is merged; leave a line the hub can add later otherwise) the simulator. Short, no marketing claims (rule 8).
4. Link `/accuracy/` from the site menu or footer and from the overlay docs if there is a fitting place. Plan row T-607, D-091 if needed, README.

## Acceptance criteria

- [ ] `/accuracy/` and `/es/accuracy/` built from the committed export, blocked share always beside accuracy.
- [ ] The JSON holds only the export's keys (test).
- [ ] "What's new" block in both languages.
- [ ] Clean-clone tests green (paste counts); `/code-review` with no open CRITICAL or HIGH; branch handed to the hub.

## Red line

D-004 (as amended by D-063), D-006, D-054 apply. Never print BattleTags, `GameAccountId`, player names or the user's email. No analytics or third-party scripts; no CSP change.

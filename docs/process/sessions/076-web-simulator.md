# Session 076: Interactive combat simulator on the website (T-606)

Model: opus
Effort: high (WebAssembly build, CSP change, untrusted input), medium for the page
Subagents: haiku for searches; sonnet for code; opus for the security review

Work in your worktree **`C:\Users\andia\tavern-ledger\.claude\worktrees\076-web-simulator`** (branch `076-web-simulator`, created by the hub) and follow its `CLAUDE.md`. Chat in caveman mode; the user writes in Spanish, answer in English. Everything in the repo is in English. **Tier R2** (CSP change, new build step, untrusted files): TDD + `/code-review` + `/security-review`.

**Other sessions at the same time:** 077 (accuracy page, a new page under `web/` and the home page). Your area: a thin WebAssembly wrapper for `crates/combat-sim` (a new crate or module; the engine's own behaviour does not change), the simulator page and its code under `web/` (`web/src/pages/[...lang]/simulator.astro`, `web/src/lib/sim/` or similar), the CSP in `web/src/lib/config.ts` and `web/astro.config.mjs`, the build steps, plus docs. Shared design-system files (`web/src/components/data/`, `web/src/lib/view/`, `web/src/lib/data/`, `data.css`): add, do not rewrite. Shared files (`plan.md`, `README.md`, `DECISIONS.md`, the site menu): only your rows and notes; decision number if needed **D-090** (077 has D-091). Every changed text file LF (`git ls-files --eol`, index side).

- To catch up: `git fetch origin && git merge origin/main`. No rebase, never force-push. Stop any dev server or `astro preview` you start (a leftover one locks the worktree folder).
- **Before you hand over, run the tests from a clean clone of your pushed branch** (`cargo test --locked --workspace --exclude desktop`, the web tests and the build checks); `.gitignore` ignores `bin/` and `*.log`, so check `git ls-files` for every new file. CI runs one `check` job (gitleaks, Rust fmt and tests, lychee `--offline`, web tests): a committed `.wasm` or a build step must work there too.
- Do not build or run the user's desktop app (the hub does). Never show a test build's overlay.
- The user may be busy: do not wait on them. Questions go in your hand-over with a recommendation. `gh` is not available: push the branch and message the hub (`tavern-ledger-2b`, `ListAgents`, use its `[ref]`) with the branch, PR title and a short PR body. **Merging deploys the site:** afterwards check every page answers 200, the CSP is as you changed it and no wider, and the simulator page runs a combat in a real browser; tell the hub.

## Clean room (absolute rule 1)

Never read or copy the code of HDT, Bob's Buddy, Firestone, `@firestone-hs/simulate-bgs-battle`, HearthMirror or Nomi's Kitchen, nor the web simulators built on them. Public docs and articles are fine; MIT or Apache code only with credit in `PROVENANCE.md`. The UI is ours; rules come only from our own engine.

## Why

The user (2026-10-11): "an interactive combat simulator on the website, using old boards or old pairings to try things", "like the deck tracker's". D-089 settled how: **the Rust engine compiled to WebAssembly and run in the browser**, `'wasm-unsafe-eval'` added to `script-src` **only on the simulator page** if the site can set the CSP per page (else site-wide, and say so), and **only the signed-in player's own games** at first. Read D-081, D-084, D-087, D-088, D-089, D-079 (the game viewer reads the report files), D-074 (card pictures), and `docs/research/combat-simulator.md` sections 2, 3, 8.

## Task

1. **WebAssembly build.** Compile `crates/combat-sim` to `wasm32-unknown-unknown` with the smallest wrapper that works: a hand-written C ABI (JSON in, JSON out through linear memory) needs no new dependency; `wasm-bindgen` is one new build dependency (R2, `PROVENANCE.md`). Pick one and say why. Same seed and trials as the app, so the same boards give the same odds as the overlay (a test proves it on a fixture: the Rust result and the browser result match). Decide whether the `.wasm` is built in CI or committed with a check that it matches the source; CI must not need a toolchain it lacks without adding it. Card facts the engine needs (`crates/cards`, saved format 5): ship only what the simulator needs, as a static file from our own origin.
2. **CSP.** Per page if the site's headers allow it (Cloudflare Pages `_headers` per path, or what `config.ts` builds), else site-wide. Nothing else in the CSP changes. A build test checks the simulator page has `'wasm-unsafe-eval'` and every other page does not (or, if site-wide, says so in D-090).
3. **The page** (`/simulator/`, `/es/simulator/`, `noindex`, English and Spanish through the D-086 tables):
   - **Load:** pick one of your games, then a round: your board and the opponent's board from that combat (Duos: both teams' boards, in the game's order, as D-087). Data comes from the player's own report files exactly as `/game/` reads them (D-079: size caps, untrusted, refuse a file naming another game). Never a `SETASIDE` board (D-054).
   - **Edit:** remove, reorder, change Attack and Health, toggle golden and the keywords the engine knows, add a minion by name from the card data (search box). Show which cards are "not modelled" and refuse to run with them, naming them, as the app does; mark approximate cards "≈" with the reason.
   - **Run:** win / tie / lose shares, damage both ways (median and 10th to 90th percentile, after the cap), trials and engine version; Simple by default, Advanced folds the rest. Runs off the main thread (Web Worker from our own origin) so the page never freezes.
   - **Without signing in:** an empty board editor still works (no data read). Signed in: their games only.
   - Card pictures from HearthstoneJSON as on `/game/` (D-074), no white edges; compact and visual, the user's taste (key numbers first, details folded, no walls of text, no cheap native scrollbars).
4. Tests: wrapper unit tests, a fixture round-trip (report file → boards → engine result), the CSP build test, the strings test for both languages. Plan row T-606, D-090, design doc section 8 updated, README.

## Acceptance criteria

- [ ] Same boards give the same odds in the browser and in Rust (test).
- [ ] CSP widened only where D-089 allows; build test proves it.
- [ ] Load a round from one of your games, edit it, run it; Duos too; not-modelled cards refused and named.
- [ ] Works signed out with an empty editor; never reads another account's data.
- [ ] English and Spanish; clean-clone tests green (paste counts); `/code-review` and `/security-review` with no open CRITICAL or HIGH; branch handed to the hub.

## Red line

Clean room above. D-004 (as amended by D-063), D-006, D-054 apply: only boards the player saw. Never print BattleTags, `GameAccountId`, player names or the user's email. No analytics or third-party scripts; no new origin in the CSP.

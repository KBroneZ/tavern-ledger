# Session 068: Combat simulator, design and engine core (F6, T-601 to T-603)

Model: opus
Effort: high
Subagents: haiku for searches; sonnet for code; opus for architecture and the reviews

Work in your worktree **`C:\Users\andia\tavern-ledger\.claude\worktrees\068-combat-sim`** (branch `068-combat-sim`, created by the hub) and follow its `CLAUDE.md`. Chat in caveman mode; the user writes in Spanish, answer in English. Everything in the repo is in English. **Tier R1** for the engine (TDD + `/code-review`); any new dependency is R2 (`/security-review`, `PROVENANCE.md`).

**Other sessions at the same time:** 065 and 066 (website, `web/`). Your area: a **new crate `crates/combat-sim/`**, a validation tool under `tools/` or the crate's `examples/`, docs (`docs/research/combat-simulator.md`), plan rows. Nothing in `web/`, `supabase/`, `crates/desktop/`; read `crates/bg-parser/` and `crates/tracker/`, change them only for a small, tested export the harness needs (tell the hub first). Shared files: only your rows and notes; decision number if needed **D-081**. Every changed text file LF (`git ls-files --eol`, index side). To catch up: `git fetch origin && git merge origin/main`; no rebase, never force-push.

Working rules: the user is asleep; do not wait on them. Put the questions only they can answer (below) in your hand-over, each with a recommendation. `gh` is not available: push the branch and message the hub (`tavern-ledger-2b`, `ListAgents`, use its `[ref]`) with the branch, PR title and a short PR body.

## What the user asked (2026-10-11)

1. "A combat simulator like Deck Tracker's" in the app: before each combat, the odds to win, tie or lose (and the damage), from the boards the player has seen.
2. "An interactive combat simulator on the website, able to use old boards or old pairings to try things."

## Clean room (absolute rule 1; the hub has told the user, who accepted)

- **Never read or copy** the code of HDT, Bob's Buddy, Firestone or `@firestone-hs/simulate-bgs-battle`, HearthMirror, or Nomi's Kitchen. Not for ideas either: their code is off limits; their public docs and articles are fine.
- Code under a compatible licence (MIT, Apache-2.0) may be read and reused with credit in `PROVENANCE.md` (for example twanvl's `hearthstone-battlegrounds-simulator`, MIT, 2020: rules are old, so ideas more than code). Search GitHub for newer simulators and check each licence before reading code; record what you looked at.
- Game rules come from card texts (HearthstoneJSON, D-038 terms: data at run time or in a cache, never bundled art), public patch notes and wiki-style descriptions, and above all **the user's own logs**, which record real combats.

## Ground truth we already have

The parser reads each combat's boards and results (D-055, `crates/bg-parser`; T-215 combat replay was planned on the same data): the player's board and the opponent's board entering combat (with golden and keywords), the result and the damage. That is a free test set: simulate each logged combat many times and score the real outcome against the predicted distribution (calibration, as `docs/research/implementation-notes.md` "Combat odds" describes), and replay the logged attack order to find the first rule that differs.

## Task

1. **Design doc** `docs/research/combat-simulator.md`: the combat rules (attack order and who goes first, target choice, taunt, divine shield, reborn, poisonous/venomous, windfury and mega-windfury, cleave, deathrattles and their order, start-of-combat, avenge, "after attack", summon limits, hero powers and trinkets that act in combat, Duos combat order), how card effects are modelled (a registry keyed by card id, golden variants, anything not modelled marked so the result says "not modelled" instead of a number), randomness and trials, performance targets (for example 10 000 trials in under 200 ms on the user's PC), and how the same engine serves both:
   - **App:** a Rust crate used by `crates/desktop` (the overlay shows odds during the recruit phase from the opponent's last seen board; D-004 allows odds from boards the player saw, see the plan note on `GAME_SEED`).
   - **Website:** options for running it in the browser (Rust → WebAssembly needs `'wasm-unsafe-eval'` in the CSP: a user decision), a TypeScript port (two engines to keep in step), or a server function (cost, rate limits). Recommend one.
2. **Engine core** in `crates/combat-sim/`: board and minion model, the combat loop with the keywords above, deathrattle and summon handling, a deterministic seedable RNG for tests, a trial runner with win/tie/loss and damage distribution, and the "not modelled" flag. TDD with small hand-built boards whose outcome is certain.
3. **Card coverage:** start with the cards seen in the user's logged combats (count them; tag names and card ids only), the most common first; list which are modelled.
4. **Validation harness:** run every logged combat in the user's history through the engine; report accuracy (the real result's predicted probability, calibration buckets, "not modelled" share), first differing rule for the misses. Synthetic fixtures in the repo; the user's own logs only read locally, never committed (fixtures rule).
5. **Plan:** a new phase **F6 — Combat simulator** with rows T-601 (design), T-602 (engine core), T-603 (validation harness), then T-604 (card coverage rounds), T-605 (overlay odds in the app), T-606 (web simulator: load an old board or pairing, edit, run), T-607 (accuracy page), with dependencies. Mark what you finished.

## Questions for the user (hand-over, each with a recommendation)

- How the website runs the engine (WebAssembly and the CSP change, a TypeScript port, or a server function).
- Whether the website's simulator may load other public players' boards, or only your own games.

## Acceptance criteria

- [ ] Design doc and plan rows; clean-room record (what was read, licences).
- [ ] Engine core with tests; `cargo test --workspace` green (paste counts); clippy, fmt.
- [ ] Harness run on the user's logged combats with an accuracy report (numbers only, no player names).
- [ ] `/code-review` with no open CRITICAL or HIGH; branch handed to the hub with the questions.

## Red line

Clean room above. D-004 (as amended by D-063), D-006 apply: odds only from boards the player saw; no memory reading beyond D-063, no injection. Never print BattleTags, `GameAccountId`, player names or the user's email.

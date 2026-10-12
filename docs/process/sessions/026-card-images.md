# Session 026: Card and hero images (T-304)

Model: opus for the source and licence research, sonnet for the code
Effort: high for the research, medium for the code
Subagents: haiku for searches; sonnet for code and reviews

Work in your worktree **`C:\Users\andia\tavern-ledger\.claude\worktrees\026-card-images`** (branch `026-card-images`, created by the hub) and follow its `CLAUDE.md`. Chat in caveman mode; the user writes in Spanish, answer in English. Everything in the repo is in English. **Tier R2** (network fetches, new data source, maybe a dependency): TDD + `/code-review` + `/security-review`.

**Other sessions may run at the same time** (028 website, 029 email sender). To stay out of their way:
- Work only in your worktree. Never touch the main checkout or other worktrees.
- Your area: `crates/desktop` (overlay and main window: where images are shown), `crates/tracker` only for a card-id lookup if needed, and a new module or crate for the card data and image cache. Do not touch `web/`, `supabase/`, `docs/legal/`.
- Shared files (`plan.md`, `README.md`, `DECISIONS.md`, `PROVENANCE.md`): only your task's row and notes. Your decision number is **D-038**.
- To catch up with `main`: `git fetch origin && git merge origin/main`. No rebase, never force-push.
- **You are the only session that may stop and restart the desktop app** the user has running: after your PR is merged, build `main` in the main checkout (`cargo build --release -p desktop`), stop the old app and `Start-Process` the new one. That is the only thing you may do in the main checkout.

Working rules (user's decisions):
- Decide technical matters yourself. Ask the user only what is theirs: which image source to use and its terms, new dependencies, accounts or API keys, anything that costs money.
- **You merge your own PRs**, once CI is green and no review has open CRITICAL or HIGH findings: `gh pr merge <n> --squash --delete-branch`.
- The user often works remotely: never ask them to do something on the PC.
- `gh` is logged in through `GH_TOKEN`; never print it. Long-running things go in a separate window (`Start-Process`).
- Python: `C:\Users\andia\tavern-ledger\.venv\Scripts\python`.

**Red line (CLAUDE.md rules 1, 3 and 5; D-004, D-006, D-032):** never read, copy or extract images or data from the game's own files or memory. Card images only under Blizzard's Fan Content Policy, always with the unofficial fan-project notice. Card data or images under "All Rights Reserved" are never committed to the repo: fetched at run time and cached on the user's PC only. The overlay still shows only what the player could see on their own screen.

## Effort and models per phase

| Phase | Effort | Model |
|-------|--------|-------|
| Read and check state | low | Haiku |
| Source and licence research, options for the user | high | Opus |
| Code (tests first) | medium | Sonnet |
| Review | medium | `code-reviewer`, `security-reviewer` |

## At the start

1. Conversation title: **`#026 Card images`**.
2. `git fetch origin && git merge origin/main`.
3. Read `README.md`, `docs/plan/plan.md` (T-109, T-201, T-301, T-302, T-304 and "Future ideas"), `docs/decisions/DECISIONS.md` (D-002, D-003, D-004, D-006, D-010, D-012, D-032, D-033) and `PROVENANCE.md`. Look at how the overlay and the recap show cards today (raw card ids and attack/health).
4. Green before touching anything: Rust (workspace and `desktop`), clippy, fmt, Python.

## Task

1. **Research (no code).** Where can card names, card art and hero portraits come from, and under what terms? At least: Blizzard's Fan Content Policy (what it allows, the non-commercial condition against D-003's optional paid extras: images only in free features?), HearthstoneJSON (card data and its art service: terms, rate limits, attribution), Blizzard's official Hearthstone Game Data API (needs a Battle.net developer client and its terms: that is an account and a secret, so the user's call), and anything else credible. Read the terms on the primary pages. For each: what is allowed, attribution, caching, rate limits, cost, risk if it goes away. Write `docs/research/card-images.md`.
2. **Ask the user** with a short comparison and your recommendation. Do not fetch any image before they choose. Record the choice as D-038.
3. **Build it** (tests first): a card-data and image client with a timeout, an identifiable User-Agent, few requests (one data file, images only when needed), response validation, a local cache on the user's PC with a size cap, and a fallback to the card id when anything fails (never a wrong picture: an unknown id shows the id and "unknown"). Then show images in the overlay boards, the recap and the hero names in the main window. Keep the source labels (T-109) on every value and readable themes (D-033 contrast test).
4. Check it with the log replay (T-D02); describe what you saw in the PR (no real names).
5. Plan (T-304), README, D-038 and `PROVENANCE.md` (the source, its terms, what is cached, never committed), in the same PR.
6. After merging: move the running desktop app to the new `main` build.

## Out of scope

- The website (session 028), the email sender (session 029), the desktop redesign (session 027, which starts after you merge).

## Acceptance criteria

- [ ] Research file with primary sources; the user chose the source; D-038 recorded.
- [ ] Nothing from the game's files; no image or card data committed.
- [ ] Client with timeout, User-Agent, validation, capped cache and fallback; tests for each failure path.
- [ ] Images in overlay, recap and hero names; source labels and theme contrast still pass.
- [ ] `/code-review` and `/security-review` with no open CRITICAL or HIGH; CI green; PR merged by you; app left running from `main`.

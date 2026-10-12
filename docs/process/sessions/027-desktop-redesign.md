# Session 027: Desktop app and overlay redesign (T-305)

Model: sonnet
Effort: medium (high for the design directions)
Subagents: haiku for searches; sonnet for code and reviews

**Start only after session 026 (card images) has merged**: both change `crates/desktop/ui/`.

Work in your worktree **`C:\Users\andia\tavern-ledger\.claude\worktrees\027-desktop-redesign`** (branch `027-desktop-redesign`, created by the hub) and follow its `CLAUDE.md`. Chat in caveman mode; the user writes in Spanish, answer in English. Everything in the repo is in English. **Tier R1** (local UI). A new dependency (a UI framework, a font, an icon set) makes it R2: ask the user first and record it in `PROVENANCE.md`.

**Other sessions may run at the same time** (028 website, 029 email sender). To stay out of their way:
- Work only in your worktree. Never touch the main checkout or other worktrees.
- Your area: `crates/desktop/ui/` (main window and overlay look) and the Rust side of `crates/desktop` only where the UI needs it. Do not touch `web/`, `supabase/`, `docs/legal/`.
- Shared files (`plan.md`, `README.md`, `DECISIONS.md`, `PROVENANCE.md`): only your task's row and notes. Your decision number is **D-039**.
- To catch up with `main`: `git fetch origin && git merge origin/main`. No rebase, never force-push.
- **You are the only session that may stop and restart the desktop app** the user has running: after your PR is merged, build `main` in the main checkout (`cargo build --release -p desktop`), stop the old app and `Start-Process` the new one.

Working rules (user's decisions):
- Decide technical matters yourself. Ask the user: the design direction, new dependencies, fonts or assets with unclear licences.
- **You merge your own PRs**, once CI is green and no review has open CRITICAL or HIGH findings: `gh pr merge <n> --squash --delete-branch`.
- The user often works remotely: never ask them to do something on the PC. Show the directions as an artifact page or screenshots they can open from the phone.
- `gh` is logged in through `GH_TOKEN`; never print it. Long-running things go in a separate window (`Start-Process`).

**Red line (D-004, D-006, D-032, D-033):** the overlay stays a separate window that only draws; nothing changes in what it shows (only what the player could see; opponents by hero only). Keep: source labels (T-109) on every value, no meaning by colour alone, the contrast test for every theme, the unofficial fan-project notice, card images only as session 026 set them up.

## Effort and models per phase

| Phase | Effort | Model |
|-------|--------|-------|
| Read and check state | low | Haiku |
| Research (GitHub repos and skills for UIs) | medium | Haiku for search, Sonnet to judge |
| Two or three directions for the user | high | Sonnet |
| Build the chosen one (tests first where logic changes) | medium | Sonnet |
| Review | medium | `code-reviewer` |

## At the start

1. Conversation title: **`#027 Desktop redesign`**.
2. `git fetch origin && git merge origin/main`.
3. Read `README.md`, `docs/plan/plan.md` (T-109, T-301, T-302, T-304, T-305), `docs/decisions/DECISIONS.md` (D-032, D-033, D-038) and everything in `crates/desktop/ui/`.
4. Green before touching anything: Rust (workspace and `desktop`), clippy, fmt, Python.

## Task

1. **Research.** The user wants a drastic redesign, "from GitHub UI skills and repos": look for open-source desktop or game-companion UIs, design systems and agent skills for interface design. Only MIT/Apache-style code or plain ideas; note each source and licence. Short notes in `docs/research/desktop-ui.md`.
2. **Directions.** Two or three distinct directions (layout, type, colour, how the history, recap and overlay look), each as a mockup page or screenshots, with what it costs (new dependency or not). **Ask the user to choose.** Record the choice as D-039.
3. **Build** the chosen direction across the main window (history, recap, upload, settings) and the overlay themes. Keep every behaviour; tests stay green; the theme contrast test covers any new theme.
4. Check it with the log replay (T-D02) and at 1280×720 and 1920×1080; describe it in the PR.
5. Plan (T-305), README, D-039 and `PROVENANCE.md` if anything new came in, in the same PR.
6. After merging: move the running desktop app to the new `main` build.

## Acceptance criteria

- [ ] Research notes with licences; the user chose a direction; D-039 recorded.
- [ ] New look in the main window and the overlay; no behaviour lost.
- [ ] Source labels, no colour-only meaning, contrast test, fan-project notice all still pass.
- [ ] `/code-review` with no open CRITICAL or HIGH; CI green; PR merged by you; app left running from `main`.

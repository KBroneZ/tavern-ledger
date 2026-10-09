# Session 028: Website redesign (T-104e)

Model: sonnet
Effort: medium (high for the design directions)
Subagents: haiku for searches; sonnet for code and reviews

Work in your worktree **`C:\Users\andia\tavern-ledger\.claude\worktrees\028-web-redesign`** (branch `028-web-redesign`, created by the hub) and follow its `CLAUDE.md`. Chat in caveman mode; the user writes in Spanish, answer in English. Everything in the repo is in English. **Tier R2** (a public site with sign-in, account, export and deletion): TDD where logic changes + `/code-review` + `/security-review`. A new dependency (framework, font, icon set) needs the user's OK and a line in `PROVENANCE.md`.

**Other sessions may run at the same time** (026 card images, 029 email sender). To stay out of their way:
- Work only in your worktree. Never touch the main checkout or other worktrees.
- Your area: `web/` only. Do not touch `crates/`, `supabase/`. `docs/legal/` belongs to session 029 this round: the site renders those files, so restyle how they look, never their text.
- Shared files (`plan.md`, `README.md`, `DECISIONS.md`, `PROVENANCE.md`): only your task's row and notes. Your decision number is **D-040**.
- To catch up with `main`: `git fetch origin && git merge origin/main`. No rebase, never force-push.
- Merging to `main` deploys the site (Cloudflare Pages). Check the live site after each merge.

Working rules (user's decisions):
- Decide technical matters yourself. Ask the user: the design direction, new dependencies, fonts or assets with unclear licences.
- **You merge your own PRs**, once CI is green and no review has open CRITICAL or HIGH findings: `gh pr merge <n> --squash --delete-branch`.
- The user often works remotely: show the directions as an artifact page or screenshots they can open from the phone.
- `gh` is logged in through `GH_TOKEN`; never print it.

**Red line (D-020, D-023, D-027, D-030, CLAUDE.md rules 5 and 8):** no analytics or third-party scripts; the CSP stays as strict as today (`script-src 'self'`, `connect-src` only the Supabase project, styles and fonts self-hosted, no external font or CDN); no Blizzard logos or name in the brand; the unofficial fan-project notice on every page; never invent features, figures or testimonials. Sign-in, account, export and deletion must keep working, and the sign-up form stays hidden while `PUBLIC_SIGNUPS_OPEN` is unset (D-037: sign-ups stay closed until the email sender is replaced).

## Effort and models per phase

| Phase | Effort | Model |
|-------|--------|-------|
| Read and check state | low | Haiku |
| Research (GitHub repos and skills for sites) | medium | Haiku for search, Sonnet to judge |
| Two or three directions for the user | high | Sonnet |
| Build (tests first where logic changes) | medium | Sonnet |
| Review | medium | `code-reviewer`, `security-reviewer` |

## At the start

1. Conversation title: **`#028 Web redesign`**.
2. `git fetch origin && git merge origin/main`.
3. Read `README.md`, `docs/plan/plan.md` (T-104c, T-104e and their notes), `docs/decisions/DECISIONS.md` (D-020, D-023, D-027, D-030, D-031, D-037), `docs/research/deploy.md` sections 1 to 4 and everything in `web/`.
4. Green before touching anything: `npm test` and the build checks in `web/` in all three modes (closed, open, open with sign-ups).

## Task

1. **Research.** The user wants a drastic redesign, "from GitHub UI skills and repos": look for open-source site designs, design systems and agent skills for web interfaces that fit a static Astro site with a strict CSP. Only MIT/Apache-style code or plain ideas; self-hostable fonts with OFL or similar. Notes in `docs/research/web-ui.md`.
2. **Directions.** Two or three distinct directions (home, sign-in, account, legal pages), each as a mockup page or screenshots, honest about what the product does today. **Ask the user to choose.** Record the choice as D-040.
3. **Build** the chosen direction across all pages; keep every check the build runs today (CSP, no inline script, no external request) and add tests for anything new.
4. Ride-along: raise HSTS from one day to one year in `web/astro.config.mjs` only if two clean weeks have passed since 2026-10-09 (on or after 2026-10-23); otherwise leave a note in the plan.
5. Check the live site after merging: every page, the CSP header, sign-in and the account page with the user's help only if they offer (never create accounts yourself).
6. Plan (add the T-104e row: "Website redesign"), README, D-040 and `PROVENANCE.md`, in the same PR.

## Acceptance criteria

- [ ] Research notes with licences; the user chose a direction; D-040 recorded.
- [ ] New look on every page; fan-project notice everywhere; no analytics, no third-party requests; CSP unchanged or stricter.
- [ ] Sign-in, account, export and deletion still work; sign-up still hidden.
- [ ] `/code-review` and `/security-review` with no open CRITICAL or HIGH; CI green; PR merged by you; live site checked.

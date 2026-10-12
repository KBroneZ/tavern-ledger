# Session 042: The website in "Lamplight glass" (T-316)

Model: opus
Effort: high (the pinned scene and the home page), medium for the rest
Subagents: haiku for searches; sonnet for code and reviews

Work in your worktree **`C:\Users\andia\tavern-ledger\.claude\worktrees\042-site-lamplight`** (branch `042-site-lamplight`, created by the hub) and follow its `CLAUDE.md`. Chat in caveman mode; the user writes in Spanish, answer in English. Everything in the repo is in English. **Tier R1** (site look; no change to auth, data or the CSP). Any new dependency, or a change to the CSP, makes it R2: ask the user first.

**Other sessions run at the same time:** 041 (app and overlay in the same look: `crates/desktop/`, `.claude/skills/`) and 044 (parser data round: `crates/bg-parser/`, `crates/tracker/`, `tests/`, `tools/`, and `supabase/functions/upload-game/` key list and validator). To stay out of their way:
- Work only in your worktree. Never touch the main checkout or other worktrees.
- Your area: `web/` only (pages, components, styles, `web/public/fonts/`, tests), plus `PROVENANCE.md` rows for the fonts the site ships. Nothing in `crates/`, `supabase/`, `tools/`, `tests/`.
- Shared files (`plan.md`, `README.md`, `DECISIONS.md`, `PROVENANCE.md`): only your task's rows and notes. Your decision number is **D-056**, used only if you decide something D-052 and `redesign.md` do not already cover. Keep every changed text file LF (`git ls-files --eol`; the checkout uses `core.autocrlf=true`, so check the index side).
- Never start Docker (044 may ask for it). Never stop or restart the user's desktop app.
- To catch up with `main`: `git fetch origin && git merge origin/main`. No rebase, never force-push.
- **Merging to `main` deploys the site** (Cloudflare Pages). After the merge, check: every page answers 200, the CSP header is unchanged, sign-in, the account page, password reset and the email-link messages from #72 still work (the sign-in check can wait for the user), and `/auth/v1/settings` still says `disable_signup: true`.

Working rules (user's decisions):
- Decide technical matters yourself. Ask the user: new dependencies, fonts beyond the two D-052 approved, images with unclear licences, and anything about the copy that makes a promise.
- **`gh` is not logged in inside task sessions.** When your branch is ready (tests green, `/code-review` done, no open CRITICAL or HIGH), push it and send the hub (`Hub prompts session 036`, find it with `ListAgents`, use its `[ref]`) a message with the branch, the PR title, a short PR body (summary, review results, test plan) and screenshots or an artifact link for the user. The hub opens the PR, waits for CI and merges it.
- The user is at home and wants to see it: publish a preview (artifact or screenshots at 375, 768 and 1440 px, both the moving and the still scene) and ask in your tab **and** through the hub (their tab may be covered).

## Why

The user chose "Lamplight glass" (D-052) after five rounds of mockups in session 041, a full redesign that keeps nothing from the current site's look. They asked for a home page that sets the site apart: the game screen, seen from the side, splits into layers (board, tavern, leaderboard, and our overlay in front) as you scroll.

## Read first

`docs/research/redesign.md` is your brief: the tokens, the fonts and licences, and its section **"Website build plan (session 042)"** (limits, pages, the pinned scene, motion, accessibility and quality checks). The mockup `docs/research/redesign-mockup.html` is a reference only; build from the plan, in the site's own components. Also D-040 (replaced) and D-052, `web/src/` as it is now (including #72's auth link handling, which must keep working), and `web/src/lib/config.ts` for the CSP.

## Effort and models per phase

| Phase | Effort | Model |
|-------|--------|-------|
| Read the plan, the current site, the tests | low | Haiku |
| Tokens, fonts, layout, every page | medium | Sonnet |
| The pinned scene (CSS scroll timelines, still fallback) | high | Opus |
| Audits (`accessibility`, `performance`, `web-quality-audit`, `seo`), review | medium | `code-reviewer`, `a11y-architect` |

## At the start

1. Conversation title: **`#042 Site in Lamplight glass`**.
2. `git fetch origin && git merge origin/main`.

## Task

1. Fonts: Bricolage Grotesque and Figtree as self-hosted `.woff2` files with their OFL texts in `web/public/fonts/`, sizes as `redesign.md` lists; `PROVENANCE.md` rows (source, version, licence). Remove the old site fonts if nothing uses them any more.
2. Tokens and components from `redesign.md` across every page: home, sign-in, account, forgot password, privacy, terms, profile, 404. The text keeps its meaning; new copy follows the plan and absolute rule 8 (no invented features, figures or testimonials; example numbers labelled made up). The fan-project notice on every page.
3. The home page's pinned scene exactly as the plan says: CSS only (scroll-driven animations), our own shapes, no game art, a still picture with every caption under reduced motion and in browsers without scroll timelines, readable by screen readers (one described image plus the captions as text).
4. The limits stay: the CSP as in `config.ts` (no `'unsafe-inline'`; no inline `style` attributes), no script beyond what the site already ships, no library, CDN, analytics or external request.
5. Tests: the build tests for the three builds (open, sign-ups open, closed), the unit tests, a check that no page loads anything from another origin, and the e2e flows from #72 still green.
6. Audits with the installed skills (`accessibility`, `performance`, `web-quality-audit`, `seo`): contrast on every token pair, keyboard and focus, reduced motion, Lighthouse-style budget (`.claude/rules` web performance targets), no layout shift from fonts.
7. Plan row **T-316** "The website in Lamplight glass (D-052)"; README; same PR.

## Out of scope

The app and the overlay (041), opening sign-ups, auth or data changes, new pages beyond what the plan lists.

## Acceptance criteria

- [ ] Every page in the new look, matching `redesign.md`; fonts self-hosted with licences.
- [ ] The pinned scene works in current Chrome, Edge and Safari, and is a complete still picture otherwise and under reduced motion.
- [ ] CSP unchanged, no external request, no new dependency; auth flows from #72 still pass.
- [ ] Audits done with results in the PR; tests green.
- [ ] The user has seen a preview; `/code-review` with no open CRITICAL or HIGH; PR merged through the hub; post-merge checks reported.

## Red line

No Blizzard logos, names in the brand, art or screenshots (absolute rule 5); the notice "Unofficial fan project. Not affiliated with or endorsed by Blizzard Entertainment." on every page. Never print keys or the user's email.

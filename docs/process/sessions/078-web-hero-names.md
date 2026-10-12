# Session 078: Hero names, bigger portraits and missing game stats on the website

Model: opus
Effort: medium
Subagents: haiku for searches; sonnet for code and the review

Work in your worktree **`C:\Users\andia\tavern-ledger\.claude\worktrees\078-web-hero-names`** (branch `078-web-hero-names`, created by the hub) and follow its `CLAUDE.md`. Chat in caveman mode; the user writes in Spanish, answer in English. Everything in the repo is in English. **Tier R1** (R2 if you touch `supabase/`): TDD + `/code-review` (+ `/security-review` for `supabase/`).

**Other sessions at the same time:** 076 (website simulator: WebAssembly, `/simulator/`, the CSP) and 077 (accuracy page, the home page's "what's new"). Your area: how the site names and draws heroes everywhere (`web/src/lib/data/`, `web/src/lib/heroes/`, `web/src/lib/game/`, `web/src/lib/leaderboards/`, `web/src/lib/community/`, `web/src/lib/view/`, `data.css`), a static hero name table, and, if the cause is there, the stats backfill in `supabase/functions/` (sweep). Do not touch the CSP, `config.ts`, `astro.config.mjs` or the home page. Shared files (`plan.md`, `README.md`, `DECISIONS.md`): only your rows and notes; decision number if needed **D-092** (076 has D-090, 077 has D-091). Every changed text file LF (`git ls-files --eol`, index side).

- To catch up: `git fetch origin && git merge origin/main` (076 and 077 may merge first; expect small conflicts in shared files). No rebase, never force-push. Stop any dev server or `astro preview` you start.
- **Before you hand over, run the tests from a clean clone of your pushed branch**; check `git ls-files` for every new file.
- Do not build or run the user's desktop app. Never touch production by hand (D-031). The user may be busy: questions go in your hand-over with a recommendation. `gh` is not available: push the branch and message the hub (`tavern-ledger-2b`, `ListAgents`, use its `[ref]`) with the branch, PR title and a short PR body. **Merging deploys the site (and functions if touched):** afterwards check every page answers 200, the CSP is unchanged, `/auth/v1/settings` says `disable_signup: false` (sign-ups are open now, D-089), functions answer `401` without a token; tell the hub.

## What the user saw (2026-10-11, live site, hub's check)

The user: "On the website the heroes' names are not the right ones; the icons should be bigger, more visual."

1. **Card ids instead of names.** `/games/`, `/player-heroes/`, `/me/` and likely `/heroes/`, `/leaderboards/`, `/game/` show `BG22_HERO_002`, `TB_BaconShop_HERO_41`, `BG34_HERO_000` where the hero's name should be (`lib/data/dashboard.ts` and `lib/heroes/heroes.ts`: `heroName = hero ?? t("hero.unknown")`, the id itself). Show the hero's real name in the page's language (English names on `/…`, the game's esES names on `/es/…`, D-086), from HearthstoneJSON card data (D-074; the app already does it, `crates/cards`, D-038, D-065). A small static table built from that data (hero card ids, including skins and `TB_BaconShop_HERO_*`, mapped to the base hero's name), committed with a script to refresh it and its source in `PROVENANCE.md`; no runtime fetch to a new origin. An unknown id shows "Unknown hero" plus the id in Advanced, never the bare id as a name. The hero filter on `/games/` lists names too, sorted by name.
2. **Portraits too small.** The hero pictures in tables are about 16 px. Make them big and visual, the user's taste: compact but picture-first, key numbers next to them, no white edges (D-074 crops), good on a phone. At least the games list, your heroes, the dashboard's hero blocks, community heroes and leaderboards' per-hero board. Use the design system tokens; add, do not rewrite.
3. **Most games have no stats.** On `/games/`, 40 of 41 games show "—" for rounds, combats, triples and APM, and `/me/`'s Solo cards say "no data in 2 games". `game_stats` should come from `upload-game` for new games and from the daily sweep's backfill for older ones (D-071, T-502 notes). Find out why with the local stack and the code (production only through its public answers and the user's own signed-in view, never by hand): the backfill not run yet, capped, failing, or the page reading the wrong thing. Fix it in code if it is ours; if it only needs the next sweep, say so with the evidence.
4. Also seen: the menu shows "Sign in" and "Account" while signed in; if it is easy and in your area, show only the right one.
5. Tests for the name table (every hero id in the user's history files has a name in both languages; skins map to the base hero), the fallback, and the strings test. Plan rows (new task ids from **T-510**), D-092 if needed, README.

## Acceptance criteria

- [ ] No page shows a hero card id as a name; English and Spanish names right.
- [ ] Hero portraits big and clean in every hero list.
- [ ] The missing stats explained with evidence and fixed or dated.
- [ ] Clean-clone tests green (paste counts); reviews with no open CRITICAL or HIGH; branch handed to the hub.

## Red line

D-004 (as amended by D-063), D-006, D-054 apply. Never print BattleTags, `GameAccountId`, player names or the user's email. No analytics, no new origin in the CSP.

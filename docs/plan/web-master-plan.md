# Website master plan (round 5)

The user's request (2026-10-11): "How all this information will look on the website: a master plan, then get to work. Lots of detailed information, clean and beautiful; leaderboards of several stats, and more; public leaderboards of stats and of everything."

This plan turns the desktop app's data (parser revision 6, D-055, D-058, D-061) into the website at `https://tavernledger.net`, in the Lamplight glass look (D-052), with the compact style the user asked for in the app (D-062: key numbers first, details folded, Simple by default).

## What we have

- **Site:** Astro, static, Cloudflare Pages (D-030). Pages: home, sign-in, account, forgot password, profile (`/profile/?u=…`), privacy, terms, 404. Strict CSP: `connect-src` only the project's Supabase host.
- **Backend:** Supabase (D-031). `public.profiles` (display name, `is_public`, private by default) and `public.games` (one row per game: mode, status, hero card id, place, build, date, tribes offered, the gzipped full report in Storage). Row-level security: a profile and its games are readable by everyone only when `is_public`. Uploads through the `upload-game` function (validator 6).
- **Data per game (the uploaded report):** hero and hero select, place, rounds with results (log or inferred), opponents met and their boards with keywords and golden, lobby heroes with triples and trinkets, shop record (offers, buys, sells, rolls, gold, tier-ups), choices (discovers, trinkets, Dark Gifts, quests), Old God, APM, Duos teammate.
- **Not uploaded, ever:** the rating and tribes read from memory (D-063), BattleTags, other players' names, `GameAccountId`.
- **Sign-ups are closed** (`disable_signup: true`). Today only the user has an account.

## Rules that shape the website

1. **Privacy first (absolute rule 6, GDPR).** A player appears anywhere public only if they made their profile public. Leaderboards list only public profiles, by the display name the player chose. No BattleTags, no other players' names (opponents are shown by hero, never by name). Every public page says where the data comes from and that it is self-reported.
2. **Honest numbers (absolute rule 8, "unknown is not zero").** Every stat shows how many games back it ("in 28 of 31 games"); a leaderboard needs a minimum number of games per player to rank (no 1-game 100% win rates); inferred values are labelled.
3. **Ratings.** The rating from memory is never uploaded (D-063). A rating leaderboard can only come from Blizzard's own public leaderboard (D-012: each user's own row, from their own app) and needs a new decision before any rating is stored on the server. Until then: **no rating leaderboard**, our leaderboards rank performance stats from uploaded games.
4. **Card art on the web.** Pictures come from HearthstoneJSON at view time under Blizzard's fan content terms (D-038, T-304 notes); never bundled in the repo. The CSP must allow the image host (`img-src`), decided and recorded before the first page with pictures.
5. **Self-reported data can be faked.** Leaderboards say "self-reported", rank only games that passed the validator, and need anti-abuse limits (rate limits, a minimum of games, outlier flags). No prizes, no claims.
6. **Accessible and fast.** WCAG 2.2 AA (as 046 did), no layout shift, server-side aggregation (no downloading every game to the browser), pages under the web performance budget.

## Information architecture

| Page | What it shows | Who sees it |
|------|---------------|-------------|
| `/` home | What the app does, screenshots, download (later), live counters only when real | everyone |
| `/me/` dashboard | Your overview: rating trend (from your own app only, later), last games, top heroes, records | you |
| `/u/<name>/` public profile | The same overview for a public profile, read only | everyone (if public) |
| `/u/<name>/games/` | The games list: filters (mode, hero, date, place), compact rows, top N with "show all" | you / public |
| `/game/<id>/` game viewer (T-201) | One game: summary bar, turns timeline (result, damage, board), boards met, shop per turn behind "See shops", choices, lobby with triples and trinkets; Simple and Advanced | owner / public |
| `/u/<name>/heroes/` | Per hero: games, avg place, top-4 or top-2, pick rate from hero select, best and worst | you / public |
| `/u/<name>/curious/` | The curious stats (T-216) | you / public |
| `/leaderboards/` | Public boards across all public profiles (below) | everyone |
| `/heroes/` community heroes (T-401) | Aggregate hero stats from public games, with volume thresholds | everyone |
| `/account/` | Settings, privacy switch, display name, export, delete (exists) | you |

Every page: Solo and Duos side by side or a mode switch; English first, Spanish next (the app's i18n approach); dark Lamplight glass; key numbers in view, the rest folded.

## Leaderboards (public, opt-in profiles only)

Ranked per mode (Solo, Duos), per period (last 7 days, last 30 days, season or all time), with a minimum games threshold (for example 20 games in the period) and the number of games shown next to each value:

- Average place; top-4 rate (Solo) / top-2 rate (Duos); win rate (1st place).
- Games played; longest top-4 streak.
- Per hero: best average place with that hero (minimum games per hero).
- Economy and play: APM, average gold beyond the fixed income, rolls per game, triples per game.
- Fun boards from the curious stats: most damage dealt in one combat, biggest board stats, most golden minions at once (each only where the data backs it).
- Later, with a decision: rating (see rule 3).

Built as SQL views or materialized views refreshed on a schedule (like the sweep), never computed in the browser; each board has an explanation fold ("how this is counted").

## Phases and tasks

| ID | Task | Depends on | Tier |
|----|------|------------|------|
| T-501 | **Design system for the web data pages:** tokens, stat tiles, compact tables with top N, folds, charts (dataviz rules), card and hero pictures (crop like 053), empty and unknown states; a static style page with invented data | — | R1 |
| T-502 | **Data API:** stats computed server-side from the stored reports: a per-game summary table filled at upload (or a parse of stored reports) and SQL views per user (overview, heroes, games list); RLS keeps private data private; tests (pgTAP-style or the existing database tests) | — | R2 |
| T-503 | **Personal dashboard and games list** (`/me/`, `/u/<name>/`, `/u/<name>/games/`) on T-501 and T-502 | T-501, T-502 | R2 |
| T-504 | **Game viewer** (T-201): one game in full, Simple and Advanced, pictures | T-501, T-502, CSP decision | R2 |
| T-505 | **Heroes and curious stats pages** | T-503 | R1 |
| T-506 | **Public leaderboards** with thresholds, periods, explanations, anti-abuse | T-502, user decisions below | R2 |
| T-507 | **Community hero stats** (T-401) with volume thresholds | T-506 | R2 |
| T-508 | **Spanish** for the site (strings, dates, numbers) | T-503 | R1 |
| T-509 | **Legal and privacy texts** for public stats and leaderboards (what is public, how to opt out, retention) | before T-506 ships | R2 |

**T-501 notes (done, D-070):** the switches need the page's script, so on a page without one (the accounts-closed build) they stay hidden and the page shows Simple and Solo; the data pages need a script for their data anyway. "Show all" and folds use native disclosures. Pictures are one setting in `web/src/lib/data/pictures.ts`; T-504 switches it once the CSP allows the image host. Build on the pieces as shown on `/style/`.

**T-503 notes (done, D-073):** the data pages stay static: a frame with reserved room and a script that draws the T-501 pieces from the data API (same markup, checked by a build test). Addresses are `/me/`, `/profile/?id=<uuid>` and `/games/` (`?id=` for a public profile) rather than `/u/<name>/`: a static host has no server routing and display names are not unique handles. New pieces for later pages: `web/src/lib/view/` (element trees), `web/src/lib/data/rows.ts` (checked rows), `dashboard.ts`, `filters.ts`, `web/src/lib/dataApi.ts`.

**T-506 and T-509 notes (done, D-082):** `/leaderboards/` lists public profiles with an allowed display name and 20 finished games in the mode and period (10 per hero), 12 boards, rebuilt every 10 minutes, read through `public.leaderboard()`; leaving is immediate. The privacy policy and terms cover public profiles and leaderboards. Details: [leaderboards.md](../research/leaderboards.md).

Order: T-501 and T-502 in parallel (they do not overlap: `web/` design vs `supabase/`), then T-503 and T-504, then T-505 to T-509.

## Decisions the user must make (the hub asks, one at a time)

**Answered 2026-10-11:** 1 → D-075 (public profiles, minimum games), 2 → D-078 (open after T-506 and T-509), 3 → D-074 (HearthstoneJSON pictures), 4 → D-076 (no rating), 5 → D-077 (current rule plus a word filter).

1. **Public leaderboards scope:** only opt-in public profiles, with a minimum of games (recommended), or more.
2. **Opening sign-ups:** leaderboards are empty with one account; opening sign-ups is the user's call (D-041 mail, legal texts out of draft).
3. **Card pictures on the website:** allow HearthstoneJSON's image host in the CSP (`img-src`) under the fan content terms, or no pictures.
4. **Rating on the website:** none (recommended for now), or the Blizzard leaderboard row from the user's own app (a new decision on storing it).
5. **Display name rules for public boards:** the existing rule (1 to 32 letters, digits, space, `_.-`) plus a basic word filter, or nothing more.

## Done means

The website shows everything the app knows, clean and compact, for the signed-in player and for public profiles; public leaderboards work with honest thresholds; privacy texts match; CI green; the user has looked at it on desktop and phone.

# Public leaderboards (T-506, D-075 to D-077, D-082)

How the public leaderboards at `/leaderboards/` are built, who is on them, and why each number is what it is. Schema: [`supabase/migrations/20261011150000_leaderboards.sql`](../../supabase/migrations/20261011150000_leaderboards.sql). Tests: [`supabase/tests/database/leaderboards.test.sql`](../../supabase/tests/database/leaderboards.test.sql) (61, with the old minimums set back), [`leaderboard_minimums.test.sql`](../../supabase/tests/database/leaderboard_minimums.test.sql) (18, the rule since D-103), `web/test/unit/leaderboards.test.ts`, `web/test/local/leaderboards.e2e.test.ts`. The privacy policy and terms say the same in plain words ([privacy.md](../legal/privacy.md) section 4, [terms.md](../legal/terms.md) section 4).

## Who is on a board (D-075)

A player is listed only when all of these hold:

1. **The profile is public** (`profiles.is_public`, off by default).
2. **It has a display name**, and the name passes the word list (below).
3. **One finished game** in the mode and period that has the board's number (one with the hero for the per-hero board). Until D-103 (T-534, 2026-10-12) this was 20 games (10 with the hero); it lives in `private.leaderboard_min_games()` and `private.leaderboard_min_hero_games()`, replaced by [`20261012120000_leaderboard_no_minimum.sql`](../../supabase/migrations/20261012120000_leaderboard_no_minimum.sql), so it can be raised again with one migration.

Rows show the display name (read at the moment of the request, so a rename shows at once), a link to `/profile/?id=<account id>` (the site's own account id, already public for a public profile), the standing, the value and the games behind it. No email, BattleTag, rating (D-076) or other players' names: there is no column for them.

**Few games (D-103).** The site has few players, and with 20 games every board was empty, so the user removed the minimum. The website instead marks a row that rests on fewer than 20 games (10 with the hero on the hero board) as "few games", next to the games behind it, so a 1-game 100% reads for what it is; the order stays by value, ties by more games. The community hero stats keep their own minimum (D-083: 50 games from 5 players): it protects players who are not public.

**Why 20 (D-082, now the "few games" line).** A 1-game 100% must not top a board (D-075). Over 20 Solo games the average place has a standard error of about 2.3 / √20 ≈ 0.5 places, enough to separate a strong player from a lucky week, and 20 games in 7 days is about three a day, which an active player reaches. One number for every board and period keeps the explanation to one sentence. **Why 10 per hero:** a player rarely plays 20 games with one hero in a month; 10 still rules out one-game flukes, and under D-082 the player also needed the 20 games overall (no longer, D-103).

## What counts

- **Finished games only** (`games.status = 'ok'` with a place), which passed the server's validator at upload (every stored game did) and have a server summary (`game_stats`, filled at upload or by the daily sweep). Unfinished and unreadable games never count.
- **Periods**: the last 7 days (today and the 6 days before, UTC, by the day the game was played), the last 30 days, all time.
- **Modes**: Solo and Duos apart. In Duos the place is the team's (1st to 4th), and the top half is the top 2.

## The boards

| Board | Value | Better | Games behind it |
|-------|-------|--------|-----------------|
| `avg_place` Average place | mean place, 2 decimals | lower | finished games |
| `top_half_rate` Top 4 rate (Top 2 in Duos) | % of finished games in the top half, 1 decimal | higher | finished games |
| `win_rate` Win rate | % of finished games in 1st, 1 decimal | higher | finished games |
| `games` Games played | finished games | higher | the same |
| `top_half_streak` Longest top 4 / top 2 streak | most top-half finishes in a row within the period, in play order; a left-out game (below) breaks a run | higher | finished games |
| `hero_avg_place` Best average place per hero | mean place with one hero (skins under their hero, D-019 rule); one row per player and hero | lower | games with that hero (10+) |
| `apm` Actions per minute | mean APM (D-046) | higher | games with APM recorded |
| `gold_beyond_income` | mean gold beyond income per game | higher | games with it recorded |
| `rolls` | mean rolls per game, free ones included | higher | games with it recorded |
| `triples` | mean triples per game (the game's own leaderboard count) | higher | games with it recorded |
| `biggest_board` | largest attack + health total of the player's board entering a combat, in any one game | higher | games with it recorded |
| `most_golden` | most golden minions on the player's board entering a combat, in any one game | higher | games with it recorded |

Simple shows the first six; Advanced adds the economy and fun boards.

**Ties.** Values are rounded as the page shows them, so equal-looking values share a standing ("1, 1, 3"). Within a tie, more games come first, then the account id, so the order is the same on every read.

**Top 50.** A board returns at most 50 rows (the page shows 10 and "Show all"). Twelve boards of 50 stay under the API's 1000-row cap per request.

## Impossible values (anti-abuse)

The data is self-reported: the desktop app uploads it and the validator checks its shape, not that the game happened. A game with any value no real game can have is left out of **every** board (counted in `leaderboard_state.excluded_games`):

| Value | Highest allowed | Why |
|-------|-------|-----|
| golden minions on the board | 7 | the board has 7 slots |
| minions on the final board | 7 | the same |
| APM | 100 | 1.7 logged actions a second for a whole game; the user's games are 8 to 19 ([shop-and-apm.md](shop-and-apm.md)) |
| triples | 50 | far above any game seen |
| rolls | 1000 | the user's games roll 8 to 55 times |
| gold beyond income | 5000 | far above any game seen |
| board attack + health | 10,000,000 | far above any board seen |
| rounds | 100 | games end long before |
| day played | tomorrow (UTC) | the day comes from the player's own clock, which can be ahead of UTC by a day at most |

Unknown values never exclude a game. Other limits are the upload's own: per-user hourly and daily upload limits, the validator, a minimum of games. Ranking needs many games, so a forged report gains little; the terms allow leaving altered games off the boards.

## How it is built

- **Snapshot**: `private.leaderboard_rows` (mode, period, board, user id, hero, value, games), in a schema the API does not expose; no client role has any right on it. `private.refresh_leaderboards()` rebuilds it in one transaction (readers keep the old one until it commits) from the games of public profiles with an allowed name only, so nothing else is copied. It works the rows out into a temporary table first and only then swaps them in (delete and insert, joined with the profiles as they are at that moment), so the stored rows are locked for a moment only. One rebuild at a time through an advisory lock, never a table lock, so reads and opt-outs never wait for a rebuild. pg_cron job `leaderboards-refresh`, every 10 minutes (`*/10 * * * *`). The migration builds it once. Dates use UTC (`now() at time zone 'utc'`), whatever the session's time zone.
- **Reads**: `public.leaderboard(p_mode, p_period)` (anon and authenticated; `security definer`, empty search path). It checks each player on the boards once (public, named, name allowed; a materialized step, so the word list runs once per player, not per row), computes the standings over the rows it returns, and refuses an unknown mode or period (`22023`). `public.leaderboard_refreshed_at()` gives the last rebuild's time.
- **Opting out**: the read check hides a profile the moment it turns private or loses its name. A trigger also deletes its stored rows then, skipping (never waiting for) rows a running rebuild holds: that rebuild's swap reads the profiles afresh and leaves the profile out; only an opt-out in the instant between the swap and the rebuild's commit keeps its rows until the next rebuild, still hidden by the read check. Turning public again shows the player at the next rebuild. A page already open keeps what it loaded until it is reloaded. Account deletion (`delete_user_data()`) deletes the rows before the profile, in the same lock order as a rebuild, so the two cannot deadlock.
- **Export**: `export_my_data()` has `leaderboard_rows` (the player's own numbers, without names or ids).

Calling it from the site:

```js
const { data } = await supabase.rpc("leaderboard", { p_mode: "solo", p_period: "30d" });
// [{ board, standing, user_id, display_name, hero, value, games }, ...]
const { data: at } = await supabase.rpc("leaderboard_refreshed_at");
```

## Display names (D-077)

Today's rule (1 to 32 letters, digits, space, `_.-`; the `profiles.display_name` check) plus a small word list in `private.name_blocklist`, kept in the migration (written for this project; no third-party list). Each word is matched in three spellings of the name, all lower case after Unicode NFKC (full-width letters read as plain): as typed, and with digits read as letters (`0` o, `1` i or l, `3` e, `4` a, `5` s, `7` t, `8` b, `9` g).

- `part` words match anywhere once spaces, `_`, `.` and `-` are removed ("F u c k", "f_u_c_k").
- `word` words match only a whole word of the name or the whole name, for words that hide in ordinary ones ("ass" in "Class", "cock" in "Peacock", "rape" in "Grape", "rapist" in "Therapist", "nazi" in "Ashkenazi", "fuk" in "Fukuoka").
- `reserved` words would make a name look official: `blizzard`, `tavernledger` (anywhere), `admin`, `administrator`, `moderator`, `staff`, `support`, `official` (whole words). Blizzard's name in a display name could read as endorsement (absolute rule 5).

**Where it is checked.** A trigger on `profiles` (`before insert or update of display_name`) refuses a changed name with SQLSTATE `23514` and the message `display_name_not_allowed`, on every write path (the account page, the API, the service role). The account page turns that into "This name is not allowed…". The list never goes to the browser. Only a *changed* name is checked, so a player whose old name fails a newly added word can still make the profile private. The boards check the name again on every read and every rebuild, so such a name is hidden from the boards until it changes.

**Limits.** A basic filter: look-alike letters from other alphabets are not mapped, and new spellings get through until a word is added. "Scunthorpe" is refused (it contains a `part` word); a whole-word entry would let more slurs through, so the list accepts that. Adding a word is a new migration (`insert into private.name_blocklist ...`); existing names that fail leave the boards at once.

## Checks

- Database (62 tests): every board's values, ties, the 50-row cap, the periods' edges (a week ago, tomorrow, the day after), the minimum, a private profile, a profile with no name, a name that fails the list later, ordinary names with a whole-word entry inside, anonymous and signed-in readers, opting out and clearing the name at once and coming back at the next rebuild, export, deletion, rights on every object, the schedule.

## Reviews (session 069)

`/code-review` (Sonnet), `/security-review` (Opus), `database-reviewer` (Opus) and a second AI (ChatGPT through Codex, with the code pasted in: its first run could not read the repository). No CRITICAL or HIGH left. Fixed from them: the rebuild held a table lock for its whole run, so an opt-out or an account deletion could wait or deadlock (now computed first, swapped at the end, advisory lock, the opt-out trigger skips locked rows, deletion takes the rows first); games dated after tomorrow counted on the 7- and 30-day boards for good (now impossible) and the dates followed the session's time zone (now UTC); the read ran the word list per row (now once per player); clearing a name left the stored rows; an excluded game joined two streaks; "Therapist", "Ashkenazi" and "Fukuoka" were refused; the hero board said 20 games where it means 20 and 10 with the hero; an unreadable rebuild time read "Not built yet" (now an error); values outside a board's range are refused in the browser; the texts promised "at once" for pages already open and "within 10 minutes" for a schedule. Left as they are: seven requests per page view (one per mode and period, so the switches never wait); the minimums written in both the database and the page (a test pins each).
- Browser (local stack, synthetic users only): axe clean (WCAG 2.2 AA and best practice) in Simple and Advanced, Solo and Duos, every fold open; CLS 0; no sideways scroll at 390 px; the period switch kept in the address.

# Community hero stats (T-507, T-401, D-085)

How the community hero stats at `/heroes/` are built, whose games they use, and why each rule is what it is. Schema: [`supabase/migrations/20261011180000_community_heroes.sql`](../../supabase/migrations/20261011180000_community_heroes.sql). Tests: [`supabase/tests/database/community_heroes.test.sql`](../../supabase/tests/database/community_heroes.test.sql), `web/test/unit/communityHeroes.test.ts`, `web/test/local/communityHeroes.e2e.test.ts`. The privacy policy says the same in plain words ([privacy.md](../legal/privacy.md) section 4, "Community hero stats").

## Consent (the plan's rule for T-401)

Community stats are **opt-in, off by default, and never switched on by an update**.

- **Where:** a switch of its own on the account page, "Include my games in community hero stats", in a "Community stats" panel with a one-line explanation and a link to the policy. A failed save puts the box back to the stored choice and says the setting is unchanged.
- **Stored:** `public.community_consent` (`user_id`, `hero_stats`, `updated_at`, `policy_version`: the privacy policy's date when the choice was saved, for GDPR Art. 7(1)). **No row means off**, so every account, including those made before this migration, starts off, and no migration or update has to write anything to keep it that way.
- **Not on the profile.** `profiles` is readable by everyone for public profiles; whether someone shares their games is nobody else's business. A public profile does not imply the opt-in, and a private profile can opt in.
- **Who sets it:** only the signed-in owner with a profile, through `public.set_community_consent(boolean)` (`auth.uid()`, security definer, empty search path; `42501` otherwise). Clients have no insert, update or delete right; `authenticated` may only select, and the policy shows only the caller's own row; anonymous visitors have no right at all. (The server's own roles can read every row, as for every table.)
- **Export:** `export_my_data()` has `community_consent: {hero_stats, updated_at, policy_version}` (`false`, `null`, `null` when never set). **Deletion:** `delete_user_data()` deletes the row; the foreign key to the profile would too.
- **Legal basis:** consent (Art. 6(1)(a) GDPR), withdrawn by turning the switch off.

## What is counted

From accounts with `hero_stats = true` at the moment of the rebuild:

- **Finished games** (`games.status = 'ok'` with a place; every stored game passed the upload validator) **with a server summary** (`game_stats`) and a **known hero**, **played up to yesterday** (UTC) at the rebuild. Today's games wait for their day to end.
- **Impossible values** left out with the leaderboards' rule (`private.leaderboard_game_plausible`, D-082) and games dated after tomorrow.
- **Hero**: skins under their base hero (`_SKIN_x` stripped), as everywhere else (D-019 rule).
- **Modes** apart: Solo and Duos (the place is the team's, 1st to 4th; top half is top 2). In Duos each player's own hero counts, so two teammates who both share give one row each, for their two heroes.
- **Periods**: the last 30 days (the 30 days before the rebuild's day) and all time. Internally also "old" (before the 30 days), never shown, only checked (below).

Per mode, period and hero:

| Number | Shown as |
|--------|----------|
| Games | finished games with the hero, floored to tens ("50+" = 50 to 59) |
| Players | distinct accounts behind those games, floored to fives ("5+" = 5 to 9) |
| Average place | mean final place, 1 decimal |
| Top half rate | % of the games in the top 4 (Solo) or top 2 (Duos), whole percent |
| Win rate | % of the games in 1st, whole percent (Advanced) |
| Pick rate | of the games whose recorded hero choice offered the hero (two skins of one hero in one choice count once), the % where it was played, whole percent; with the offers and players behind it, floored the same way (Advanced) |

**Why the rounding.** With 50 to 100 games behind a number, the average place has a standard error of about 0.25 to 0.33 places and a rate one of about 5 to 7 points, so a second decimal (or a tenth of a percent) is noise. The same rounding stops two readings of the page from being subtracted to recover single games (below).

**Why no "current patch" period.** Games carry the client's build number, but a build is not a patch: hotfixes change the build without changing balance, and the site has no list of balance patches yet (it would come with the card data, D-074). A "patch" made of the newest builds would cut at the wrong places, so the periods are time windows until such a list exists.

## Thresholds

A hero shows only when all of these hold for its games in the mode and period:

1. **At least 50 games.** With a place standard deviation of about 2.3 in Solo, the average over 50 games has a standard error of about 0.33 places (over 20, the leaderboards' minimum for one player, it is 0.5). With about 100 heroes in the pool, the community needs about 5,000 games in a period before the average hero reaches 50, so the page says "Not enough games yet" for a while; that is the honest answer (D-075's spirit: never a 1-game 100%).
2. **At least 5 different players.** Five is the usual minimum cell size for published counts, so a number never describes one or two people.
3. **No single player with more than half of the games, and no two with more than three quarters** (dominance rules: otherwise one player, or two friends, would see a number that is mostly their own, and could subtract themselves to read the others').
4. **All time, only when the older games pass too.** When the 30-day row is shown, the all-time row is shown only if the games before the 30 days pass rules 1 to 3 on their own or there are none; otherwise all time minus 30 days would single out a few old games.

The pick rate follows the same rules over the games that offered the hero; below them its cell is a dash read as "Not enough hero choices yet". No totals per mode or period are published (totals minus the shown heroes would describe the hidden ones).

The thresholds live in one place in the database (`private.community_min_games()`, `community_min_players()`, `community_enough()`) and are mirrored in `web/src/lib/community/heroes.ts`, which refuses a row below them, or not floored, as a fault on the server.

## How it is built

- **Snapshot**: `private.community_hero_rows` (per mode, period `30d` / `old` / `all` and hero: games, players, the most games one player and the top two have, place sum, top half and wins, and the same for offers and picks) and `private.community_state` (when it was built, the last day counted, the games left out as impossible). **Sums and counts only: no account id, name, session or game.** No client role has any right on these tables.
- **Rebuild**: `private.refresh_community_heroes()`, in one transaction (readers keep the old snapshot until it commits), one at a time through an advisory lock, never a table lock, so it never waits on or blocks an opt-out or a deletion. pg_cron job `community-heroes-refresh` at `5 4 * * *` (04:05 UTC, after the daily sweep). The migration builds it once.
- **Reads**: `public.community_heroes()` (anon and authenticated; security definer, empty search path) returns every mode and period in one read, so from one snapshot, only the rows that pass the rules above, floored and rounded, most games first. `public.community_heroes_state()` returns when it was built, the last day counted and whether it is available. **Fresh or nothing:** when the snapshot is more than 26 hours old (a rebuild failed), `community_heroes()` returns nothing and the page says the stats are being rebuilt, so a withdrawn consent or a deleted account never stays in what is shown for more than 26 hours.
- **Opting out and deletion**: the account's games leave at the next daily rebuild. Nothing stored names the account, so there is nothing to delete at once; a page already open keeps what it loaded until it is reloaded.

```js
const { data } = await supabase.rpc("community_heroes");
// [{ mode, period, hero, games, players, avg_place, top_half_rate, win_rate, offered_games, offered_players, pick_rate }, ...]
const { data: state } = await supabase.rpc("community_heroes_state");
// [{ refreshed_at, counted_until, available }]
```

**Cost.** Each rebuild scans the opted-in accounts' games with their summaries once and unnests the hero choices. Once a day this is cheap at any size the project expects; if it grows to millions of games, keep per-day partial sums so all time does not rescan history.

## The page

`/heroes/` (in the menu as "Hero stats", `noindex` like the leaderboards; with accounts closed it shows the usual notice): Solo/Duos, period (30 days by default, in the address) and Simple/Advanced switches; "Updated …, with games up to …"; per mode and period a compact table (hero picture and card id, games, players, pick rate, average place, top 4 or top 2; Advanced adds win rate and the offers behind the pick rate), sortable by games (default), average place, top 4 / top 2 rate or pick rate (in the address, focus kept on the pressed button, announced); the top 20 with "Show all". Empty state: "Not enough games yet" with the thresholds and a link to the account page's switch. A stale snapshot: "Being rebuilt". A "How this is counted" fold says: opt-in only, the thresholds and the all-time rule, the rounding, finished games, self-reported, the server's checks and the impossible values, the daily rebuild, nothing per player, and links the policy.

## Limits

Reviewed by `database-reviewer`, `security-reviewer` and ChatGPT (session 072); the first version (10-minute rebuilds, exact values, totals per mode) let two readings be subtracted to recover single games or a player's whole record, and these rules come from those findings.

- **Differencing over time.** Someone who saves the page every day sees the change between two rebuilds. Each change mixes a whole day of everyone's games, and the rounding hides single games, but with few players a big change (someone starting or stopping sharing, or deleting an account) can still be guessed at, and so can whether a public profile (whose games are public) shares. Exact protection would need added noise (differential privacy), which would make small numbers wrong for everyone; the policy says plainly what remains possible.
- **Several accounts.** Accounts are cheap and the data is self-reported, so someone with several accounts could push a hero's numbers, or try to isolate another player within a small group. The two dominance rules and the rounding make the second hard; sign-ups are closed today, uploads are rate-limited and checked, and games with impossible values are left out. A per-account weight or a minimum account age are options if this becomes real.
- **Card names.** Heroes go by card id until the card data reaches the site (D-074, T-504).

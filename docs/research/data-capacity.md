# Data capacity before real users (T-537)

Research only, session 097 (2026-10-12). Nothing in the code, the migrations or the dashboards was changed. The choice that follows touches user data and money (R2/R3), so the user decides; the decision number reserved for the answer is **D-109**.

Every number below is marked **measured** (counted on the user's own history or read in this repo), **read** (on a provider's page, with the date) or **assumed** (a guess, stated so it can be replaced).

## 1. What a game weighs

### 1.1 On the PC (measured)

Source: the user's own `%APPDATA%\TavernLedger\games.jsonl`, read only on 2026-10-12. Only counts and sizes were printed: no names, BattleTags or ids. 147 games (145 `ok`, 2 `incomplete`): 20 Solo, 127 Duos. They span parser revisions 2 to 8 (46 games with no revision, 26 at r2, 21 at r3, 11 at r4, 11 at r5, 23 at r6, 1 at r7, 8 at r8), so most were saved before the shop data (r3) or the combat replay (r8) existed and are smaller than a game saved today.

"Raw" is the record as the app uploads it (compact JSON, `crates/uploader/src/queue.rs` `body_of`); "gz" is the same bytes gzipped at the default level, which is what the app sends (`flate2` default) and about what the server stores (it re-compresses with `CompressionStream("gzip")`).

| Games | n | Raw, median (max) | Gz, median (max) |
|---|---|---|---|
| Duos, r8, with replays | 8 | 132,875 B (196,198 B) | **14,965 B (21,278 B)** |
| Same 8 games with `rounds[].replay` removed | 8 | 47,959 B (82,170 B) | 6,519 B (10,290 B) |
| Duos, r6 (shop data, no replay) | 23 | 51,849 B | 6,982 B (10,288 B) |
| Duos, r3 to r5 | 35 | 44,191 to 57,070 B | 6,103 to 7,517 B |
| Solo, r3 to r5 | 8 | 24,481 to 30,187 B | 4,040 to 4,733 B (max 5,016 B) |
| Solo or Duos, before r3 | 72 | 8,592 to 21,766 B | 1,479 to 2,853 B |

No Solo game at r8 is in the history, so a Solo game with replays is **not measured**; it is assumed to be no larger than a Duos one (one board per combat instead of a teammate coming in).

Where the bytes go in the 8 r8 Duos games (share of raw): replays 62.0 %, the rest of `rounds` (boards, health, opponents) 16.0 %, `shop.turns` 9.8 %, `shop.actions` 6.7 %, `discovers` 2.6 %, `lobby` 1.4 %, everything else 1.4 %. Gzipped, the replay is 45 % to 63 % of each file. Replay bytes per round: 5,100 to 18,814 B raw, median about 6,600 B.

So: **a game saved today is about 15 KB gzipped with its replays and about 7 KB without**; the largest seen is 21 KB. The whole history is 5.57 MB raw on the PC and would be 0.72 MB gzipped.

Other compressions on the same 8 games (measured): gzip level 9 gives 13,435 B (7 % less), xz level 9 gives 11,026 B (24 % less). xz and brotli are not in the Edge runtime's `CompressionStream` nor in every browser's `DecompressionStream`, so a format change would need a library on both sides.

How often the user plays (measured, small sample): the games that carry a play time fall on 5 days, from 4 to 36 games a day, median 14.

### 1.2 On the server (read in this repo)

What one upload leaves behind (`supabase/functions/upload-game/handler.ts`, migrations under `supabase/migrations/`):

- **One file in Storage**, bucket `games`, `<user_id>/<session>-<index>.json.gz`: the full checked record, gzipped. The bucket refuses files over 64 KiB (`file_size_limit 65536`), the function refuses bodies over 64 KiB gzipped or 1 MiB inflated. So the full report is kept, compressed. Replays go up with it (validator 7).
- **A row in `public.games`** (summary: mode, status, hero, place, build, date, tribes offered as JSONB, revision, SHA-256, size, generated `storage_path`) with its primary key and the index `games_user_played`.
- **A row in `public.game_stats`** (about 45 numeric columns plus `final_board` and `heroes_offered` arrays) with its primary key, filled from the file at upload or by the sweep's backfill.
- **A row in `storage.objects`** for the file. That table lives in the same Postgres database, so its rows and indexes count toward database size, not file storage.
- **A row in `private.upload_events`**, deleted after a day.
- Derived tables that do **not** grow per game: `private.leaderboard_rows` (per board, period and public player), `private.community_hero_rows` (per mode, period and hero), both rebuilt every 10 minutes. `private.sweep_runs` holds one row a day, kept 90 days.

Per-user limits already in `upload_begin()` (migration `20261009150000_upload.sql`): 600 uploads an hour, 5,000 a day, **10,000 games and 50 MiB of files in total**. Past either total the upload is refused (`quota_games` / `quota_bytes`, HTTP 403).

The database bytes per game were measured on the hosted project with the query in section 7 (section 7.1): 41 games, a 13.3 MB database, and 409,600 B in `public.games`, `public.game_stats` and `storage.objects` together, about 10 KB per game. With so few games that figure is mostly each table's and index's fixed pages (Postgres stores 8 KB pages, and every table and index takes at least one), so it is an upper bound. The rows themselves are estimated at about 2.5 KB per game (**assumed**: about 300 B in `games`, 400 B in `game_stats` and 400 B in `storage.objects`, plus their indexes, `storage.objects` having several). The files average 7,137 B on the server today, matching the older, smaller games uploaded so far.

The privacy draft and `docs/legal/data-inventory.md` (row S8) say game files are kept "while the account exists". Any retention window changes that line.

## 2. Limits (read)

### 2.1 Supabase

The project is on the **Free plan** (deploy.md section 10). Read on 2026-10-12 on [supabase.com/pricing](https://supabase.com/pricing), [Database size](https://supabase.com/docs/guides/platform/database-size), [Cost control](https://supabase.com/docs/guides/platform/cost-control), [Billing FAQ](https://supabase.com/docs/guides/platform/billing-faq), [Egress](https://supabase.com/docs/guides/platform/manage-your-usage/egress) and [Edge Function limits](https://supabase.com/docs/guides/functions/limits).

| Item | Free ($0) | Pro (from $25 a month) | Pro over the quota |
|---|---|---|---|
| Database size | 500 MB | 8 GB disk per project | $0.125 per GB |
| File storage | 1 GB | 100 GB | $0.0213 per GB |
| Egress (uncached / cached) | 5 GB / 5 GB | 250 GB / 250 GB | $0.09 / $0.03 per GB |
| Edge Function invocations | 500,000 | 2 million | $2 per million |
| Monthly active users | 50,000 | 100,000 | $0.00325 per MAU |
| Max upload size | 50 MB | 500 GB | |
| Backups | none | daily, kept 7 days | |
| Pausing | after 1 week of inactivity | never | |
| Log retention | 1 day | 7 days | |

Edge Functions on every plan: 256 MB memory, 2 s of CPU per request, 150 s wall clock on Free (400 s paid), 100 functions per project on Free.

What happens at a limit:

- **Database on Free:** "Free Plan projects enter read-only mode when your database size exceeds 500 MB." Database size is "data, indexes, and materialized views"; WAL is not counted. Recovery: upgrade, or leave read-only mode and shrink the data.
- **Database on Pro:** the disk grows by 50 % when it reaches 90 % full (at most four times in 24 hours); at 95 % with no resize left, read-only until under 95 %. With the spend cap on (the default) it stops at the 8 GB included.
- **Any other Free quota:** a grace period of unstated length, then the Fair Use Policy, whose restrictions include "Pausing projects", "Switching databases to read-only mode" and "Responding with a 402 status code for all API requests". No second grace period; the warning stays until the project is under the limits for several billing cycles. For this project that would mean uploads, sign-in and the website's data all failing at once.
- **Pro with the spend cap on:** "further usage of that item is disallowed until the next billing cycle"; with the cap off, overage is billed per unit.
- **Egress** counts data sent to clients by the database, Auth, Storage (every file the game viewer or the export downloads) and Edge Functions. Uploads are ingress and do not count.

### 2.2 Cloudflare Pages

Read on 2026-10-12 on [Pages limits](https://developers.cloudflare.com/pages/platform/limits/): on Free, 500 builds a month, one build at a time, 20 minutes a build, 20,000 files and 25 MiB per file per site, 100 custom domains a project. The page sets no bandwidth limit for static assets. The website is static (Astro) and reads games straight from Supabase in the browser, so game data never passes through Cloudflare and none of these limits grows with players or games. Pages Functions would count against the Workers quota, but the site uses none.

## 3. Projection

Assumptions (all **assumed** unless marked):

- An active player plays **5 games a day** on average; a heavy player 14 (the user's own median on the days they played, measured).
- One file is **15 KB** with replays, **7 KB** without (measured medians, rounded up).
- The database holds **2.5 KB** per game (assumed; the measured upper bound is 10 KB, section 1.2) on top of a base of **13.3 MB** that does not grow with games (measured, section 7.1), leaving about 487 MB of the Free plan's 500 MB.
- Each active player downloads 5 game files and 100 KB of lists and stats a day: about **5.25 MB of egress a month** (assumed).
- A month is 30 days: 150 games per active player.

Growth per active player and month: 2.25 MB of files with replays (1.05 MB without), 0.375 MB of database, 5.25 MB of egress, about 150 Edge Function calls.

**Time until the Free plan's first limit, keeping everything forever:**

| Active players | Files, 1 GB, with replays | Files, without replays | Database, 500 MB | Egress, 5 GB a month |
|---|---|---|---|---|
| 10 | about 3.7 years | about 8 years | about 11 years | 1 % used |
| 100 | **about 4.5 months** | about 9.5 months | about 13 months | 10 % used |
| 1,000 | **about 2 weeks** | about 4 weeks | about 5.5 weeks | **over (105 %)** |

If the database really took the measured upper bound of 10 KB per game, its column would read about 2.7 years, **3.2 months** and 10 days, making it the first wall at 100 players. The storage alert (option A) watches both, and the query in section 7 should be run again at a few thousand games to settle the real figure.

Edge Function calls stay under the quota in all three (1,000 players: about 150,000 of 500,000 a month). Monthly active users are far from 50,000.

**Per player, with today's 50 MiB total:** about 3,500 games with replays, so an average player is refused after about 23 months and a heavy one after about **8 months**. That cap is already, in effect, a retention policy the player is not told about.

**On Pro** (100 GB files, 8 GB disk, 250 GB egress): 1,000 active players keeping everything with replays use about 27 GB of files and 4.5 GB of database in a year, both inside the $25. Kept forever, one player's year of files (27 MB) would cost about $0.0006 a month in Pro storage overage. **Storage costs little on Pro; the hard wall is the Free plan.**

**With a 90-day window** for the files (summaries kept): files stop growing at about 6.75 MB per active player with replays (3.15 MB without), so the Free plan's 1 GB holds about **150 active players with replays** (about 320 without), and accounts that stop playing empty themselves. The database keeps growing by about 0.375 MB per active player and month and becomes the Free plan's limit (100 players: about 13 months).

## 4. Options

| Option | Cost | Effort | What the player loses | D-002 / D-003 | GDPR (rule 6) |
|---|---|---|---|---|---|
| **A. Storage alert from the daily sweep** | none | small: the sweep reads bucket bytes, file count and database size, records them in `private.sweep_runs` and flags 50 % and 80 % of the Free quotas; the hub's weekly check reads it (deploy.md 10.5) until there is real alerting | nothing | neutral | no new personal data (totals only) |
| **B. Replays kept on the PC only** | none | small: the app strips `rounds[].replay` before upload, the validator accepts its absence | the website replay (T-536) never works; replays are lost if the PC is lost | goes against D-003 ("replays with a limit" are free) and leaves T-536 without data | less data uploaded (data minimisation) |
| **C. Retention window** (full file N days, then only the summary rows) | none | medium: a sweep step deletes files older than N days through the Storage API; upload refuses files for games older than N days (or a reparse would bring them back); the viewer says "older than N days: summary only"; privacy policy and data inventory S8 updated | opening an old game on the website (its boards, shop, replay); the app keeps every game on the PC, so the stats in the app lose nothing, and the website's stats and leaderboards come from the summary rows, which stay | fits: the free tier keeps a limit, the paid extra (E) lifts it | better: a stated, shorter retention |
| **C2. Replay-only window** (replays removed from the file after N days, the rest kept) | none | larger than C: the sweep re-reads, strips and rewrites each file | old replays on the website | fits D-003's wording closely | as C, partly |
| **D. Compressing harder** | none | medium: xz or brotli on both sides, a new file format and a migration of old files | nothing | neutral | neutral |
| **E. Long history as a paid extra** | brings money in; payments are R3 and do not exist yet | large (payments, plans, terms) | nothing for free players beyond C | the D-002/D-003 model exactly | needs terms and the policy to say how long paid history lasts after the plan ends |
| **F. Move to Pro** | $25 a month | trivial (billing, by the user) | nothing; adds daily backups and no pausing | neutral | backups add a 7-day copy to state in the policy |

D saves 7 % (gzip 9) to 24 % (xz) and is not worth a new format. C2 costs more than C for a small gain.

## 5. Check of hub 061's recommendation

Hub 061: replays on the PC only and the storage alert now; a 90-day retention when sign-ups open; long history as a paid extra later.

- **Storage alert now: agree.** It is cheap and is the only way to see the Free plan's walls coming.
- **Replays on the PC only: disagree.** Replays are about half of each file, so dropping them doubles the Free plan's runway, but that is a few months at 100 players and two weeks at 1,000; it costs T-536 (the website replay, planned) and the free "replays with a limit" of D-003, and at 10 players nothing is near a limit. A retention window bounds storage better and keeps the feature. Keep this option in reserve if the alert fires before C is built.
- **90-day retention when sign-ups open: agree**, with two additions: the upload must refuse files for games older than the window (or reparsed old games come back), and the privacy policy and data inventory must say "90 days" before it starts. It also replaces today's silent 50 MiB refusal for heavy players.
- **Long history as a paid extra later: agree**, framed honestly: kept files cost almost nothing on Pro, so the extra is about the product, not about covering storage.
- **Missing: when to move to Pro.** The real wall is the Free plan, not money. With 100 active players the Free plan's files last about 4.5 months even before any retention; $25 a month buys 100 GB, backups and no pausing.

## 6. Questions for the user (D-109)

One at a time, each with a recommendation.

1. **Should the daily sweep record how full the project is (file bytes, file count, database size) and flag 50 % and 80 % of the Free plan's limits?** Recommended: yes, now. No personal data, small change.
2. **Should combat replays keep going up with each game (for the website replay, T-536), or stay on the PC only?** Recommended: keep uploading them; retention (question 3) keeps them bounded.
3. **How long should the server keep each game's full file for a free account, with its summary kept while the account exists?** Recommended: 90 days, starting when sign-ups open, with the privacy policy and data inventory updated first. The app keeps every game on the PC either way.
4. **When should the project move to Supabase Pro ($25 a month)?** Recommended: when the storage alert reaches 80 % of any Free limit, or before passing about 100 active players, whichever comes first; the user pays and switches, never Claude.
5. **Should a longer history (full files beyond 90 days) become a paid extra later?** Recommended: yes, but not built until payments exist (R3); until then the 90 days apply to everyone.
6. **Should the stored file be compressed harder (xz or brotli)?** Recommended: no; it saves 7 % to 24 % for a new format on both sides.

### 6.1 Built: storage alert (T-543, session 098)

Answer 1 is built: each daily sweep stores the database size, the `games` bucket's file bytes and file count, and the level reached against each Free limit (`none`, `50%`, `80%`) on its row of `private.sweep_runs`, and its answer carries the same numbers. The limits of section 2.1 live in `private.capacity_limits()` (migration `20261012150000_storage_alert.sql`). The hub's query and what it does at each level: [deploy.md](deploy.md) section 10.5.

## 7. Query for the hosted project (read only)

Run once by the hub on 2026-10-12; results in 7.1. Run it again at a few thousand games to measure the database bytes per game. It returns only totals and sizes, no ids or names. To run in the Supabase SQL editor:

```sql
select 'database' as what, pg_database_size(current_database()) as bytes, null::bigint as rows
union all
select n.nspname || '.' || c.relname, pg_total_relation_size(c.oid), c.reltuples::bigint
from pg_class c join pg_namespace n on n.oid = c.relnamespace
where (n.nspname, c.relname) in (
  ('public', 'games'), ('public', 'game_stats'), ('public', 'profiles'),
  ('storage', 'objects'), ('private', 'leaderboard_rows'),
  ('private', 'community_hero_rows'), ('private', 'sweep_runs'),
  ('private', 'upload_events'), ('auth', 'users'), ('auth', 'audit_log_entries')
)
union all
select 'games bucket files', coalesce(sum((o.metadata->>'size')::bigint), 0), count(*)
from storage.objects o where o.bucket_id = 'games'
order by 1;
```

Database bytes per game = (`public.games` + `public.game_stats` + `storage.objects`) bytes divided by the games bucket's file count, once there are enough games for the fixed overhead of each table to matter little.

### 7.1 Results (hosted project, 2026-10-12 about 02:15 local, run by the hub)

Row counts are Postgres's estimates (`reltuples`; "-" where the table was never analysed). 41 games is too few for per-game figures: each table's and index's fixed pages dominate.

| What | Bytes | Rows |
|---|---|---|
| database | 13,276,851 | |
| games bucket files | 292,626 | 41 |
| `public.games` | 90,112 | 41 |
| `public.game_stats` | 114,688 | 41 |
| `storage.objects` | 204,800 | 29 (stale estimate) |
| `public.profiles` | 32,768 | - |
| `auth.users` | 221,184 | 1 |
| `auth.audit_log_entries` | 81,920 | - |
| `private.upload_events` | 24,576 | 57 |
| `private.sweep_runs` | 49,152 | - |
| `private.leaderboard_rows` | 24,576 | - |
| `private.community_hero_rows` | 16,384 | - |

Read: the database's base is 13.3 MB (2.7 % of the Free plan's 500 MB); game files use 0.29 MB (0.03 % of 1 GB), 7,137 B per file on average; the three per-game tables hold 409,600 B, about 10 KB per game as an upper bound.

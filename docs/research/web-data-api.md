# Website data API (T-502, D-071)

What the website reads from Supabase to show a player's games and stats, without downloading every report into the browser. Schema: [`supabase/migrations/20261011120000_game_stats.sql`](../../supabase/migrations/20261011120000_game_stats.sql). Summary code: [`supabase/functions/_shared/game_summary.ts`](../../supabase/functions/_shared/game_summary.ts).

## Rules

- **Readable like `games`.** Every table and view below is readable by the owner of a row, and by everyone once the owner's profile is public (`profiles.is_public`). The views are `security_invoker`, so row-level security is the caller's, never the view owner's. A private profile is invisible to anonymous callers and to other users (database and end-to-end tests).
- **Nothing more public than before.** Each value comes from the game row and the report file, both already readable on the same terms. No name, BattleTag, rating, rank, email or account id; a test fails if a column with such a name appears in the public schema.
- **Unknown is null, never zero.** A value the report cannot back (an older parser revision, a log that lacks it, an unfinished game) is null. Every total in a view comes with the number of games behind it (`*_games`, `games`), so the site can say "in 14 of 29 games".
- **Inferred is marked.** Combat results come from the log; when the log has none they are worked out from health (`combats_inferred` counts those). Damage taken and dealt are health drops, so always inferred; `user_records.inferred` says so per record.
- **Clients only read.** Writes go through the upload function and the daily sweep with the service role.

## How the summary is filled

1. **At upload.** `upload-game` validates the record (validator 6, unchanged), works out the summary with `summarize()` and passes it to `upload_commit(..., p_stats, p_summary_version)`, which writes the game row and its `game_stats` row in one transaction. An older copy changes neither.
2. **Games stored before.** The daily sweep (`sweep`, 03:17 UTC) lists games with no summary or one older than `SUMMARY_VERSION` (`game_stats_backfill_list`), reads each stored file through the Storage API, checks its SHA-256 against the row and runs `validateRecord()` again, then stores the summary (`game_stats_backfill_put`, which refuses it if another copy was uploaded meanwhile). At most 200 games per run; its answer has `stats: {listed, stored, skipped, failed}`. Nothing is typed by hand in production: the first sweep after the merge fills the existing games.
3. **A new summary version.** Raising `SUMMARY_VERSION` makes the sweep fill every game again with the new code.
4. **Deploy order.** `upload_commit`'s two new arguments have defaults, so the function deployed before this change keeps working; a game it replaces loses its stale summary and the sweep makes a new one.

## Tables and views

### `game_stats` (one row per game)

Key `(user_id, session, game_index)`, foreign key to `games` (deleted with it). `summary_version`, `computed_at`, then:

| Column | Meaning |
|--------|---------|
| `teammate_hero` | Duos teammate's hero card id |
| `rounds` | Rounds the report holds |
| `combats_won`, `combats_lost`, `combats_tied`, `combats_unknown` | One result per round; they add up to `rounds` |
| `combats_inferred` | Known results worked out from health, not from the log |
| `win_streak`, `win_streak_inferred` | Most combats won in rounds in a row (null when every result is unknown) |
| `damage_taken`, `damage_taken_combats`, `damage_taken_max` | Own health lost per combat, over the combats with health before and after (inferred) |
| `damage_dealt`, `damage_dealt_combats`, `damage_dealt_max` | The one opposing side's health lost per round (inferred; in Duos the team shares one health) |
| `shop_turns`, `final_tier` | Turns of the shop record, highest tavern tier |
| `gold_spent` + `gold_spent_turns` | Gold spent, summed over the turns that have it (of `shop_turns`) |
| `gold_beyond_income` + `gold_beyond_turns` | Extra gold plus sell gold, over the turns that have both |
| `sell_gold` + `sell_gold_turns` | Gold from selling |
| `rolls`, `free_rolls`, `buys`, `spell_buys`, `sells`, `freezes` | Shop totals |
| `actions`, `apm` | Logged actions and actions per minute (D-046); null when none is logged |
| `triples` | The leaderboard's own count at the end of a finished game (D-058) |
| `final_board`, `final_board_round` | The player's last board seen entering a combat, card ids (null: card unknown) |
| `board_stats_max`, `golden_max` | Largest attack + health total and most golden minions on the player's board |
| `heroes_offered`, `hero_rerolls` | The hero choice (parser revision 4) |

### `game_summaries` (the games list)

One row per game: `user_id`, `session`, `game_index`, `mode` (`solo` or `duos`), `game_type`, `status`, `played_on`, `hero_card_id`, `hero` (skins grouped under the base hero by the D-019 card id rule), `final_place`, `counted_place` (finished games only), `top_half` (top 4 in Solo, top 2 in Duos), `won`, `build`, `parser_version`, `tribes_offered`, `storage_path` and every `game_stats` column (null when the game has no summary yet).

### `user_overview` (per user and mode)

`games`, `incomplete_games`, `unsupported_games`, `placed_games`, `avg_place`, `top_half`, `top_half_pct`, `wins`, `win_pct`, `first_played_on`, `last_played_on`, `summarized_games`; combat totals with `combat_games`; `damage_dealt`/`damage_taken` with their `*_games`; `gold_spent`, `gold_beyond_income`, `sell_gold` with their `*_games`; `rolls`, `free_rolls`, `buys`, `sells` with `shop_games`; `avg_apm` with `apm_games`; `triples` with `triples_games`. Places and rates count finished games only (`placed_games` is the denominator).

### `user_heroes` (per user, mode and hero)

`games`, `placed_games`, `avg_place`, `top_half`, `top_half_pct`, `wins`, `best_place`, `worst_place`, `offered_games` (games whose known hero choice offered this hero) and `picked_games` (games played with it after a known choice). Pick rate = `picked_games / offered_games`.

### `user_records` (per user, mode and record)

`record`, `value`, the game (`session`, `game_index`, `played_on`; the first game that reached the value), `games` (games that back the record) and `inferred`. Records: `win_streak`, `damage_dealt_max`, `damage_taken_max`, `board_stats_max`, `golden_max`, `gold_spent`, `gold_beyond_income`, `rolls`, `triples`, `apm`, `rounds`, `top_half_streak` (most top finishes in a row over finished games; the game is the run's last). A record no game backs has no row.

## Calling it from the site

The site already talks to the project's REST API with the publishable key (CSP `connect-src`). With `@supabase/supabase-js` or plain `fetch`; signed in, add the user's access token as the bearer, otherwise the publishable key reads public profiles only.

```js
// Overview of one profile (yours, or a public one).
const { data } = await supabase.from("user_overview").select("*").eq("user_id", userId);

// Games list: Solo, one hero, a date range, top 4 only, 25 a page, with the total.
const { data: games, count } = await supabase
  .from("game_summaries")
  .select("session,game_index,played_on,hero,final_place,rounds,triples,apm", { count: "exact" })
  .eq("user_id", userId)
  .eq("mode", "solo")
  .eq("hero", "BG20_HERO_202")
  .gte("played_on", "2026-10-01")
  .lte("played_on", "2026-10-31")
  .lte("counted_place", 4)
  .order("played_on", { ascending: false })
  .order("session", { ascending: false })
  .order("game_index", { ascending: false })
  .range(0, 24);
```

Plain HTTP: `GET <project>/rest/v1/game_summaries?user_id=eq.<id>&mode=eq.solo&order=played_on.desc,session.desc,game_index.desc&limit=25&offset=0` with headers `apikey: <publishable key>`, `Authorization: Bearer <access token or publishable key>` and `Prefer: count=exact` for the total (in `Content-Range`). The API caps a page at 1000 rows.

One game in full (the game viewer, T-504) still reads its report file: `storage/v1/object/authenticated/games/<storage_path>`, readable on the same terms.

## Left in the report files

Not summarized, read per game by the viewer when needed: opponents' boards, keywords and trinkets, the lobby's triples, every discover and Dark Gift, the shop's offers per roll, the Old God. **Top tribes on the final board** need each minion's tribe, which is in the card data, not the report: the site can map `final_board` card ids with the card data (T-504's pictures decision) instead of the server guessing.

## Leaderboards later (T-506)

Not built here. They can rank `game_stats` joined with `games` over public profiles only (a materialized view refreshed by a schedule, with minimum game counts), using the same columns; the user's decisions in the master plan come first.

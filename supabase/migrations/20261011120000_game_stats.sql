-- T-502 (D-071): the website's data API.
--
-- * public.game_stats: one summary row per game, worked out by the
--   upload-game function from the validated report (supabase/functions/
--   _shared/game_summary.ts) and written in the same transaction as the game
--   row. Every value is null when the report cannot back it: unknown, never
--   zero. Games stored before this have no row; the daily sweep fills them
--   from the stored reports (game_stats_backfill_list/_put), and again for
--   every game when the summary's version goes up.
-- * Views for the pages, all `security_invoker`, so the caller's row-level
--   security applies exactly as on public.games: a row is readable by its
--   owner, and by everyone once the owner's profile is public. They hold
--   nothing that is not already readable today (the summary row and the
--   report file are as public as the game row), and no name, BattleTag,
--   rating or account id.
--     game_summaries  one row per game with its summary (the games list)
--     user_overview   per user and mode: games, places, totals
--     user_heroes     per user, mode and hero: games, places, picks
--     user_records    per user, mode and record: the best value and its game
--   Every value carries how many games back it.
-- * Export and account deletion cover the new table; a summary goes with its
--   game (foreign key), so the sweep's clean-up of files with no game row
--   cannot leave one behind.
-- Leaderboards (T-506) are not here: they need user decisions, and can be
-- built on game_stats over public profiles only.

-- ---------------------------------------------------------------------------
-- public.game_stats
-- ---------------------------------------------------------------------------

create table public.game_stats (
  user_id uuid not null,
  session text not null,
  game_index integer not null,
  -- SUMMARY_VERSION of the code that wrote the row (game_summary.ts).
  summary_version smallint not null check (summary_version between 1 and 1000),
  computed_at timestamptz not null default now(),
  teammate_hero text check (teammate_hero ~ '^[A-Za-z0-9_]{1,64}$'),
  -- Rounds the report holds, and the result of each: from the log, or worked
  -- out from health (combats_inferred of the known ones), or unknown.
  rounds smallint check (rounds between 1 and 1001),
  combats_won smallint check (combats_won between 0 and 1001),
  combats_lost smallint check (combats_lost between 0 and 1001),
  combats_tied smallint check (combats_tied between 0 and 1001),
  combats_unknown smallint check (combats_unknown between 0 and 1001),
  combats_inferred smallint check (combats_inferred between 0 and 1001),
  win_streak smallint check (win_streak between 0 and 1001),
  win_streak_inferred boolean,
  -- Health drops between rounds in a row (inferred: health paid or healed in
  -- the shop counts too), with the number of combats that back them.
  damage_taken integer check (damage_taken >= 0),
  damage_taken_combats smallint check (damage_taken_combats between 1 and 1001),
  damage_taken_max integer check (damage_taken_max >= 0),
  damage_dealt integer check (damage_dealt >= 0),
  damage_dealt_combats smallint check (damage_dealt_combats between 1 and 1001),
  damage_dealt_max integer check (damage_dealt_max >= 0),
  -- The player's own shop (parser revision 3 and later). Gold sums count only
  -- the turns that have the number (the *_turns columns, of shop_turns).
  shop_turns smallint check (shop_turns between 1 and 100),
  final_tier smallint check (final_tier between 1 and 7),
  gold_spent integer check (gold_spent >= 0),
  gold_spent_turns smallint check (gold_spent_turns between 1 and 100),
  gold_beyond_income integer check (gold_beyond_income >= 0),
  gold_beyond_turns smallint check (gold_beyond_turns between 1 and 100),
  sell_gold integer check (sell_gold >= 0),
  sell_gold_turns smallint check (sell_gold_turns between 1 and 100),
  rolls integer check (rolls >= 0),
  free_rolls integer check (free_rolls >= 0),
  buys integer check (buys >= 0),
  spell_buys integer check (spell_buys >= 0),
  sells integer check (sells >= 0),
  freezes integer check (freezes >= 0),
  -- Logged actions and actions per minute (D-046); null when none is logged.
  actions integer check (actions between 1 and 3000),
  apm numeric(7, 2) check (apm >= 0),
  -- The leaderboard's own count at the end of a finished game (D-058).
  triples smallint check (triples between 0 and 1000),
  -- The local player's last board seen entering a combat, by card id (null:
  -- a minion whose card the log did not give).
  final_board text[] check (
    cardinality(final_board) <= 16
    and coalesce(array_ndims(final_board), 1) = 1
    and array_to_string(final_board, ',', '') ~ '^([A-Za-z0-9_]{1,64})?(,([A-Za-z0-9_]{1,64})?)*$'
  ),
  final_board_round smallint check (final_board_round between 0 and 1000),
  board_stats_max integer check (board_stats_max >= 0),
  golden_max smallint check (golden_max between 0 and 16),
  -- The player's hero choice (parser revision 4).
  heroes_offered text[] check (
    cardinality(heroes_offered) <= 32
    and coalesce(array_ndims(heroes_offered), 1) = 1
    and array_position(heroes_offered, null) is null
    and array_to_string(heroes_offered, ',') ~ '^([A-Za-z0-9_]{1,64}(,[A-Za-z0-9_]{1,64})*)?$'
  ),
  hero_rerolls smallint check (hero_rerolls between 0 and 32),
  primary key (user_id, session, game_index),
  foreign key (user_id, session, game_index)
    references public.games (user_id, session, game_index) on delete cascade,
  check (combats_won + combats_lost + combats_tied + combats_unknown = rounds),
  check (combats_inferred <= combats_won + combats_lost + combats_tied),
  check (win_streak <= combats_won),
  check (damage_taken_max <= damage_taken),
  check (damage_dealt_max <= damage_dealt),
  check (free_rolls <= rolls),
  check (gold_spent_turns <= shop_turns),
  check (gold_beyond_turns <= shop_turns),
  check (sell_gold_turns <= shop_turns)
);

comment on table public.game_stats is
  'Summary of one uploaded game, worked out by the server from the validated report (T-502, D-071). Null is unknown, never zero. Written only by the service role.';

alter table public.game_stats enable row level security;
revoke all on public.game_stats from anon, authenticated;
grant select on public.game_stats to anon, authenticated;

create policy game_stats_read_own_or_public on public.game_stats
  for select to anon, authenticated
  using (
    user_id = (select auth.uid())
    or exists (
      select 1 from public.profiles p
      where p.user_id = game_stats.user_id and p.is_public
    )
  );

-- No insert, update or delete policy: clients cannot write.

-- The games list reads a user's games newest first.
create index games_user_played
  on public.games (user_id, played_on desc, session desc, game_index desc);

-- ---------------------------------------------------------------------------
-- Writing a summary (service role only, through the functions below)
-- ---------------------------------------------------------------------------

-- Replaces the summary of one game. p_stats holds only game_stats columns
-- (a missing one is unknown); a key that is not a value column is refused,
-- so it can never name another row or carry anything the table does not have.
create function private.put_game_stats(
  target uuid,
  p_session text,
  p_game_index integer,
  p_version integer,
  p_stats jsonb
)
returns void
language plpgsql
security definer
set search_path = ''
as $$
declare
  bad text;
begin
  if p_version is null or jsonb_typeof(p_stats) is distinct from 'object' then
    raise exception 'put_game_stats: a summary is an object with a version' using errcode = '22023';
  end if;
  select k into bad
  from jsonb_object_keys(p_stats) as k
  where k not in (
    select a.attname from pg_catalog.pg_attribute a
    where a.attrelid = 'public.game_stats'::regclass and a.attnum > 0 and not a.attisdropped
  ) or k in ('user_id', 'session', 'game_index', 'summary_version', 'computed_at')
  limit 1;
  if bad is not null then
    -- The key itself stays out of the message: it came from the caller.
    raise exception 'put_game_stats: the summary has a key that is not a value column'
      using errcode = '22023';
  end if;

  delete from public.game_stats s
  where s.user_id = target and s.session = p_session and s.game_index = p_game_index;
  insert into public.game_stats
  select (jsonb_populate_record(
    null::public.game_stats,
    p_stats || jsonb_build_object(
      'user_id', target,
      'session', p_session,
      'game_index', p_game_index,
      'summary_version', p_version,
      'computed_at', now()
    )
  )).*;
end;
$$;

revoke all on function private.put_game_stats(uuid, text, integer, integer, jsonb)
  from public, anon, authenticated, service_role;

-- As in 20261009150000_upload.sql, plus the summary: with p_stats the summary
-- is written with the game row; without it (the upload function deployed
-- before T-502 still calls with the first 14 arguments) a replaced game drops
-- its stale summary and the sweep fills a new one. An older copy changes
-- neither. Returns created | replaced | older.
drop function public.upload_commit(
  uuid, text, integer, text, text, text, integer, integer, date, jsonb, bigint, text, integer, text
);

create function public.upload_commit(
  target uuid,
  p_session text,
  p_game_index integer,
  p_game_type text,
  p_status text,
  p_hero_card_id text,
  p_final_place integer,
  p_build integer,
  p_played_on date,
  p_tribes_offered jsonb,
  p_saved_at bigint,
  p_sha256 text,
  p_size integer,
  p_parser_version text,
  p_stats jsonb default null,
  p_summary_version integer default null
)
returns text
language plpgsql
security definer
set search_path = ''
as $$
declare
  inserted boolean;
begin
  insert into public.games as g (
    user_id, session, game_index, game_type, status, hero_card_id, final_place,
    build, played_on, tribes_offered, saved_at, content_sha256, size_bytes, parser_version
  ) values (
    target, p_session, p_game_index, p_game_type, p_status, p_hero_card_id, p_final_place,
    p_build, p_played_on, p_tribes_offered, p_saved_at, p_sha256, p_size, p_parser_version
  )
  on conflict (user_id, session, game_index) do update set
    game_type = excluded.game_type,
    status = excluded.status,
    hero_card_id = excluded.hero_card_id,
    final_place = excluded.final_place,
    build = excluded.build,
    played_on = excluded.played_on,
    tribes_offered = excluded.tribes_offered,
    saved_at = excluded.saved_at,
    content_sha256 = excluded.content_sha256,
    size_bytes = excluded.size_bytes,
    parser_version = excluded.parser_version
  where g.saved_at <= excluded.saved_at
  returning (xmax = 0) into inserted;

  if inserted is null then
    return 'older';
  end if;
  if p_stats is null then
    delete from public.game_stats s
    where s.user_id = target and s.session = p_session and s.game_index = p_game_index;
  else
    perform private.put_game_stats(target, p_session, p_game_index, p_summary_version, p_stats);
  end if;
  return case when inserted then 'created' else 'replaced' end;
end;
$$;

revoke all on function public.upload_commit(
  uuid, text, integer, text, text, text, integer, integer, date, jsonb, bigint, text, integer, text,
  jsonb, integer
) from public, anon, authenticated;
grant execute on function public.upload_commit(
  uuid, text, integer, text, text, text, integer, integer, date, jsonb, bigint, text, integer, text,
  jsonb, integer
) to service_role;

-- Games whose summary is missing or older than p_version, for the sweep: at
-- most p_limit (capped at 500) per call. content_sha256 names the stored copy.
create function public.game_stats_backfill_list(p_version integer, p_limit integer)
returns table (
  user_id uuid,
  session text,
  game_index integer,
  storage_path text,
  content_sha256 text
)
language sql
stable
security definer
set search_path = ''
as $$
  select g.user_id, g.session, g.game_index, g.storage_path, g.content_sha256
  from public.games g
  left join public.game_stats s
    on s.user_id = g.user_id and s.session = g.session and s.game_index = g.game_index
  where s.user_id is null or s.summary_version < p_version
  order by g.user_id, g.session, g.game_index
  limit least(greatest(coalesce(p_limit, 0), 0), 500);
$$;

-- Stores a summary the sweep worked out from the stored file whose SHA-256
-- is p_sha256. Returns stored, or why not: missing (the game is gone),
-- changed (another copy was uploaded meanwhile), newer (a summary of a newer
-- version is there).
create function public.game_stats_backfill_put(
  target uuid,
  p_session text,
  p_game_index integer,
  p_sha256 text,
  p_summary_version integer,
  p_stats jsonb
)
returns text
language plpgsql
security definer
set search_path = ''
as $$
declare
  sha text;
  version integer;
begin
  -- Locks the game row, so an upload of the same game waits (or this does).
  select g.content_sha256 into sha
  from public.games g
  where g.user_id = target and g.session = p_session and g.game_index = p_game_index
  for update;
  if sha is null then
    return 'missing';
  end if;
  if sha <> p_sha256 then
    return 'changed';
  end if;
  select s.summary_version into version
  from public.game_stats s
  where s.user_id = target and s.session = p_session and s.game_index = p_game_index;
  if version > p_summary_version then
    return 'newer';
  end if;
  perform private.put_game_stats(target, p_session, p_game_index, p_summary_version, p_stats);
  return 'stored';
end;
$$;

revoke all on function public.game_stats_backfill_list(integer, integer)
  from public, anon, authenticated;
revoke all on function public.game_stats_backfill_put(uuid, text, integer, text, integer, jsonb)
  from public, anon, authenticated;
grant execute on function public.game_stats_backfill_list(integer, integer) to service_role;
grant execute on function public.game_stats_backfill_put(uuid, text, integer, text, integer, jsonb)
  to service_role;

-- ---------------------------------------------------------------------------
-- Views for the website. security_invoker: the caller's rights and row-level
-- security apply, never the owner's.
-- ---------------------------------------------------------------------------

-- One row per game. `hero` groups skins under their base hero by the card id
-- rule (D-019: <hero>_SKIN_<letters>); the site can regroup by the card data.
-- counted_place: the place of a finished game, else null. top_half: top 4 in
-- Solo, top 2 in Duos (places are per team).
create view public.game_summaries with (security_invoker = true) as
select
  g.user_id,
  g.session,
  g.game_index,
  case g.game_type when 'GT_BATTLEGROUNDS' then 'solo' else 'duos' end as mode,
  g.game_type,
  g.status,
  g.played_on,
  g.hero_card_id,
  coalesce(substring(g.hero_card_id from '^(.+)_SKIN_[A-Za-z0-9]+$'), g.hero_card_id) as hero,
  g.final_place,
  p.counted_place,
  p.counted_place <= case g.game_type when 'GT_BATTLEGROUNDS' then 4 else 2 end as top_half,
  p.counted_place = 1 as won,
  g.build,
  g.parser_version,
  g.tribes_offered,
  g.storage_path,
  s.summary_version,
  s.teammate_hero,
  s.rounds,
  s.combats_won,
  s.combats_lost,
  s.combats_tied,
  s.combats_unknown,
  s.combats_inferred,
  s.win_streak,
  s.win_streak_inferred,
  s.damage_taken,
  s.damage_taken_combats,
  s.damage_taken_max,
  s.damage_dealt,
  s.damage_dealt_combats,
  s.damage_dealt_max,
  s.shop_turns,
  s.final_tier,
  s.gold_spent,
  s.gold_spent_turns,
  s.gold_beyond_income,
  s.gold_beyond_turns,
  s.sell_gold,
  s.sell_gold_turns,
  s.rolls,
  s.free_rolls,
  s.buys,
  s.spell_buys,
  s.sells,
  s.freezes,
  s.actions,
  s.apm,
  s.triples,
  s.final_board,
  s.final_board_round,
  s.board_stats_max,
  s.golden_max,
  s.heroes_offered,
  s.hero_rerolls
from public.games g
cross join lateral (
  select case when g.status = 'ok' then g.final_place end as counted_place
) p
left join public.game_stats s
  on s.user_id = g.user_id and s.session = g.session and s.game_index = g.game_index;

comment on view public.game_summaries is
  'One row per game with its summary (T-502). Readable like public.games: by the owner, or by everyone when the profile is public. Null is unknown.';

-- Per user and mode. Each total comes with the games that back it (*_games):
-- a game without the number is left out, never counted as 0.
create view public.user_overview with (security_invoker = true) as
select
  user_id,
  mode,
  count(*) as games,
  count(*) filter (where status = 'incomplete') as incomplete_games,
  count(*) filter (where status = 'unsupported') as unsupported_games,
  count(counted_place) as placed_games,
  round(avg(counted_place), 2) as avg_place,
  count(*) filter (where top_half) as top_half,
  round(100.0 * count(*) filter (where top_half) / nullif(count(counted_place), 0), 1)
    as top_half_pct,
  count(*) filter (where won) as wins,
  round(100.0 * count(*) filter (where won) / nullif(count(counted_place), 0), 1) as win_pct,
  min(played_on) as first_played_on,
  max(played_on) as last_played_on,
  count(summary_version) as summarized_games,
  sum(combats_won) as combats_won,
  sum(combats_lost) as combats_lost,
  sum(combats_tied) as combats_tied,
  sum(combats_unknown) as combats_unknown,
  sum(combats_inferred) as combats_inferred,
  count(combats_won) as combat_games,
  sum(damage_dealt) as damage_dealt,
  count(damage_dealt) as damage_dealt_games,
  sum(damage_taken) as damage_taken,
  count(damage_taken) as damage_taken_games,
  sum(gold_spent) as gold_spent,
  count(gold_spent) as gold_spent_games,
  sum(gold_beyond_income) as gold_beyond_income,
  count(gold_beyond_income) as gold_beyond_games,
  sum(sell_gold) as sell_gold,
  count(sell_gold) as sell_gold_games,
  sum(rolls) as rolls,
  sum(free_rolls) as free_rolls,
  sum(buys) as buys,
  sum(sells) as sells,
  count(rolls) as shop_games,
  round(avg(apm), 2) as avg_apm,
  count(apm) as apm_games,
  sum(triples) as triples,
  count(triples) as triples_games
from public.game_summaries
group by user_id, mode;

comment on view public.user_overview is
  'Per user and mode: games, places (finished games only), top 4 in Solo / top 2 in Duos, wins, and totals with the games that back each (T-502).';

-- Per user, mode and hero (skins under their base hero). Picks count the
-- games whose hero choice is known: offered_games had the hero among the
-- choices, picked_games played it after a known choice.
create view public.user_heroes with (security_invoker = true) as
with played as (
  select
    user_id,
    mode,
    hero,
    count(*) as games,
    count(counted_place) as placed_games,
    round(avg(counted_place), 2) as avg_place,
    count(*) filter (where top_half) as top_half,
    round(100.0 * count(*) filter (where top_half) / nullif(count(counted_place), 0), 1)
      as top_half_pct,
    count(*) filter (where won) as wins,
    min(counted_place) as best_place,
    max(counted_place) as worst_place,
    count(*) filter (where heroes_offered is not null) as picked_games
  from public.game_summaries
  where hero is not null
  group by user_id, mode, hero
),
offered as (
  select user_id, mode, base as hero, count(distinct (session, game_index)) as offered_games
  from public.game_summaries
  cross join lateral unnest(heroes_offered) as o(card)
  cross join lateral (
    select coalesce(substring(o.card from '^(.+)_SKIN_[A-Za-z0-9]+$'), o.card) as base
  ) b
  group by user_id, mode, base
)
select
  coalesce(p.user_id, o.user_id) as user_id,
  coalesce(p.mode, o.mode) as mode,
  coalesce(p.hero, o.hero) as hero,
  coalesce(p.games, 0) as games,
  coalesce(p.placed_games, 0) as placed_games,
  p.avg_place,
  coalesce(p.top_half, 0) as top_half,
  p.top_half_pct,
  coalesce(p.wins, 0) as wins,
  p.best_place,
  p.worst_place,
  coalesce(o.offered_games, 0) as offered_games,
  coalesce(p.picked_games, 0) as picked_games
from played p
full join offered o on o.user_id = p.user_id and o.mode = p.mode and o.hero = p.hero;

comment on view public.user_heroes is
  'Per user, mode and hero (skins under the base hero, D-019 rule): games, places, top 4 / top 2, best and worst place, and picks from known hero choices (T-502).';

-- Per user, mode and record: the best value, the game it happened in (the
-- first game that reached it), how many games back the record, and whether
-- the value is inferred (worked out from health). A record no game backs
-- has no row.
create view public.user_records with (security_invoker = true) as
with streaks as (
  -- Runs of top finishes in a row over the finished games, in play order.
  select user_id, mode, session, game_index, played_on, top_half,
    count(*) over (partition by user_id, mode) as placed,
    row_number() over (partition by user_id, mode order by played_on, session, game_index)
      - row_number() over (
          partition by user_id, mode, top_half order by played_on, session, game_index
        ) as run
  from public.game_summaries
  where counted_place is not null
),
top_runs as (
  select user_id, mode, run, max(placed) as placed, count(*) as len,
    max(array[played_on::text, session, lpad(game_index::text, 4, '0')]) as last_game
  from streaks
  where top_half
  group by user_id, mode, run
),
candidates as (
  select user_id, mode, 'win_streak' as record, win_streak::numeric as value, session,
    game_index, played_on, coalesce(win_streak_inferred, false) as inferred,
    count(*) over (partition by user_id, mode) as games,
    -- At equal value, a streak from the log beats an inferred one.
    coalesce(win_streak_inferred, false)::int as tiebreak
  from public.game_summaries where win_streak is not null
  union all
  select user_id, mode, 'damage_dealt_max', damage_dealt_max, session, game_index, played_on,
    true, count(*) over (partition by user_id, mode), 0
  from public.game_summaries where damage_dealt_max is not null
  union all
  select user_id, mode, 'damage_taken_max', damage_taken_max, session, game_index, played_on,
    true, count(*) over (partition by user_id, mode), 0
  from public.game_summaries where damage_taken_max is not null
  union all
  select user_id, mode, 'board_stats_max', board_stats_max, session, game_index, played_on,
    false, count(*) over (partition by user_id, mode), 0
  from public.game_summaries where board_stats_max is not null
  union all
  select user_id, mode, 'golden_max', golden_max, session, game_index, played_on,
    false, count(*) over (partition by user_id, mode), 0
  from public.game_summaries where golden_max is not null
  union all
  select user_id, mode, 'gold_spent', gold_spent, session, game_index, played_on,
    false, count(*) over (partition by user_id, mode), 0
  from public.game_summaries where gold_spent is not null
  union all
  select user_id, mode, 'gold_beyond_income', gold_beyond_income, session, game_index,
    played_on, false, count(*) over (partition by user_id, mode), 0
  from public.game_summaries where gold_beyond_income is not null
  union all
  select user_id, mode, 'rolls', rolls, session, game_index, played_on,
    false, count(*) over (partition by user_id, mode), 0
  from public.game_summaries where rolls is not null
  union all
  select user_id, mode, 'triples', triples, session, game_index, played_on,
    false, count(*) over (partition by user_id, mode), 0
  from public.game_summaries where triples is not null
  union all
  select user_id, mode, 'apm', apm, session, game_index, played_on,
    false, count(*) over (partition by user_id, mode), 0
  from public.game_summaries where apm is not null
  union all
  select user_id, mode, 'rounds', rounds, session, game_index, played_on,
    false, count(*) over (partition by user_id, mode), 0
  from public.game_summaries where rounds is not null
  union all
  -- The run's last game; its date and key come from the array above.
  select user_id, mode, 'top_half_streak', len, last_game[2], last_game[3]::integer,
    last_game[1]::date, false, placed, 0
  from top_runs
)
select distinct on (user_id, mode, record)
  user_id, mode, record, value, session, game_index, played_on, games, inferred
from candidates
order by user_id, mode, record, value desc, tiebreak, played_on, session, game_index;

comment on view public.user_records is
  'Per user, mode and record: the best value, the first game that reached it, the games that back it and whether it is inferred from health (T-502).';

revoke all on public.game_summaries, public.user_overview, public.user_heroes, public.user_records
  from anon, authenticated;
grant select on public.game_summaries, public.user_overview, public.user_heroes, public.user_records
  to anon, authenticated;

-- ---------------------------------------------------------------------------
-- Export and deletion
-- ---------------------------------------------------------------------------

-- As in 20261009150000_upload.sql, plus the summaries.
create or replace function public.export_my_data()
returns jsonb
language plpgsql
stable
security definer
set search_path = ''
as $$
declare
  uid uuid := auth.uid();
begin
  if uid is null then
    raise exception 'not signed in' using errcode = '42501';
  end if;

  return jsonb_build_object(
    'format', 'tavern-ledger-export',
    'format_version', 1,
    'exported_at', now(),
    'account', (
      select jsonb_build_object(
        'id', u.id,
        'email', u.email,
        'phone', u.phone,
        'created_at', u.created_at,
        'updated_at', u.updated_at,
        'email_confirmed_at', u.email_confirmed_at,
        'last_sign_in_at', u.last_sign_in_at,
        'app_metadata', u.raw_app_meta_data,
        'user_metadata', u.raw_user_meta_data
      )
      from auth.users u where u.id = uid
    ),
    'identities', coalesce((
      select jsonb_agg(jsonb_build_object(
        'provider', i.provider,
        'identity_data', i.identity_data,
        'created_at', i.created_at,
        'updated_at', i.updated_at,
        'last_sign_in_at', i.last_sign_in_at
      ) order by i.created_at)
      from auth.identities i where i.user_id = uid
    ), '[]'::jsonb),
    'sessions', coalesce((
      select jsonb_agg(jsonb_build_object(
        'created_at', s.created_at,
        'updated_at', s.updated_at,
        'not_after', s.not_after,
        'user_agent', s.user_agent,
        'ip', s.ip
      ) order by s.created_at)
      from auth.sessions s where s.user_id = uid
    ), '[]'::jsonb),
    -- Second factors (metadata only; secrets stay out).
    'mfa_factors', coalesce((
      select jsonb_agg(jsonb_build_object(
        'factor_type', f.factor_type,
        'friendly_name', f.friendly_name,
        'status', f.status,
        'created_at', f.created_at,
        'updated_at', f.updated_at
      ) order by f.created_at)
      from auth.mfa_factors f where f.user_id = uid
    ), '[]'::jsonb),
    'auth_events', coalesce((
      select jsonb_agg(jsonb_build_object(
        'created_at', a.created_at,
        'ip_address', a.ip_address,
        'payload', a.payload
      ) order by a.created_at)
      from auth.audit_log_entries a where (a.payload ->> 'actor_id' = uid::text
             or a.payload -> 'traits' ->> 'user_id' = uid::text)
    ), '[]'::jsonb),
    'profile', (
      select to_jsonb(p) from public.profiles p where p.user_id = uid
    ),
    'games', coalesce((
      select jsonb_agg(to_jsonb(g) order by g.session, g.game_index)
      from public.games g where g.user_id = uid
    ), '[]'::jsonb),
    'game_stats', coalesce((
      select jsonb_agg(to_jsonb(s) order by s.session, s.game_index)
      from public.game_stats s where s.user_id = uid
    ), '[]'::jsonb),
    'files', coalesce((
      select jsonb_agg(jsonb_build_object(
        'bucket', o.bucket_id,
        'name', o.name,
        'created_at', o.created_at,
        'updated_at', o.updated_at,
        'metadata', o.metadata
      ) order by o.name)
      from storage.objects o
      where o.bucket_id = 'games' and (storage.foldername(o.name))[1] = uid::text
    ), '[]'::jsonb),
    'upload_events', coalesce((
      select jsonb_agg(e.at order by e.at)
      from private.upload_events e where e.user_id = uid
    ), '[]'::jsonb)
  );
end;
$$;

-- As in 20261009120100_export_delete.sql, plus the summaries (they would go
-- with the games anyway; deleting them first gives their count).
create or replace function public.delete_user_data(target uuid)
returns jsonb
language plpgsql
security definer
set search_path = ''
as $$
declare
  n_stats bigint;
  n_games bigint;
  n_profiles bigint;
  n_events bigint;
begin
  if exists (
    select 1 from storage.objects o
    where o.bucket_id = 'games' and (storage.foldername(o.name))[1] = target::text
  ) then
    raise exception 'files remain for this user; delete them first'
      using errcode = '55000';
  end if;

  delete from public.game_stats where user_id = target;
  get diagnostics n_stats = row_count;
  delete from public.games where user_id = target;
  get diagnostics n_games = row_count;
  delete from public.profiles where user_id = target;
  get diagnostics n_profiles = row_count;
  -- Auth audit entries have no foreign key to the user and would survive the
  -- auth user's deletion.
  delete from auth.audit_log_entries
  where payload ->> 'actor_id' = target::text
     or payload -> 'traits' ->> 'user_id' = target::text;
  get diagnostics n_events = row_count;

  return jsonb_build_object(
    'game_stats', n_stats, 'games', n_games, 'profiles', n_profiles, 'auth_events', n_events
  );
end;
$$;

revoke all on function public.delete_user_data(uuid) from public, anon, authenticated;
grant execute on function public.delete_user_data(uuid) to service_role;

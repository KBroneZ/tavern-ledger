-- Public game links and file names no longer show when a game was played.
--
-- The game's key was its log folder name (Hearthstone_2026_10_09_18_30_00:
-- the local start time to the second), and it was public three times: the
-- `session` column, the file name in the `games` bucket (`<user>/<session>-
-- <index>.json.gz`, which anyone can list for a public profile) and the
-- game page's address. Now:
--
--   * every game has `public_id`, a random uuid that names it in the page
--     address and in its file (`<user>/<public_id>.json.gz`);
--   * clients can no longer read `games.session` or `game_stats.session`;
--     the game_summaries view (the owner's rights, the row rule in the view)
--     shows `session` and the exact file name to the game's owner only, and
--     adds `public_id` and `play_order`, the game's place in its player's
--     order of play (so lists still sort by when a game was played, without
--     the time);
--   * the upload protocol is unchanged: the desktop app still names the
--     session, the database makes the id (upload_begin) and the Edge
--     Function builds the file name from it.
--
-- Existing games keep their row and key; their `public_id` is made here. Their
-- files keep the old name until the daily sweep renames them, a few hundred
-- per run, through the Storage API (game_files_to_move / game_file_moved:
-- safe to repeat, and a file is never without a row naming it, see
-- sweep_orphan_files). Until a file is renamed, the public cannot read it
-- (the old name carries the time, so the bucket policy hides it from everyone
-- but the owner) and the game page says it is not available.

-- ---------------------------------------------------------------------------
-- public_id and a plain storage_path
-- ---------------------------------------------------------------------------

alter table public.games add column public_id uuid not null default gen_random_uuid();
create unique index games_public_id_key on public.games (public_id);

comment on column public.games.public_id is
  'Random id that names the game in public (page address, file name). Carries nothing about the game.';

-- storage_path was generated from the session. It is now stored as it was
-- (the old names stay valid) and new games get `<user>/<public_id>.json.gz`.
alter table public.games alter column storage_path drop expression;

create function private.games_set_path()
returns trigger
language plpgsql
set search_path = ''
as $$
begin
  new.storage_path := coalesce(new.storage_path, new.user_id::text || '/' || new.public_id::text || '.json.gz');
  return new;
end;
$$;

revoke all on function private.games_set_path() from public, anon, authenticated, service_role;

create trigger games_set_path before insert on public.games
  for each row execute function private.games_set_path();

comment on column public.games.storage_path is
  '<user>/<public_id>.json.gz, or the older <user>/<session>-<index>.json.gz until the sweep renames the file (game_files_to_move).';

-- ---------------------------------------------------------------------------
-- What clients can read
-- ---------------------------------------------------------------------------

revoke select on public.games from anon, authenticated;
grant select (
  user_id, game_index, game_type, status, hero_card_id, final_place,
  build, played_on, tribes_offered, parser_version, public_id
) on public.games to anon, authenticated;

revoke select on public.game_stats from anon, authenticated;
grant select (
  user_id, game_index, summary_version, teammate_hero, rounds, combats_won,
  combats_lost, combats_tied, combats_unknown, combats_inferred, win_streak,
  win_streak_inferred, damage_taken, damage_taken_combats, damage_taken_max, damage_dealt,
  damage_dealt_combats, damage_dealt_max, shop_turns, final_tier, gold_spent,
  gold_spent_turns, gold_beyond_income, gold_beyond_turns, sell_gold, sell_gold_turns,
  rolls, free_rolls, buys, spell_buys, sells, freezes, actions, apm, triples, final_board,
  final_board_round, board_stats_max, golden_max, heroes_offered, hero_rerolls,
  damage_taken_max_inferred, damage_dealt_max_inferred, damage_taken_logged,
  damage_dealt_logged
) on public.game_stats to anon, authenticated;

-- As in 20261014160000_public_fields.sql, plus the owner-only session and file
-- name, and public_id and play_order at the end.
create or replace view public.game_summaries with (security_invoker = false, security_barrier = true) as
select
  g.user_id,
  case when g.user_id = (select auth.uid()) then g.session end as session,
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
  case when g.user_id = (select auth.uid()) then g.storage_path
       else g.user_id::text || '/' || g.public_id::text || '.json.gz' end as storage_path,
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
  s.hero_rerolls,
  case when g.user_id = (select auth.uid()) then g.file_expired_at end as file_expired_at,
  s.damage_taken_max_inferred,
  s.damage_dealt_max_inferred,
  s.damage_taken_logged,
  s.damage_dealt_logged,
  g.public_id,
  (row_number() over (partition by g.user_id order by g.session, g.game_index))::integer as play_order
from public.games g
cross join lateral (
  select case when g.status = 'ok' then g.final_place end as counted_place
) p
left join public.game_stats s
  on s.user_id = g.user_id and s.session = g.session and s.game_index = g.game_index
where g.user_id = (select auth.uid())
  or exists (
    select 1 from public.profiles pr
    where pr.user_id = g.user_id and pr.is_public
  );

comment on view public.game_summaries is
  'One row per game with its summary (T-502). Runs with the owner''s rights and shows the caller''s own games, or those of a public profile. session, the stored file name and file_expired_at are the owner''s only; public_id names the game in public; play_order orders a player''s games by play without the time. Null is unknown.';

-- Per user, mode and hero: a game is told apart by its public_id now that the
-- session is the owner's only (a null session would merge every game).
create or replace view public.user_heroes with (security_invoker = true) as
with picks as (
  select g.user_id, g.mode, g.hero, g.counted_place, g.top_half, g.won,
    1 as played,
    exists (
      select 1 from unnest(g.heroes_offered) as c(card)
      where coalesce(substring(c.card from '^(.+)_SKIN_[A-Za-z0-9]+$'), c.card) = g.hero
    )::int as picked,
    0 as offered
  from public.game_summaries g
  where g.hero is not null
  union all
  select o.user_id, o.mode, o.hero, null::smallint, null::boolean, null::boolean,
    0, 0, 1
  from (
    select g.user_id, g.mode, g.public_id,
      coalesce(substring(c.card from '^(.+)_SKIN_[A-Za-z0-9]+$'), c.card) as hero
    from public.game_summaries g
    cross join lateral unnest(g.heroes_offered) as c(card)
  ) o
  -- A hero offered twice in one game (two of its skins) counts once.
  group by o.user_id, o.mode, o.public_id, o.hero
)
select
  user_id,
  mode,
  hero,
  sum(played) as games,
  count(counted_place) as placed_games,
  round(avg(counted_place), 2) as avg_place,
  count(*) filter (where top_half) as top_half,
  round(100.0 * count(*) filter (where top_half) / nullif(count(counted_place), 0), 1)
    as top_half_pct,
  count(*) filter (where won) as wins,
  min(counted_place) as best_place,
  max(counted_place) as worst_place,
  sum(offered) as offered_games,
  sum(picked) as picked_games
from picks
group by user_id, mode, hero;

-- As in 20261012210000_damage_record_flag.sql, in play order by play_order
-- (the session is the owner's only) and with the record game's public_id at the end.
create or replace view public.user_records with (security_invoker = true) as
with streaks as (
  -- Runs of top finishes in a row over the finished games, in play order.
  select user_id, mode, public_id, session, game_index, played_on, play_order, top_half,
    count(*) over (partition by user_id, mode) as placed,
    row_number() over (partition by user_id, mode order by play_order)
      - row_number() over (
          partition by user_id, mode, top_half order by play_order
        ) as run
  from public.game_summaries
  where counted_place is not null
),
top_runs as (
  -- Each run with its last game.
  select distinct on (user_id, mode, run)
    user_id, mode, run, placed, public_id, session, game_index, played_on, play_order,
    count(*) over (partition by user_id, mode, run) as len
  from streaks
  where top_half
  order by user_id, mode, run, play_order desc
),
candidates as (
  select g.user_id, g.mode, r.record, r.value, g.public_id, g.session, g.game_index,
    g.played_on, g.play_order, r.inferred, r.tiebreak
  from public.game_summaries g
  cross join lateral (values
    -- At equal value, a record from the log beats an inferred one.
    ('win_streak', g.win_streak::numeric, coalesce(g.win_streak_inferred, false),
      coalesce(g.win_streak_inferred, false)::int),
    -- The max is from the game's own hit when the summary says so (false);
    -- true or unknown (a summary written before the flag) stays inferred.
    ('damage_dealt_max', g.damage_dealt_max::numeric,
      g.damage_dealt_max_inferred is distinct from false,
      (g.damage_dealt_max_inferred is distinct from false)::int),
    ('damage_taken_max', g.damage_taken_max::numeric,
      g.damage_taken_max_inferred is distinct from false,
      (g.damage_taken_max_inferred is distinct from false)::int),
    ('board_stats_max', g.board_stats_max::numeric, false, 0),
    ('golden_max', g.golden_max::numeric, false, 0),
    ('gold_spent', g.gold_spent::numeric, false, 0),
    ('gold_beyond_income', g.gold_beyond_income::numeric, false, 0),
    ('rolls', g.rolls::numeric, false, 0),
    ('triples', g.triples::numeric, false, 0),
    ('apm', g.apm, false, 0),
    ('rounds', g.rounds::numeric, false, 0)
  ) as r(record, value, inferred, tiebreak)
  where r.value is not null
),
counted as (
  select user_id, mode, record, value, public_id, session, game_index, played_on, play_order,
    inferred, tiebreak,
    count(*) over (partition by user_id, mode, record) as games
  from candidates
  union all
  select user_id, mode, 'top_half_streak', len::numeric, public_id, session, game_index,
    played_on, play_order, false, 0, placed
  from top_runs
)
select distinct on (user_id, mode, record)
  user_id, mode, record, value, session, game_index, played_on, games, inferred, public_id
from counted
order by user_id, mode, record, value desc, tiebreak, play_order;

-- ---------------------------------------------------------------------------
-- Storage: the public reads only opaque names
-- ---------------------------------------------------------------------------

-- As in 20261009120000_profiles_games.sql, but a file whose name still carries
-- the session is the owner's only (listing the bucket would show the time).
drop policy games_bucket_read_own_or_public on storage.objects;
create policy games_bucket_read_own_or_public on storage.objects
  for select to anon, authenticated
  using (
    bucket_id = 'games'
    and (
      (storage.foldername(name))[1] = (select auth.uid())::text
      or (
        name !~ '/Hearthstone_'
        and exists (
          select 1 from public.profiles p
          where p.user_id::text = (storage.foldername(name))[1] and p.is_public
        )
      )
    )
  );

-- ---------------------------------------------------------------------------
-- Upload: the database makes the id and the file name
-- ---------------------------------------------------------------------------

-- As in 20261013150000_upload_hardening.sql, plus `path` and `public_id` in
-- the answer for 'create' and 'replace': where to write the file, always
-- `<user>/<public_id>.json.gz` (a new game gets a new id, a replaced one keeps
-- its id). The desktop app's request is unchanged.
create or replace function public.upload_begin(
  target uuid,
  p_session text,
  p_game_index integer,
  p_saved_at bigint,
  p_sha256 text,
  p_size integer
)
returns jsonb
language plpgsql
security definer
set search_path = ''
as $$
declare
  hour_limit constant integer := 600;
  day_limit constant integer := 5000;
  games_limit constant integer := 10000;
  bytes_limit constant bigint := 50 * 1024 * 1024;
  n_hour bigint;
  n_day bigint;
  n_games bigint;
  n_bytes bigint;
  has_row boolean;
  old_saved_at bigint;
  old_sha text;
  old_size integer;
  old_id uuid;
  new_id uuid;
begin
  -- The row could not be written with this session: refuse before the file is.
  if private.session_date(p_session) is null then
    return jsonb_build_object('result', 'bad_session');
  end if;

  -- One check at a time per user, so two uploads at once cannot both pass
  -- the same last free slot. (The quota can still be passed by the uploads
  -- in flight between this call and their commit; the app sends one at a time.)
  perform pg_advisory_xact_lock(hashtextextended('upload:' || target::text, 0));

  select count(*) filter (where e.at > now() - interval '1 hour'), count(*)
  into n_hour, n_day
  from private.upload_events e
  where e.user_id = target and e.at > now() - interval '1 day';
  if n_hour >= hour_limit then
    return jsonb_build_object('result', 'rate_limited', 'retry_after',
      private.retry_after(target, now() - interval '1 hour', interval '1 hour'));
  end if;
  if n_day >= day_limit then
    return jsonb_build_object('result', 'rate_limited', 'retry_after',
      private.retry_after(target, now() - interval '1 day', interval '1 day'));
  end if;

  select true, g.saved_at, g.content_sha256,
    case when g.file_expired_at is null then g.size_bytes else 0 end,
    g.public_id
  into has_row, old_saved_at, old_sha, old_size, old_id
  from public.games g
  where g.user_id = target and g.session = p_session and g.game_index = p_game_index;
  has_row := coalesce(has_row, false);

  if has_row and old_sha = p_sha256 then
    insert into private.upload_events (user_id) values (target);
    return jsonb_build_object('result', 'unchanged');
  end if;
  if has_row and old_saved_at > p_saved_at then
    return jsonb_build_object('result', 'older');
  end if;

  select count(*), coalesce(sum(g.size_bytes) filter (where g.file_expired_at is null), 0)
  into n_games, n_bytes
  from public.games g
  where g.user_id = target;
  if not has_row and n_games >= games_limit then
    return jsonb_build_object('result', 'quota_games');
  end if;
  if n_bytes - coalesce(old_size, 0) + p_size > bytes_limit then
    return jsonb_build_object('result', 'quota_bytes');
  end if;

  insert into private.upload_events (user_id) values (target);
  if has_row then
    -- Always the opaque name, also for a game stored under the old one: the
    -- upload then moves the game to it (upload_commit), and the old file is
    -- swept as an orphan. A rename by the sweep and a replacement cannot cross.
    return jsonb_build_object('result', 'replace', 'public_id', old_id,
      'path', target::text || '/' || old_id::text || '.json.gz');
  end if;
  new_id := gen_random_uuid();
  return jsonb_build_object('result', 'create', 'public_id', new_id,
    'path', target::text || '/' || new_id::text || '.json.gz');
end;
$$;

-- As in 20261011120000_game_stats.sql, plus p_public_id: the id upload_begin()
-- gave the game, whose file was written under it (null from an older Edge
-- Function: a new game then has the file name its session gives, a replaced
-- one keeps the name it has, and the sweep renames them).
drop function public.upload_commit(
  uuid, text, integer, text, text, text, integer, integer, date, jsonb, bigint, text, integer, text,
  jsonb, integer
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
  p_summary_version integer default null,
  p_public_id uuid default null
)
returns text
language plpgsql
security definer
set search_path = ''
as $$
declare
  inserted boolean;
  new_id uuid := coalesce(p_public_id, gen_random_uuid());
begin
  insert into public.games as g (
    user_id, session, game_index, game_type, status, hero_card_id, final_place,
    build, played_on, tribes_offered, saved_at, content_sha256, size_bytes, parser_version,
    public_id, storage_path
  ) values (
    target, p_session, p_game_index, p_game_type, p_status, p_hero_card_id, p_final_place,
    p_build, p_played_on, p_tribes_offered, p_saved_at, p_sha256, p_size, p_parser_version,
    new_id,
    case when p_public_id is null
      then target::text || '/' || p_session || '-' || p_game_index::text || '.json.gz' end
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
    parser_version = excluded.parser_version,
    -- The file was written under the id given: the row names it (a game still
    -- under its old name moves to the opaque one; two first uploads of the
    -- same game at once end with the row naming the file of the last copy).
    -- No id given (an older Edge Function): the row keeps what it has.
    public_id = coalesce(p_public_id, g.public_id),
    storage_path = case when p_public_id is null then g.storage_path
      else target::text || '/' || p_public_id::text || '.json.gz' end
  where g.saved_at <= excluded.saved_at
  returning (xmax = 0) into inserted;

  if inserted is null then
    return 'older';
  end if;
  if p_stats is not null then
    begin
      perform private.put_game_stats(target, p_session, p_game_index, p_summary_version, p_stats);
      return case when inserted then 'created' else 'replaced' end;
    exception when data_exception or integrity_constraint_violation then
      raise warning 'upload_commit: summary refused (%); game stored without it', sqlstate;
    end;
  end if;
  delete from public.game_stats s
  where s.user_id = target and s.session = p_session and s.game_index = p_game_index;
  return case when inserted then 'created' else 'replaced' end;
end;
$$;

revoke all on function public.upload_commit(
  uuid, text, integer, text, text, text, integer, integer, date, jsonb, bigint, text, integer, text,
  jsonb, integer, uuid
) from public, anon, authenticated;
grant execute on function public.upload_commit(
  uuid, text, integer, text, text, text, integer, integer, date, jsonb, bigint, text, integer, text,
  jsonb, integer, uuid
) to service_role;

-- ---------------------------------------------------------------------------
-- The daily sweep: orphan files, and renaming the old files
-- ---------------------------------------------------------------------------

-- As in 20261009150000_upload.sql, but a file already under its game's new
-- name is not an orphan (the sweep renamed it and has not yet written the
-- new name in the row, or died between the two).
create or replace function public.sweep_orphan_files(min_age interval)
returns setof text
language sql
stable
security definer
set search_path = ''
as $$
  select o.name
  from storage.objects o
  where o.bucket_id = 'games'
    and greatest(o.created_at, coalesce(o.updated_at, o.created_at)) < now() - min_age
    and not exists (select 1 from public.games g where g.storage_path = o.name)
    and not exists (
      select 1 from public.games g
      where g.user_id::text || '/' || g.public_id::text || '.json.gz' = o.name
    )
  order by o.name
  limit 1000;
$$;

-- Games whose file still has the old name, with the new one; at most
-- p_limit (capped at 500) per call, in no fixed order. Games whose file
-- was deleted (file_expired_at) have nothing to rename: the name is only
-- used if a new copy is uploaded, and it is renamed then.
create function public.game_files_to_move(p_limit integer)
returns table (old_path text, new_path text)
language sql
stable
security definer
set search_path = ''
as $$
  select g.storage_path, g.user_id::text || '/' || g.public_id::text || '.json.gz'
  from public.games g
  where g.file_expired_at is null
    and g.storage_path is distinct from g.user_id::text || '/' || g.public_id::text || '.json.gz'
  -- In random order, so a game whose file cannot be found never holds back the others.
  order by random()
  limit least(greatest(coalesce(p_limit, 0), 0), 500);
$$;

-- Writes the new name in the row once the file is under it. True if the
-- row had the old name and has the new one now; false (nothing changed) if
-- the row is gone, already moved, or the names do not belong together.
create function public.game_file_moved(p_old text, p_new text)
returns boolean
language plpgsql
security definer
set search_path = ''
as $$
declare
  n integer;
begin
  update public.games g set storage_path = p_new
  where g.storage_path = p_old
    and p_new = g.user_id::text || '/' || g.public_id::text || '.json.gz';
  get diagnostics n = row_count;
  return n = 1;
end;
$$;

revoke all on function public.game_files_to_move(integer) from public, anon, authenticated;
grant execute on function public.game_files_to_move(integer) to service_role;
revoke all on function public.game_file_moved(text, text) from public, anon, authenticated;
grant execute on function public.game_file_moved(text, text) to service_role;

-- create or replace keeps a function's settings; the ones above are new, and
-- sweep_orphan_files was given its own timeout in 20261015120000_community_trust_scale.sql.
alter function public.sweep_orphan_files(interval) set statement_timeout = '60s';

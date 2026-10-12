-- T-544 (D-109, D-113): 28-day rolling retention of full game
-- files for accounts without unlimited history.
--
-- A game's full file (the gzipped report in the `games` bucket: turn-by-turn
-- boards, combat replays) is deleted by the daily sweep once the game is
-- older than the retention; its summary rows (public.games, public.game_stats)
-- stay while the account exists. Accounts in private.unlimited_history keep
-- every file (the owner, beta testers, later the paid plan).
--
-- * private.file_retention: the one place for the number of days (28) and
--   the switch. It ships with live = false: the sweep then only counts what it would
--   delete (report-only). Turning it on is one line (deploy.md 10.5):
--     update private.file_retention set live = true, updated_at = now();
-- * private.unlimited_history: one row per exempt account. Written by the
--   owner only (SQL editor); no grant to anon, authenticated or service_role,
--   no client UI. Removed with the account.
-- * public.games.file_expired_at: set when the sweep has deleted the game's
--   file, so the website can say the details have expired instead of failing.
--   A new copy of the game (another SHA-256) clears it, and a later sweep
--   deletes that copy again.
-- * public.file_retention_list(), file_retention_check() and
--   file_retention_mark(): what is due, which of a batch is still due right
--   before its delete, and the mark after it (below). Service role only.
-- * The sweep's run record gains "retention" (counts only, never ids).
-- * The backfill of game summaries skips games whose file has expired.
-- * The upload's bytes quota leaves out files that have expired.
-- * Export: the games rows carry file_expired_at, and the export holds the
--   failed delete attempts and whether the account has unlimited history.
--   Account deletion removes the mark (the attempts go with the games).
--
-- When a game is "played": its played_on date (the client's local date) or
-- the UTC day it was first uploaded (created_at, set by the server and kept
-- on every replace), whichever is earlier, so a date in the future cannot
-- keep a file forever. The file is past the retention once that day is more
-- than days + 1 days before today (UTC): with 28 days, a game played on
-- 1 September is due from 1 October on. The extra day covers time zones.
--
-- A file past the retention is kept, and counted as skipped, when:
-- * exempt: the account has unlimited history;
-- * recent: the game row changed in the last hour (an upload may be writing
--   a new copy; it is due again on a later run);
-- * no_summary: the game has no summary of the current version (its file is
--   the only way to make or rebuild one). Never deleted while so; the run
--   record counts them and says how old the oldest is, and the storage alert
--   (T-543) bounds their volume.
--
-- The sweep lists the due files (files that failed before go after the
-- untried ones), then for each batch of 100 asks file_retention_check()
-- which are still due (live still on, no new exemption, the same copy, no
-- recent change), deletes those through the Storage API, and calls
-- file_retention_mark(), which marks only games whose Storage object is
-- really gone and counts an attempt on the ones still there. Anything not
-- marked is tried again on the next run.
--
-- Accepted residual (D-113): the check and the Storage delete are two calls,
-- milliseconds apart. If, in that gap, the switch is turned off, the account
-- gets unlimited history, or a new copy of the game is uploaded, the file is
-- deleted anyway and cannot be recovered from the server (the app keeps its
-- own copy). Only games more than 29 days old are concerned, and the owner
-- changes the switch and the exemptions outside the sweep's time (03:17 UTC).

-- ---------------------------------------------------------------------------
-- Settings and the exemption
-- ---------------------------------------------------------------------------

create table private.file_retention (
  id boolean primary key default true check (id),
  days integer not null check (days between 1 and 3650),
  live boolean not null default false,
  updated_at timestamptz not null default now()
);
insert into private.file_retention (days, live) values (28, false);
revoke all on private.file_retention from public, anon, authenticated, service_role;

create table private.unlimited_history (
  user_id uuid primary key references auth.users (id) on delete cascade,
  since timestamptz not null default now()
);
revoke all on private.unlimited_history from public, anon, authenticated, service_role;

comment on table private.unlimited_history is
  'Accounts whose full game files are kept without limit (T-544). Written by the owner only.';

-- Games whose file was handed to the Storage delete but is still there (the
-- delete failed or did not take): how often and when last. They are listed
-- after the untried ones, so a file that keeps failing cannot hold back the
-- rest. The row goes when the game is marked, or with the game.
create table private.file_retention_attempts (
  user_id uuid not null,
  session text not null,
  game_index integer not null,
  fails integer not null default 1 check (fails >= 1),
  last_at timestamptz not null default now(),
  primary key (user_id, session, game_index),
  foreign key (user_id, session, game_index)
    references public.games (user_id, session, game_index) on delete cascade
);
revoke all on private.file_retention_attempts from public, anon, authenticated, service_role;

-- ---------------------------------------------------------------------------
-- games.file_expired_at
-- ---------------------------------------------------------------------------

alter table public.games add column file_expired_at timestamptz;

comment on column public.games.file_expired_at is
  'When the daily sweep deleted this game''s full file (T-544); null while the file is kept.';

-- A new copy of the game is a new file: it is kept until a later sweep.
create function private.games_file_replaced()
returns trigger
language plpgsql
set search_path = ''
as $$
begin
  if new.content_sha256 is distinct from old.content_sha256 then
    new.file_expired_at := null;
  end if;
  return new;
end;
$$;

revoke all on function private.games_file_replaced() from public, anon, authenticated, service_role;

create trigger games_file_replaced before update of content_sha256 on public.games
  for each row execute function private.games_file_replaced();

-- ---------------------------------------------------------------------------
-- Selection
-- ---------------------------------------------------------------------------

-- The game's day (see the header).
create function private.game_day(p_played_on date, p_created_at timestamptz)
returns date
language sql
immutable
set search_path = ''
as $$
  select least(p_played_on, (p_created_at at time zone 'utc')::date);
$$;

-- What the retention does with a game's file now: null when it is not past
-- the retention (or already expired, or the settings row is missing), else
-- 'due', 'exempt', 'recent' or 'no_summary' (see the header). p_version is
-- the current summary version (SUMMARY_VERSION in game_summary.ts).
create function private.file_retention_state(g public.games, p_version integer)
returns text
language sql
stable
set search_path = ''
as $$
  select case
    when g.file_expired_at is not null then null
    when private.game_day(g.played_on, g.created_at)
         >= (now() at time zone 'utc')::date - (r.days + 1) then null
    when exists (select 1 from private.unlimited_history u where u.user_id = g.user_id)
      then 'exempt'
    when g.updated_at > now() - interval '1 hour' then 'recent'
    when exists (
      select 1 from public.game_stats s
      where s.user_id = g.user_id and s.session = g.session and s.game_index = g.game_index
        and s.summary_version >= p_version
    ) then 'due'
    else 'no_summary'
  end
  from private.file_retention r;
$$;

revoke all on function private.game_day(date, timestamptz)
  from public, anon, authenticated, service_role;
revoke all on function private.file_retention_state(public.games, integer)
  from public, anon, authenticated, service_role;

-- What the retention would delete now, for the sweep. Returns
--   {"live": bool, "days": 28, "due": n,
--    "skipped": {"exempt": n, "recent": n, "no_summary": n},
--    "no_summary_oldest_days": n | null,
--    "files": [{"path": "...", "sha256": "..."}, ...]}
-- due counts the files that would be deleted; skipped the ones kept, by
-- reason; no_summary_oldest_days is the age in days of the oldest game kept
-- for want of a summary (null when none). files is empty in report-only
-- mode, and at most p_limit (capped at 1000) in live mode: untried files
-- first, then by failed attempts, each oldest game day first. Raises 55000 when the
-- settings row is missing and 22023 without a summary version.
create function public.file_retention_list(p_limit integer, p_version integer)
returns jsonb
language plpgsql
stable
security definer
set search_path = ''
as $$
declare
  v_days integer;
  v_live boolean;
  v_counts jsonb;
  v_files jsonb := '[]'::jsonb;
begin
  select r.days, r.live into v_days, v_live from private.file_retention r;
  if v_days is null then
    raise exception 'file_retention_list: settings missing' using errcode = '55000';
  end if;
  if p_version is null or p_version < 1 then
    raise exception 'file_retention_list: summary version missing' using errcode = '22023';
  end if;

  select jsonb_build_object(
    'due', count(*) filter (where c.state = 'due'),
    'exempt', count(*) filter (where c.state = 'exempt'),
    'recent', count(*) filter (where c.state = 'recent'),
    'no_summary', count(*) filter (where c.state = 'no_summary'),
    'no_summary_oldest_days',
      (now() at time zone 'utc')::date - min(c.day) filter (where c.state = 'no_summary')
  )
  into v_counts
  from (
    select private.file_retention_state(g, p_version) as state,
      private.game_day(g.played_on, g.created_at) as day
    from public.games g
    where g.file_expired_at is null
  ) c;

  if v_live then
    select coalesce(jsonb_agg(jsonb_build_object('path', f.storage_path, 'sha256', f.content_sha256)
                              order by f.fails, f.day, f.storage_path), '[]'::jsonb)
    into v_files
    from (
      select g.storage_path, g.content_sha256, private.game_day(g.played_on, g.created_at) as day,
        coalesce(a.fails, 0) as fails
      from public.games g
      left join private.file_retention_attempts a
        on a.user_id = g.user_id and a.session = g.session and a.game_index = g.game_index
      where g.file_expired_at is null
        and private.file_retention_state(g, p_version) = 'due'
      order by 4, 3, 1
      limit least(greatest(coalesce(p_limit, 0), 0), 1000)
    ) f;
  end if;

  return jsonb_build_object(
    'live', v_live,
    'days', v_days,
    'due', (v_counts ->> 'due')::bigint,
    'skipped', jsonb_build_object(
      'exempt', (v_counts ->> 'exempt')::bigint,
      'recent', (v_counts ->> 'recent')::bigint,
      'no_summary', (v_counts ->> 'no_summary')::bigint
    ),
    'no_summary_oldest_days', v_counts -> 'no_summary_oldest_days',
    'files', v_files
  );
end;
$$;

-- Right before a batch is deleted: which of the listed files are still due,
-- the same copy (SHA-256) and in live mode. Empty when live is off. Refuses
-- (22023) lists of different lengths or longer than 1000.
create function public.file_retention_check(p_paths text[], p_sha256 text[], p_version integer)
returns text[]
language plpgsql
stable
security definer
set search_path = ''
as $$
begin
  if p_paths is null or p_sha256 is null or p_version is null
     or coalesce(array_length(p_paths, 1), 0) <> coalesce(array_length(p_sha256, 1), 0)
     or coalesce(array_length(p_paths, 1), 0) > 1000 then
    raise exception 'file_retention_check: lists out of shape' using errcode = '22023';
  end if;
  if not coalesce((select r.live from private.file_retention r), false) then
    return '{}';
  end if;
  return coalesce((
    select array_agg(g.storage_path order by g.storage_path)
    from public.games g
    join unnest(p_paths, p_sha256) as f(path, sha256)
      on g.storage_path = f.path and g.content_sha256 = f.sha256
    where private.file_retention_state(g, p_version) = 'due'
  ), '{}');
end;
$$;

-- After a Storage delete (answered or failed): marks the games whose file is
-- gone, only the copy the list named (same SHA-256), only while the game is
-- past the retention on an account without unlimited history, and only in
-- live mode; counts an attempt on the named games whose object is still
-- there. Returns how many rows it marked. Refuses
-- (22023) lists of different lengths or longer than 1000.
create function public.file_retention_mark(p_paths text[], p_sha256 text[])
returns integer
language plpgsql
security definer
set search_path = ''
as $$
declare
  v_days integer;
  n integer;
begin
  if p_paths is null or p_sha256 is null
     or coalesce(array_length(p_paths, 1), 0) <> coalesce(array_length(p_sha256, 1), 0)
     or coalesce(array_length(p_paths, 1), 0) > 1000 then
    raise exception 'file_retention_mark: lists out of shape' using errcode = '22023';
  end if;
  select r.days into v_days from private.file_retention r where r.live;
  if v_days is null then
    return 0;
  end if;
  with marked as (
    update public.games g
    set file_expired_at = now()
    from unnest(p_paths, p_sha256) as f(path, sha256)
    where g.storage_path = f.path
      and g.content_sha256 = f.sha256
      and g.file_expired_at is null
      and private.game_day(g.played_on, g.created_at)
          < (now() at time zone 'utc')::date - (v_days + 1)
      and not exists (select 1 from private.unlimited_history u where u.user_id = g.user_id)
      and not exists (
        select 1 from storage.objects o where o.bucket_id = 'games' and o.name = g.storage_path
      )
    returning g.user_id, g.session, g.game_index
  ), cleared as (
    delete from private.file_retention_attempts a
    using marked m
    where a.user_id = m.user_id and a.session = m.session and a.game_index = m.game_index
  )
  select count(*) into n from marked;

  -- A game deleted meanwhile (its account being deleted) would fail the
  -- foreign key: then no attempt is counted, and the marks above stay.
  begin
    insert into private.file_retention_attempts as a (user_id, session, game_index)
    select g.user_id, g.session, g.game_index
    from public.games g
    join unnest(p_paths, p_sha256) as f(path, sha256)
      on g.storage_path = f.path and g.content_sha256 = f.sha256
    where g.file_expired_at is null
      and exists (
        select 1 from storage.objects o where o.bucket_id = 'games' and o.name = g.storage_path
      )
    on conflict (user_id, session, game_index)
    do update set fails = a.fails + 1, last_at = now();
  exception when foreign_key_violation then
    null;
  end;
  return n;
end;
$$;

revoke all on function public.file_retention_list(integer, integer) from public, anon, authenticated;
revoke all on function public.file_retention_check(text[], text[], integer)
  from public, anon, authenticated;
revoke all on function public.file_retention_mark(text[], text[]) from public, anon, authenticated;
grant execute on function public.file_retention_list(integer, integer) to service_role;
grant execute on function public.file_retention_check(text[], text[], integer) to service_role;
grant execute on function public.file_retention_mark(text[], text[]) to service_role;

-- ---------------------------------------------------------------------------
-- The upload quota counts only the files still kept
-- ---------------------------------------------------------------------------

-- As in 20261009150000_upload.sql, except that the bytes quota leaves out
-- games whose file has expired (their rows stay, their bytes are gone). The
-- games quota still counts every row.
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
begin
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
    case when g.file_expired_at is null then g.size_bytes else 0 end
  into has_row, old_saved_at, old_sha, old_size
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
  return jsonb_build_object('result', case when has_row then 'replace' else 'create' end);
end;
$$;


-- ---------------------------------------------------------------------------
-- The backfill skips games whose file has expired
-- ---------------------------------------------------------------------------

-- As in 20261011120000_game_stats.sql, without games whose file has expired
-- (they would count as no_file on every run).
create or replace function public.game_stats_backfill_list(p_version integer, p_limit integer)
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
  where (s.user_id is null or s.summary_version < p_version)
    and g.file_expired_at is null
  -- Random, so files that keep failing cannot hold back the others forever.
  order by random()
  limit least(greatest(coalesce(p_limit, 0), 0), 500);
$$;

-- ---------------------------------------------------------------------------
-- The website reads file_expired_at with the summary
-- ---------------------------------------------------------------------------

-- As in 20261011120000_game_stats.sql, plus file_expired_at at the end
-- (create or replace keeps the grants and the views built on it).
create or replace view public.game_summaries with (security_invoker = true) as
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
  s.hero_rerolls,
  g.file_expired_at
from public.games g
cross join lateral (
  select case when g.status = 'ok' then g.final_place end as counted_place
) p
left join public.game_stats s
  on s.user_id = g.user_id and s.session = g.session and s.game_index = g.game_index;

-- ---------------------------------------------------------------------------
-- The run record
-- ---------------------------------------------------------------------------

-- The retention's counts as the sweep records them, one of:
--   {"error": "list"}   the list call failed or answered out of shape
--   {"mode": "report" | "live", "days": 28, "due": n, "listed": n,
--    "changed": n, "deleted": n, "failed": n, "unmarked": n, "left": n,
--    "skipped": {"exempt": n, "recent": n, "no_summary": n},
--    "no_summary_oldest_days": n | null}
-- due: files that would be deleted; listed: handed to the sweep this run;
-- changed: no longer due when their batch was checked; deleted: gone and
-- marked (confirmed by file_retention_mark); unmarked: the Storage API
-- answered the delete but the game was not marked (the object was still
-- there, or the mark failed); failed: the check or the delete call failed;
-- left: not reached before the time budget. What is not deleted is tried
-- again on the next run. listed = changed + deleted + unmarked + failed + left.
-- Report-only runs list, delete and mark nothing.
create function private.sweep_retention_valid(r jsonb)
returns boolean
language plpgsql
immutable
set search_path = ''
as $$
declare
  top constant text[] := array[
    'mode', 'days', 'due', 'listed', 'changed', 'deleted', 'failed', 'unmarked', 'left', 'skipped',
    'no_summary_oldest_days'
  ];
  reasons constant text[] := array['exempt', 'recent', 'no_summary'];
  counts constant text[] := array[
    'due', 'listed', 'changed', 'deleted', 'failed', 'unmarked', 'left'
  ];
  k text;
  v jsonb;
  n numeric;
begin
  if jsonb_typeof(r) is distinct from 'object' then
    return false;
  end if;
  if r = '{"error": "list"}'::jsonb then
    return true;
  end if;
  if jsonb_typeof(r -> 'skipped') is distinct from 'object'
     or not (array(select jsonb_object_keys(r)) @> top and array(select jsonb_object_keys(r)) <@ top)
     or not (array(select jsonb_object_keys(r -> 'skipped')) @> reasons
             and array(select jsonb_object_keys(r -> 'skipped')) <@ reasons)
     or r ->> 'mode' is null or r ->> 'mode' not in ('report', 'live') then
    return false;
  end if;
  for k, v in
    select e.key, e.value from jsonb_each(r) e where e.key = any(counts) or e.key = 'days'
    union all
    select e.key, e.value from jsonb_each(r -> 'skipped') e
  loop
    if jsonb_typeof(v) <> 'number' then
      return false;
    end if;
    n := (v #>> '{}')::numeric;
    if n <> trunc(n) or n < 0 or n > 1000000000000 then
      return false;
    end if;
  end loop;
  if (r ->> 'days')::numeric not between 1 and 3650 or (r ->> 'listed')::numeric > 1000 then
    return false;
  end if;
  v := r -> 'no_summary_oldest_days';
  if jsonb_typeof(v) <> 'null' and (
       jsonb_typeof(v) <> 'number' or (v #>> '{}')::numeric <> trunc((v #>> '{}')::numeric)
       or (v #>> '{}')::numeric not between 0 and 100000) then
    return false;
  end if;
  if r ->> 'mode' = 'report' then
    return (r ->> 'listed')::numeric = 0 and (r ->> 'changed')::numeric = 0
       and (r ->> 'deleted')::numeric = 0
       and (r ->> 'failed')::numeric = 0 and (r ->> 'unmarked')::numeric = 0
       and (r ->> 'left')::numeric = 0;
  end if;
  return (r ->> 'listed')::numeric
           = (r ->> 'changed')::numeric + (r ->> 'deleted')::numeric
             + (r ->> 'unmarked')::numeric + (r ->> 'failed')::numeric + (r ->> 'left')::numeric
     and (r ->> 'listed')::numeric <= (r ->> 'due')::numeric;
end;
$$;

revoke all on function private.sweep_retention_valid(jsonb)
  from public, anon, authenticated, service_role;

-- As in 20261011200000_sweep_counts.sql, plus the retention's counts. The
-- new argument has a default, so a sweep function deployed before this
-- migration's (three arguments) still records its runs.
drop function public.sweep_record_result(bigint, integer, jsonb);

create function public.sweep_record_result(
  p_run_id bigint,
  p_files integer,
  p_stats jsonb,
  p_retention jsonb default null
)
returns void
language plpgsql
security definer
set search_path = ''
as $$
begin
  if p_files is null or p_files < 0 or not private.sweep_stats_valid(p_stats)
     or (p_retention is not null and not private.sweep_retention_valid(p_retention)) then
    raise exception 'sweep_record_result: counts out of shape' using errcode = '22023';
  end if;
  update private.sweep_runs r
  set result = r.result || jsonb_build_object('files', p_files, 'stats', p_stats)
    || case when p_retention is null then '{}'::jsonb
            else jsonb_build_object('retention', p_retention) end
  where r.id = p_run_id and not (r.result ? 'stats') and not (r.result ? 'failed_step');
  if not found then
    raise exception 'sweep_record_result: no open run %', p_run_id using errcode = '22023';
  end if;
end;
$$;

revoke all on function public.sweep_record_result(bigint, integer, jsonb, jsonb)
  from public, anon, authenticated;
grant execute on function public.sweep_record_result(bigint, integer, jsonb, jsonb) to service_role;

-- ---------------------------------------------------------------------------
-- Export and account deletion
-- ---------------------------------------------------------------------------

-- As in 20261011180000_community_heroes.sql, plus file_retention_attempts
-- (failed deletes of the user's files, while any) and unlimited_history (when
-- the account has unlimited file history, else null). The games rows carry
-- file_expired_at.
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
    -- The switch as it is now; updated_at and policy_version are null when
    -- it was never set.
    'community_consent', jsonb_build_object(
      'hero_stats', coalesce((
        select c.hero_stats from public.community_consent c where c.user_id = uid
      ), false),
      'updated_at', (
        select c.updated_at from public.community_consent c where c.user_id = uid
      ),
      'policy_version', (
        select c.policy_version from public.community_consent c where c.user_id = uid
      )
    ),
    'games', coalesce((
      select jsonb_agg(to_jsonb(g) order by g.session, g.game_index)
      from public.games g where g.user_id = uid
    ), '[]'::jsonb),
    'game_stats', coalesce((
      select jsonb_agg(to_jsonb(s) order by s.session, s.game_index)
      from public.game_stats s where s.user_id = uid
    ), '[]'::jsonb),
    'leaderboard_rows', coalesce((
      select jsonb_agg(jsonb_build_object(
        'mode', r.mode,
        'period', r.period,
        'board', r.board,
        'hero', r.hero,
        'value', r.value,
        'games', r.games
      ) order by r.mode, r.period, r.board, r.hero)
      from private.leaderboard_rows r where r.user_id = uid
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
    'file_retention_attempts', coalesce((
      select jsonb_agg(jsonb_build_object(
        'session', a.session,
        'game_index', a.game_index,
        'fails', a.fails,
        'last_at', a.last_at
      ) order by a.session, a.game_index)
      from private.file_retention_attempts a where a.user_id = uid
    ), '[]'::jsonb),
    'unlimited_history', (
      select jsonb_build_object('since', u.since)
      from private.unlimited_history u where u.user_id = uid
    ),
    'upload_events', coalesce((
      select jsonb_agg(e.at order by e.at)
      from private.upload_events e where e.user_id = uid
    ), '[]'::jsonb)
  );
end;
$$;

-- As in 20261011180000_community_heroes.sql, plus the unlimited history mark
-- (the foreign key to auth.users would also remove it with the account).
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

  delete from private.leaderboard_rows where user_id = target;
  delete from public.community_consent where user_id = target;
  delete from private.unlimited_history where user_id = target;
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

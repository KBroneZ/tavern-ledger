-- Upload pipeline hardening.
--
-- 1. upload_begin() checks the session's date itself: a real day from 2000
--    to the end of next year (UTC), as the upload function's validator does.
--    Anything else answers {"result": "bad_session"} before the function
--    writes a file, and is not counted for the rate limits.
-- 2. The sweep function runs every step even when another one fails, and
--    housekeeping comes first. sweep_record_result() takes the steps that
--    failed (p_failed) and stores them on the run's row as failed_steps,
--    next to the counts of the steps that ran:
--    * 'list files' or 'delete files': files is what was deleted before it;
--    * 'list summaries': stats is null (empty, not zero);
--    * 'housekeeping': there is no run id, so the call adds a row of its own
--      (no housekeeping counts in it).
--    A row with failed_step (a run that stopped, sweep_record_failure()) is
--    unchanged. The new argument has a default, so a sweep function deployed
--    before this migration still records its runs. Deploy this migration
--    before the functions: the new sweep function sends p_failed.

-- ---------------------------------------------------------------------------
-- Session dates
-- ---------------------------------------------------------------------------

-- The date in a session name (Hearthstone_YYYY_MM_DD_hh_mm_ss), or null when
-- the name is not of that shape, the day does not exist, or the year is
-- before 2000 or after next year. Next year, because the client names its
-- logs with the PC's local time, which may run ahead of UTC.
create function private.session_date(p_session text)
returns date
language plpgsql
stable
set search_path = ''
as $$
declare
  m text[];
begin
  m := regexp_match(p_session,
    '^Hearthstone_([0-9]{4})_([0-9]{2})_([0-9]{2})_[0-9]{2}_[0-9]{2}_[0-9]{2}$');
  if m is null
     or m[1]::integer < 2000
     or m[1]::integer > extract(year from now() at time zone 'UTC')::integer + 1 then
    return null;
  end if;
  return make_date(m[1]::integer, m[2]::integer, m[3]::integer);
exception when datetime_field_overflow then
  return null;
end;
$$;

revoke all on function private.session_date(text) from public, anon, authenticated, service_role;

-- As in 20261012180000_file_retention.sql, plus the session check first.
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
-- Sweep runs that record every failed step
-- ---------------------------------------------------------------------------

drop function public.sweep_record_result(bigint, integer, jsonb, jsonb);

-- As in 20261012180000_file_retention.sql, plus p_failed (see the top).
-- Refuses (22023) counts that do not add up, steps it does not know or that
-- do not match the counts, and a row that is missing or already ended.
create function public.sweep_record_result(
  p_run_id bigint,
  p_files integer,
  p_stats jsonb,
  p_retention jsonb default null,
  p_failed text[] default null
)
returns void
language plpgsql
security definer
set search_path = ''
as $$
declare
  v_failed constant text[] := coalesce(p_failed, '{}');
  v_add jsonb;
begin
  if p_files is null or p_files < 0
     or not v_failed <@ array['list files', 'delete files', 'housekeeping', 'list summaries']
     or cardinality(v_failed) <> (select count(distinct f) from unnest(v_failed) as f)
     or (p_run_id is null) <> ('housekeeping' = any(v_failed))
     or (p_stats is null) <> ('list summaries' = any(v_failed))
     or (p_stats is not null and not private.sweep_stats_valid(p_stats))
     or (p_retention is not null and not private.sweep_retention_valid(p_retention)) then
    raise exception 'sweep_record_result: counts out of shape' using errcode = '22023';
  end if;
  v_add := jsonb_build_object('files', p_files, 'stats', p_stats)
    || case when p_retention is null then '{}'::jsonb
            else jsonb_build_object('retention', p_retention) end
    || case when cardinality(v_failed) = 0 then '{}'::jsonb
            else jsonb_build_object('failed_steps', to_jsonb(v_failed)) end;
  if p_run_id is null then
    delete from private.sweep_runs r where r.ran_at < now() - interval '90 days';
    insert into private.sweep_runs (result) values (v_add);
    return;
  end if;
  update private.sweep_runs r
  set result = r.result || v_add
  where r.id = p_run_id and not (r.result ? 'stats') and not (r.result ? 'failed_step');
  if not found then
    raise exception 'sweep_record_result: no open run %', p_run_id using errcode = '22023';
  end if;
end;
$$;

revoke all on function public.sweep_record_result(bigint, integer, jsonb, jsonb, text[])
  from public, anon, authenticated;
grant execute on function public.sweep_record_result(bigint, integer, jsonb, jsonb, text[])
  to service_role;

-- T-524 (session 084, D-096): every sweep run records what its stats
-- backfill did, or which step failed.
--
-- private.sweep_runs held only the housekeeping counts, written half-way
-- through the run (sweep_housekeeping()), so nobody could tell later whether
-- a run filled any game summaries (T-502) or stopped part-way. Now:
--
-- * sweep_housekeeping() also returns its row's id as run_id (not stored in
--   the row itself);
-- * the sweep function ends a run with sweep_record_result(run_id, files,
--   stats): the orphan files it deleted and the backfill's counts, checked
--   here (games listed, looked at, filled, left for the next run, and the
--   ones skipped, by reason);
-- * a step that fails after the caller has proved it is the service role
--   or the scheduled job calls sweep_record_failure(run_id, step): it marks
--   that run's row, or adds a row of its own when the run failed before
--   housekeeping made one.
--
-- So a row is one of: housekeeping counts + files + stats (finished),
-- + failed_step (failed at that step), only failed_step (failed before
-- housekeeping), or housekeeping counts alone (still running, or stopped
-- without reporting: the function hit its wall-clock limit or crashed).
-- Empty is never zero. The table keeps no grants and its 90 days.
--
-- Last runs, in the SQL editor (read only):
--   select ran_at, result from private.sweep_runs order by ran_at desc limit 10;

-- Skip reasons: no_file (the stored file could not be read), bad_file (too
-- large, not gzip or JSON, or it fails the upload check), file_mismatch (its
-- SHA-256 is not the row's), write_failed (the write call failed or gave an
-- unknown answer), and the answers of game_stats_backfill_put(): missing,
-- changed, newer.
create function private.sweep_stats_valid(s jsonb)
returns boolean
language plpgsql
immutable
set search_path = ''
as $$
declare
  top constant text[] := array['listed', 'looked_at', 'filled', 'left', 'skipped'];
  reasons constant text[] := array[
    'no_file', 'bad_file', 'file_mismatch', 'write_failed', 'missing', 'changed', 'newer'
  ];
  k text;
  v jsonb;
  skipped bigint := 0;
begin
  if jsonb_typeof(s) is distinct from 'object'
     or jsonb_typeof(s -> 'skipped') is distinct from 'object' then
    return false;
  end if;
  if not (array(select jsonb_object_keys(s)) @> top and array(select jsonb_object_keys(s)) <@ top)
     or not (array(select jsonb_object_keys(s -> 'skipped')) @> reasons
             and array(select jsonb_object_keys(s -> 'skipped')) <@ reasons) then
    return false;
  end if;
  -- Whole numbers from 0 to 500 (the backfill's cap per run).
  for k, v in
    select e.key, e.value from jsonb_each(s - 'skipped') e
    union all
    select e.key, e.value from jsonb_each(s -> 'skipped') e
  loop
    if jsonb_typeof(v) <> 'number' or (v #>> '{}')::numeric not between 0 and 500
       or (v #>> '{}')::numeric <> trunc((v #>> '{}')::numeric) then
      return false;
    end if;
    if k = any(reasons) then
      skipped := skipped + (v #>> '{}')::numeric::bigint;
    end if;
  end loop;
  return (s ->> 'looked_at')::numeric + (s ->> 'left')::numeric = (s ->> 'listed')::numeric
     and (s ->> 'filled')::numeric + skipped = (s ->> 'looked_at')::numeric;
end;
$$;

-- As in 20261011090000_auth_events_retention.sql, plus run_id in the result.
create or replace function public.sweep_housekeeping()
returns jsonb
language plpgsql
security definer
set search_path = ''
as $$
declare
  n_auth bigint;
  n_events bigint;
  n_ip bigint;
  v_result jsonb;
  v_id bigint;
begin
  delete from auth.audit_log_entries a
  where a.created_at < now() - interval '90 days'
     or (a.created_at < now() - interval '1 hour'
    and (
      (
        a.payload ->> 'actor_id' ~ '^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$'
        and a.payload ->> 'actor_id' <> '00000000-0000-0000-0000-000000000000'
        and not exists (select 1 from auth.users u where u.id::text = a.payload ->> 'actor_id')
      )
      or (
        a.payload -> 'traits' ->> 'user_id' is not null
        and not exists (
          select 1 from auth.users u where u.id::text = a.payload -> 'traits' ->> 'user_id'
        )
      )
    ));
  get diagnostics n_auth = row_count;
  delete from private.upload_events e where e.at < now() - interval '2 days';
  get diagnostics n_events = row_count;
  delete from private.upload_ip_failures f where f.window_start < now() - interval '1 hour';
  get diagnostics n_ip = row_count;
  v_result := jsonb_build_object('auth_events', n_auth, 'upload_events', n_events, 'ip_failures', n_ip);
  delete from private.sweep_runs r where r.ran_at < now() - interval '90 days';
  insert into private.sweep_runs (result) values (v_result) returning id into v_id;
  return v_result || jsonb_build_object('run_id', v_id);
end;
$$;

-- Ends a run: adds the files deleted and the backfill's counts to its row.
-- Refuses (22023) counts that do not add up and a row that is missing or
-- already ended, so a bad call fails loudly instead of recording nonsense.
create function public.sweep_record_result(p_run_id bigint, p_files integer, p_stats jsonb)
returns void
language plpgsql
security definer
set search_path = ''
as $$
begin
  if p_files is null or p_files < 0 or not private.sweep_stats_valid(p_stats) then
    raise exception 'sweep_record_result: counts out of shape' using errcode = '22023';
  end if;
  update private.sweep_runs r
  set result = r.result || jsonb_build_object('files', p_files, 'stats', p_stats)
  where r.id = p_run_id and not (r.result ? 'stats') and not (r.result ? 'failed_step');
  if not found then
    raise exception 'sweep_record_result: no open run %', p_run_id using errcode = '22023';
  end if;
end;
$$;

-- Records the step a run failed at: on its row when it has one, else on a
-- new row. Returns that row's id. A row that has already ended is left as it
-- is (its result was stored and only the answer to that call was lost), so a
-- row with only failed_step always means a run that failed before
-- housekeeping gave it a row.
create function public.sweep_record_failure(p_run_id bigint, p_step text)
returns bigint
language plpgsql
security definer
set search_path = ''
as $$
declare
  v_id bigint;
begin
  if p_step is null or p_step not in (
    'list files', 'delete files', 'housekeeping', 'list summaries', 'record run', 'unexpected'
  ) then
    raise exception 'sweep_record_failure: unknown step' using errcode = '22023';
  end if;
  update private.sweep_runs r
  set result = r.result || jsonb_build_object('failed_step', p_step)
  where r.id = p_run_id and not (r.result ? 'stats') and not (r.result ? 'failed_step')
  returning r.id into v_id;
  if v_id is null then
    select r.id into v_id from private.sweep_runs r where r.id = p_run_id;
  end if;
  if v_id is null then
    delete from private.sweep_runs r where r.ran_at < now() - interval '90 days';
    insert into private.sweep_runs (result)
    values (jsonb_build_object('failed_step', p_step))
    returning id into v_id;
  end if;
  return v_id;
end;
$$;

revoke all on function private.sweep_stats_valid(jsonb) from public, anon, authenticated, service_role;
revoke all on function public.sweep_record_result(bigint, integer, jsonb)
  from public, anon, authenticated;
revoke all on function public.sweep_record_failure(bigint, text) from public, anon, authenticated;
grant execute on function public.sweep_record_result(bigint, integer, jsonb) to service_role;
grant execute on function public.sweep_record_failure(bigint, text) to service_role;

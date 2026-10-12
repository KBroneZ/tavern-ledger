-- T-543 (D-109): every sweep run records how full the hosted
-- project is, and flags 50 % and 80 % of the Free plan's limits.
--
-- sweep_housekeeping() now also measures the database size
-- (pg_database_size) and the games bucket's file bytes and file count (from
-- storage.objects metadata, as data-capacity.md section 7), and stores them
-- as "capacity" in its row of private.sweep_runs and in its answer, so the
-- sweep function's response carries the same numbers:
--
--   "capacity": {
--     "database": {"bytes": ..., "limit": 500000000, "level": "none"},
--     "files": {"bytes": ..., "count": ..., "limit": 1000000000, "level": "none"},
--     "level": "none",            -- the highest of the two
--     "limits_read_on": "2026-10-12"
--   }
--
-- Levels: "none" under 50 % of the limit, "50%" from half, "80%" from 80 %
-- (D-109: Pro comes at 80 % of any Free limit). The measurement never stops
-- housekeeping or the rest of the sweep: if it fails, the row holds
-- "capacity": {"failed": true, "sqlstate": "..."} (no message, which could
-- name tables or values), no level, and the clean-up goes on. A row without
-- "capacity" is from before this migration or from a run that failed before
-- housekeeping. Empty is never "none".
--
-- The Free plan's 1 GB of file storage covers every bucket; "games" is the
-- only bucket today, so its bytes are the project's. A new bucket must be
-- added to the measurement.
--
-- Latest level, read only (deploy.md 10.5):
--   select ran_at, result -> 'capacity' from private.sweep_runs order by ran_at desc limit 1;

-- The limits, in one place: a plan change is a migration that replaces this
-- function. Read on 2026-10-12 on https://supabase.com/pricing and
-- https://supabase.com/docs/guides/platform/database-size (data-capacity.md
-- section 2.1): Free plan, database 500 MB (read-only above it), file storage
-- 1 GB. Taken as decimal bytes (500,000,000 and 1,000,000,000), the lower
-- reading, so the alert comes early rather than late.
create function private.capacity_limits()
returns jsonb
language sql
stable
set search_path = ''
as $$
  select jsonb_build_object(
    'plan', 'free',
    'database_bytes', 500000000,
    'files_bytes', 1000000000,
    'read_on', '2026-10-12',
    'source', 'https://supabase.com/pricing'
  );
$$;

-- "80%" at 80 % of the limit or more (also above it), "50%" from half,
-- "none" below; null when the measure or the limit is missing.
create function private.capacity_level(p_bytes bigint, p_limit bigint)
returns text
language sql
immutable
set search_path = ''
as $$
  select case
    when p_bytes is null or p_limit is null or p_limit <= 0 then null
    when p_bytes::numeric * 100 >= p_limit::numeric * 80 then '80%'
    when p_bytes::numeric * 100 >= p_limit::numeric * 50 then '50%'
    else 'none'
  end;
$$;

-- Totals and sizes only: no names, ids or paths. Objects whose metadata has
-- no size yet (an upload in progress) are counted, not sized.
create function private.sweep_capacity()
returns jsonb
language plpgsql
stable
set search_path = ''
as $$
declare
  lim jsonb := private.capacity_limits();
  db_bytes bigint := pg_database_size(current_database());
  db_limit bigint := (lim ->> 'database_bytes')::bigint;
  f_limit bigint := (lim ->> 'files_bytes')::bigint;
  f_bytes bigint;
  f_count bigint;
  db_level text;
  f_level text;
begin
  select coalesce(sum((o.metadata ->> 'size')::bigint), 0), count(*)
  into f_bytes, f_count
  from storage.objects o
  where o.bucket_id = 'games';
  db_level := private.capacity_level(db_bytes, db_limit);
  f_level := private.capacity_level(f_bytes, f_limit);
  return jsonb_build_object(
    'database', jsonb_build_object('bytes', db_bytes, 'limit', db_limit, 'level', db_level),
    'files', jsonb_build_object(
      'bytes', f_bytes, 'count', f_count, 'limit', f_limit, 'level', f_level
    ),
    'level', case
      when db_level is null or f_level is null then null
      when '80%' in (db_level, f_level) then '80%'
      when '50%' in (db_level, f_level) then '50%'
      else 'none'
    end,
    'limits_read_on', lim ->> 'read_on'
  );
end;
$$;

-- As in 20261011200000_sweep_counts.sql, plus the capacity in the row and
-- the answer.
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
  v_capacity jsonb;
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
  -- Measured after the clean-up; a failure here is recorded, never raised.
  -- "others" leaves out query_canceled (a statement timeout), which would
  -- undo the clean-up above, so it is caught by name.
  begin
    v_capacity := private.sweep_capacity();
  exception when query_canceled or others then
    v_capacity := jsonb_build_object('failed', true, 'sqlstate', sqlstate);
  end;
  v_result := jsonb_build_object(
    'auth_events', n_auth, 'upload_events', n_events, 'ip_failures', n_ip,
    'capacity', v_capacity
  );
  delete from private.sweep_runs r where r.ran_at < now() - interval '90 days';
  insert into private.sweep_runs (result) values (v_result) returning id into v_id;
  return v_result || jsonb_build_object('run_id', v_id);
end;
$$;

revoke all on function private.capacity_limits() from public, anon, authenticated, service_role;
revoke all on function private.capacity_level(bigint, bigint)
  from public, anon, authenticated, service_role;
revoke all on function private.sweep_capacity() from public, anon, authenticated, service_role;

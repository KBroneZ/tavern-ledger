-- T-564: the daily sweep caps sign-in sessions per user and drops auth audit
-- entries after 30 days.
--
-- sweep_housekeeping() now also runs private.sweep_auth_cleanup():
--
-- * sessions: for each user, auth.sessions beyond the 10 most recently used
--   (updated_at, else created_at) are deleted, with their refresh tokens
--   (auth.refresh_tokens.session_id cascades). A user who signs in from the
--   app and from a browser keeps both: only the oldest beyond 10 go.
-- * audit: auth.audit_log_entries older than 30 days are deleted (the 90-day
--   limit and the clean-up of deleted accounts' entries in housekeeping stay).
--
-- The counts are stored as "auth_cleanup" in the run's row and in
-- housekeeping's answer:
--   "auth_cleanup": {"sessions": 2, "refresh_tokens": 2, "audit_entries": 5}
-- A failure there never stops housekeeping or the rest of the sweep: the row
-- holds "auth_cleanup": {"failed": true, "sqlstate": "..."} (no message) and
-- the clean-up is tried again on the next run. A row without "auth_cleanup"
-- is from before this migration or from a run that failed before housekeeping.
--
-- Last result, read only (deploy.md 10.5):
--   select ran_at, result -> 'auth_cleanup' from private.sweep_runs order by ran_at desc limit 1;

-- Counts only: no ids. The limits are the two constants below.
create function private.sweep_auth_cleanup()
returns jsonb
language plpgsql
set search_path = ''
as $$
declare
  max_sessions constant integer := 10;
  audit_days constant integer := 30;
  n_sessions bigint;
  n_tokens bigint;
  n_audit bigint;
begin
  with doomed as (
    select s.id
    from (
      select x.id,
             row_number() over (
               partition by x.user_id
               order by coalesce(x.updated_at, x.created_at) desc, x.created_at desc, x.id desc
             ) as rank
      from auth.sessions x
    ) s
    where s.rank > max_sessions
  ),
  tokens as (
    select count(*) as n from auth.refresh_tokens t where t.session_id in (select id from doomed)
  ),
  gone as (
    delete from auth.sessions s where s.id in (select id from doomed) returning s.id
  )
  select (select count(*) from gone), (select n from tokens) into n_sessions, n_tokens;

  delete from auth.audit_log_entries a
  where a.created_at < now() - make_interval(days => audit_days);
  get diagnostics n_audit = row_count;

  return jsonb_build_object(
    'sessions', n_sessions, 'refresh_tokens', n_tokens, 'audit_entries', n_audit
  );
end;
$$;

-- As in 20261012150000_storage_alert.sql, plus the clean-up of the auth tables.
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
  v_cleanup jsonb;
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
  -- A failure here is recorded, never raised; the block undoes its own
  -- deletes. "others" leaves out query_canceled (a statement timeout), which
  -- would undo the clean-up above, so it is caught by name.
  begin
    v_cleanup := private.sweep_auth_cleanup();
  exception when query_canceled or others then
    v_cleanup := jsonb_build_object('failed', true, 'sqlstate', sqlstate);
  end;
  -- Measured after the clean-up; same rule.
  begin
    v_capacity := private.sweep_capacity();
  exception when query_canceled or others then
    v_capacity := jsonb_build_object('failed', true, 'sqlstate', sqlstate);
  end;
  v_result := jsonb_build_object(
    'auth_events', n_auth, 'upload_events', n_events, 'ip_failures', n_ip,
    'auth_cleanup', v_cleanup, 'capacity', v_capacity
  );
  delete from private.sweep_runs r where r.ran_at < now() - interval '90 days';
  insert into private.sweep_runs (result) values (v_result) returning id into v_id;
  return v_result || jsonb_build_object('run_id', v_id);
end;
$$;

revoke all on function private.sweep_auth_cleanup() from public, anon, authenticated, service_role;

-- T-104i (session 037, D-048): Auth's security log is kept 90 days.
--
-- auth.audit_log_entries (data inventory S5: event, time, IP address) is
-- exported with the account (export_my_data() -> auth_events) and deleted
-- with it. Until now it also stayed as long as the account; the daily sweep
-- now removes every entry older than 90 days as well, of any account or of
-- none. Long enough to look into an abuse report, short enough that a
-- years-old account does not keep every IP address it ever signed in from.
--
-- Why the hosted export had no auth_events (session 029): the hosted
-- project's "Write audit logs to the database" switch (Authentication ->
-- Audit Logs) was off, so Supabase Auth kept the log only in its own logs
-- (one day on the Free plan) and never wrote this table. Nothing in the
-- migrations, the export's filter or row-level security was at fault: the
-- local stack writes the table and its export holds the entries (web
-- test/local). The switch is a dashboard setting with no SQL or config.toml
-- equivalent; the owner turns it on (docs/research/deploy.md, section 10.13).

-- As in 20261010090000_sweep_schedule.sql, plus the 90-day limit.
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
  insert into private.sweep_runs (result) values (v_result);
  return v_result;
end;
$$;

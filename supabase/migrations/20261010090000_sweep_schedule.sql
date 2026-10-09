-- T-104a hosted (session 022): run the daily sweep (T-104d) from the database.
--
-- pg_cron calls private.run_sweep() once a day; it posts to the `sweep` Edge
-- Function with a token kept in Vault. The token is made here from random
-- bytes inside the database, so nobody has to copy a key anywhere; the
-- function asks public.sweep_token_valid() (service role only) whether the
-- token it got is the right one. The project's own URL is a second Vault
-- entry, `project_url`, set by hand on the hosted project
-- (docs/research/deploy.md, section 10.5). Without it the job sends nothing
-- and says so, which is what happens on the local stack.
--
-- pg_cron only says that the request was queued and pg_net keeps the answer
-- for a few hours, so a sweep that keeps failing would go unnoticed: every
-- finished sweep now leaves a row in private.sweep_runs (kept 90 days).

create extension if not exists pg_cron with schema pg_catalog;
create extension if not exists pg_net with schema extensions;

do $$
begin
  if not exists (select 1 from vault.secrets where name = 'sweep_token') then
    perform vault.create_secret(
      encode(extensions.gen_random_bytes(32), 'hex'),
      'sweep_token',
      'Sent by the daily sweep job to the sweep Edge Function (X-Sweep-Token).'
    );
  end if;
end;
$$;

-- Compares digests so the time taken does not depend on where the strings
-- differ. False when the token is missing on either side.
create function public.sweep_token_valid(p_token text)
returns boolean
language sql
stable
security definer
set search_path = ''
as $$
  select coalesce((
    select extensions.digest(s.decrypted_secret, 'sha256')
         = extensions.digest(p_token, 'sha256')
    from vault.decrypted_secrets s
    where s.name = 'sweep_token'
  ), false);
$$;

-- The pg_net request id, or null when Vault lacks project_url or the token.
-- pg_net sends the request after the calling transaction commits; its answer
-- is kept in net._http_response for a few hours.
create function private.run_sweep()
returns bigint
language plpgsql
security definer
set search_path = ''
as $$
declare
  v_url text;
  v_token text;
begin
  select s.decrypted_secret into v_url from vault.decrypted_secrets s where s.name = 'project_url';
  select s.decrypted_secret into v_token from vault.decrypted_secrets s where s.name = 'sweep_token';
  if v_url is null or v_token is null then
    raise warning 'sweep: Vault entry project_url or sweep_token is missing; nothing sent';
    return null;
  end if;
  return net.http_post(
    url := rtrim(v_url, '/') || '/functions/v1/sweep',
    body := '{}'::jsonb,
    headers := jsonb_build_object(
      'Content-Type', 'application/json',
      'X-Sweep-Token', v_token
    ),
    timeout_milliseconds := 30000
  );
end;
$$;

create table private.sweep_runs (
  ran_at timestamptz primary key default clock_timestamp(),
  result jsonb not null
);
revoke all on private.sweep_runs from public, anon, authenticated, service_role;

-- As in 20261009150000_upload.sql, plus the run record at the end.
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
  where a.created_at < now() - interval '1 hour'
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
    );
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

revoke all on function public.sweep_token_valid(text) from public, anon, authenticated;
grant execute on function public.sweep_token_valid(text) to service_role;
revoke all on function private.run_sweep() from public, anon, authenticated, service_role;

-- Every day at 03:17 UTC (a quiet hour, off the round hour most jobs use).
-- cron.schedule replaces a job of the same name, so this can run again.
select cron.schedule('sweep-daily', '17 3 * * *', 'select private.run_sweep()');

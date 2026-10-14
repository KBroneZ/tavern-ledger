-- The daily sweep no longer sends the Vault secret itself.
--
-- pg_net keeps each request, headers included, in net.http_request_queue and
-- net._http_response for a few hours, so the secret sat there readable to
-- anyone who could read those tables. private.run_sweep() now sends
-- '<unix seconds>.<hex HMAC-SHA256 of the seconds, keyed with the Vault
-- secret>', and public.sweep_token_valid() accepts it for five minutes (one
-- minute of clock skew ahead). A copy taken from the queue is useless after
-- five minutes, and the secret never leaves the database. The same Vault secret is the
-- key, so nothing has to be set anywhere.
--
-- Cutover: the sweep function that understands the signed token may reach the
-- hosted project after this migration. So for two days (signed_from) the job
-- still sends the bare secret, which both old and new function accept; after
-- that it signs. The bare secret is accepted for seven days (legacy_until).
-- A later migration drops the table and the legacy branch.

create table private.sweep_token_transition (
  signed_from timestamptz not null,
  legacy_until timestamptz not null
);
insert into private.sweep_token_transition (signed_from, legacy_until)
values (now() + interval '2 days', now() + interval '7 days');
revoke all on private.sweep_token_transition from public, anon, authenticated, service_role;

create or replace function public.sweep_token_valid(p_token text)
returns boolean
language plpgsql
stable
security definer
set search_path = ''
as $$
declare
  v_secret text;
  v_seconds bigint;
  v_age bigint;
begin
  select s.decrypted_secret into v_secret
  from vault.decrypted_secrets s where s.name = 'sweep_token';
  if v_secret is null or p_token is null then
    return false;
  end if;

  if p_token ~ '^[0-9]{1,12}\.[0-9a-f]{64}$' then
    v_seconds := split_part(p_token, '.', 1)::bigint;
    v_age := floor(extract(epoch from now()))::bigint - v_seconds;
    if v_age > 300 or v_age < -60 then
      return false;
    end if;
    -- Digests are compared so the time taken does not depend on where the
    -- strings differ.
    return extensions.digest(
             encode(extensions.hmac(split_part(p_token, '.', 1), v_secret, 'sha256'), 'hex'), 'sha256')
         = extensions.digest(split_part(p_token, '.', 2), 'sha256');
  end if;

  return exists (select 1 from private.sweep_token_transition t where t.legacy_until > now())
     and extensions.digest(v_secret, 'sha256') = extensions.digest(p_token, 'sha256');
end;
$$;

revoke all on function public.sweep_token_valid(text) from public, anon, authenticated;
grant execute on function public.sweep_token_valid(text) to service_role;

create or replace function private.run_sweep()
returns bigint
language plpgsql
security definer
set search_path = ''
as $$
declare
  v_url text;
  v_secret text;
  v_seconds text := floor(extract(epoch from now()))::bigint::text;
  v_signed boolean;
begin
  select s.decrypted_secret into v_url from vault.decrypted_secrets s where s.name = 'project_url';
  select s.decrypted_secret into v_secret from vault.decrypted_secrets s where s.name = 'sweep_token';
  if v_url is null or v_secret is null then
    raise warning 'sweep: Vault entry project_url or sweep_token is missing; nothing sent';
    return null;
  end if;
  select coalesce(bool_and(t.signed_from <= now()), true) into v_signed
  from private.sweep_token_transition t;
  return net.http_post(
    url := rtrim(v_url, '/') || '/functions/v1/sweep',
    body := '{}'::jsonb,
    headers := jsonb_build_object(
      'Content-Type', 'application/json',
      'X-Sweep-Token', case when v_signed
        then v_seconds || '.' || encode(extensions.hmac(v_seconds, v_secret, 'sha256'), 'hex')
        else v_secret end
    ),
    timeout_milliseconds := 30000
  );
end;
$$;

revoke all on function private.run_sweep() from public, anon, authenticated, service_role;

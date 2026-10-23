-- The sweep token cutover is over (migration 20261014180000 announced this).
--
-- The scheduled job signs its token ('<unix seconds>.<hex HMAC-SHA256 of the
-- seconds, keyed with the Vault secret>') once the cutover's signed_from date
-- has passed, so nothing sends the bare Vault secret any more. Apply this
-- only after that date (checked on the hosted project before merging). Now:
--   * public.sweep_token_valid() accepts only the signed token, for five
--     minutes (one minute of clock skew ahead); the bare secret is refused;
--   * private.run_sweep() always signs;
--   * private.sweep_token_transition is dropped.
-- The Vault secret 'sweep_token' stays: it is the HMAC key. Nothing has to be
-- set anywhere, and the sweep function accepts a signed token before and after
-- this migration, so the order of the two deploys does not matter.

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

  if p_token !~ '^[0-9]{1,12}\.[0-9a-f]{64}$' then
    return false;
  end if;

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
begin
  select s.decrypted_secret into v_url from vault.decrypted_secrets s where s.name = 'project_url';
  select s.decrypted_secret into v_secret from vault.decrypted_secrets s where s.name = 'sweep_token';
  if v_url is null or v_secret is null then
    raise warning 'sweep: Vault entry project_url or sweep_token is missing; nothing sent';
    return null;
  end if;
  return net.http_post(
    url := rtrim(v_url, '/') || '/functions/v1/sweep',
    body := '{}'::jsonb,
    headers := jsonb_build_object(
      'Content-Type', 'application/json',
      'X-Sweep-Token', v_seconds || '.' || encode(extensions.hmac(v_seconds, v_secret, 'sha256'), 'hex')
    ),
    timeout_milliseconds := 30000
  );
end;
$$;

revoke all on function private.run_sweep() from public, anon, authenticated, service_role;

drop table private.sweep_token_transition;

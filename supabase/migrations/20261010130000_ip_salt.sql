-- T-104g (D-042): salted keys for the refused-upload counter per IP.
--
-- Until now private.upload_ip_failures kept the plain SHA-256 of the client
-- IP that upload-game sends. The IPv4 space is small, so that hash can be
-- reversed by trying every address. From here on:
--
-- * the function still sends the plain SHA-256 (the address itself never
--   leaves the function), and the database stores only
--   HMAC-SHA-256(that hash, salt), so a stored key says nothing without the
--   salt;
-- * the salt is 32 random bytes made inside the database and kept in Vault
--   (`ip_salt`), encrypted with a key that is never in the database, so a dump
--   (tools/backup_supabase.ps1) holds the salt only encrypted; nobody ever
--   copies it;
-- * pg_cron replaces the salt every day and drops every counter in the same
--   transaction, so no row outlives the salt it was made with. A rotation
--   resets the counters: an IP that was blocked gets 30 more tries once a day.
--   A failure counted at the very moment of a rotation may land under the old
--   salt; it is then unreadable and goes within the hour like any counter;
-- * without a salt the checks fail (upload-game answers 500) instead of
--   storing or comparing an unsalted key.
--
-- The RPC signatures do not change, so the deployed function and this
-- migration can land in either order.

-- Unsalted keys (at most an hour old) go now.
delete from private.upload_ip_failures;

do $$
begin
  if not exists (select 1 from vault.secrets where name = 'ip_salt') then
    perform vault.create_secret(
      encode(extensions.gen_random_bytes(32), 'hex'),
      'ip_salt',
      'Salt for private.upload_ip_failures keys; replaced daily by private.rotate_ip_salt().'
    );
  end if;
end;
$$;

-- The stored key for the SHA-256 (lowercase hex) of a client IP.
create function private.ip_key(p_ip_hash text)
returns text
language plpgsql
stable
set search_path = ''
as $$
declare
  v_salt text;
begin
  if p_ip_hash is null or p_ip_hash !~ '^[0-9a-f]{64}$' then
    raise exception 'ip_key: expected the SHA-256 of an IP as lowercase hex'
      using errcode = '22023';
  end if;
  select s.decrypted_secret into v_salt from vault.decrypted_secrets s where s.name = 'ip_salt';
  if v_salt is null or v_salt !~ '^[0-9a-f]{64}$' then
    raise exception 'ip_key: Vault entry ip_salt is missing' using errcode = '55000';
  end if;
  return encode(
    extensions.hmac(convert_to(p_ip_hash, 'UTF8'), decode(v_salt, 'hex'), 'sha256'),
    'hex'
  );
end;
$$;

-- New salt, old salt and every counter gone, in one transaction.
create function private.rotate_ip_salt()
returns void
language plpgsql
security definer
set search_path = ''
as $$
declare
  v_id uuid;
  v_salt text := encode(extensions.gen_random_bytes(32), 'hex');
begin
  -- Two rotations at once would race on the Vault name.
  perform pg_advisory_xact_lock(hashtextextended('rotate_ip_salt', 0));
  select s.id into v_id from vault.secrets s where s.name = 'ip_salt';
  if v_id is null then
    perform vault.create_secret(
      v_salt,
      'ip_salt',
      'Salt for private.upload_ip_failures keys; replaced daily by private.rotate_ip_salt().'
    );
  else
    -- Vault's own API: the entry keeps its id and gets a new ciphertext.
    perform vault.update_secret(v_id, v_salt);
  end if;
  delete from private.upload_ip_failures;
end;
$$;

-- As in 20261009150000_upload.sql, with the salted key.
create or replace function public.upload_ip_blocked(p_ip_key text)
returns boolean
language sql
stable
security definer
set search_path = ''
as $$
  select coalesce(sum(f.failures), 0) >= 30
  from private.upload_ip_failures f
  where f.ip_key = private.ip_key(p_ip_key) and f.window_start > now() - interval '10 minutes';
$$;

create or replace function public.upload_ip_failed(p_ip_key text)
returns void
language sql
security definer
set search_path = ''
as $$
  delete from private.upload_ip_failures f where f.window_start < now() - interval '1 hour';
  insert into private.upload_ip_failures (ip_key, window_start, failures)
  values (private.ip_key(p_ip_key), date_trunc('minute', now()), 1)
  on conflict (ip_key, window_start)
  do update set failures = private.upload_ip_failures.failures + 1;
$$;

revoke all on function private.ip_key(text) from public, anon, authenticated, service_role;
revoke all on function private.rotate_ip_salt() from public, anon, authenticated, service_role;
revoke all on function public.upload_ip_blocked(text) from public, anon, authenticated;
revoke all on function public.upload_ip_failed(text) from public, anon, authenticated;
grant execute on function public.upload_ip_blocked(text) to service_role;
grant execute on function public.upload_ip_failed(text) to service_role;

comment on column private.upload_ip_failures.ip_key is
  'HMAC-SHA-256 of the SHA-256 of the client IP under the Vault salt ip_salt (D-042).';

-- Every day at 03:41 UTC, after the sweep (03:17). cron.schedule replaces a
-- job of the same name, so this can run again.
select cron.schedule('ip-salt-daily', '41 3 * * *', 'select private.rotate_ip_salt()');

-- Salted IP keys for refused uploads (T-104g, D-042). Run: npx supabase test db
-- The upload-game function sends a plain SHA-256 of the client IP; the
-- database stores only an HMAC of it under a random salt kept in Vault, and
-- a daily job replaces the salt and drops every counter.
begin;
create extension if not exists pgtap with schema extensions;
select plan(24);

delete from private.upload_ip_failures;

create function pg_temp.salt() returns text language sql as $$
  select decrypted_secret from vault.decrypted_secrets where name = 'ip_salt'
$$;
create function pg_temp.expected_key(ip_hash text) returns text language sql as $$
  select encode(extensions.hmac(convert_to(ip_hash, 'UTF8'), decode(pg_temp.salt(), 'hex'), 'sha256'), 'hex')
$$;

-- ------------------------------------------------------------ salt
select is((select count(*)::int from vault.secrets where name = 'ip_salt'), 1,
  'Vault holds one IP salt');
select ok(pg_temp.salt() ~ '^[0-9a-f]{64}$', 'the salt is 32 random bytes as hex');

-- ------------------------------------------------------------ stored key
select lives_ok($$ select public.upload_ip_failed(repeat('1', 64)) $$, 'a failure is counted');
select is((select count(*)::int from private.upload_ip_failures where ip_key = repeat('1', 64)), 0,
  'the hash the function sends is never stored as it is');
select is((select ip_key from private.upload_ip_failures), pg_temp.expected_key(repeat('1', 64)),
  'the stored key is HMAC-SHA-256 of that hash under the salt');
select ok((select ip_key ~ '^[0-9a-f]{64}$' from private.upload_ip_failures),
  'the stored key keeps the 64-hex format');
select throws_ok($$ select public.upload_ip_failed('1.2.3.4') $$, '22023', null,
  'an address in clear is refused, never stored');
select throws_ok($$ select public.upload_ip_blocked('1.2.3.4') $$, '22023', null,
  'an address in clear cannot be checked either');
select throws_ok($$ select public.upload_ip_failed(upper(repeat('a', 64))) $$, '22023', null,
  'only lowercase hex is accepted, so one IP has one key');

-- ------------------------------------------------------------ blocking
do $$ begin for i in 1..29 loop perform public.upload_ip_failed(repeat('2', 64)); end loop; end $$;
select ok(not public.upload_ip_blocked(repeat('2', 64)), '29 failures do not block');
do $$ begin perform public.upload_ip_failed(repeat('2', 64)); end $$;
select ok(public.upload_ip_blocked(repeat('2', 64)), '30 failures in 10 minutes block the IP');
select ok(not public.upload_ip_blocked(repeat('3', 64)), 'other IPs are not blocked');

-- ------------------------------------------------------------ rotation
create temporary table before as
  select pg_temp.salt() as salt, (select updated_at from vault.secrets where name = 'ip_salt') as at;
select lives_ok($$ select private.rotate_ip_salt() $$, 'the salt rotates');
select is((select count(*)::int from vault.secrets where name = 'ip_salt'), 1,
  'after a rotation Vault still holds exactly one salt: the old one is gone');
select ok(pg_temp.salt() ~ '^[0-9a-f]{64}$' and pg_temp.salt() <> (select salt from before),
  'the new salt is a different random value');
select ok((select updated_at > (select at from before) from vault.secrets where name = 'ip_salt'),
  'a rotation moves updated_at, which the weekly check reads (deploy.md 10.5)');
select is((select count(*)::int from private.upload_ip_failures), 0,
  'a rotation drops every counter made under the old salt');
select ok(not public.upload_ip_blocked(repeat('2', 64)),
  'a rotation resets the counters (accepted: at most one extra burst a day)');
do $$ begin perform public.upload_ip_failed(repeat('2', 64)); end $$;
select is((select ip_key from private.upload_ip_failures), pg_temp.expected_key(repeat('2', 64)),
  'new failures use the new salt');

-- ------------------------------------------------------------ fail loud
delete from vault.secrets where name = 'ip_salt';
select throws_ok($$ select public.upload_ip_blocked(repeat('1', 64)) $$, '55000', null,
  'without a salt the check fails instead of storing or comparing an unsalted key');
select lives_ok($$ select private.rotate_ip_salt() $$, 'a rotation makes a missing salt again');

-- ------------------------------------------------------------ job, privileges
select results_eq(
  $$ select schedule, command, active from cron.job where jobname = 'ip-salt-daily' $$,
  $$ values ('41 3 * * *'::text, 'select private.rotate_ip_salt()'::text, true) $$,
  'the salt rotates every day at 03:41 UTC'
);
select ok(
  not exists (
    select 1 from unnest(array['private.rotate_ip_salt()', 'private.ip_key(text)']) as f(sig),
      unnest(array['anon', 'authenticated', 'service_role']) as r(role)
    where has_function_privilege(r.role, f.sig, 'execute')
  ),
  'only the database itself can rotate the salt or compute a key'
);
-- service_role can read Vault (Supabase's own grant, not ours): whoever holds
-- the secret key can recompute keys, as they can read every row anyway.
select ok(
  not exists (
    select 1 from unnest(array['vault.secrets', 'vault.decrypted_secrets']) as t(tbl),
      unnest(array['anon', 'authenticated']) as r(role)
    where has_table_privilege(r.role, t.tbl, 'select')
  ),
  'clients cannot read the salt'
);

select * from finish();
rollback;

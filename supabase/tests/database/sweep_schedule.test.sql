-- Daily sweep schedule. Run: npx supabase test db
-- The sweep function's side is tested in supabase/functions/sweep/.
begin;
create extension if not exists pgtap with schema extensions;
select plan(29);

-- ------------------------------------------------------------ extensions, job
select has_extension('pg_cron', 'pg_cron is installed');
select has_extension('pg_net', 'pg_net is installed');
select results_eq(
  $$ select schedule, command, active from cron.job where jobname = 'sweep-daily' $$,
  $$ values ('17 3 * * *'::text, 'select private.run_sweep()'::text, true) $$,
  'the sweep runs every day at 03:17 UTC'
);

-- ------------------------------------------------------------ token
select is(
  (select decrypted_secret ~ '^[0-9a-f]{64}$' from vault.decrypted_secrets where name = 'sweep_token'),
  true,
  'Vault holds a 32-byte random sweep token'
);
select is(
  public.sweep_token_valid((select decrypted_secret from vault.decrypted_secrets where name = 'sweep_token')),
  false,
  'the bare Vault token is refused: only the signed token is accepted'
);
select is(public.sweep_token_valid(repeat('0', 64)), false, 'a wrong bare token is refused');
select hasnt_table('private', 'sweep_token_transition', 'the cutover table is gone');

-- Time-bound token: '<unix seconds>.<HMAC-SHA256 of the seconds, keyed with the Vault token>'.
create function pg_temp.signed(at_seconds bigint, key text default null) returns text language sql as $$
  select at_seconds || '.' || encode(extensions.hmac(
    at_seconds::text,
    coalesce(key, (select decrypted_secret from vault.decrypted_secrets where name = 'sweep_token')),
    'sha256'), 'hex');
$$;
create function pg_temp.now_s(shift int default 0) returns bigint language sql as $$
  select floor(extract(epoch from now()))::bigint + shift;
$$;
select is(public.sweep_token_valid(pg_temp.signed(pg_temp.now_s())), true, 'a fresh signed token is accepted');
select is(public.sweep_token_valid(pg_temp.signed(pg_temp.now_s(-240))), true, 'a token four minutes old is accepted');
select is(public.sweep_token_valid(pg_temp.signed(pg_temp.now_s(-360))), false, 'a token six minutes old is refused');
select is(public.sweep_token_valid(pg_temp.signed(pg_temp.now_s(30))), true, 'a little clock skew ahead is accepted');
select is(public.sweep_token_valid(pg_temp.signed(pg_temp.now_s(300))), false, 'a token from the future is refused');
select is(public.sweep_token_valid(pg_temp.signed(pg_temp.now_s(), repeat('0', 64))), false,
  'a token signed with another key is refused');
select is(
  public.sweep_token_valid(
    (pg_temp.now_s(-1000))::text || '.' || split_part(pg_temp.signed(pg_temp.now_s()), '.', 2)),
  false, 'a signature cannot be moved to another time');
select is(public.sweep_token_valid(pg_temp.now_s()::text || '.' || repeat('a', 63)), false,
  'a malformed signed token is refused');
select is(public.sweep_token_valid(''), false, 'an empty token is refused');
select is(public.sweep_token_valid(null), false, 'no token is refused');

-- ------------------------------------------------------------ privileges
select ok(
  not has_function_privilege('anon', 'public.sweep_token_valid(text)', 'execute')
  and not has_function_privilege('authenticated', 'public.sweep_token_valid(text)', 'execute'),
  'clients cannot test tokens'
);
select ok(
  has_function_privilege('service_role', 'public.sweep_token_valid(text)', 'execute'),
  'the service role can test tokens (the sweep function does)'
);
select ok(
  not exists (
    select 1 from unnest(array['anon', 'authenticated', 'service_role']) as r(role)
    where has_function_privilege(r.role, 'private.run_sweep()', 'execute')
  ),
  'only the database itself can start the job'
);
select ok(
  not exists (
    select 1 from unnest(array['anon', 'authenticated', 'service_role']) as r(role)
    where has_table_privilege(r.role, 'private.sweep_runs', 'select,insert,update,delete')
  ),
  'nobody but the owner reads or writes the run record'
);

-- ------------------------------------------------------------ run_sweep
delete from vault.secrets where name = 'project_url';
select is(private.run_sweep(), null, 'without project_url the job sends nothing');

select lives_ok($$ select vault.create_secret('https://example.test/', 'project_url') $$, 'project_url can be set');
create temporary table sent as select private.run_sweep() as id;
select isnt((select id from sent), null, 'with project_url the job queues one request');
select results_eq(
  $$ select q.method::text, q.url, q.timeout_milliseconds
     from net.http_request_queue q where q.id = (select id from sent) $$,
  $$ values ('POST'::text, 'https://example.test/functions/v1/sweep'::text, 30000) $$,
  'the request posts to the sweep function'
);
select results_eq(
  $$ select q.headers ->> 'X-Sweep-Token' <> s.decrypted_secret,
            q.headers ->> 'X-Sweep-Token' ~ '^[0-9]{1,12}\.[0-9a-f]{64}$',
            public.sweep_token_valid(q.headers ->> 'X-Sweep-Token')
     from net.http_request_queue q, vault.decrypted_secrets s
     where q.id = (select id from sent) and s.name = 'sweep_token' $$,
  $$ values (true, true, true) $$,
  'the request carries a signed token that is valid now, never the Vault secret itself'
);

-- ------------------------------------------------------------ run record
delete from private.sweep_runs;
insert into private.sweep_runs (ran_at, result) values (now() - interval '91 days', '{}');
select lives_ok($$ select public.sweep_housekeeping() $$, 'a sweep finishes');
select is((select count(*)::int from private.sweep_runs), 1,
  'each finished sweep leaves one record and old ones go');
select ok(
  (select result ? 'auth_events' and result ? 'upload_events' and result ? 'ip_failures'
   from private.sweep_runs),
  'the record holds the housekeeping counts'
);

select * from finish();
rollback;

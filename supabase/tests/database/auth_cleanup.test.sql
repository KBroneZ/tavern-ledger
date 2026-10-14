-- The daily sweep keeps at most 10 sessions per user and drops auth audit
-- entries after 30 days (T-564). Run: npx supabase test db
begin;
create extension if not exists pgtap with schema extensions;
select plan(20);

delete from auth.audit_log_entries;
delete from auth.sessions;
delete from private.sweep_runs;

insert into auth.users (id, email) values
  ('00000000-0000-4000-8000-0000000000a1', 'a1@example.test'),
  ('00000000-0000-4000-8000-0000000000a2', 'a2@example.test'),
  ('00000000-0000-4000-8000-0000000000a3', 'a3@example.test');

-- a1: 12 sessions, session n last used n days ago (01 = newest, 12 = oldest).
insert into auth.sessions (id, user_id, created_at, updated_at)
select ('00000000-0000-4000-8000-0000000001' || lpad(n::text, 2, '0'))::uuid,
       '00000000-0000-4000-8000-0000000000a1', now() - make_interval(days => n + 100),
       now() - make_interval(days => n)
from generate_series(1, 12) n;
-- a2: 3 sessions (the app and two browsers), all old.
insert into auth.sessions (id, user_id, created_at, updated_at)
select ('00000000-0000-4000-8000-0000000002' || lpad(n::text, 2, '0'))::uuid,
       '00000000-0000-4000-8000-0000000000a2', now() - make_interval(days => n * 40),
       now() - make_interval(days => n * 40)
from generate_series(1, 3) n;
-- a3: exactly 10 sessions.
insert into auth.sessions (id, user_id, created_at, updated_at)
select ('00000000-0000-4000-8000-0000000003' || lpad(n::text, 2, '0'))::uuid,
       '00000000-0000-4000-8000-0000000000a3', now() - make_interval(days => n),
       now() - make_interval(days => n)
from generate_series(1, 10) n;
-- Every session has a refresh token.
insert into auth.refresh_tokens (token, user_id, session_id)
select 'tok-' || s.id, s.user_id::text, s.id from auth.sessions s;

insert into auth.audit_log_entries (id, payload, created_at, ip_address) values
  ('00000000-0000-4000-8000-0000000009a1', '{"action": "login"}', now() - interval '31 days', ''),
  ('00000000-0000-4000-8000-0000000009a2', '{"action": "login"}', now() - interval '45 days', ''),
  ('00000000-0000-4000-8000-0000000009a3', '{"action": "login"}', now() - interval '29 days', ''),
  ('00000000-0000-4000-8000-0000000009a4', '{"action": "login"}', now(), '');

-- ------------------------------------------------------------ privileges
select ok(
  not exists (
    select 1 from unnest(array['public', 'anon', 'authenticated', 'service_role']) as r(role)
    where has_function_privilege(r.role, 'private.sweep_auth_cleanup()', 'execute')
  ),
  'nobody but the owner calls the clean-up'
);
select ok(
  not exists (
    select 1 from unnest(array['anon', 'authenticated']) as r(role)
    where has_table_privilege(r.role, 'auth.sessions', 'select')
       or has_table_privilege(r.role, 'auth.refresh_tokens', 'select')
       or has_table_privilege(r.role, 'auth.audit_log_entries', 'select')
  ),
  'clients cannot read the auth tables'
);

-- ------------------------------------------------------------ run
create temporary table hk as select public.sweep_housekeeping() as r;

select is(
  (select r -> 'auth_cleanup' from hk),
  '{"sessions": 2, "refresh_tokens": 2, "audit_entries": 2}'::jsonb,
  'the answer counts the sessions, their refresh tokens and the audit entries removed'
);
select is(
  (select result -> 'auth_cleanup' from private.sweep_runs),
  (select r -> 'auth_cleanup' from hk),
  'the run''s row holds the same counts'
);

-- ------------------------------------------------------------ cap
select is(
  (select count(*)::int from auth.sessions where user_id = '00000000-0000-4000-8000-0000000000a1'),
  10,
  'a user with 12 sessions is left with 10'
);
select is(
  (select array_agg(right(id::text, 2) order by id) from auth.sessions
   where user_id = '00000000-0000-4000-8000-0000000000a1'),
  array['01', '02', '03', '04', '05', '06', '07', '08', '09', '10'],
  'the ones kept are the 10 most recently used'
);
select is(
  (select count(*)::int from auth.sessions
   where id in ('00000000-0000-4000-8000-000000000111', '00000000-0000-4000-8000-000000000112')),
  0,
  'the two oldest are gone'
);
select is(
  (select count(*)::int from auth.refresh_tokens where user_id = '00000000-0000-4000-8000-0000000000a1'),
  10,
  'their refresh tokens went with them and the others stay'
);
select is(
  (select count(*)::int from auth.sessions where user_id = '00000000-0000-4000-8000-0000000000a2'),
  3,
  'a user with 3 sessions keeps all 3, however old (the app and the browser both stay)'
);
select is(
  (select count(*)::int from auth.sessions where user_id = '00000000-0000-4000-8000-0000000000a3'),
  10,
  'a user with exactly 10 keeps them'
);
select is(
  (select count(*)::int from auth.refresh_tokens), 23,
  'no refresh token of a kept session is touched'
);

-- ------------------------------------------------------------ audit
select is(
  (select array_agg(right(id::text, 2) order by id) from auth.audit_log_entries),
  array['a3', 'a4'],
  'audit entries of the last 30 days stay, older ones go'
);

-- A second run has nothing to do.
select is(
  (public.sweep_housekeeping() -> 'auth_cleanup'),
  '{"sessions": 0, "refresh_tokens": 0, "audit_entries": 0}'::jsonb,
  'a second run removes nothing'
);

-- ------------------------------------------------------------ failure
create or replace function private.sweep_auth_cleanup()
returns jsonb language plpgsql set search_path = '' as $$
begin
  raise exception 'no access' using errcode = '42501';
end;
$$;
insert into private.upload_ip_failures (ip_key, window_start) values
  (repeat('cd', 32), now() - interval '2 hours');
delete from private.sweep_runs;
select lives_ok(
  $$ create temporary table hk2 as select public.sweep_housekeeping() as r $$,
  'a clean-up that fails never fails housekeeping'
);
select is(
  (select result -> 'auth_cleanup' from private.sweep_runs),
  '{"failed": true, "sqlstate": "42501"}'::jsonb,
  'the row records the failure (its SQLSTATE only)'
);
select is(
  (select r -> 'auth_cleanup' from hk2),
  '{"failed": true, "sqlstate": "42501"}'::jsonb,
  'and so does the answer'
);
select results_eq(
  $$ select result ? 'auth_events', result ? 'capacity' from private.sweep_runs $$,
  $$ values (true, true) $$,
  'the other counts and the capacity are still recorded'
);
select is(
  (select count(*)::int from private.upload_ip_failures where window_start < now() - interval '1 hour'),
  0,
  'the rest of housekeeping still ran'
);

-- A statement timeout in the clean-up is caught too, by name.
create or replace function private.sweep_auth_cleanup()
returns jsonb language plpgsql set search_path = '' as $$
begin
  raise exception 'canceling statement due to statement timeout' using errcode = 'query_canceled';
end;
$$;
delete from private.sweep_runs;
select lives_ok($$ select public.sweep_housekeeping() $$, 'a timeout in the clean-up is caught');
select is(
  (select result #>> '{auth_cleanup,sqlstate}' from private.sweep_runs),
  '57014',
  'and recorded by its SQLSTATE'
);

select * from finish();
rollback;

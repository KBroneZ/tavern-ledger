-- Upload support (T-104d). Run: npx supabase test db
-- The upload-game Edge Function calls these functions with the service role;
-- the function itself is tested in supabase/functions/upload-game/ and end to
-- end in tests/test_supabase_local.py.
begin;
create extension if not exists pgtap with schema extensions;
select plan(42);

-- Start from a known state (all rolled back at the end).
delete from auth.audit_log_entries;
delete from private.upload_events;
delete from private.upload_ip_failures;

insert into auth.users (id, email) values
  ('00000000-0000-4000-8000-00000000000a', 'a@example.test'),
  ('00000000-0000-4000-8000-00000000000b', 'b@example.test');

-- Shorthands for the tests below.
create function pg_temp.begin_a(idx integer, saved bigint, sha text, size integer default 2000)
returns jsonb language sql as $$
  select public.upload_begin('00000000-0000-4000-8000-00000000000a',
    'Hearthstone_2026_10_09_18_30_00', idx, saved, sha, size)
$$;
create function pg_temp.commit_a(idx integer, saved bigint, sha text, size integer default 2000)
returns text language sql as $$
  select public.upload_commit('00000000-0000-4000-8000-00000000000a',
    'Hearthstone_2026_10_09_18_30_00', idx, 'GT_BATTLEGROUNDS', 'ok', 'BG20_HERO_202',
    3, 253216, '2026-10-09', '{"BEAST": 4, "RACE_99": 1}', saved, sha, size, '0.1.0+r1')
$$;

-- ------------------------------------------------------------ schema changes
select has_column('public', 'games', 'parser_version', 'games has the parser version (T-107)');
select lives_ok(
  $$ update public.games set parser_version = '0.1.0+r1' where false $$,
  'parser_version can be written by the service role'
);
select throws_ok(
  $$ insert into public.games (user_id, session, game_index, game_type, status, played_on,
       saved_at, content_sha256, size_bytes, parser_version)
     values ('00000000-0000-4000-8000-00000000000a', 'Hearthstone_2026_10_09_18_30_00', 900,
       'GT_BATTLEGROUNDS', 'ok', '2026-10-09', 1, repeat('a', 64), 1, 'Name#1234') $$,
  '23514', null, 'a parser version that is not <x.y.z>+r<n> is rejected'
);
select ok(private.is_tribe_counts('{"RACE_99": 1, "BEAST": 2}'),
  'unknown tribes (RACE_<n>, as the parser prints them) are accepted');
select ok(not private.is_tribe_counts('{"Name#1234": 1}'),
  'tribe keys still refuse anything but upper-case letters, digits and _');

-- ------------------------------------------------------------ begin / commit
select is(pg_temp.begin_a(1, 100, repeat('a', 64)) ->> 'result', 'create',
  'a new game is accepted for creation');
select is((select count(*)::int from private.upload_events), 1,
  'an accepted upload is counted for the rate limits');
select is(pg_temp.commit_a(1, 100, repeat('a', 64)), 'created', 'the row is created');
select is(
  (select row(parser_version, tribes_offered, final_place)::text from public.games
   where game_index = 1),
  row('0.1.0+r1', '{"BEAST": 4, "RACE_99": 1}'::jsonb, 3)::text,
  'the row holds what the function sent'
);
select is(pg_temp.begin_a(1, 100, repeat('a', 64)) ->> 'result', 'unchanged',
  'the same bytes again are not written twice');
select is(pg_temp.begin_a(1, 99, repeat('b', 64)) ->> 'result', 'older',
  'an older revision is refused before anything is written');
select is(pg_temp.begin_a(1, 100, repeat('b', 64)) ->> 'result', 'replace',
  'other bytes with the same revision replace the game');
select is(pg_temp.begin_a(1, 101, repeat('b', 64)) ->> 'result', 'replace',
  'a newer revision replaces the game');
select is(pg_temp.commit_a(1, 101, repeat('b', 64)), 'replaced', 'the row is replaced');
select is(pg_temp.commit_a(1, 50, repeat('c', 64)), 'older',
  'a commit older than the stored row changes nothing');
select is((select content_sha256 from public.games where game_index = 1), repeat('b', 64),
  'the newer row is kept');
select throws_ok(
  $$ select public.upload_commit('00000000-0000-4000-8000-00000000000a',
       'Hearthstone_2026_10_09_18_30_00', 2, 'GT_BATTLEGROUNDS_DUO', 'ok', 'BG20_HERO_202',
       6, 253216, '2026-10-09', '{}', 1, repeat('a', 64), 10, null) $$,
  '23514', null, 'the table checks still apply to a commit (Duos place 6)'
);

-- ------------------------------------------------------------ rate limits
delete from private.upload_events;
insert into private.upload_events (user_id, at)
select '00000000-0000-4000-8000-00000000000a', now() - interval '30 minutes'
from generate_series(1, 600);
select is(pg_temp.begin_a(3, 1, repeat('a', 64)) ->> 'result', 'rate_limited',
  '600 uploads in the last hour stop the next one');
select ok((pg_temp.begin_a(3, 1, repeat('a', 64)) ->> 'retry_after')::int between 1700 and 1801,
  'retry_after says when the oldest upload leaves the hour');
select is((select count(*)::int from private.upload_events), 600,
  'a refused upload is not counted');
select is(
  (select public.upload_begin('00000000-0000-4000-8000-00000000000b',
     'Hearthstone_2026_10_09_18_30_00', 1, 1, repeat('a', 64), 10) ->> 'result'),
  'create', 'the limit is per user'
);
delete from private.upload_events;
insert into private.upload_events (user_id, at)
select '00000000-0000-4000-8000-00000000000a', now() - interval '5 hours'
from generate_series(1, 5000);
select is(pg_temp.begin_a(3, 1, repeat('a', 64)) ->> 'result', 'rate_limited',
  '5000 uploads in a day stop the next one');
select ok((pg_temp.begin_a(3, 1, repeat('a', 64)) ->> 'retry_after')::int > 3600 * 18,
  'retry_after waits for the day window');
delete from private.upload_events;

-- ------------------------------------------------------------ total quota
insert into public.games (user_id, session, game_index, game_type, status, played_on,
  saved_at, content_sha256, size_bytes)
select '00000000-0000-4000-8000-00000000000a', format('Hearthstone_2026_01_01_00_%s_%s',
    lpad((i / 60)::text, 2, '0'), lpad((i % 60)::text, 2, '0')),
  1, 'GT_BATTLEGROUNDS', 'ok', '2026-01-01', 1, repeat('d', 64), 65536
from generate_series(1, 799) as i;
-- 800 games of 64 KiB = 50 MiB with the one already stored (size 2000).
select is(pg_temp.begin_a(3, 1, repeat('a', 64), 65536) ->> 'result', 'quota_bytes',
  'the byte quota (50 MiB) stops a new game');
select is(pg_temp.begin_a(1, 200, repeat('e', 64), 2000) ->> 'result', 'replace',
  'replacing a game with one of the same size still fits');
delete from public.games where game_index = 1 and session <> 'Hearthstone_2026_10_09_18_30_00';
insert into public.games (user_id, session, game_index, game_type, status, played_on,
  saved_at, content_sha256, size_bytes)
select '00000000-0000-4000-8000-00000000000a', 'Hearthstone_2026_02_02_00_00_00',
  i, 'GT_BATTLEGROUNDS', 'ok', '2026-02-02', 1, repeat('d', 64), 1
from generate_series(1, 1000) as i;
insert into public.games (user_id, session, game_index, game_type, status, played_on,
  saved_at, content_sha256, size_bytes)
select '00000000-0000-4000-8000-00000000000a', format('Hearthstone_2026_03_%s_00_00_00',
    lpad(s::text, 2, '0')),
  i, 'GT_BATTLEGROUNDS', 'ok', '2026-03-01', 1, repeat('d', 64), 1
from generate_series(1, 9) as s, generate_series(1, 1000) as i;
delete from private.upload_events;
select is((select count(*)::int from public.games
           where user_id = '00000000-0000-4000-8000-00000000000a'), 10001,
  'setup: the user holds more than 10,000 games');
select is(pg_temp.begin_a(3, 1, repeat('a', 64), 10) ->> 'result', 'quota_games',
  'the game quota (10,000) stops a new game');
select is(pg_temp.begin_a(1, 300, repeat('f', 64), 2000) ->> 'result', 'replace',
  'an existing game can still be replaced at the quota');
delete from public.games where session <> 'Hearthstone_2026_10_09_18_30_00';

-- ------------------------------------------------------------ IP limit
select ok(not public.upload_ip_blocked(repeat('1', 64)), 'a new IP is not blocked');
do $$ begin for i in 1..29 loop perform public.upload_ip_failed(repeat('1', 64)); end loop; end $$;
select ok(not public.upload_ip_blocked(repeat('1', 64)), '29 failures do not block');
do $$ begin perform public.upload_ip_failed(repeat('1', 64)); end $$;
select ok(public.upload_ip_blocked(repeat('1', 64)), '30 failures in 10 minutes block the IP');
select ok(not public.upload_ip_blocked(repeat('2', 64)), 'other IPs are not blocked');
update private.upload_ip_failures set window_start = window_start - interval '2 hours';
select ok(not public.upload_ip_blocked(repeat('1', 64)), 'failures older than 10 minutes expire');
select throws_ok($$ select public.upload_ip_failed('1.2.3.4') $$, '22023', null,
  'only a hashed IP is accepted, never the address (salted in ip_salt.test.sql)');
insert into private.upload_ip_failures (ip_key, window_start, failures)
values (repeat('3', 64), now() - interval '3 hours', 5);
do $$ begin perform public.upload_ip_failed(repeat('4', 64)); end $$;
select is((select count(*)::int from private.upload_ip_failures where ip_key = repeat('3', 64)), 0,
  'a failure write also removes counters older than an hour');

-- ------------------------------------------------------------ sweep
insert into storage.objects (bucket_id, name, created_at, updated_at) values
  ('games', '00000000-0000-4000-8000-00000000000a/Hearthstone_2026_10_09_18_30_00-1.json.gz',
   now() - interval '3 days', now() - interval '3 days'),
  ('games', '00000000-0000-4000-8000-00000000000a/Hearthstone_2026_10_09_18_30_00-7.json.gz',
   now() - interval '3 days', now() - interval '3 days'),
  ('games', '00000000-0000-4000-8000-00000000000a/Hearthstone_2026_10_09_18_30_00-8.json.gz',
   now(), now());
select results_eq(
  $$ select public.sweep_orphan_files(interval '1 hour') $$,
  $$ values ('00000000-0000-4000-8000-00000000000a/Hearthstone_2026_10_09_18_30_00-7.json.gz') $$,
  'the sweep lists old files with no row, not files with a row or uploads in progress'
);

delete from private.upload_events;
insert into private.upload_events (user_id, at) values
  ('00000000-0000-4000-8000-00000000000a', now() - interval '3 days'),
  ('00000000-0000-4000-8000-00000000000a', now());
insert into auth.audit_log_entries (id, payload, created_at) values
  (gen_random_uuid(), '{"action": "user_deleted", "actor_id": "00000000-0000-0000-0000-000000000000",
     "traits": {"user_id": "00000000-0000-4000-8000-0000000000dd", "user_email": "gone@example.test"}}',
   now() - interval '2 hours'),
  (gen_random_uuid(), '{"action": "login", "actor_id": "00000000-0000-4000-8000-0000000000dd"}',
   now() - interval '2 hours'),
  (gen_random_uuid(), '{"action": "login", "actor_id": "00000000-0000-4000-8000-00000000000a"}',
   now() - interval '2 hours'),
  (gen_random_uuid(), '{"action": "user_signedup", "actor_id": "00000000-0000-0000-0000-000000000000",
     "traits": {"user_id": "00000000-0000-4000-8000-00000000000b"}}', now() - interval '2 hours'),
  (gen_random_uuid(), '{"action": "token_revoked"}', now() - interval '2 hours'),
  (gen_random_uuid(), '{"action": "login", "actor_id": "00000000-0000-4000-8000-0000000000ee"}',
   now());
select is(public.sweep_housekeeping(),
  '{"auth_events": 2, "upload_events": 1, "ip_failures": 0}'::jsonb,
  'housekeeping removes leftovers of deleted accounts and expired counters');
select is(
  (select count(*)::int from auth.audit_log_entries where payload::text like '%0000dd%'), 0,
  'no audit entry of a deleted account is left (T-104a leftover)');
select is((select count(*)::int from auth.audit_log_entries), 4,
  'entries of existing users, entries with no user and recent entries stay');

-- ------------------------------------------------------------ export, deletion
set local role authenticated;
select set_config('request.jwt.claims',
  '{"sub": "00000000-0000-4000-8000-00000000000a", "role": "authenticated"}', true);
select is((select jsonb_array_length(public.export_my_data() -> 'upload_events')), 1,
  'the export lists the upload times kept for the rate limits');
reset role;
delete from auth.users where id = '00000000-0000-4000-8000-00000000000a';
select is((select count(*)::int from private.upload_events), 0,
  'deleting the account removes its upload times');

-- ------------------------------------------------------------ privileges
select ok(
  not exists (
    select 1 from unnest(array[
      'public.upload_begin(uuid,text,integer,bigint,text,integer)',
      'public.upload_commit(uuid,text,integer,text,text,text,integer,integer,date,jsonb,bigint,text,integer,text)',
      'public.upload_ip_blocked(text)', 'public.upload_ip_failed(text)',
      'public.sweep_orphan_files(interval)', 'public.sweep_housekeeping()']) as f(sig),
      unnest(array['anon', 'authenticated']) as r(role)
    where has_function_privilege(r.role, f.sig, 'execute')
  ),
  'clients cannot call the upload or sweep functions'
);

select * from finish();
rollback;

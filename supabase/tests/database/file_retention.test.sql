-- 28-day rolling retention of game files (T-544, D-109, D-113).
-- Run: npx supabase test db
-- The sweep function's side is tested in supabase/functions/sweep/.
begin;
create extension if not exists pgtap with schema extensions;
select plan(66);

-- A: a free account; B: another free account; E: unlimited history.
insert into auth.users (id, email) values
  ('00000000-0000-4000-8000-0000000000a1', 'a@example.test'),
  ('00000000-0000-4000-8000-0000000000b1', 'b@example.test'),
  ('00000000-0000-4000-8000-0000000000e1', 'e@example.test');

-- Only this test's games, so the counts are exact.
delete from public.game_stats;
delete from public.games;

create temporary table today as select (now() at time zone 'utc')::date as d;

-- A's games, by index: 1 boundary (kept), 2 one day past (due), 3 past with
-- no summary (kept), 4 today, 5 a future played_on first uploaded 40 days
-- ago (due), 6 already expired, 7 no summary 100 days old (still kept),
-- 8 past but changed in the last hour (kept).
insert into public.games (user_id, session, game_index, game_type, status, played_on,
  saved_at, content_sha256, size_bytes, created_at, updated_at, file_expired_at)
select u::uuid, 'Hearthstone_2026_01_01_00_00_00', i, 'GT_BATTLEGROUNDS', 'ok', p, 1791000000,
  repeat(s, 64), 2000, c, m, x
from (values
  ('00000000-0000-4000-8000-0000000000a1', 1, (select d from today) - 29, now(),
    now() - interval '2 hours', null::timestamptz, 'a'),
  ('00000000-0000-4000-8000-0000000000a1', 2, (select d from today) - 30, now(),
    now() - interval '2 hours', null, 'b'),
  ('00000000-0000-4000-8000-0000000000a1', 3, (select d from today) - 40, now(),
    now() - interval '2 hours', null, 'c'),
  ('00000000-0000-4000-8000-0000000000a1', 4, (select d from today), now(),
    now() - interval '2 hours', null, 'd'),
  ('00000000-0000-4000-8000-0000000000a1', 5, (select d from today) + 400,
    now() - interval '40 days', now() - interval '2 hours', null, 'e'),
  ('00000000-0000-4000-8000-0000000000a1', 6, (select d from today) - 30, now(),
    now() - interval '2 hours', now(), 'f'),
  ('00000000-0000-4000-8000-0000000000a1', 7, (select d from today) - 100, now(),
    now() - interval '2 hours', null, '7'),
  ('00000000-0000-4000-8000-0000000000a1', 8, (select d from today) - 35, now(),
    now(), null, '8'),
  ('00000000-0000-4000-8000-0000000000b1', 1, (select d from today) - 35, now(),
    now() - interval '2 hours', null, 'a'),
  ('00000000-0000-4000-8000-0000000000e1', 1, (select d from today) - 60, now(),
    now() - interval '2 hours', null, 'a')
) as v(u, i, p, c, m, x, s);
insert into public.game_stats (user_id, session, game_index, summary_version)
select g.user_id, g.session, g.game_index, 1 from public.games g
where not (g.user_id = '00000000-0000-4000-8000-0000000000a1' and g.game_index in (3, 7));
insert into private.unlimited_history (user_id) values ('00000000-0000-4000-8000-0000000000e1');

create function pg_temp.path(u text, i int) returns text language sql immutable as $$
  select u || '/Hearthstone_2026_01_01_00_00_00-' || i || '.json.gz';
$$;
create function pg_temp.a(i int) returns text language sql immutable as $$
  select pg_temp.path('00000000-0000-4000-8000-0000000000a1', i);
$$;
create function pg_temp.paths(l jsonb) returns text[] language sql immutable as $$
  select coalesce(array_agg(f ->> 'path' order by f ->> 'path'), '{}')
  from jsonb_array_elements(l -> 'files') f;
$$;
create function pg_temp.counts(l jsonb) returns int[] language sql immutable as $$
  select array[(l ->> 'due')::int, (l #>> '{skipped,exempt}')::int,
               (l #>> '{skipped,recent}')::int, (l #>> '{skipped,no_summary}')::int];
$$;

-- ------------------------------------------------------------ privileges
select ok(
  not exists (
    select 1 from unnest(array['public', 'anon', 'authenticated', 'service_role']) as r(role)
    where has_table_privilege(r.role, 'private.file_retention', 'select,insert,update,delete')
       or has_table_privilege(r.role, 'private.unlimited_history', 'select,insert,update,delete')
       or has_table_privilege(r.role, 'private.file_retention_attempts', 'select,insert,update,delete')
  ),
  'nobody but the owner reads or writes the settings, the attempts or the unlimited history mark'
);
select ok(
  not exists (
    select 1 from unnest(array['public', 'anon', 'authenticated', 'service_role']) as r(role)
    where has_function_privilege(r.role, 'private.game_day(date, timestamptz)', 'execute')
       or has_function_privilege(r.role, 'private.file_retention_state(public.games, integer)', 'execute')
       or has_function_privilege(r.role, 'private.games_file_replaced()', 'execute')
       or has_function_privilege(r.role, 'private.sweep_retention_valid(jsonb)', 'execute')
  ),
  'nobody but the owner calls the private helpers'
);
select ok(
  not exists (
    select 1 from unnest(array['anon', 'authenticated']) as r(role)
    where has_function_privilege(r.role, 'public.file_retention_list(integer, integer)', 'execute')
       or has_function_privilege(r.role, 'public.file_retention_check(text[], text[], integer)', 'execute')
       or has_function_privilege(r.role, 'public.file_retention_mark(text[], text[])', 'execute')
       or has_function_privilege(r.role,
            'public.sweep_record_result(bigint, integer, jsonb, jsonb, text[])', 'execute')
  ),
  'clients cannot list, check, mark or record'
);
select ok(
  has_function_privilege('service_role', 'public.file_retention_list(integer, integer)', 'execute')
  and has_function_privilege('service_role', 'public.file_retention_check(text[], text[], integer)', 'execute')
  and has_function_privilege('service_role', 'public.file_retention_mark(text[], text[])', 'execute')
  and has_function_privilege('service_role',
        'public.sweep_record_result(bigint, integer, jsonb, jsonb, text[])', 'execute'),
  'the service role can (the sweep function does)'
);
select ok(
  not exists (
    select 1 from unnest(array['anon', 'authenticated']) as r(role)
    where has_column_privilege(r.role, 'public.games', 'file_expired_at', 'insert,update')
  )
  and not has_column_privilege('authenticated', 'public.games', 'file_expired_at', 'select')
  and has_column_privilege('authenticated', 'public.game_summaries', 'file_expired_at', 'select'),
  'clients read file_expired_at only through game_summaries (the owner''s rows) and cannot write it'
);

set local role authenticated;
select set_config('request.jwt.claims',
  '{"sub": "00000000-0000-4000-8000-0000000000a1", "role": "authenticated"}', true);
select throws_ok(
  $$ insert into private.unlimited_history (user_id) values ('00000000-0000-4000-8000-0000000000a1') $$,
  '42501', null, 'a signed-in user cannot give themselves unlimited history'
);
select throws_ok($$ update public.games set file_expired_at = null $$, '42501', null,
  'a signed-in user cannot clear the expiry');
select throws_ok($$ select public.file_retention_list(10, 1) $$, '42501', null,
  'a signed-in user cannot list files');
select is(
  (select file_expired_at is not null from public.game_summaries
   where session = 'Hearthstone_2026_01_01_00_00_00' and game_index = 6),
  true, 'the owner reads the expiry with the summary'
);
reset role;

-- ------------------------------------------------------------ settings
select results_eq(
  $$ select days, live from private.file_retention $$,
  $$ values (28, true) $$,
  '28 days, live since migration 20261012190000 (the user turned it on)'
);
select throws_ok($$ insert into private.file_retention (days) values (90) $$,
  '23505', null, 'one settings row only');
select is(private.game_day((select d + 400 from today), now() - interval '40 days'),
  (select d - 40 from today), 'a played_on in the future counts from the first upload');

-- ------------------------------------------------------------ states
select results_eq(
  $$ select g.user_id::text, g.game_index, private.file_retention_state(g, 1)
     from public.games g order by 1, 2 $$,
  $$ values
     ('00000000-0000-4000-8000-0000000000a1'::text, 1, null::text),
     ('00000000-0000-4000-8000-0000000000a1', 2, 'due'),
     ('00000000-0000-4000-8000-0000000000a1', 3, 'no_summary'),
     ('00000000-0000-4000-8000-0000000000a1', 4, null),
     ('00000000-0000-4000-8000-0000000000a1', 5, 'due'),
     ('00000000-0000-4000-8000-0000000000a1', 6, null),
     ('00000000-0000-4000-8000-0000000000a1', 7, 'no_summary'),
     ('00000000-0000-4000-8000-0000000000a1', 8, 'recent'),
     ('00000000-0000-4000-8000-0000000000b1', 1, 'due'),
     ('00000000-0000-4000-8000-0000000000e1', 1, 'exempt') $$,
  'states: 29 days kept, 30 due; no summary, recent and exempt kept'
);
select is(
  (select private.file_retention_state(g, 2) from public.games g
   where g.user_id = '00000000-0000-4000-8000-0000000000a1' and g.game_index = 2),
  'no_summary', 'a summary of an older version is no summary (the backfill needs the file)'
);

-- ------------------------------------------------------------ report-only
update private.file_retention set live = false;
create temporary table report as select public.file_retention_list(1000, 1) as l;
select is((select pg_temp.counts(l) from report), array[3, 1, 1, 2],
  'report-only: due A2, A5, B1; kept by reason: exempt, recent, no summary (A3, A7)');
select is((select (l ->> 'no_summary_oldest_days')::int from report), 100,
  'report-only: the oldest game kept for want of a summary, in days');
select is((select (l ->> 'live')::boolean from report), false, 'report-only: says so');
select is((select jsonb_array_length(l -> 'files') from report), 0, 'report-only: hands out no file');
select is(pg_temp.counts(public.file_retention_list(1000, 2)), array[0, 1, 1, 5],
  'a newer summary version keeps every file whose summary is older, however old');
select throws_ok($$ select public.file_retention_list(10, null) $$, '22023', null,
  'list: needs the summary version');
select is(
  public.file_retention_check(array[pg_temp.a(2)], array[repeat('b', 64)], 1), '{}'::text[],
  'report-only: the check keeps everything'
);
select is(public.file_retention_mark(array[pg_temp.a(2)], array[repeat('b', 64)]), 0,
  'report-only: marking does nothing');
select is((select count(*)::int from public.games where file_expired_at is not null), 1,
  'report-only: only the game expired before is marked');

-- ------------------------------------------------------------ live
update private.file_retention set live = true;
create temporary table live as select public.file_retention_list(1000, 1) as l;
select is((select (l ->> 'live')::boolean from live), true, 'live: says so');
select is(
  (select pg_temp.paths(l) from live),
  array[pg_temp.a(2), pg_temp.a(5), pg_temp.path('00000000-0000-4000-8000-0000000000b1', 1)],
  'live: the due files only'
);
select is(
  (select array[l -> 'files' -> 0 ->> 'path', l -> 'files' -> 0 ->> 'sha256'] from live),
  array[pg_temp.a(5), repeat('e', 64)],
  'live: oldest game day first (A5 counts from its first upload), with the stored copy''s SHA-256'
);
select is(jsonb_array_length(public.file_retention_list(1, 1) -> 'files'), 1, 'live: bounded by the limit');
select is(jsonb_array_length(public.file_retention_list(0, 1) -> 'files'), 0, 'live: limit 0 lists nothing');
select is(jsonb_array_length(public.file_retention_list(null, 1) -> 'files'), 0, 'live: no limit lists nothing');
select is((public.file_retention_list(100000, 1) ->> 'due')::int, 3, 'live: the count is not capped');

-- The check right before a delete.
select is(
  public.file_retention_check(
    array[pg_temp.a(2), pg_temp.a(1), pg_temp.path('00000000-0000-4000-8000-0000000000e1', 1),
          pg_temp.path('00000000-0000-4000-8000-0000000000b1', 1), pg_temp.a(8), pg_temp.a(3)],
    array[repeat('b', 64), repeat('a', 64), repeat('a', 64), repeat('f', 64), repeat('8', 64),
          repeat('c', 64)],
    1),
  array[pg_temp.a(2)],
  'check: only still-due files of the same copy (not kept, exempt, another copy, recent, no summary)'
);
insert into private.unlimited_history (user_id) values ('00000000-0000-4000-8000-0000000000b1');
select is(
  public.file_retention_check(
    array[pg_temp.path('00000000-0000-4000-8000-0000000000b1', 1)], array[repeat('a', 64)], 1),
  '{}'::text[], 'check: an exemption added after the list keeps the file'
);
delete from private.unlimited_history where user_id = '00000000-0000-4000-8000-0000000000b1';
select throws_ok($$ select public.file_retention_check(array['x'], array[]::text[], 1) $$,
  '22023', null, 'check: lists of different lengths are refused');

-- Marks: only once the object is gone, only the named copy, never exempt.
-- A5's object is still there (its delete did not take); A2 had failed before.
insert into storage.objects (bucket_id, name) values ('games', pg_temp.a(5));
insert into private.file_retention_attempts (user_id, session, game_index, fails)
values ('00000000-0000-4000-8000-0000000000a1', 'Hearthstone_2026_01_01_00_00_00', 2, 3);
select is(
  public.file_retention_mark(
    array[pg_temp.a(2), pg_temp.a(5), pg_temp.a(1),
          pg_temp.path('00000000-0000-4000-8000-0000000000e1', 1),
          pg_temp.path('00000000-0000-4000-8000-0000000000b1', 1)],
    array[repeat('b', 64), repeat('e', 64), repeat('a', 64), repeat('a', 64), repeat('f', 64)]),
  1, 'mark: one game marked'
);
select results_eq(
  $$ select user_id::text, game_index from public.games
     where file_expired_at is not null order by user_id, game_index $$,
  $$ values ('00000000-0000-4000-8000-0000000000a1'::text, 2), ('00000000-0000-4000-8000-0000000000a1', 6) $$,
  'mark: A2 only (A5''s object is still there, A1 is kept, E is exempt, B1 named another copy)'
);
select is((select count(*)::int from public.game_stats), 8, 'mark: every summary stays');
select results_eq(
  $$ select game_index, fails from private.file_retention_attempts order by 1 $$,
  $$ values (5, 1) $$,
  'mark: an attempt is counted on A5 (still there); A2''s goes once it is marked'
);
select is(
  public.file_retention_mark(array[pg_temp.a(5)], array[repeat('e', 64)]), 0,
  'mark: A5 again, still there'
);
select is((select fails from private.file_retention_attempts where game_index = 5), 2,
  'mark: its attempts add up');
select is((public.file_retention_list(1000, 1) ->> 'due')::int, 2, 'a marked game is no longer due');
select is(
  public.file_retention_list(1000, 1) -> 'files' -> 0 ->> 'path',
  pg_temp.path('00000000-0000-4000-8000-0000000000b1', 1),
  'a file that keeps failing goes after the untried ones'
);
select throws_ok(
  $$ select public.file_retention_mark(array['x'], array[]::text[]) $$, '22023', null,
  'mark: lists of different lengths are refused'
);
select throws_ok(
  $$ select public.file_retention_mark(
       array(select 'x' from generate_series(1, 1001)), array(select 'y' from generate_series(1, 1001))) $$,
  '22023', null, 'mark: more than 1000 is refused'
);
select throws_ok($$ select public.file_retention_mark(null, null) $$, '22023', null,
  'mark: null lists are refused');

-- A new copy of an expired game, through the upload's own commit, is a new
-- file: the mark goes, and the game is recent (kept) for an hour.
select is(
  public.upload_commit('00000000-0000-4000-8000-0000000000a1', 'Hearthstone_2026_01_01_00_00_00', 2,
    'GT_BATTLEGROUNDS', 'ok', null, null, null, (select d - 30 from today), '{}'::jsonb,
    1791000001, repeat('9', 64), 2000, null),
  'replaced', 'a new copy of an expired game is stored'
);
select results_eq(
  $$ select g.file_expired_at, private.file_retention_state(g, 1) from public.games g
     where g.user_id = '00000000-0000-4000-8000-0000000000a1' and g.game_index = 2 $$,
  $$ values (null::timestamptz, 'recent'::text) $$,
  'a new copy clears the expiry and waits for a later sweep'
);
update public.games set status = 'incomplete'
where user_id = '00000000-0000-4000-8000-0000000000a1' and game_index = 6;
select isnt(
  (select file_expired_at from public.games
   where user_id = '00000000-0000-4000-8000-0000000000a1' and game_index = 6),
  null, 'other changes keep it'
);

-- ------------------------------------------------------------ backfill
select ok(
  exists (select 1 from public.game_stats_backfill_list(1, 500) where game_index = 3),
  'backfill: a game with no summary is listed'
);
update public.games set file_expired_at = now()
where user_id = '00000000-0000-4000-8000-0000000000a1' and game_index = 3;
select ok(
  not exists (select 1 from public.game_stats_backfill_list(1, 500) where game_index = 3),
  'backfill: not once its file has expired'
);

-- ------------------------------------------------------------ upload quota
-- B fills the 50 MiB bytes quota; expired files no longer count.
insert into public.games (user_id, session, game_index, game_type, status, played_on,
  saved_at, content_sha256, size_bytes)
select '00000000-0000-4000-8000-0000000000b1', 'Hearthstone_2026_02_02_00_00_00', i,
  'GT_BATTLEGROUNDS', 'ok', '2026-02-02', 1791000000, repeat('a', 64), 65536
from generate_series(1, 800) as i;
select is(
  public.upload_begin('00000000-0000-4000-8000-0000000000b1', 'Hearthstone_2026_03_03_00_00_00',
    1, 1791000000, repeat('c', 64), 2000) ->> 'result',
  'quota_bytes', 'quota: full'
);
update public.games set file_expired_at = now()
where user_id = '00000000-0000-4000-8000-0000000000b1' and session = 'Hearthstone_2026_02_02_00_00_00'
  and game_index <= 10;
select is(
  public.upload_begin('00000000-0000-4000-8000-0000000000b1', 'Hearthstone_2026_03_03_00_00_00',
    1, 1791000000, repeat('c', 64), 2000) ->> 'result',
  'create', 'quota: expired files free their bytes'
);

-- ------------------------------------------------------------ run record
delete from private.sweep_runs;
create temporary table run as select (public.sweep_housekeeping() ->> 'run_id')::bigint as id;
create function pg_temp.stats() returns jsonb language sql immutable as $$
  select '{"listed": 0, "looked_at": 0, "filled": 0, "left": 0, "skipped": {"no_file": 0,
    "bad_file": 0, "file_mismatch": 0, "write_failed": 0, "missing": 0, "changed": 0,
    "newer": 0}}'::jsonb;
$$;
create function pg_temp.ret(mode text, due int, listed int, changed int, deleted int, failed int,
  unmarked int, lft int) returns jsonb language sql immutable as $$
  select jsonb_build_object('mode', mode, 'days', 28, 'due', due, 'listed', listed,
    'changed', changed, 'deleted', deleted, 'failed', failed, 'unmarked', unmarked, 'left', lft,
    'skipped', jsonb_build_object('exempt', 1, 'recent', 0, 'no_summary', 0),
    'no_summary_oldest_days', null);
$$;
select ok(private.sweep_retention_valid(pg_temp.ret('report', 3, 0, 0, 0, 0, 0, 0)), 'record: report-only');
select ok(private.sweep_retention_valid(pg_temp.ret('live', 9, 8, 1, 3, 1, 1, 2)), 'record: live');
select ok(private.sweep_retention_valid(pg_temp.ret('live', 9, 8, 1, 3, 1, 1, 2)
  || '{"no_summary_oldest_days": 40}'), 'record: with the oldest game kept for want of a summary');
select ok(private.sweep_retention_valid('{"error": "list"}'), 'record: a failed list');
select ok(
  not private.sweep_retention_valid(pg_temp.ret('report', 3, 1, 0, 1, 0, 0, 0))
  and not private.sweep_retention_valid(pg_temp.ret('report', 3, 0, 1, 0, 0, 0, 0))
  and not private.sweep_retention_valid(pg_temp.ret('live', 9, 7, 1, 3, 1, 1, 2))
  and not private.sweep_retention_valid(pg_temp.ret('live', 9, 3, 0, 3, 0, 0, 0)
    || '{"no_summary_oldest_days": -1}')
  and not private.sweep_retention_valid(pg_temp.ret('live', 9, 3, 0, 3, 0, 0, 0)
    - 'no_summary_oldest_days')
  and not private.sweep_retention_valid(pg_temp.ret('live', 2, 6, 0, 6, 0, 0, 0))
  and not private.sweep_retention_valid(pg_temp.ret('live', 9, 3, 0, 3, 0, 0, 0) || '{"ids": 1}')
  and not private.sweep_retention_valid(pg_temp.ret('live', 9, 3, 0, 3, 0, 0, 0) || '{"due": -1}')
  and not private.sweep_retention_valid(pg_temp.ret('live', 9, 3, 0, 3, 0, 0, 0) || '{"due": 1.5}')
  and not private.sweep_retention_valid(pg_temp.ret('other', 0, 0, 0, 0, 0, 0, 0))
  and not private.sweep_retention_valid('{"error": "other"}')
  and not private.sweep_retention_valid('[]'),
  'record: counts that do not add up, extra keys, bad numbers or modes are refused'
);
select throws_ok(
  format($$ select public.sweep_record_result(%s, 0, %L, %L) $$,
    (select id from run), pg_temp.stats(), pg_temp.ret('report', 3, 1, 0, 1, 0, 0, 0)),
  '22023', null, 'record: refuses retention counts out of shape'
);
select lives_ok(
  format($$ select public.sweep_record_result(%s, 0, %L, %L) $$,
    (select id from run), pg_temp.stats(), pg_temp.ret('report', 3, 0, 0, 0, 0, 0, 0)),
  'record: stores retention counts'
);
select is(
  (select result -> 'retention' from private.sweep_runs where id = (select id from run)),
  pg_temp.ret('report', 3, 0, 0, 0, 0, 0, 0), 'record: the retention is on the run''s row'
);
create temporary table run2 as select (public.sweep_housekeeping() ->> 'run_id')::bigint as id;
select lives_ok(
  format($$ select public.sweep_record_result(p_run_id => %s, p_files => 0, p_stats => %L) $$,
    (select id from run2), pg_temp.stats()),
  'record: a sweep without the retention step still records its run'
);

-- ------------------------------------------------------------ export, deletion
set local role authenticated;
select set_config('request.jwt.claims',
  '{"sub": "00000000-0000-4000-8000-0000000000e1", "role": "authenticated"}', true);
select isnt((public.export_my_data() -> 'unlimited_history' ->> 'since'), null,
  'export: says the account has unlimited history');
select set_config('request.jwt.claims',
  '{"sub": "00000000-0000-4000-8000-0000000000a1", "role": "authenticated"}', true);
select is((public.export_my_data() -> 'unlimited_history'), 'null'::jsonb,
  'export: null without it');
select is(
  (public.export_my_data() -> 'file_retention_attempts' -> 0 ->> 'fails')::int, 2,
  'export: the failed delete attempts on the user''s files'
);
reset role;

select is(
  (public.delete_user_data('00000000-0000-4000-8000-0000000000e1') ->> 'games')::int, 1,
  'deletion: the rows go'
);
select is(
  (select count(*)::int from private.unlimited_history
   where user_id = '00000000-0000-4000-8000-0000000000e1'),
  0, 'deletion: the mark goes with them'
);
insert into private.unlimited_history (user_id) values ('00000000-0000-4000-8000-0000000000b1');
delete from auth.users where id = '00000000-0000-4000-8000-0000000000b1';
select is((select count(*)::int from private.unlimited_history), 0,
  'the mark goes with the auth user too');

select * from finish();
rollback;

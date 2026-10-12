-- 28-day rolling retention of game files (T-544, D-109, D-113).
-- Run: npx supabase test db
-- The sweep function's side is tested in supabase/functions/sweep/.
begin;
create extension if not exists pgtap with schema extensions;
select plan(49);

-- A: a free account; B: another free account; E: unlimited history.
insert into auth.users (id, email) values
  ('00000000-0000-4000-8000-0000000000a1', 'a@example.test'),
  ('00000000-0000-4000-8000-0000000000b1', 'b@example.test'),
  ('00000000-0000-4000-8000-0000000000e1', 'e@example.test');

-- Only this test's games, so the counts are exact.
delete from public.game_stats;
delete from public.games;

create temporary table today as select (now() at time zone 'utc')::date as d;

-- index: 1 boundary (kept), 2 one day past (due), 3 past with no summary,
-- 4 today, 5 a future played_on first uploaded 40 days ago (due),
-- 6 past and already expired.
insert into public.games (user_id, session, game_index, game_type, status, played_on,
  saved_at, content_sha256, size_bytes, created_at, file_expired_at)
select u::uuid, 'Hearthstone_2026_01_01_00_00_00', i, 'GT_BATTLEGROUNDS', 'ok', p, 1791000000,
  repeat(s, 64), 2000, c, x
from today, (values
  ('00000000-0000-4000-8000-0000000000a1', 1, (select d from today) - 29, now(), null::timestamptz, 'a'),
  ('00000000-0000-4000-8000-0000000000a1', 2, (select d from today) - 30, now(), null, 'b'),
  ('00000000-0000-4000-8000-0000000000a1', 3, (select d from today) - 40, now(), null, 'c'),
  ('00000000-0000-4000-8000-0000000000a1', 4, (select d from today), now(), null, 'd'),
  ('00000000-0000-4000-8000-0000000000a1', 5, (select d from today) + 400,
    now() - interval '40 days', null, 'e'),
  ('00000000-0000-4000-8000-0000000000a1', 6, (select d from today) - 30, now(), now(), 'f'),
  ('00000000-0000-4000-8000-0000000000b1', 1, (select d from today) - 35, now(), null, 'a'),
  ('00000000-0000-4000-8000-0000000000e1', 1, (select d from today) - 60, now(), null, 'a')
) as v(u, i, p, c, x, s);
insert into public.game_stats (user_id, session, game_index, summary_version)
select g.user_id, g.session, g.game_index, 1 from public.games g
where not (g.user_id = '00000000-0000-4000-8000-0000000000a1' and g.game_index = 3);
insert into private.unlimited_history (user_id) values ('00000000-0000-4000-8000-0000000000e1');

create function pg_temp.path(u text, i int) returns text language sql immutable as $$
  select u || '/Hearthstone_2026_01_01_00_00_00-' || i || '.json.gz';
$$;
create function pg_temp.paths(l jsonb) returns text[] language sql immutable as $$
  select coalesce(array_agg(f ->> 'path' order by f ->> 'path'), '{}')
  from jsonb_array_elements(l -> 'files') f;
$$;

-- ------------------------------------------------------------ privileges
select ok(
  not exists (
    select 1 from unnest(array['public', 'anon', 'authenticated', 'service_role']) as r(role)
    where has_table_privilege(r.role, 'private.file_retention', 'select,insert,update,delete')
       or has_table_privilege(r.role, 'private.unlimited_history', 'select,insert,update,delete')
  ),
  'nobody but the owner reads or writes the settings or the unlimited history mark'
);
select ok(
  not exists (
    select 1 from unnest(array['public', 'anon', 'authenticated', 'service_role']) as r(role)
    where has_function_privilege(r.role, 'private.file_retention_cutoff()', 'execute')
       or has_function_privilege(r.role, 'private.file_past_retention(date, timestamptz, date)', 'execute')
       or has_function_privilege(r.role, 'private.games_file_replaced()', 'execute')
       or has_function_privilege(r.role, 'private.sweep_retention_valid(jsonb)', 'execute')
  ),
  'nobody but the owner calls the private helpers'
);
select ok(
  not exists (
    select 1 from unnest(array['anon', 'authenticated']) as r(role)
    where has_function_privilege(r.role, 'public.file_retention_list(integer)', 'execute')
       or has_function_privilege(r.role, 'public.file_retention_mark(text[], text[])', 'execute')
       or has_function_privilege(r.role,
            'public.sweep_record_result(bigint, integer, jsonb, jsonb)', 'execute')
  ),
  'clients cannot list, mark or record'
);
select ok(
  has_function_privilege('service_role', 'public.file_retention_list(integer)', 'execute')
  and has_function_privilege('service_role', 'public.file_retention_mark(text[], text[])', 'execute')
  and has_function_privilege('service_role',
        'public.sweep_record_result(bigint, integer, jsonb, jsonb)', 'execute'),
  'the service role can (the sweep function does)'
);
select ok(
  not exists (
    select 1 from unnest(array['anon', 'authenticated']) as r(role)
    where has_column_privilege(r.role, 'public.games', 'file_expired_at', 'insert,update')
  )
  and has_column_privilege('authenticated', 'public.games', 'file_expired_at', 'select')
  and has_column_privilege('authenticated', 'public.game_summaries', 'file_expired_at', 'select'),
  'clients read file_expired_at (games and game_summaries) and cannot write it'
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
select throws_ok($$ select public.file_retention_list(10) $$, '42501', null,
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
  $$ values (28, false) $$,
  'ships with 28 days, report-only'
);
select throws_ok($$ insert into private.file_retention (days) values (90) $$, '23505', null,
  'one settings row only');
select is(private.file_retention_cutoff(), (select d - 29 from today),
  'cutoff: 28 days plus one for time zones');
select ok(
  private.file_past_retention((select d - 30 from today), now(), private.file_retention_cutoff())
  and not private.file_past_retention((select d - 29 from today), now(), private.file_retention_cutoff()),
  'boundary: 29 days ago is kept, 30 days ago is past'
);
select ok(
  private.file_past_retention((select d + 400 from today), now() - interval '40 days',
    private.file_retention_cutoff()),
  'a played_on in the future counts from the first upload'
);

-- ------------------------------------------------------------ report-only
create temporary table report as select public.file_retention_list(1000) as l;
select results_eq(
  $$ select (l ->> 'live')::boolean, (l ->> 'days')::int, (l ->> 'due')::int,
            (l #>> '{skipped,exempt}')::int, (l #>> '{skipped,no_summary}')::int,
            jsonb_array_length(l -> 'files')
     from report $$,
  $$ values (false, 28, 3, 1, 1, 0) $$,
  'report-only: counts what is due (A2, A5, B1) and what is kept, hands out no file'
);
select is(
  public.file_retention_mark(
    array[pg_temp.path('00000000-0000-4000-8000-0000000000a1', 2)], array[repeat('b', 64)]),
  0, 'report-only: marking does nothing'
);
select is((select count(*)::int from public.games where file_expired_at is not null), 1,
  'report-only: only the game expired before is marked');

-- ------------------------------------------------------------ live
update private.file_retention set live = true;
create temporary table live as select public.file_retention_list(1000) as l;
select is((select (l ->> 'live')::boolean from live), true, 'live: says so');
select is(
  (select pg_temp.paths(l) from live),
  array[
    pg_temp.path('00000000-0000-4000-8000-0000000000a1', 2),
    pg_temp.path('00000000-0000-4000-8000-0000000000a1', 5),
    pg_temp.path('00000000-0000-4000-8000-0000000000b1', 1)
  ],
  'live: the files past the retention of accounts without unlimited history, with a summary'
);
select is(
  (select l -> 'files' -> 0 ->> 'sha256' from live), repeat('a', 64),
  'live: oldest first, with the SHA-256 of the stored copy'
);
select is(jsonb_array_length(public.file_retention_list(1) -> 'files'), 1, 'live: bounded by the limit');
select is(jsonb_array_length(public.file_retention_list(0) -> 'files'), 0, 'live: limit 0 lists nothing');
select is(jsonb_array_length(public.file_retention_list(null) -> 'files'), 0, 'live: no limit lists nothing');
select is((public.file_retention_list(100000) ->> 'due')::int, 3, 'live: the count is not capped');
select is(
  (select bool_and(f ->> 'path' not like '00000000-0000-4000-8000-0000000000e1/%')
   from jsonb_array_elements(public.file_retention_list(1000) -> 'files') f),
  true, 'live: never a file of an account with unlimited history'
);

-- Marks: only the named copy, only past the retention, never an exempt account.
select is(
  public.file_retention_mark(
    array[
      pg_temp.path('00000000-0000-4000-8000-0000000000a1', 2),
      pg_temp.path('00000000-0000-4000-8000-0000000000a1', 1),
      pg_temp.path('00000000-0000-4000-8000-0000000000e1', 1),
      pg_temp.path('00000000-0000-4000-8000-0000000000b1', 1)
    ],
    array[repeat('b', 64), repeat('a', 64), repeat('a', 64), repeat('f', 64)]),
  1, 'mark: one game marked'
);
select results_eq(
  $$ select user_id::text, game_index from public.games
     where file_expired_at is not null order by user_id, game_index $$,
  $$ values ('00000000-0000-4000-8000-0000000000a1'::text, 2), ('00000000-0000-4000-8000-0000000000a1', 6) $$,
  'mark: A2 only (A1 is kept, E has unlimited history, B1 named another copy)'
);
select is((select count(*)::int from public.game_stats), 7, 'mark: every summary stays');
select is((public.file_retention_list(1000) ->> 'due')::int, 2, 'a marked game is no longer due');
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

-- A new copy of an expired game is a new file, kept until the next sweep.
update public.games set content_sha256 = repeat('9', 64)
where user_id = '00000000-0000-4000-8000-0000000000a1' and game_index = 2;
select is(
  (select file_expired_at from public.games
   where user_id = '00000000-0000-4000-8000-0000000000a1' and game_index = 2),
  null, 'a new copy clears the expiry'
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

-- ------------------------------------------------------------ run record
delete from private.sweep_runs;
create temporary table run as select (public.sweep_housekeeping() ->> 'run_id')::bigint as id;
create function pg_temp.stats() returns jsonb language sql immutable as $$
  select '{"listed": 0, "looked_at": 0, "filled": 0, "left": 0, "skipped": {"no_file": 0,
    "bad_file": 0, "file_mismatch": 0, "write_failed": 0, "missing": 0, "changed": 0,
    "newer": 0}}'::jsonb;
$$;
create function pg_temp.ret(mode text, due int, listed int, deleted int, failed int,
  unmarked int, lft int) returns jsonb language sql immutable as $$
  select jsonb_build_object('mode', mode, 'days', 28, 'due', due, 'listed', listed,
    'deleted', deleted, 'failed', failed, 'unmarked', unmarked, 'left', lft,
    'skipped', jsonb_build_object('exempt', 1, 'no_summary', 0));
$$;
select ok(private.sweep_retention_valid(pg_temp.ret('report', 3, 0, 0, 0, 0, 0)), 'record: report-only');
select ok(private.sweep_retention_valid(pg_temp.ret('live', 9, 6, 3, 1, 1, 2)), 'record: live');
select ok(private.sweep_retention_valid('{"error": "list"}'), 'record: a failed list');
select ok(
  not private.sweep_retention_valid(pg_temp.ret('report', 3, 1, 1, 0, 0, 0))
  and not private.sweep_retention_valid(pg_temp.ret('live', 9, 6, 3, 1, 1, 1))
  and not private.sweep_retention_valid(pg_temp.ret('live', 2, 6, 6, 0, 0, 0))
  and not private.sweep_retention_valid(pg_temp.ret('live', 9, 3, 3, 0, 4, 0))
  and not private.sweep_retention_valid(pg_temp.ret('live', 9, 3, 3, 0, 0, 0) || '{"ids": 1}')
  and not private.sweep_retention_valid(pg_temp.ret('live', 9, 3, 3, 0, 0, 0) || '{"due": -1}')
  and not private.sweep_retention_valid(pg_temp.ret('live', 9, 3, 3, 0, 0, 0) || '{"due": 1.5}')
  and not private.sweep_retention_valid(pg_temp.ret('other', 0, 0, 0, 0, 0, 0))
  and not private.sweep_retention_valid('{"error": "other"}')
  and not private.sweep_retention_valid('[]'),
  'record: counts that do not add up, extra keys, bad numbers or modes are refused'
);
select throws_ok(
  format($$ select public.sweep_record_result(%s, 0, %L, %L) $$,
    (select id from run), pg_temp.stats(), pg_temp.ret('report', 3, 1, 1, 0, 0, 0)),
  '22023', null, 'record: refuses retention counts out of shape'
);
select lives_ok(
  format($$ select public.sweep_record_result(%s, 0, %L, %L) $$,
    (select id from run), pg_temp.stats(), pg_temp.ret('report', 3, 0, 0, 0, 0, 0)),
  'record: stores retention counts'
);
select is(
  (select result -> 'retention' from private.sweep_runs where id = (select id from run)),
  pg_temp.ret('report', 3, 0, 0, 0, 0, 0), 'record: the retention is on the run''s row'
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

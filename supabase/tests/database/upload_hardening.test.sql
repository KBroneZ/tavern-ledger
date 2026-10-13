-- Session dates checked by upload_begin() and sweep runs that record every
-- failed step. Run: npx supabase test db
-- The functions' side is tested in supabase/functions/upload-game/ and
-- supabase/functions/sweep/.
begin;
create extension if not exists pgtap with schema extensions;
select plan(47);

delete from private.upload_events;
insert into auth.users (id, email) values
  ('00000000-0000-4000-8000-0000000000c1', 'c1@example.test');

create function pg_temp.begin_c(p_session text)
returns text language sql as $$
  select public.upload_begin('00000000-0000-4000-8000-0000000000c1', p_session, 1, 1,
    repeat('a', 64), 2000) ->> 'result'
$$;
create function pg_temp.session_of(p_year integer)
returns text language sql as $$
  select format('Hearthstone_%s_01_01_00_00_00', lpad(p_year::text, 4, '0'))
$$;
create temporary table next_year as
  select extract(year from now() at time zone 'UTC')::int + 1 as y;

-- ------------------------------------------------------------ session dates
select is(private.session_date('Hearthstone_2026_10_09_18_30_00'), '2026-10-09'::date,
  'a session gives its date');
select is(private.session_date('Hearthstone_2000_01_01_00_00_00'), '2000-01-01'::date,
  'the first day of 2000 is accepted');
select is(
  private.session_date(format('Hearthstone_%s_12_31_23_59_59', (select y from next_year))),
  make_date((select y from next_year), 12, 31),
  'the last day of next year is accepted'
);
select is(private.session_date(s), null, why)
from (values
  (pg_temp.session_of(0), 'year 0000 is refused'),
  (pg_temp.session_of(1), 'year 0001 is refused'),
  (pg_temp.session_of(1999), 'a year before 2000 is refused'),
  (pg_temp.session_of((select y + 1 from next_year)), 'a year after next year is refused'),
  ('Hearthstone_9999_01_01_00_00_00', 'year 9999 is refused'),
  ('Hearthstone_2026_02_31_00_00_00', 'an impossible day is refused'),
  ('Hearthstone_2026_13_01_00_00_00', 'month 13 is refused'),
  ('Hearthstone_2026_00_10_00_00_00', 'month 00 is refused'),
  ('Hearthstone_2026_10_09_18_30', 'a session that is not the client''s pattern is refused'),
  (null, 'no session is refused')
) as t(s, why);

-- ------------------------------------------------------------ upload_begin
select is(pg_temp.begin_c(pg_temp.session_of(0)), 'bad_session',
  'upload_begin refuses a session from year 0000 before any file is written');
select is(pg_temp.begin_c(pg_temp.session_of((select y + 1 from next_year))), 'bad_session',
  'and one from after next year');
select is(pg_temp.begin_c('Hearthstone_2026_02_31_00_00_00'), 'bad_session',
  'and an impossible day');
select is((select count(*)::int from private.upload_events), 0,
  'a refused session is not counted for the rate limits');
select is(pg_temp.begin_c('Hearthstone_2026_10_09_18_30_00'), 'create',
  'a session of this year is still accepted');
select is(pg_temp.begin_c(pg_temp.session_of((select y from next_year))), 'create',
  'and one of next year');

-- ------------------------------------------------------------ failed steps
create temporary table ok_stats as select jsonb_build_object(
  'listed', 1, 'looked_at', 1, 'filled', 1, 'left', 0,
  'skipped', jsonb_build_object(
    'no_file', 0, 'bad_file', 0, 'file_mismatch', 0, 'write_failed', 0,
    'missing', 0, 'changed', 0, 'newer', 0
  )
) as s;
grant select on ok_stats to service_role;

delete from private.sweep_runs;
create temporary table hk as select (public.sweep_housekeeping() ->> 'run_id')::bigint as id;
grant select on hk to service_role;

set local role service_role;
select lives_ok(
  $$ select public.sweep_record_result((select id from hk), 2, (select s from ok_stats),
       null, array['delete files']) $$,
  'the service role ends a run with the steps that failed'
);
reset role;
select results_eq(
  $$ select (result ->> 'files')::int, result -> 'stats', result -> 'failed_steps',
       result ? 'auth_events', result ? 'failed_step'
     from private.sweep_runs $$,
  $$ select 2, s, '["delete files"]'::jsonb, true, false from ok_stats $$,
  'the row keeps the counts of the steps that ran and names the one that failed'
);
select throws_ok(
  $$ select public.sweep_record_result((select id from hk), 0, (select s from ok_stats)) $$,
  '22023', null, 'a run with failed steps is ended once'
);

delete from private.sweep_runs;
create temporary table hk2 as select (public.sweep_housekeeping() ->> 'run_id')::bigint as id;
select lives_ok(
  $$ select public.sweep_record_result((select id from hk2), 0, null, null,
       array['list summaries']) $$,
  'a failed backfill list is recorded with no counts'
);
select results_eq(
  $$ select result ? 'stats', result -> 'stats', result -> 'failed_steps' from private.sweep_runs $$,
  $$ values (true, 'null'::jsonb, '["list summaries"]'::jsonb) $$,
  'its counts are empty, not zero, and the run is ended'
);

delete from private.sweep_runs;
select lives_ok(
  $$ select public.sweep_record_result(null, 4, (select s from ok_stats),
       null, array['housekeeping', 'list files']) $$,
  'a run whose housekeeping failed is still recorded'
);
select results_eq(
  $$ select count(*)::int, bool_and(result -> 'failed_steps' = '["housekeeping", "list files"]'),
       bool_and((result ->> 'files')::int = 4), bool_or(result ? 'auth_events')
     from private.sweep_runs $$,
  $$ values (1, true, true, false) $$,
  'on a row of its own, with the counts of the steps that ran'
);

create temporary table hk3 as select (public.sweep_housekeeping() ->> 'run_id')::bigint as id;
select throws_ok(
  format($$ select public.sweep_record_result(%L, 0, %L, null, %L) $$,
    run_id, (select s from ok_stats), failed),
  '22023', null, why
)
from (values
  (null::bigint, null::text[], 'no run id is refused unless housekeeping failed'),
  ((select id from hk3), array['housekeeping'], 'housekeeping cannot fail on a run it opened'),
  ((select id from hk3), array['list summaries'], 'a failed backfill list has no counts'),
  ((select id from hk3), array['record run'], 'a step that is not a run step is refused'),
  ((select id from hk3), array['delete files', 'delete files'], 'a step named twice is refused'),
  ((select id from hk3), array['delete files', null], 'a null step is refused')
) as t(run_id, failed, why);
select throws_ok(
  format($$ select public.sweep_record_result(%s, %s, %L, null, %L) $$,
    (select id from hk3), files, (select s from ok_stats), failed),
  '22023', 'sweep_record_result: counts out of shape', why
)
from (values
  (-1, array['delete files'], 'a negative file count is refused with a failed step'),
  (0, '{{"list files"},{"delete files"}}'::text[], 'a nested list of steps is refused')
) as t(files, failed, why);
select throws_ok(
  format($$ select public.sweep_record_result(%s, 0, null) $$, (select id from hk3)),
  '22023', 'sweep_record_result: counts out of shape',
  'no counts without a failed backfill list is refused'
);
select is(
  (select result ? 'stats' from private.sweep_runs where id = (select id from hk3)),
  false,
  'a refused call leaves the run open'
);
select lives_ok(
  format($$ select public.sweep_record_result(%s, 0, %L, null, '{}') $$,
    (select id from hk3), (select s from ok_stats)),
  'no failed step is a finished run'
);
select is(
  (select result ? 'failed_steps' from private.sweep_runs where id = (select id from hk3)),
  false,
  'and names none'
);

-- A sweep function deployed before this migration still records its runs.
create temporary table hk4 as select (public.sweep_housekeeping() ->> 'run_id')::bigint as id;
create temporary table hk5 as select (public.sweep_housekeeping() ->> 'run_id')::bigint as id;
grant select on hk4, hk5 to service_role;
set local role service_role;
select lives_ok(
  $$ select public.sweep_record_result(p_run_id => (select id from hk4), p_files => 0,
       p_stats => (select s from ok_stats), p_retention => null) $$,
  'four named arguments, as before, still end a run'
);
select lives_ok(
  $$ select public.sweep_record_result(p_run_id => (select id from hk5), p_files => 0,
       p_stats => (select s from ok_stats)) $$,
  'and three'
);
reset role;
select is(
  (select count(*)::int from private.sweep_runs
   where id in ((select id from hk4), (select id from hk5))
     and result ? 'stats' and not result ? 'failed_steps'),
  2, 'both runs are ended with no failed step'
);

-- A run that stopped (sweep_record_failure) stays as it is.
create temporary table hk6 as select (public.sweep_housekeeping() ->> 'run_id')::bigint as id;
select public.sweep_record_failure((select id from hk6), 'unexpected');
select throws_ok(
  format($$ select public.sweep_record_result(%s, 0, %L, null, '{delete files}') $$,
    (select id from hk6), (select s from ok_stats)),
  '22023', null, 'a run already marked as stopped cannot be ended with failed steps'
);

-- Housekeeping and the backfill list both failed: a row of its own, no counts.
delete from private.sweep_runs;
select lives_ok(
  $$ select public.sweep_record_result(null, 0, null, null,
       array['housekeeping', 'list summaries']) $$,
  'housekeeping and the backfill list can fail in the same run'
);
select results_eq(
  $$ select result -> 'stats', result -> 'failed_steps' from private.sweep_runs $$,
  $$ values ('null'::jsonb, '["housekeeping", "list summaries"]'::jsonb) $$,
  'the row says so, with no counts'
);

-- ------------------------------------------------------------ privileges
select ok(
  not exists (
    select 1 from unnest(array['anon', 'authenticated']) as r(role)
    where has_function_privilege(r.role,
      'public.sweep_record_result(bigint, integer, jsonb, jsonb, text[])', 'execute')
  ),
  'clients cannot write run records'
);
select ok(
  has_function_privilege('service_role',
    'public.sweep_record_result(bigint, integer, jsonb, jsonb, text[])', 'execute'),
  'the service role can (the sweep function does)'
);
select ok(
  not exists (
    select 1 from unnest(array['anon', 'authenticated', 'service_role']) as r(role)
    where has_function_privilege(r.role, 'private.session_date(text)', 'execute')
  ),
  'the session check has no grants'
);

select * from finish();
rollback;

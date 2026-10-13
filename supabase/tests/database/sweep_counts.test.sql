-- Sweep run record with the backfill's counts and failed steps (T-524).
-- Run: npx supabase test db
-- The sweep function's side is tested in supabase/functions/sweep/.
begin;
create extension if not exists pgtap with schema extensions;
select plan(32);

create temporary table ok_stats as select jsonb_build_object(
  'listed', 5, 'looked_at', 4, 'filled', 2, 'left', 1,
  'skipped', jsonb_build_object(
    'no_file', 1, 'bad_file', 0, 'file_mismatch', 0, 'write_failed', 0,
    'missing', 0, 'changed', 1, 'newer', 0
  )
) as s;
grant select on ok_stats to service_role;

-- ------------------------------------------------------------ privileges
select ok(
  not exists (
    select 1 from unnest(array['anon', 'authenticated']) as r(role)
    where has_function_privilege(r.role, 'public.sweep_record_result(bigint, integer, jsonb, jsonb, text[])', 'execute')
       or has_function_privilege(r.role, 'public.sweep_record_failure(bigint, text)', 'execute')
  ),
  'clients cannot write run records'
);
select ok(
  has_function_privilege('service_role', 'public.sweep_record_result(bigint, integer, jsonb, jsonb, text[])', 'execute')
  and has_function_privilege('service_role', 'public.sweep_record_failure(bigint, text)', 'execute'),
  'the service role can (the sweep function does)'
);
select ok(
  not exists (
    select 1 from unnest(array['anon', 'authenticated', 'service_role']) as r(role)
    where has_function_privilege(r.role, 'private.sweep_stats_valid(jsonb)', 'execute')
       or has_table_privilege(r.role, 'private.sweep_runs', 'select,insert,update,delete')
  ),
  'the run record stays private: no grants on the table or its check'
);
select is(
  (select relrowsecurity from pg_class where oid = 'private.sweep_runs'::regclass),
  false,
  'row-level security on the run record is unchanged (off; no grants)'
);

-- ------------------------------------------------------------ finished run
delete from private.sweep_runs;
create temporary table hk as select public.sweep_housekeeping() as r;
grant select on hk to service_role;
select ok((select (r ->> 'run_id')::bigint from hk) is not null, 'housekeeping returns its run id');
select is(
  (select result ? 'run_id' from private.sweep_runs),
  false,
  'the run id is not stored inside the row'
);

set local role service_role;
select lives_ok(
  $$ select public.sweep_record_result((select (r ->> 'run_id')::bigint from hk), 3, (select s from ok_stats)) $$,
  'the service role ends a run with its counts'
);
reset role;
select results_eq(
  $$ select (result ->> 'files')::int, result -> 'stats', result ? 'auth_events', result ? 'failed_step'
     from private.sweep_runs $$,
  $$ select 3, s, true, false from ok_stats $$,
  'the row holds the files, the backfill counts and the housekeeping counts'
);
select is(
  (select result #>> '{stats,skipped,changed}' from private.sweep_runs),
  '1',
  'skips are kept by reason'
);
select throws_ok(
  $$ select public.sweep_record_result((select (r ->> 'run_id')::bigint from hk), 3, (select s from ok_stats)) $$,
  '22023', null,
  'a run is ended once'
);
select is(
  public.sweep_record_failure((select (r ->> 'run_id')::bigint from hk), 'record run'),
  (select (r ->> 'run_id')::bigint from hk),
  'a failure reported after the run ended (its answer was lost) points at that run'
);
select results_eq(
  $$ select count(*)::int, bool_or(result ? 'failed_step') from private.sweep_runs $$,
  $$ values (1, false) $$,
  'and adds no row and changes nothing: the run did finish'
);
select throws_ok(
  $$ select public.sweep_record_result(-1, 0, (select s from ok_stats)) $$,
  '22023', null,
  'a run that does not exist cannot be ended'
);

-- ------------------------------------------------------------ counts checked
delete from private.sweep_runs;
create temporary table hk2 as select (public.sweep_housekeeping() ->> 'run_id')::bigint as id;
select throws_ok(
  format($$ select public.sweep_record_result(%s, 0, %L) $$, (select id from hk2), v),
  '22023', null, why
)
from (values
  ((select s - 'left' from ok_stats), 'a missing count is refused'),
  ((select s || '{"extra": 0}' from ok_stats), 'an unknown count is refused'),
  ((select jsonb_set(s, '{skipped,other}', '0') from ok_stats), 'an unknown skip reason is refused'),
  ((select s #- '{skipped,newer}' from ok_stats), 'a missing skip reason is refused'),
  ((select jsonb_set(s, '{filled}', '-1') from ok_stats), 'a negative count is refused'),
  ((select jsonb_set(s, '{filled}', '"2"') from ok_stats), 'a count that is not a number is refused'),
  ((select jsonb_set(s, '{filled}', '1.5') from ok_stats), 'a fraction is refused'),
  ((select jsonb_set(s, '{left}', '0') from ok_stats), 'looked at + left must be listed'),
  ((select jsonb_set(s, '{filled}', '3') from ok_stats), 'filled + skipped must be looked at'),
  ('null'::jsonb, 'no counts is not zero counts')
) as t(v, why);
select throws_ok(
  format($$ select public.sweep_record_result(%s, -1, %L) $$, (select id from hk2), (select s from ok_stats)),
  '22023', null, 'a negative file count is refused'
);
select is(
  (select result ? 'stats' from private.sweep_runs),
  false,
  'a refused call leaves the run open, not zero'
);

-- ------------------------------------------------------------ failed runs
select is(
  public.sweep_record_failure((select id from hk2), 'list summaries'),
  (select id from hk2),
  'a step that fails after housekeeping marks the run''s own row'
);
select results_eq(
  $$ select result ->> 'failed_step', result ? 'auth_events', result ? 'stats' from private.sweep_runs $$,
  $$ values ('list summaries'::text, true, false) $$,
  'the row says which step failed, next to the housekeeping counts'
);

select lives_ok(
  $$ select public.sweep_record_failure(null, 'delete files') $$,
  'a step that fails before housekeeping still leaves a row'
);
select results_eq(
  $$ select result from private.sweep_runs where not (result ? 'auth_events') $$,
  $$ values ('{"failed_step": "delete files"}'::jsonb) $$,
  'that row holds only the failed step'
);
select throws_ok(
  $$ select public.sweep_record_failure(null, 'anything else') $$,
  '22023', null,
  'only known step names are recorded'
);

-- Old rows still go after 90 days, also when a run fails before housekeeping.
delete from private.sweep_runs;
insert into private.sweep_runs (ran_at, result) values (now() - interval '91 days', '{}');
select lives_ok($$ select public.sweep_record_failure(null, 'list files') $$, 'a run can fail before housekeeping');
select is((select count(*)::int from private.sweep_runs), 1, 'old rows go on a failure row too');

select * from finish();
rollback;

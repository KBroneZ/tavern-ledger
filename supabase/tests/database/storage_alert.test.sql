-- Storage alert in the sweep run record (T-543, D-109).
-- Run: npx supabase test db
-- The sweep function's side is tested in supabase/functions/sweep/.
begin;
create extension if not exists pgtap with schema extensions;
select plan(26);

-- ------------------------------------------------------------ privileges
select ok(
  not exists (
    select 1 from unnest(array['public', 'anon', 'authenticated', 'service_role']) as r(role)
    where has_function_privilege(r.role, 'private.capacity_limits()', 'execute')
       or has_function_privilege(r.role, 'private.capacity_level(bigint, bigint)', 'execute')
       or has_function_privilege(r.role, 'private.sweep_capacity()', 'execute')
  ),
  'nobody but the owner calls the limits, the level or the measurement'
);
select ok(
  not exists (
    select 1 from unnest(array['anon', 'authenticated']) as r(role)
    where has_function_privilege(r.role, 'public.sweep_housekeeping()', 'execute')
  ),
  'clients still cannot run housekeeping'
);
select ok(
  has_function_privilege('service_role', 'public.sweep_housekeeping()', 'execute'),
  'the service role still can (the sweep function does)'
);
select ok(
  not exists (
    select 1 from unnest(array['anon', 'authenticated', 'service_role']) as r(role)
    where has_table_privilege(r.role, 'private.sweep_runs', 'select,insert,update,delete')
  ),
  'the run record stays private'
);

-- ------------------------------------------------------------ limits
select results_eq(
  $$ select (l ->> 'database_bytes')::bigint, (l ->> 'files_bytes')::bigint, l ->> 'plan',
            l ->> 'read_on', l ? 'source'
     from private.capacity_limits() l $$,
  $$ values (500000000::bigint, 1000000000::bigint, 'free'::text, '2026-10-12'::text, true) $$,
  'the Free plan limits, with the date and source they were read from'
);

-- ------------------------------------------------------------ levels
select results_eq(
  $$ select private.capacity_level(b, 1000)
     from unnest(array[0, 499, 500, 799, 800, 1000, 2000]::bigint[]) with ordinality t(b, i)
     order by i $$,
  $$ values ('none'::text), ('none'), ('50%'), ('50%'), ('80%'), ('80%'), ('80%') $$,
  'levels: none under 50 %, 50% from half, 80% from 80 % (and above the limit)'
);
select is(private.capacity_level(null, 1000), null, 'no measure is no level, never none');
select is(private.capacity_level(10, null), null, 'no limit is no level');
select is(private.capacity_level(10, 0), null, 'a limit of 0 is no level');

-- ------------------------------------------------------------ measured
-- Storage refuses deletes in SQL, so counts are taken on top of what is there.
create temporary table before as
  select coalesce(sum((metadata ->> 'size')::bigint), 0) as bytes, count(*) as n
  from storage.objects where bucket_id = 'games';
insert into storage.objects (bucket_id, name, metadata) values
  ('games', '00000000-0000-4000-8000-0000000000c1/Hearthstone_2026_10_12_10_00_00-1.json.gz',
   '{"size": 7000}'),
  ('games', '00000000-0000-4000-8000-0000000000c1/Hearthstone_2026_10_12_10_00_00-2.json.gz',
   '{"size": 3000}'),
  ('games', '00000000-0000-4000-8000-0000000000c1/Hearthstone_2026_10_12_10_00_00-3.json.gz',
   null);
delete from private.sweep_runs;

create temporary table hk as select public.sweep_housekeeping() as r;
create temporary table cap as select result -> 'capacity' as c from private.sweep_runs;

select is((select count(*)::int from private.sweep_runs), 1, 'housekeeping leaves one row');
select results_eq(
  $$ select (c #>> '{files,bytes}')::bigint, (c #>> '{files,count}')::int,
            (c #>> '{files,limit}')::bigint, c #>> '{files,level}' from cap $$,
  $$ select (bytes + 10000)::bigint, n::int + 3, 1000000000::bigint, 'none'::text from before $$,
  'the games bucket''s file bytes (from metadata), file count, limit and level'
);
-- The size moves a little as this test makes temporary tables.
select results_eq(
  $$ select abs((c #>> '{database,bytes}')::bigint - pg_database_size(current_database())) < 1048576,
            (c #>> '{database,bytes}')::bigint > 0, (c #>> '{database,limit}')::bigint from cap $$,
  $$ values (true, true, 500000000::bigint) $$,
  'the database size (pg_database_size) and its limit'
);
select is(
  (select c #>> '{database,level}' from cap),
  (select private.capacity_level((c #>> '{database,bytes}')::bigint, 500000000) from cap),
  'the database level follows its size'
);
select is((select c ->> 'limits_read_on' from cap), '2026-10-12', 'the row says which limits it used');
select is(
  (select r -> 'capacity' from hk),
  (select c from cap),
  'housekeeping''s answer carries the same capacity as the row'
);
select results_eq(
  $$ select r ? 'auth_events', r ? 'upload_events', r ? 'ip_failures', r ? 'run_id' from hk $$,
  $$ values (true, true, true, true) $$,
  'housekeeping''s own counts and run id are unchanged'
);

-- Tiny limits for this transaction: both measures pass 80 %.
create or replace function private.capacity_limits()
returns jsonb language sql stable set search_path = '' as $$
  select jsonb_build_object('plan', 'test', 'database_bytes', 1000, 'files_bytes', 18000,
                            'read_on', '2000-01-01', 'source', 'test')
$$;
delete from private.sweep_runs;
do $$ begin perform public.sweep_housekeeping(); end $$;
select results_eq(
  $$ select result #>> '{capacity,database,level}', result #>> '{capacity,files,level}',
            result #>> '{capacity,level}' from private.sweep_runs $$,
  $$ values ('80%'::text, '50%'::text, '80%'::text) $$,
  'each limit has its level and the overall level is the highest'
);

create or replace function private.capacity_limits()
returns jsonb language sql stable set search_path = '' as $$
  select jsonb_build_object('plan', 'test', 'database_bytes', 1000000000000, 'files_bytes', 20000,
                            'read_on', '2000-01-01', 'source', 'test')
$$;
delete from private.sweep_runs;
do $$ begin perform public.sweep_housekeeping(); end $$;
select results_eq(
  $$ select result #>> '{capacity,database,level}', result #>> '{capacity,files,level}',
            result #>> '{capacity,level}' from private.sweep_runs $$,
  $$ values ('none'::text, '50%'::text, '50%'::text) $$,
  'files at 50 % flag the run at 50% while the database is low'
);

-- ------------------------------------------------------------ failed measure
create or replace function private.capacity_limits()
returns jsonb language plpgsql stable set search_path = '' as $$
begin
  raise exception 'limits unreadable' using errcode = 'XX000';
end;
$$;
delete from private.sweep_runs;
select lives_ok(
  $$ select public.sweep_housekeeping() $$,
  'a measurement that fails never fails housekeeping'
);
select results_eq(
  $$ select result -> 'capacity', result ? 'auth_events' from private.sweep_runs $$,
  $$ values ('{"failed": true, "sqlstate": "XX000"}'::jsonb, true) $$,
  'the row records the failure (its SQLSTATE only) next to the housekeeping counts'
);
select is(
  (select result #>> '{capacity,level}' from private.sweep_runs),
  null,
  'a failed measurement has no level, never none'
);

-- A statement timeout in the measurement is caught too, by name.
create or replace function private.capacity_limits()
returns jsonb language plpgsql stable set search_path = '' as $$
begin
  raise exception 'canceling statement due to statement timeout' using errcode = 'query_canceled';
end;
$$;
delete from private.sweep_runs;
select lives_ok(
  $$ select public.sweep_housekeeping() $$,
  'a measurement cut off by a statement timeout never fails housekeeping'
);
select is(
  (select result -> 'capacity' from private.sweep_runs),
  '{"failed": true, "sqlstate": "57014"}'::jsonb,
  'and is recorded as failed with its SQLSTATE'
);

-- Housekeeping's deletes still happen when the measurement fails.
insert into private.upload_ip_failures (ip_key, window_start) values
  (repeat('ab', 32), now() - interval '2 hours');
delete from private.sweep_runs;
select is(
  (select (public.sweep_housekeeping() ->> 'ip_failures')::int >= 1),
  true,
  'and its clean-up still runs'
);
select is(
  (select count(*)::int from private.upload_ip_failures where window_start < now() - interval '1 hour'),
  0,
  'old IP counters are gone'
);

-- A run ended by the sweep keeps its capacity next to the backfill's counts.
create or replace function private.capacity_limits()
returns jsonb language sql stable set search_path = '' as $$
  select jsonb_build_object('plan', 'test', 'database_bytes', 1000000000000,
                            'files_bytes', 1000000000000, 'read_on', '2000-01-01', 'source', 'test')
$$;
delete from private.sweep_runs;
create temporary table hk3 as select (public.sweep_housekeeping() ->> 'run_id')::bigint as id;
do $$
begin
  perform public.sweep_record_result((select id from hk3), 0, jsonb_build_object(
    'listed', 0, 'looked_at', 0, 'filled', 0, 'left', 0,
    'skipped', jsonb_build_object(
      'no_file', 0, 'bad_file', 0, 'file_mismatch', 0, 'write_failed', 0,
      'missing', 0, 'changed', 0, 'newer', 0
    )
  ));
end;
$$;
select is(
  (select (result ? 'stats') and result #>> '{capacity,level}' = 'none' from private.sweep_runs),
  true,
  'an ended run holds both the backfill counts and the capacity'
);

select * from finish();
rollback;

-- Sweep scaling (DB-4): an index on games(storage_path), a statement_timeout on
-- each heavy function, pg_cron's run log purged. Run: npx supabase test db
begin;
create extension if not exists pgtap with schema extensions;
select plan(8);

select ok(
  exists (
    select 1 from pg_index i
    where i.indrelid = 'public.games'::regclass and i.indisvalid
      and (select a.attname from pg_attribute a where a.attrelid = i.indrelid and a.attnum = i.indkey[0]) = 'storage_path'
  ),
  'games has an index that starts with storage_path');

select is_empty(
  $$ select f.sig
     from (values
       ('public.sweep_housekeeping()'),
       ('public.sweep_orphan_files(interval)'),
       ('public.file_retention_list(integer, integer)'),
       ('public.file_retention_check(text[], text[], integer)'),
       ('public.file_retention_mark(text[], text[])'),
       ('public.game_stats_backfill_list(integer, integer)'),
       ('private.refresh_community_heroes()'),
       ('private.refresh_leaderboards()')
     ) as f(sig)
     where not exists (
       select 1 from pg_proc p, unnest(coalesce(p.proconfig, '{}')) as c
       where p.oid = f.sig::regprocedure and c like 'statement_timeout=%'
     ) $$,
  'every heavy sweep, retention and rebuild function has its own statement_timeout');

select ok(
  (select bool_and(proconfig @> array['search_path=""']) from pg_proc
   where oid in ('public.sweep_housekeeping()'::regprocedure, 'private.refresh_community_heroes()'::regprocedure)),
  'setting the timeout kept the empty search path');

select ok(
  (select bool_and(substring(c from 'statement_timeout=(.*)') ~ '^[0-9]+s$')
   from pg_proc p, unnest(p.proconfig) as c
   where p.proname in ('sweep_housekeeping', 'refresh_community_heroes') and c like 'statement_timeout=%'),
  'the limits are in seconds');

-- ----------------------------------------------------------- cron log purge
delete from cron.job_run_details;
insert into cron.job_run_details (jobid, runid, job_pid, database, username, command, status, return_message, start_time, end_time)
values
  (1, 9001, 1, 'postgres', 'postgres', 'select 1', 'succeeded', '', now() - interval '20 days', now() - interval '20 days'),
  (1, 9002, 1, 'postgres', 'postgres', 'select 1', 'succeeded', '', now() - interval '15 days', now() - interval '15 days'),
  (1, 9003, 1, 'postgres', 'postgres', 'select 1', 'succeeded', '', now() - interval '13 days', now() - interval '13 days'),
  (1, 9004, 1, 'postgres', 'postgres', 'select 1', 'succeeded', '', now() - interval '1 day', now() - interval '1 day');
select is(private.purge_cron_log(), 2::bigint, 'the purge removes the runs older than 14 days');
select is((select array_agg(runid order by runid) from cron.job_run_details), array[9003::bigint, 9004],
  'and keeps the newer ones');
select results_eq(
  $$ select schedule, command, active from cron.job where jobname = 'cron-log-purge' $$,
  $$ values ('25 4 * * *'::text, 'select private.purge_cron_log()'::text, true) $$,
  'it runs once a day');
select ok(
  not has_function_privilege('anon', 'private.purge_cron_log()', 'execute')
  and not has_function_privilege('authenticated', 'private.purge_cron_log()', 'execute')
  and not has_function_privilege('service_role', 'private.purge_cron_log()', 'execute'),
  'only the database itself runs the purge');

select * from finish();
rollback;

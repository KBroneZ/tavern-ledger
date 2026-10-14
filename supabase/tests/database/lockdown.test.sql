-- Guard for everything a browser can reach. Run: npx supabase test db
-- Fails when a new table, view or function is open to `anon` / `authenticated`
-- without being listed here. Adding a row to a list below is the review step:
-- say why in the PR.
begin;
create extension if not exists pgtap with schema extensions;
select plan(15);

-- ------------------------------------------------------------ tables
-- Every table in `public` has row-level security. `private` tables are not
-- reachable by clients (no schema usage, no grants), so they may go without.
select is_empty(
  $$ select n.nspname || '.' || c.relname
     from pg_class c join pg_namespace n on n.oid = c.relnamespace
     where n.nspname = 'public' and c.relkind in ('r', 'p') and not c.relrowsecurity $$,
  'every table in public has row-level security'
);
select is_empty(
  $$ select n.nspname || '.' || c.relname
     from pg_class c join pg_namespace n on n.oid = c.relnamespace
     where n.nspname = 'private' and c.relkind in ('r', 'p') and not c.relrowsecurity
       and (
         has_schema_privilege('anon', n.oid, 'usage')
         or has_schema_privilege('authenticated', n.oid, 'usage')
         or exists (
           select 1 from unnest(array['anon', 'authenticated']) as r(role)
           where has_table_privilege(r.role, c.oid, 'select,insert,update,delete,truncate,references,trigger')
              or has_any_column_privilege(r.role, c.oid, 'select,insert,update,references')
         )
       ) $$,
  'a private table without row-level security is closed to clients'
);
select is_empty(
  $$ select n.nspname || '.' || c.relname
     from pg_class c join pg_namespace n on n.oid = c.relnamespace
     where n.nspname in ('public', 'private') and c.relkind in ('m', 'f')
       and exists (
         select 1 from unnest(array['anon', 'authenticated']) as r(role)
         where has_any_column_privilege(r.role, c.oid, 'select')
       ) $$,
  'no materialized view or foreign table (no row-level security possible) is readable by clients'
);

-- ------------------------------------------------------------ views
-- A view runs as its owner unless it is `security_invoker`, which would skip
-- row-level security. Reviewed exceptions (security_barrier views that filter
-- by auth.uid() themselves):
--   game_summaries  public rows only by design; 20261011150000_leaderboards.sql
select is_empty(
  $$ select n.nspname || '.' || c.relname
     from pg_class c join pg_namespace n on n.oid = c.relnamespace
     where n.nspname in ('public', 'private') and c.relkind = 'v'
       and coalesce('security_invoker=true' = any (c.reloptions), false) is false
       and not (
         'security_barrier=true' = any (coalesce(c.reloptions, '{}'))
         and n.nspname || '.' || c.relname in ('public.game_summaries')
       ) $$,
  'every view is security_invoker, except the reviewed security_barrier list'
);

-- ------------------------------------------------------------ reads
-- The tables and views a client can read, and who. Row-level security (or the
-- view's own filter) decides which rows; this list decides which relations.
create temporary table readable_allowed (relation text, role text);
insert into readable_allowed values
  ('public.community_consent', 'authenticated'),
  ('public.simulator_consent', 'authenticated'),
  ('public.game_summaries', 'anon'),
  ('public.game_summaries', 'authenticated'),
  ('public.user_heroes', 'anon'),
  ('public.user_heroes', 'authenticated'),
  ('public.user_overview', 'anon'),
  ('public.user_overview', 'authenticated'),
  ('public.user_records', 'anon'),
  ('public.user_records', 'authenticated'),
  -- Column-level SELECT only (the public fields; public_fields.test.sql pins the columns).
  ('public.profiles', 'anon'),
  ('public.profiles', 'authenticated'),
  ('public.games', 'anon'),
  ('public.games', 'authenticated'),
  ('public.game_stats', 'anon'),
  ('public.game_stats', 'authenticated');
select is_empty(
  $$ select n.nspname || '.' || c.relname || ' ' || r.role
     from pg_class c join pg_namespace n on n.oid = c.relnamespace
     cross join unnest(array['anon', 'authenticated']) as r(role)
     where n.nspname in ('public', 'private') and c.relkind in ('r', 'p', 'v', 'm', 'f')
       and has_any_column_privilege(r.role, c.oid, 'select')
       and not exists (
         select 1 from readable_allowed a
         where a.relation = n.nspname || '.' || c.relname and a.role = r.role
       ) $$,
  'clients can read only the tables and views on the allow-list'
);

-- ------------------------------------------------------------ writes
-- Clients write only through functions: no table-level write privilege, and
-- one reviewed column-level exception (a user edits their own profile; row-level
-- security limits it to their row and check constraints to valid names):
--   public.profiles (display_name, is_public) for authenticated
select is_empty(
  $$ select n.nspname || '.' || c.relname || ' ' || r.role
     from pg_class c join pg_namespace n on n.oid = c.relnamespace
     cross join unnest(array['anon', 'authenticated']) as r(role)
     where n.nspname in ('public', 'private') and c.relkind in ('r', 'p', 'v', 'm', 'f')
       and has_table_privilege(r.role, c.oid, 'insert,update,delete,truncate,references,trigger') $$,
  'anon and authenticated hold no table-level write privilege'
);
select is_empty(
  $$ select n.nspname || '.' || c.relname || '.' || a.attname || ' ' || r.role
     from pg_class c join pg_namespace n on n.oid = c.relnamespace
     join pg_attribute a on a.attrelid = c.oid and a.attnum > 0 and not a.attisdropped
     cross join unnest(array['anon', 'authenticated']) as r(role)
     where n.nspname in ('public', 'private') and c.relkind in ('r', 'p', 'v', 'm', 'f')
       and has_column_privilege(r.role, c.oid, a.attnum, 'insert,update,references')
       and not (
         n.nspname || '.' || c.relname = 'public.profiles'
         and a.attname in ('display_name', 'is_public')
         and r.role = 'authenticated'
         and not has_column_privilege(r.role, c.oid, a.attnum, 'insert,references')
       ) $$,
  'column-level writes are only the reviewed profile columns'
);
select is_empty(
  $$ select n.nspname || '.' || c.relname || ' ' || r.role
     from pg_class c join pg_namespace n on n.oid = c.relnamespace
     cross join unnest(array['anon', 'authenticated']) as r(role)
     where n.nspname in ('public', 'private') and c.relkind = 'S'
       and has_sequence_privilege(r.role, c.oid, 'usage,select,update') $$,
  'anon and authenticated hold nothing on any sequence'
);

-- ------------------------------------------------------------ functions
-- The contract: the functions a client may call, and who may call them.
-- Checked where the role can reach the schema, so helpers in `private` that
-- only carry the default EXECUTE for PUBLIC do not count.
create temporary table callable_allowed (signature text, role text);
insert into callable_allowed values
  ('public.community_heroes()', 'anon'),
  ('public.community_heroes()', 'authenticated'),
  ('public.community_heroes_state()', 'anon'),
  ('public.community_heroes_state()', 'authenticated'),
  ('public.export_my_data()', 'authenticated'),
  ('public.leaderboard(text, text)', 'anon'),
  ('public.leaderboard(text, text)', 'authenticated'),
  ('public.leaderboard_refreshed_at()', 'anon'),
  ('public.leaderboard_refreshed_at()', 'authenticated'),
  ('public.set_community_consent(boolean)', 'authenticated'),
  ('public.set_simulator_consent(boolean)', 'authenticated');

select is_empty(
  $$ select n.nspname || '.' || p.proname || '(' || oidvectortypes(p.proargtypes) || ') ' || r.role
     from pg_proc p join pg_namespace n on n.oid = p.pronamespace
     cross join unnest(array['anon', 'authenticated']) as r(role)
     where has_schema_privilege(r.role, n.oid, 'usage')
       and n.nspname not in ('pg_catalog', 'information_schema', 'extensions', 'graphql',
                             'graphql_public', 'net', 'vault', 'pgsodium', 'cron',
                             'realtime', 'storage', 'auth', 'supabase_functions')
       and has_function_privilege(r.role, p.oid, 'execute')
       and not exists (
         select 1 from callable_allowed a
         where a.signature = n.nspname || '.' || p.proname || '(' || oidvectortypes(p.proargtypes) || ')' and a.role = r.role
       ) $$,
  'clients can call only the functions on the allow-list'
);
select is_empty(
  $$ select a.signature || ' ' || a.role
     from callable_allowed a
     where not exists (
       select 1 from pg_proc p join pg_namespace n on n.oid = p.pronamespace
       where n.nspname || '.' || p.proname || '(' || oidvectortypes(p.proargtypes) || ')' = a.signature
         and has_function_privilege(a.role, p.oid, 'execute')
     ) $$,
  'every allow-listed function exists and is callable by its role'
);

-- ------------------------------------------------------------ secure by default
select is_empty(
  $$ select d.defaclobjtype::text || ' ' || coalesce(acl.grantee::regrole::text, 'PUBLIC')
     from pg_default_acl d
     join pg_roles o on o.oid = d.defaclrole and o.rolname = 'postgres'
     join pg_namespace n on n.oid = d.defaclnamespace and n.nspname = 'public'
     cross join lateral aclexplode(d.defaclacl) acl
     where acl.grantee = 0
        or acl.grantee in (select oid from pg_roles where rolname in ('anon', 'authenticated')) $$,
  'the default privileges of postgres in public grant nothing to clients'
);

create table public.guard_probe_table (id int);
create sequence public.guard_probe_sequence;
create function public.guard_probe_function() returns int language sql as 'select 1';
create function private.guard_probe_function() returns int language sql as 'select 1';

select ok(
  not exists (
    select 1 from unnest(array['anon', 'authenticated']) as r(role)
    where has_any_column_privilege(r.role, 'public.guard_probe_table', 'select,insert,update,references')
       or has_table_privilege(r.role, 'public.guard_probe_table', 'delete,truncate,trigger')
  ),
  'a new table grants nothing to clients'
);
select ok(
  not exists (
    select 1 from unnest(array['anon', 'authenticated']) as r(role)
    where has_sequence_privilege(r.role, 'public.guard_probe_sequence', 'usage,select,update')
  ),
  'a new sequence grants nothing to clients'
);
select ok(
  not exists (
    select 1 from unnest(array['anon', 'authenticated']) as r(role)
    where has_function_privilege(r.role, 'public.guard_probe_function()', 'execute')
       or has_function_privilege(r.role, 'private.guard_probe_function()', 'execute')
  ),
  'a new function is not callable by clients (PUBLIC included)'
);
select ok(
  has_function_privilege('postgres', 'public.guard_probe_function()', 'execute'),
  'the owner can still call its own function'
);

select * from finish();
rollback;

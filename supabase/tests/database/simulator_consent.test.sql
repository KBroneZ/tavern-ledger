-- Consent to use uploaded games to improve the simulator (T-547, D-119).
-- Run: npx supabase test db. Users and games are made up; everything rolls
-- back at the end.
begin;
create extension if not exists pgtap with schema extensions;
select plan(37);

-- Players P1..P5 (ids from pg_temp.pid). P1 opts in; P2 opts in and then
-- withdraws; P3 never uses the switch; P4 loses its profile (a broken
-- account); P5 opts in and has unlimited history.
create function pg_temp.pid(i int) returns uuid language sql immutable as $$
  select ('00000000-0000-4000-8000-' || lpad(to_hex(12288 + i), 12, '0'))::uuid
$$;

create function pg_temp.as_user(i int) returns text language sql as $$
  select set_config('request.jwt.claims', format('{"sub": "%s", "role": "authenticated"}', pg_temp.pid(i)), true);
$$;

-- One game row and, unless the file has expired, its file in the bucket.
create function pg_temp.game(i int, idx int, expired boolean) returns text language plpgsql as $$
declare
  path text;
begin
  insert into public.games (user_id, session, game_index, game_type, status, hero_card_id,
    final_place, build, played_on, saved_at, content_sha256, size_bytes, file_expired_at)
  values (pg_temp.pid(i), 'Hearthstone_2026_10_09_18_30_00', idx, 'GT_BATTLEGROUNDS_DUO', 'ok',
    'BG20_HERO_202', 2, 253216, current_date - idx, 1791000000, repeat('d', 64), 3000,
    case when expired then now() end)
  returning storage_path into path;
  if not expired then
    insert into storage.objects (bucket_id, name) values ('games', path);
  end if;
  return path;
end;
$$;

insert into auth.users (id, email)
select pg_temp.pid(i), 'sc' || i || '@example.test' from generate_series(1, 5) as i;

select pg_temp.game(1, 1, false), pg_temp.game(1, 2, false), pg_temp.game(1, 3, true),
       pg_temp.game(2, 1, false), pg_temp.game(3, 1, false), pg_temp.game(5, 1, false);
-- P1's second game has a row but its file is gone without the mark (a
-- failed or manual delete): it is not listed either.
set local storage.allow_delete_query = 'true';
delete from storage.objects
where bucket_id = 'games' and name = pg_temp.pid(1)::text || '/Hearthstone_2026_10_09_18_30_00-2.json.gz';

-- ---------------------------------------------------------------- the switch, off by default
select has_table('public', 'simulator_consent', 'the choice has its own table');
select is((select count(*)::int from public.simulator_consent), 0,
  'no account has opted in until it uses the switch (existing and new accounts are off)');
select ok(
  not exists (select 1 from information_schema.columns
              where table_schema = 'public' and table_name = 'profiles' and column_name ~ 'simulator|consent'),
  'the choice is not on the profile, which everyone can read for public profiles');

set local role authenticated;
select pg_temp.as_user(1);
select is(public.set_simulator_consent(true), true, 'the owner turns the switch on');
select results_eq(
  $$ select improve_simulator, policy_version, updated_at = now() from public.simulator_consent $$,
  $$ values (true, '2026-10-12'::text, true) $$,
  'and reads it back, with when and under which privacy policy version it was given');
select throws_ok(
  format($$ insert into public.simulator_consent (user_id, improve_simulator, policy_version) values ('%s', true, '2026-10-12') $$, pg_temp.pid(2)),
  '42501', null, 'nobody writes the table directly, not even for another account');
select throws_ok(
  $$ update public.simulator_consent set improve_simulator = false $$,
  '42501', null, 'nor updates it directly');
select throws_ok(
  $$ delete from public.simulator_consent $$,
  '42501', null, 'nor deletes it directly');
select throws_ok($$ select public.set_simulator_consent(null) $$, '22023', null,
  'the switch is on or off, never unknown');
reset role;

set local role authenticated;
select pg_temp.as_user(2);
select is_empty($$ select * from public.simulator_consent $$,
  'another account cannot see someone else''s choice');
select is(public.set_simulator_consent(true), true, 'P2 turns it on too');
reset role;
-- P2's consent was given an hour ago; withdrawing records the new time.
update public.simulator_consent set updated_at = now() - interval '1 hour' where user_id = pg_temp.pid(2);
set local role authenticated;
select pg_temp.as_user(2);
select is(public.set_simulator_consent(false), false, 'and withdraws it');
select results_eq(
  $$ select improve_simulator, updated_at = now() from public.simulator_consent $$,
  $$ values (false, true) $$,
  'the row keeps the withdrawal and when it happened');
reset role;

delete from public.profiles where user_id = pg_temp.pid(4);
set local role authenticated;
select pg_temp.as_user(4);
select throws_ok($$ select public.set_simulator_consent(true) $$, '42501', null,
  'an account with no profile gets a clean refusal');
reset role;
set local role anon;
select set_config('request.jwt.claims', '{"role": "anon"}', true);
select throws_ok($$ select public.set_simulator_consent(true) $$, '42501', null,
  'an anonymous visitor cannot set anything');
select throws_ok($$ select * from public.simulator_consent $$, '42501', null,
  'nor read the choices');
select throws_ok($$ select * from public.simulator_files(10) $$, '42501', null,
  'nor list the games');
reset role;
select throws_ok($$ select public.set_simulator_consent(true) $$, '42501', null,
  'without a signed-in user (the service role or the database) the switch refuses');

-- P5 opts in (its files are kept without limit; that changes nothing here).
set local role authenticated;
select pg_temp.as_user(5);
select is(public.set_simulator_consent(true), true, 'P5 turns it on');
select throws_ok($$ select * from public.simulator_files(10) $$, '42501', null,
  'a signed-in user cannot list the games, not even their own');
reset role;

-- ---------------------------------------------------------------- the owner's list
set local role service_role;
select results_eq(
  $$ select storage_path from public.simulator_files(10) $$,
  format($$ values ('%s/Hearthstone_2026_10_09_18_30_00-1.json.gz'::text),
                   ('%s/Hearthstone_2026_10_09_18_30_00-1.json.gz'::text) $$,
         pg_temp.pid(1), pg_temp.pid(5)),
  'the list holds only the files still kept of accounts whose switch is on now, newest first');
select results_eq(
  $$ select content_sha256, size_bytes from public.simulator_files(1) $$,
  $$ values (repeat('d', 64), 3000) $$,
  'with the hash and size to check each download, and no more than asked for');
select throws_ok($$ select * from public.simulator_files(0) $$, '22023', null,
  'the limit is at least 1');
select throws_ok($$ select * from public.simulator_files(10001) $$, '22023', null,
  'and at most 10 000');
select throws_ok($$ select * from public.simulator_files(null) $$, '22023', null,
  'and never unknown');
reset role;

set local role authenticated;
select pg_temp.as_user(1);
select is(public.set_simulator_consent(false), false, 'P1 withdraws');
reset role;
set local role service_role;
select results_eq(
  $$ select storage_path from public.simulator_files(10) $$,
  format($$ values ('%s/Hearthstone_2026_10_09_18_30_00-1.json.gz'::text) $$, pg_temp.pid(5)),
  'from that moment P1''s games are out of the list');
reset role;

-- ---------------------------------------------------------------- export and deletion
set local role authenticated;
select pg_temp.as_user(5);
select is(public.export_my_data() -> 'simulator_consent' -> 'improve_simulator', 'true'::jsonb,
  'the export has the switch''s state');
select ok(public.export_my_data() -> 'simulator_consent' ->> 'updated_at' is not null
  and public.export_my_data() -> 'simulator_consent' ->> 'policy_version' = '2026-10-12',
  'and when and under which policy it was set');
reset role;
set local role authenticated;
select pg_temp.as_user(3);
select is(public.export_my_data() -> 'simulator_consent',
  '{"improve_simulator": false, "updated_at": null, "policy_version": null}'::jsonb,
  'an account that never used the switch exports it as off');
reset role;
delete from storage.objects where bucket_id = 'games' and name like pg_temp.pid(5)::text || '/%';
select is((public.delete_user_data(pg_temp.pid(5)) ->> 'profiles')::int, 1,
  'account deletion removes the profile');
select is((select count(*)::int from public.simulator_consent where user_id = pg_temp.pid(5)), 0,
  'and the choice with it');

-- ---------------------------------------------------------------- rights
select is_empty(
  $$ select privilege_type from information_schema.role_table_grants
     where table_schema = 'public' and table_name = 'simulator_consent'
       and grantee in ('anon', 'PUBLIC') $$,
  'anonymous visitors have no right on the choices');
select is(
  (select string_agg(privilege_type::text, ',') from information_schema.role_table_grants
   where table_schema = 'public' and table_name = 'simulator_consent' and grantee = 'authenticated'),
  'SELECT',
  'signed-in users may only read (their own row, by the policy)');
select ok(
  not has_function_privilege('anon', 'public.set_simulator_consent(boolean)', 'execute')
  and has_function_privilege('authenticated', 'public.set_simulator_consent(boolean)', 'execute'),
  'only signed-in users can call the switch');
select ok(
  not has_function_privilege('anon', 'public.simulator_files(integer)', 'execute')
  and not has_function_privilege('authenticated', 'public.simulator_files(integer)', 'execute')
  and has_function_privilege('service_role', 'public.simulator_files(integer)', 'execute'),
  'only the service role (the owner) can list the games');
select ok(
  (select bool_and(prosecdef and proconfig @> array['search_path=""']) from pg_proc
   where oid in ('public.set_simulator_consent(boolean)'::regprocedure,
                 'public.simulator_files(integer)'::regprocedure)),
  'the functions run with a fixed, empty search path');

select * from finish();
rollback;

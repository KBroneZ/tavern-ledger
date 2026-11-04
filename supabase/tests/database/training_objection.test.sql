-- The switch to object to training the expected-place model (T-651, D-174).
-- Run: npx supabase test db. Users and games are made up; everything rolls
-- back at the end.
begin;
create extension if not exists pgtap with schema extensions;
select plan(38);

-- New functions are closed to clients by default; the helpers below run under their roles.
alter default privileges for role postgres grant execute on functions to public;

-- Players P1..P5 (ids from pg_temp.pid). P1 never uses the switch; P2 turns
-- it off; P3 turns it off and on again; P4 loses its profile (a broken
-- account); P5 has unlimited history and turns it off late.
create function pg_temp.pid(i int) returns uuid language sql immutable as $$
  select ('00000000-0000-4000-8000-' || lpad(to_hex(16384 + i), 12, '0'))::uuid
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
    final_place, build, played_on, saved_at, content_sha256, size_bytes, file_expired_at,
    storage_path)
  values (pg_temp.pid(i), 'Hearthstone_2026_10_20_18_30_00', idx, 'GT_BATTLEGROUNDS', 'ok',
    'BG20_HERO_202', 3, 253216, current_date - idx - i, 1791000000, repeat('e', 64), 4000,
    case when expired then now() end,
    pg_temp.pid(i)::text || '/training-' || idx || '.json.gz')
  returning storage_path into path;
  if not expired then
    insert into storage.objects (bucket_id, name) values ('games', path);
  end if;
  return path;
end;
$$;

create function pg_temp.path(i int, idx int) returns text language sql immutable as $$
  select pg_temp.pid(i)::text || '/training-' || idx || '.json.gz'
$$;

insert into auth.users (id, email)
select pg_temp.pid(i), 'tr' || i || '@example.test' from generate_series(1, 5) as i;

select pg_temp.game(1, 1, false), pg_temp.game(1, 2, false), pg_temp.game(1, 3, true),
       pg_temp.game(2, 1, false), pg_temp.game(3, 1, false), pg_temp.game(5, 1, false);
-- P1's second game has a row but its file is gone without the mark (a
-- failed or manual delete): it is not listed either.
set local storage.allow_delete_query = 'true';
delete from storage.objects where bucket_id = 'games' and name = pg_temp.path(1, 2);

-- ---------------------------------------------------------------- the switch, on by default
select has_table('public', 'training_objection', 'the choice has its own table');
select is((select count(*)::int from public.training_objection), 0,
  'no account has a row until it uses the switch: no row is on (existing and new accounts)');
select ok(
  not exists (select 1 from information_schema.columns
              where table_schema = 'public' and table_name = 'profiles' and column_name ~ 'training|objection'),
  'the choice is not on the profile, which everyone can read for public profiles');

set local role service_role;
select results_eq(
  $$ select storage_path from public.training_files(10) $$,
  format($$ values (%L::text), (%L::text), (%L::text), (%L::text) $$,
         pg_temp.path(1, 1), pg_temp.path(2, 1), pg_temp.path(3, 1), pg_temp.path(5, 1)),
  'before anyone objects, the list holds every file still kept, newest first');
reset role;

set local role authenticated;
select pg_temp.as_user(2);
select is(public.set_training_choice(false), false, 'the owner turns the switch off (objects)');
select results_eq(
  $$ select train_model, policy_version, updated_at = now() from public.training_objection $$,
  $$ values (false, '2026-10-27'::text, true) $$,
  'and reads it back, with when and under which privacy policy version');
select throws_ok(
  format($$ insert into public.training_objection (user_id, train_model, policy_version) values ('%s', false, '2026-10-27') $$, pg_temp.pid(1)),
  '42501', null, 'nobody writes the table directly, not even for another account');
select throws_ok(
  $$ update public.training_objection set train_model = true $$,
  '42501', null, 'nor updates it directly');
select throws_ok(
  $$ delete from public.training_objection $$,
  '42501', null, 'nor deletes it directly');
select throws_ok($$ select public.set_training_choice(null) $$, '22023', null,
  'the switch is on or off, never unknown');
reset role;

set local role authenticated;
select pg_temp.as_user(3);
select is_empty($$ select * from public.training_objection $$,
  'another account cannot see someone else''s choice');
select is(public.set_training_choice(false), false, 'P3 turns it off too');
reset role;
-- P3's objection was an hour ago; turning it back on records the new time.
update public.training_objection set updated_at = now() - interval '1 hour' where user_id = pg_temp.pid(3);
set local role authenticated;
select pg_temp.as_user(3);
select is(public.set_training_choice(true), true, 'and back on');
select results_eq(
  $$ select train_model, updated_at = now() from public.training_objection $$,
  $$ values (true, true) $$,
  'the row keeps the new choice and when it was made');
reset role;

delete from public.profiles where user_id = pg_temp.pid(4);
set local role authenticated;
select pg_temp.as_user(4);
select throws_ok($$ select public.set_training_choice(false) $$, '42501', null,
  'an account with no profile gets a clean refusal');
reset role;
set local role anon;
select set_config('request.jwt.claims', '{"role": "anon"}', true);
select throws_ok($$ select public.set_training_choice(false) $$, '42501', null,
  'an anonymous visitor cannot set anything');
select throws_ok($$ select * from public.training_objection $$, '42501', null,
  'nor read the choices');
select throws_ok($$ select * from public.training_files(10) $$, '42501', null,
  'nor list the games');
reset role;
select throws_ok($$ select public.set_training_choice(false) $$, '42501', null,
  'without a signed-in user (the service role or the database) the switch refuses');

set local role authenticated;
select pg_temp.as_user(1);
select throws_ok($$ select * from public.training_files(10) $$, '42501', null,
  'a signed-in user cannot list the games, not even their own');
reset role;

-- ---------------------------------------------------------------- the owner's list
set local role service_role;
select results_eq(
  $$ select storage_path from public.training_files(10) $$,
  format($$ values (%L::text), (%L::text), (%L::text) $$, pg_temp.path(1, 1), pg_temp.path(3, 1), pg_temp.path(5, 1)),
  'the list holds only the files still kept of accounts that have not objected now, newest first');
select results_eq(
  $$ select content_sha256, size_bytes from public.training_files(1) $$,
  $$ values (repeat('e', 64), 4000) $$,
  'with the hash and size to check each download, and no more than asked for');
select throws_ok($$ select * from public.training_files(0) $$, '22023', null,
  'the limit is at least 1');
select throws_ok($$ select * from public.training_files(10001) $$, '22023', null,
  'and at most 10 000');
select throws_ok($$ select * from public.training_files(null) $$, '22023', null,
  'and never unknown');
reset role;

set local role authenticated;
select pg_temp.as_user(5);
select is(public.set_training_choice(false), false, 'P5 objects');
reset role;
set local role service_role;
select results_eq(
  $$ select storage_path from public.training_files(10) $$,
  format($$ values (%L::text), (%L::text) $$, pg_temp.path(1, 1), pg_temp.path(3, 1)),
  'from that moment P5''s games are out of the list');
reset role;

-- ---------------------------------------------------------------- export and deletion
set local role authenticated;
select pg_temp.as_user(5);
select is(public.export_my_data() -> 'training_choice' -> 'train_model', 'false'::jsonb,
  'the export has the switch''s state');
select ok(public.export_my_data() -> 'training_choice' ->> 'updated_at' is not null
  and public.export_my_data() -> 'training_choice' ->> 'policy_version' = '2026-10-27',
  'and when and under which policy it was set');
reset role;
set local role authenticated;
select pg_temp.as_user(1);
select is(public.export_my_data() -> 'training_choice',
  '{"train_model": true, "updated_at": null, "policy_version": null}'::jsonb,
  'an account that never used the switch exports it as on, with no date or version');
select ok(public.export_my_data() ? 'simulator_consent' and public.export_my_data() ? 'community_consent',
  'the rest of the export is unchanged');
reset role;
delete from storage.objects where bucket_id = 'games' and name like pg_temp.pid(5)::text || '/%';
select is((public.delete_user_data(pg_temp.pid(5)) ->> 'profiles')::int, 1,
  'account deletion removes the profile');
select is((select count(*)::int from public.training_objection where user_id = pg_temp.pid(5)), 0,
  'and the choice with it');

-- ---------------------------------------------------------------- rights
select is_empty(
  $$ select privilege_type from information_schema.role_table_grants
     where table_schema = 'public' and table_name = 'training_objection'
       and grantee in ('anon', 'PUBLIC') $$,
  'anonymous visitors have no right on the choices');
select is(
  (select string_agg(privilege_type::text, ',') from information_schema.role_table_grants
   where table_schema = 'public' and table_name = 'training_objection' and grantee = 'authenticated'),
  'SELECT',
  'signed-in users may only read (their own row, by the policy)');
select ok(
  not has_function_privilege('anon', 'public.set_training_choice(boolean)', 'execute')
  and has_function_privilege('authenticated', 'public.set_training_choice(boolean)', 'execute'),
  'only signed-in users can call the switch');
select ok(
  not has_function_privilege('anon', 'public.training_files(integer)', 'execute')
  and not has_function_privilege('authenticated', 'public.training_files(integer)', 'execute')
  and has_function_privilege('service_role', 'public.training_files(integer)', 'execute'),
  'only the service role (the owner) can list the games');
select ok(
  (select bool_and(prosecdef and proconfig @> array['search_path=""']) from pg_proc
   where oid in ('public.set_training_choice(boolean)'::regprocedure,
                 'public.training_files(integer)'::regprocedure)),
  'the functions run with a fixed, empty search path');

select * from finish();
rollback;

-- Row-level security and privileges (T-104a). Run: npx supabase test db
-- Users are made up; everything rolls back at the end.
begin;
create extension if not exists pgtap with schema extensions;
select plan(32);

-- Two users. The trigger gives each a private profile.
insert into auth.users (id, email) values
  ('00000000-0000-4000-8000-00000000000a', 'a@example.test'),
  ('00000000-0000-4000-8000-00000000000b', 'b@example.test');

select results_eq(
  $$ select count(*)::int, bool_or(is_public), count(display_name)::int from public.profiles $$,
  $$ values (2, false, 0) $$,
  'new users get a private profile with no display name'
);

-- Game rows and files are written by the service role (here: postgres).
insert into public.games (user_id, session, game_index, game_type, status, hero_card_id,
  final_place, build, played_on, tribes_offered, saved_at, content_sha256, size_bytes)
values
  ('00000000-0000-4000-8000-00000000000a', 'Hearthstone_2026_10_09_18_30_00', 1,
   'GT_BATTLEGROUNDS_DUO', 'ok', 'BG36_HERO_000', 1, 253216, '2026-10-09',
   '{"BEAST": 10, "NAGA": 7}', 1791000000, repeat('a', 64), 2500);
insert into storage.objects (bucket_id, name, owner_id)
values ('games', '00000000-0000-4000-8000-00000000000a/Hearthstone_2026_10_09_18_30_00-1.json.gz', null);

select is(
  (select storage_path from public.games),
  '00000000-0000-4000-8000-00000000000a/Hearthstone_2026_10_09_18_30_00-1.json.gz',
  'storage_path is derived from the key'
);

-- No column for names, ratings or MMR in the public schema (D-012, D-017).
select is_empty(
  $$ select table_name, column_name from information_schema.columns
     where table_schema = 'public'
       and column_name ~* '(mmr|rating|battle_?tag|player_?name|account_?id|rank)' $$,
  'no name, BattleTag, rating, MMR or rank column'
);

-- Checks on what the service role writes.
select throws_ok(
  $$ update public.profiles set display_name = 'Name#1234'
     where user_id = '00000000-0000-4000-8000-00000000000a' $$,
  '23514', null, 'a BattleTag-shaped display name is rejected'
);
select throws_ok(
  $$ update public.profiles set display_name = repeat('x', 33)
     where user_id = '00000000-0000-4000-8000-00000000000a' $$,
  '23514', null, 'display names are capped at 32 characters'
);
select throws_ok(
  $$ update public.games set final_place = 5 $$,
  '23514', null, 'a Duos place above 4 is rejected'
);
select throws_ok(
  $$ update public.games set tribes_offered = '{"beast": 1}' $$,
  '23514', null, 'tribe keys must be upper-case tokens'
);
select throws_ok(
  $$ update public.games set tribes_offered = '{"BEAST": -1}' $$,
  '23514', null, 'tribe counts cannot be negative'
);
select throws_ok(
  $$ update public.games set session = '../etc' $$,
  '23514', null, 'session must look like a log folder name'
);
select throws_ok(
  $$ update public.games set hero_card_id = 'BG<script>' $$,
  '23514', null, 'hero must be a card id'
);

-- ---------------------------------------------------------------- anon
set local role anon;
select set_config('request.jwt.claims', '{"role": "anon"}', true);

select is_empty($$ select * from public.profiles $$, 'anon sees no private profile');
select is_empty($$ select * from public.games $$, 'anon sees no game of a private profile');
select is_empty($$ select * from storage.objects where bucket_id = 'games' $$,
  'anon sees no file of a private profile');
select throws_ok($$ select public.export_my_data() $$, '42501', null,
  'anon cannot call export');

-- ---------------------------------------------------------------- user B
reset role;
set local role authenticated;
select set_config('request.jwt.claims',
  '{"sub": "00000000-0000-4000-8000-00000000000b", "role": "authenticated"}', true);

select results_eq(
  $$ select user_id from public.profiles $$,
  $$ values ('00000000-0000-4000-8000-00000000000b'::uuid) $$,
  'another user sees only their own profile, not a private one'
);
select is_empty($$ select * from public.games $$, 'another user cannot read private games');
select is_empty($$ select * from storage.objects where bucket_id = 'games' $$,
  'another user cannot read private files');
select is_empty(
  $$ update public.profiles set is_public = true
     where user_id = '00000000-0000-4000-8000-00000000000a' returning 1 $$,
  'another user cannot change someone else''s profile'
);
select throws_ok($$ select public.delete_user_data('00000000-0000-4000-8000-00000000000a') $$,
  '42501', null, 'clients cannot call delete_user_data');
select throws_ok($$ select public.user_file_names('00000000-0000-4000-8000-00000000000a') $$,
  '42501', null, 'clients cannot call user_file_names');

-- ---------------------------------------------------------------- user A
reset role;
set local role authenticated;
select set_config('request.jwt.claims',
  '{"sub": "00000000-0000-4000-8000-00000000000a", "role": "authenticated"}', true);

select throws_ok(
  $$ insert into public.games (user_id, session, game_index, game_type, status, played_on,
       saved_at, content_sha256, size_bytes)
     values ('00000000-0000-4000-8000-00000000000a', 'Hearthstone_2026_10_09_18_30_00', 2,
       'GT_BATTLEGROUNDS', 'ok', '2026-10-09', 1, repeat('b', 64), 1) $$,
  '42501', null, 'the owner cannot insert a game row'
);
select throws_ok($$ update public.games set final_place = 2 $$, '42501', null,
  'the owner cannot update a game row');
select throws_ok($$ delete from public.games $$, '42501', null,
  'the owner cannot delete a game row');
select throws_ok(
  $$ insert into storage.objects (bucket_id, name)
     values ('games', '00000000-0000-4000-8000-00000000000a/x.json.gz') $$,
  '42501', null, 'the owner cannot write a file in the bucket'
);
select is_empty(
  $$ update storage.objects set name = name || '.bak' where bucket_id = 'games' returning 1 $$,
  'the owner cannot rename a file in the bucket'
);
select throws_ok(
  $$ insert into public.profiles (user_id) values ('00000000-0000-4000-8000-0000000000ff') $$,
  '42501', null, 'clients cannot create profiles'
);
select throws_ok(
  $$ update public.profiles set created_at = now() $$, '42501', null,
  'the owner can only change display_name and is_public'
);
select is(
  (select count(*)::int from storage.objects where bucket_id = 'games'), 1,
  'the owner reads their own file'
);
select is(
  (select jsonb_array_length(public.export_my_data() -> 'games')), 1,
  'export has the owner''s game'
);

update public.profiles set display_name = 'Tester', is_public = true;

-- ---------------------------------------------------------------- public
reset role;
set local role anon;
select set_config('request.jwt.claims', '{"role": "anon"}', true);

select results_eq(
  $$ select display_name from public.profiles $$, $$ values ('Tester'::text) $$,
  'a public profile is readable by anyone'
);
select is((select count(*)::int from public.games), 1, 'a public profile''s games are readable');
select is((select count(*)::int from storage.objects where bucket_id = 'games'), 1,
  'a public profile''s files are readable');

reset role;
select * from finish();
rollback;

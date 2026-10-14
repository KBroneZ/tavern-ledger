-- What a public profile shows (T-572). Run: npx supabase test db.
-- Users and games are made up; everything rolls back at the end.
begin;
create extension if not exists pgtap with schema extensions;
select plan(19);

-- A: public, B: private, C: another signed-in user.
insert into auth.users (id, email) values
  ('00000000-0000-4000-8000-00000000000a', 'a@example.test'),
  ('00000000-0000-4000-8000-00000000000b', 'b@example.test'),
  ('00000000-0000-4000-8000-00000000000c', 'c@example.test');
update public.profiles set display_name = 'Alpha', is_public = true
  where user_id = '00000000-0000-4000-8000-00000000000a';

insert into public.games (user_id, session, game_index, game_type, status, hero_card_id,
  final_place, build, played_on, tribes_offered, saved_at, content_sha256, size_bytes, file_expired_at)
values
  ('00000000-0000-4000-8000-00000000000a', 'Hearthstone_2026_10_09_18_30_00', 1,
   'GT_BATTLEGROUNDS', 'ok', 'BG36_HERO_000', 1, 253216, '2026-10-09',
   '{"BEAST": 10}', 1791000000, repeat('a', 64), 2500, now()),
  ('00000000-0000-4000-8000-00000000000b', 'Hearthstone_2026_10_09_19_30_00', 1,
   'GT_BATTLEGROUNDS', 'ok', 'BG36_HERO_000', 2, 253216, '2026-10-09',
   '{"BEAST": 10}', 1791000000, repeat('b', 64), 2500, now());
insert into public.game_stats (user_id, session, game_index, summary_version, rounds,
  combats_won, combats_lost, combats_tied, combats_unknown)
values ('00000000-0000-4000-8000-00000000000a', 'Hearthstone_2026_10_09_18_30_00', 1, 1, 3, 3, 0, 0, 0);

-- ---------------------------------------------------------------- column privileges
select is_empty(
  $$ select r.role, t.tbl, t.col
     from unnest(array['anon', 'authenticated']) as r(role),
       (values ('public.profiles', 'created_at'), ('public.profiles', 'updated_at'),
               ('public.profiles', 'display_name_legacy_dup'),
               ('public.games', 'content_sha256'), ('public.games', 'file_expired_at'),
               ('public.games', 'saved_at'), ('public.games', 'size_bytes'),
               ('public.games', 'created_at'), ('public.games', 'updated_at'),
               ('public.game_stats', 'computed_at')) as t(tbl, col)
     where has_column_privilege(r.role, t.tbl, t.col, 'select') $$,
  'clients cannot read the hash, the size, the expiry mark or any stored time');
select ok(
  has_column_privilege('anon', 'public.profiles', 'display_name', 'select')
  and has_column_privilege('anon', 'public.profiles', 'is_public', 'select')
  and has_column_privilege('anon', 'public.games', 'session', 'select')
  and has_column_privilege('anon', 'public.games', 'played_on', 'select')
  and has_column_privilege('anon', 'public.game_stats', 'rounds', 'select')
  and has_table_privilege('anon', 'public.game_summaries', 'select'),
  'the columns the pages use are still readable');

-- ---------------------------------------------------------------- anon
set local role anon;
select set_config('request.jwt.claims', '{"role": "anon"}', true);

select throws_ok($$ select content_sha256 from public.games $$, '42501', null,
  'anon cannot read the file hash');
select throws_ok($$ select * from public.games $$, '42501', null,
  'a select of every column is refused');
select throws_ok($$ select updated_at from public.profiles $$, '42501', null,
  'anon cannot read a profile''s times');
select throws_ok($$ select computed_at from public.game_stats $$, '42501', null,
  'anon cannot read when a summary was worked out');
select is((select count(*)::int from public.games), 1, 'anon still reads the public profile''s game row');
select results_eq(
  $$ select user_id::text, game_index, mode, counted_place, rounds, file_expired_at is null
     from public.game_summaries $$,
  $$ values ('00000000-0000-4000-8000-00000000000a', 1, 'solo', 1::smallint, 3::smallint, true) $$,
  'anon reads the public game''s summary, without the expiry mark, and nothing of a private profile');
select is((select count(*)::int from public.user_overview), 1, 'the overview of the public profile is readable');
select is((select count(*)::int from public.user_heroes), 1, 'the hero totals of the public profile are readable');

-- ---------------------------------------------------------------- another user
reset role;
set local role authenticated;
select set_config('request.jwt.claims',
  '{"sub": "00000000-0000-4000-8000-00000000000c", "role": "authenticated"}', true);
select results_eq(
  $$ select user_id::text, file_expired_at is null from public.game_summaries $$,
  $$ values ('00000000-0000-4000-8000-00000000000a', true) $$,
  'another signed-in user sees the same reduced rows');
select throws_ok($$ select file_expired_at from public.games $$, '42501', null,
  'and cannot read the mark from the table either');

-- ---------------------------------------------------------------- the owner
reset role;
set local role authenticated;
select set_config('request.jwt.claims',
  '{"sub": "00000000-0000-4000-8000-00000000000a", "role": "authenticated"}', true);
select results_eq(
  $$ select user_id::text, file_expired_at is not null from public.game_summaries $$,
  $$ values ('00000000-0000-4000-8000-00000000000a', true) $$,
  'the owner sees the expiry mark of their own game');
select is((select display_name from public.profiles where user_id = '00000000-0000-4000-8000-00000000000a'),
  'Alpha', 'the owner reads their own profile as before');
select lives_ok(
  $$ update public.profiles set is_public = true where user_id = '00000000-0000-4000-8000-00000000000a' $$,
  'and can still change it');
select ok(
  (select (public.export_my_data() -> 'profile') ? 'created_at'),
  'the owner''s times are in their data export');

reset role;
set local role authenticated;
select set_config('request.jwt.claims',
  '{"sub": "00000000-0000-4000-8000-00000000000b", "role": "authenticated"}', true);
select results_eq(
  $$ select user_id::text, file_expired_at is null from public.game_summaries order by 1 $$,
  $$ values ('00000000-0000-4000-8000-00000000000a', true), ('00000000-0000-4000-8000-00000000000b', false) $$,
  'a private profile''s owner sees their own games and the public ones');

-- ---------------------------------------------------------------- the view
reset role;
select ok(
  (select c.reloptions @> array['security_invoker=false'] or c.reloptions is null
   from pg_class c where c.oid = 'public.game_summaries'::regclass),
  'game_summaries runs with the owner''s rights (the row rule is in the view)');
select ok(
  (select pg_get_viewdef('public.game_summaries'::regclass) ~ 'auth\.uid\(\)'),
  'and applies the caller''s identity');

select * from finish();
rollback;

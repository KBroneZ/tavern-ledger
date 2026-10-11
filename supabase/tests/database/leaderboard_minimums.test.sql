-- Leaderboards with no minimum games (T-534, D-103, amends D-082): any public
-- profile with at least one finished game in the mode and period is listed,
-- and every other D-082 rule still holds. The machinery with a raised
-- minimum is tested in leaderboards.test.sql.
-- Run: npx supabase test db. Users and games are made up; everything rolls
-- back at the end.
begin;
create extension if not exists pgtap with schema extensions;
select plan(18);

-- Players (ids end in their letter where hex allows):
--   A  Alpha      public, 1 Solo win yesterday with BG20, APM 20
--   B  Bravo      public, 3 Solo wins yesterday with a BG20 skin
--   C  Charlie    private, 1 Solo win
--   D  (none)     public with no display name, 1 Solo win
--   E  Echo       public, 1 Solo game with 8 golden minions (impossible)
--   F  Foxtrot    public, 1 unfinished Solo game
--   1A Golf       public, 1 Duos game 40 days ago, 4th
--   1B Kilo Fruit public, 1 Solo win; "fruit" is listed after the name is set
insert into auth.users (id, email)
select ('00000000-0000-4000-8000-0000000000' || x)::uuid, x || '@example.test'
from unnest(array['0a', '0b', '0c', '0d', '0e', '0f', '1a', '1b']) as x;

update public.profiles p set display_name = v.name, is_public = v.pub
from (values
  ('00000000-0000-4000-8000-00000000000a'::uuid, 'Alpha', true),
  ('00000000-0000-4000-8000-00000000000b', 'Bravo', true),
  ('00000000-0000-4000-8000-00000000000c', 'Charlie', false),
  ('00000000-0000-4000-8000-00000000000d', null, true),
  ('00000000-0000-4000-8000-00000000000e', 'Echo', true),
  ('00000000-0000-4000-8000-00000000000f', 'Foxtrot', true),
  ('00000000-0000-4000-8000-00000000001a', 'Golf', true),
  ('00000000-0000-4000-8000-00000000001b', 'Kilo Fruit', true)
) as v(id, name, pub)
where p.user_id = v.id;
insert into private.name_blocklist values ('fruit', 'part', 'offensive');

-- n games for one player, stored the way the upload function stores them.
create function pg_temp.seed(
  uid uuid, first_index int, n int, game_type text, status text, place int,
  hero text, played date, stats jsonb
) returns int language sql as $$
  select count(*)::int from generate_series(first_index, first_index + n - 1) as i,
  lateral public.upload_commit(
    uid, 'Hearthstone_2026_10_12_18_30_00', i, game_type, status, hero, place, 253216, played,
    '{}'::jsonb, 1791000000, repeat('a', 64), 2000, '0.1.0+r7', stats, 1
  ) as r
  where r = 'created';
$$;

select pg_temp.seed('00000000-0000-4000-8000-00000000000a', 1, 1, 'GT_BATTLEGROUNDS', 'ok', 1,
  'BG20_HERO_202', current_date - 1, '{"rounds": 9, "apm": 20}');
select pg_temp.seed('00000000-0000-4000-8000-00000000000b', 1, 3, 'GT_BATTLEGROUNDS', 'ok', 1,
  'BG20_HERO_202_SKIN_A', current_date - 1, '{"rounds": 9}');
select pg_temp.seed('00000000-0000-4000-8000-00000000000c', 1, 1, 'GT_BATTLEGROUNDS', 'ok', 1,
  'BG20_HERO_202', current_date - 1, '{"rounds": 9}');
select pg_temp.seed('00000000-0000-4000-8000-00000000000d', 1, 1, 'GT_BATTLEGROUNDS', 'ok', 1,
  'BG20_HERO_202', current_date - 1, '{"rounds": 9}');
select pg_temp.seed('00000000-0000-4000-8000-00000000000e', 1, 1, 'GT_BATTLEGROUNDS', 'ok', 1,
  'BG20_HERO_202', current_date - 1, '{"rounds": 12, "golden_max": 8}');
select pg_temp.seed('00000000-0000-4000-8000-00000000000f', 1, 1, 'GT_BATTLEGROUNDS', 'incomplete', null,
  'BG20_HERO_202', current_date - 1, '{"rounds": 3}');
select pg_temp.seed('00000000-0000-4000-8000-00000000001a', 1, 1, 'GT_BATTLEGROUNDS_DUO', 'ok', 4,
  'BG24_HERO_100', current_date - 40, '{"rounds": 9}');
select pg_temp.seed('00000000-0000-4000-8000-00000000001b', 1, 1, 'GT_BATTLEGROUNDS', 'ok', 1,
  'BG20_HERO_202', current_date - 1, '{"rounds": 9}');

-- ---------------------------------------------------------------- the rule
select is(private.leaderboard_min_games(), 1, 'a board needs one finished game');
select is(private.leaderboard_min_hero_games(), 1, 'the hero board needs one game with the hero');
select ok(private.refresh_leaderboards() > 0, 'the rebuild stores rows');
select is((select excluded_games from private.leaderboard_state), 1,
  'the impossible game is still counted as excluded');

-- ---------------------------------------------------------------- boards, as anonymous
set local role anon;
select set_config('request.jwt.claims', '{"role": "anon"}', true);

select results_eq(
  $$ select board, standing, display_name, value, games from public.leaderboard('solo', '7d')
     where board in ('avg_place', 'win_rate', 'games') $$,
  $$ values
     ('avg_place', 1, 'Bravo', 1.00::numeric, 3), ('avg_place', 1, 'Alpha', 1.00, 1),
     ('games', 1, 'Bravo', 3, 3), ('games', 2, 'Alpha', 1, 1),
     ('win_rate', 1, 'Bravo', 100.0, 3), ('win_rate', 1, 'Alpha', 100.0, 1) $$,
  'one finished game is enough; equal values share a standing, more games first');
select results_eq(
  $$ select standing, display_name, hero, value, games from public.leaderboard('solo', '7d')
     where board = 'hero_avg_place' $$,
  $$ values (1, 'Bravo', 'BG20_HERO_202', 1.00::numeric, 3), (1, 'Alpha', 'BG20_HERO_202', 1.00, 1) $$,
  'the hero board lists one game with a hero, skins under the base hero');
select results_eq(
  $$ select display_name, value, games from public.leaderboard('solo', '7d') where board = 'apm' $$,
  $$ values ('Alpha', 20.00::numeric, 1) $$,
  'a per-game number needs one game that has it');
select is(
  (select count(*)::int from public.leaderboard('solo', '7d') where display_name = 'Alpha'), 7,
  'a one-game player is on every board that game backs (5 boards, APM and 1 hero)');
select is_empty(
  $$ select 1 from public.leaderboard('solo', 'all')
     where display_name in ('Charlie', 'Echo', 'Foxtrot') or display_name ~* 'fruit'
        or user_id in ('00000000-0000-4000-8000-00000000000c', '00000000-0000-4000-8000-00000000000d',
                       '00000000-0000-4000-8000-00000000000e', '00000000-0000-4000-8000-00000000000f',
                       '00000000-0000-4000-8000-00000000001b') $$,
  'still out: a private profile, no display name, a refused name, only impossible or unfinished games');
select is_empty(
  $$ select 1 from public.leaderboard('duos', '30d') $$,
  'a game from 40 days ago is not in the last 30 days');
select results_eq(
  $$ select standing, display_name, value, games from public.leaderboard('duos', 'all')
     where board = 'avg_place' $$,
  $$ values (1, 'Golf', 4.00::numeric, 1) $$,
  'but it is in all time');
select is_empty(
  $$ select 1 from public.leaderboard('solo', 'all') where value is null or games < 1 $$,
  'every row has a value and at least one game behind it');

-- ---------------------------------------------------------------- opting out still works at once
reset role;
update public.profiles set is_public = false where user_id = '00000000-0000-4000-8000-00000000000a';
set local role anon;
select set_config('request.jwt.claims', '{"role": "anon"}', true);
select is_empty(
  $$ select 1 from public.leaderboard('solo', '7d') where user_id = '00000000-0000-4000-8000-00000000000a' $$,
  'a one-game player made private leaves every board at once');

-- ---------------------------------------------------------------- privileges unchanged
reset role;
select ok(
  not has_function_privilege('anon', 'private.leaderboard_min_games()', 'execute')
  and not has_function_privilege('authenticated', 'private.leaderboard_min_games()', 'execute')
  and not has_function_privilege('service_role', 'private.leaderboard_min_games()', 'execute')
  and not has_function_privilege('anon', 'private.leaderboard_min_hero_games()', 'execute')
  and not has_function_privilege('authenticated', 'private.leaderboard_min_hero_games()', 'execute')
  and not has_function_privilege('service_role', 'private.leaderboard_min_hero_games()', 'execute'),
  'no client role can call the minimums (replacing them kept the revokes)');
select ok(
  has_function_privilege('anon', 'public.leaderboard(text, text)', 'execute')
  and not has_function_privilege('anon', 'private.refresh_leaderboards()', 'execute')
  and not has_function_privilege('authenticated', 'private.refresh_leaderboards()', 'execute'),
  'reads stay public and the rebuild stays private');
select is_empty(
  $$ select table_name, privilege_type from information_schema.role_table_grants
     where table_schema = 'private'
       and table_name in ('leaderboard_rows', 'leaderboard_state', 'name_blocklist')
       and grantee in ('anon', 'authenticated', 'service_role', 'PUBLIC') $$,
  'no client role has any right on the snapshot or the word list');
select ok(
  (select prosecdef and proconfig @> array['search_path=""'] from pg_proc
   where oid = 'public.leaderboard(text, text)'::regprocedure),
  'the read still runs with a fixed, empty search path');

-- ---------------------------------------------------------------- community stats unchanged (D-083)
select ok(private.community_min_games() = 50 and private.community_min_players() = 5,
  'community hero stats keep 50 games from 5 players: they protect players who are not public');

select * from finish();
rollback;

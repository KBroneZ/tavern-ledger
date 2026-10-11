-- Public leaderboards and the display name filter (T-506, D-075, D-077).
-- Run: npx supabase test db. Users and games are made up; everything rolls
-- back at the end.
begin;
create extension if not exists pgtap with schema extensions;
select plan(62);

-- Players (ids end in their letter where hex allows):
--   A  Alpha   public, 25 Solo games yesterday (+1 impossible, +2 unfinished)
--   B  Bravo   public, 20 Solo games 40 days ago (all time only)
--   C  Charlie private, 30 Solo wins
--   D  Delta   public, 19 Solo wins (under the minimum)
--   E  Echo Fruit public, 20 Solo games; its name fails once "fruit" is listed
--   F  (none)  public with no display name, 20 Solo wins
--   G  Golf    public, 20 Duos wins
--   H  Hotel   public, 20 Solo games yesterday and 5 a week ago, no hero
--   P1..P55    public, 20 Duos games each in 3rd place (the 50-row cap, ties)
insert into auth.users (id, email)
select ('00000000-0000-4000-8000-0000000000' || x)::uuid, x || '@example.test'
from unnest(array['0a', '0b', '0c', '0d', '0e', '0f', '1a', '1b']) as x;
insert into auth.users (id, email)
select ('00000000-0000-4000-8000-' || lpad(to_hex(4096 + i), 12, '0'))::uuid, 'p' || i || '@example.test'
from generate_series(1, 55) as i;

update public.profiles p set display_name = v.name, is_public = v.pub
from (values
  ('00000000-0000-4000-8000-00000000000a'::uuid, 'Alpha', true),
  ('00000000-0000-4000-8000-00000000000b', 'Bravo', true),
  ('00000000-0000-4000-8000-00000000000c', 'Charlie', false),
  ('00000000-0000-4000-8000-00000000000d', 'Delta', true),
  ('00000000-0000-4000-8000-00000000000e', 'Echo Fruit', true),
  ('00000000-0000-4000-8000-00000000000f', null, true),
  ('00000000-0000-4000-8000-00000000001a', 'Golf', true),
  ('00000000-0000-4000-8000-00000000001b', 'Hotel', true)
) as v(id, name, pub)
where p.user_id = v.id;
update public.profiles p set display_name = 'P' || (('x' || right(p.user_id::text, 8))::bit(32)::int - 4096),
  is_public = true
where p.user_id::text like '00000000-0000-4000-8000-000000001%'
  and p.user_id::text not in ('00000000-0000-4000-8000-00000000001a', '00000000-0000-4000-8000-00000000001b');

-- n games for one player, stored the way the upload function stores them.
create function pg_temp.seed(
  uid uuid, first_index int, n int, game_type text, status text, place int,
  hero text, played date, stats jsonb
) returns int language sql as $$
  select count(*)::int from generate_series(first_index, first_index + n - 1) as i,
  lateral public.upload_commit(
    uid, 'Hearthstone_2026_10_09_18_30_00', i, game_type, status, hero, place, 253216, played,
    '{}'::jsonb, 1791000000, repeat('a', 64), 2000, '0.1.0+r7', stats, 1
  ) as r
  where r = 'created';
$$;

-- A: place (i % 8) + 1 for game i = 1..25: 2,3,4,5,6,7,8,1,... (avg 4.40,
-- top 4 in 13, wins 3, longest top-4 run 4). Heroes: 12 games with BG20, 13
-- with BG22. Board size 100 + i.
select is(
  (select sum(pg_temp.seed('00000000-0000-4000-8000-00000000000a', i, 1, 'GT_BATTLEGROUNDS', 'ok',
     (i % 8) + 1, case when i <= 12 then 'BG20_HERO_202' else 'BG22_HERO_000_SKIN_A' end,
     current_date - 1,
     jsonb_build_object('rounds', 10, 'apm', 12.5, 'rolls', 30, 'triples', 2,
       'board_stats_max', 100 + i, 'golden_max', 1, 'gold_beyond_income', 5)))::int
   from generate_series(1, 25) as i),
  25, 'player A has 25 finished games');
-- An impossible game (8 golden minions on a 7-slot board) and two unfinished.
select pg_temp.seed('00000000-0000-4000-8000-00000000000a', 26, 1, 'GT_BATTLEGROUNDS', 'ok', 1,
  'BG20_HERO_202', current_date - 1, '{"rounds": 12, "golden_max": 8}');
select pg_temp.seed('00000000-0000-4000-8000-00000000000a', 27, 2, 'GT_BATTLEGROUNDS', 'incomplete',
  null, 'BG20_HERO_202', current_date - 1, '{"rounds": 3}');
select pg_temp.seed('00000000-0000-4000-8000-00000000000b', 1, 20, 'GT_BATTLEGROUNDS', 'ok', 4,
  'BG20_HERO_202', current_date - 40, '{"rounds": 9}');
select pg_temp.seed('00000000-0000-4000-8000-00000000000c', 1, 30, 'GT_BATTLEGROUNDS', 'ok', 1,
  'BG20_HERO_202', current_date - 1, '{"rounds": 9}');
select pg_temp.seed('00000000-0000-4000-8000-00000000000d', 1, 19, 'GT_BATTLEGROUNDS', 'ok', 1,
  'BG20_HERO_202', current_date - 1, '{"rounds": 9}');
select pg_temp.seed('00000000-0000-4000-8000-00000000000e', 1, 20, 'GT_BATTLEGROUNDS', 'ok', 2,
  'BG20_HERO_202', current_date - 1, '{"rounds": 9}');
select pg_temp.seed('00000000-0000-4000-8000-00000000000f', 1, 20, 'GT_BATTLEGROUNDS', 'ok', 1,
  'BG20_HERO_202', current_date - 1, '{"rounds": 9}');
select pg_temp.seed('00000000-0000-4000-8000-00000000001a', 1, 20, 'GT_BATTLEGROUNDS_DUO', 'ok', 1,
  'BG24_HERO_100', current_date - 2, '{"rounds": 9}');
select pg_temp.seed('00000000-0000-4000-8000-00000000001b', 1, 20, 'GT_BATTLEGROUNDS', 'ok', 8,
  null, current_date - 1, '{"rounds": 9}');
-- A week ago: outside the last 7 days (today and the six before).
select pg_temp.seed('00000000-0000-4000-8000-00000000001b', 21, 5, 'GT_BATTLEGROUNDS', 'ok', 8,
  null, current_date - 7, '{"rounds": 9}');
-- Dated tomorrow (UTC): a player ahead of UTC can play then, so it counts.
select pg_temp.seed('00000000-0000-4000-8000-00000000001a', 21, 1, 'GT_BATTLEGROUNDS_DUO', 'ok', 1,
  'BG24_HERO_100', current_date + 1, '{"rounds": 9}');
-- Dated the day after: impossible, left out of every board.
select pg_temp.seed('00000000-0000-4000-8000-00000000001b', 30, 1, 'GT_BATTLEGROUNDS', 'ok', 1,
  null, current_date + 2, '{"rounds": 9}');
-- A game with no server summary yet does not count.
select public.upload_commit('00000000-0000-4000-8000-00000000001b', 'Hearthstone_2026_10_09_18_30_00',
  26, 'GT_BATTLEGROUNDS', 'ok', null, 1, 253216, current_date - 1, '{}'::jsonb, 1791000000,
  repeat('a', 64), 2000, '0.1.0+r7');
select sum(pg_temp.seed(p.user_id, 1, 20, 'GT_BATTLEGROUNDS_DUO', 'ok', 3, 'BG24_HERO_100',
  current_date - 3, '{"rounds": 9}'))
from public.profiles p where p.display_name ~ '^P[0-9]+$';

-- Tries a display name as player D; the error text, or 'saved'.
create function pg_temp.name_refused(name text) returns text language plpgsql as $$
begin
  update public.profiles set display_name = name where user_id = '00000000-0000-4000-8000-00000000000d';
  return 'saved';
exception when check_violation then
  return sqlerrm;
end;
$$;

-- ---------------------------------------------------------------- the rebuild
select ok(private.refresh_leaderboards() > 0, 'the rebuild stores rows');
select ok(public.leaderboard_refreshed_at() between now() - interval '1 minute' and now(),
  'the rebuild time is kept');
select is((select excluded_games from private.leaderboard_state), 2,
  'the impossible games (8 golden minions, a date after tomorrow) are counted as excluded');
select results_eq(
  $$ select schedule, command, active from cron.job where jobname = 'leaderboards-refresh' $$,
  $$ values ('*/10 * * * *'::text, 'select private.refresh_leaderboards()'::text, true) $$,
  'the boards are rebuilt every 10 minutes');

-- ---------------------------------------------------------------- boards, as anonymous
set local role anon;
select set_config('request.jwt.claims', '{"role": "anon"}', true);

select results_eq(
  $$ select board, standing, display_name, value, games from public.leaderboard('solo', 'all')
     where board in ('avg_place', 'top_half_rate', 'win_rate', 'games', 'top_half_streak') $$,
  $$ values
     ('avg_place', 1, 'Echo Fruit', 2.00::numeric, 20), ('avg_place', 2, 'Bravo', 4.00, 20),
     ('avg_place', 3, 'Alpha', 4.40, 25), ('avg_place', 4, 'Hotel', 8.00, 25),
     ('games', 1, 'Alpha', 25, 25), ('games', 1, 'Hotel', 25, 25),
     ('games', 3, 'Bravo', 20, 20), ('games', 3, 'Echo Fruit', 20, 20),
     ('top_half_rate', 1, 'Bravo', 100.0, 20), ('top_half_rate', 1, 'Echo Fruit', 100.0, 20),
     ('top_half_rate', 3, 'Alpha', 52.0, 25), ('top_half_rate', 4, 'Hotel', 0.0, 25),
     ('top_half_streak', 1, 'Bravo', 20, 20), ('top_half_streak', 1, 'Echo Fruit', 20, 20),
     ('top_half_streak', 3, 'Alpha', 4, 25), ('top_half_streak', 4, 'Hotel', 0, 25),
     ('win_rate', 1, 'Alpha', 12.0, 25), ('win_rate', 2, 'Hotel', 0.0, 25),
     ('win_rate', 2, 'Bravo', 0.0, 20), ('win_rate', 2, 'Echo Fruit', 0.0, 20) $$,
  'Solo, all time: places, rates, games and streaks; equal values share a standing, more games first');
select results_eq(
  $$ select standing, display_name, hero, value, games from public.leaderboard('solo', 'all')
     where board = 'hero_avg_place' $$,
  $$ values (1, 'Echo Fruit', 'BG20_HERO_202', 2.00::numeric, 20),
     (2, 'Bravo', 'BG20_HERO_202', 4.00, 20),
     (3, 'Alpha', 'BG20_HERO_202', 4.17, 12), (4, 'Alpha', 'BG22_HERO_000', 4.62, 13) $$,
  'best average place per hero: 10 games with the hero, skins under the base hero, no unknown hero');
select results_eq(
  $$ select board, display_name, value, games from public.leaderboard('solo', 'all')
     where board in ('apm', 'gold_beyond_income', 'rolls', 'triples', 'biggest_board', 'most_golden') $$,
  $$ values ('apm', 'Alpha', 12.50::numeric, 25), ('biggest_board', 'Alpha', 125, 25),
     ('gold_beyond_income', 'Alpha', 5.00, 25), ('most_golden', 'Alpha', 1, 25),
     ('rolls', 'Alpha', 30.00, 25), ('triples', 'Alpha', 2.00, 25) $$,
  'economy and fun boards: only players with the number in 20 games; the impossible game left out');
select results_eq(
  $$ select board, display_name, value, games from public.leaderboard('solo', '7d')
     where board in ('avg_place', 'games') $$,
  $$ values ('avg_place', 'Echo Fruit', 2.00::numeric, 20), ('avg_place', 'Alpha', 4.40, 25),
     ('avg_place', 'Hotel', 8.00, 20),
     ('games', 'Alpha', 25, 25), ('games', 'Echo Fruit', 20, 20), ('games', 'Hotel', 20, 20) $$,
  'last 7 days: games of a week ago and 40 days ago are out');
select results_eq(
  $$ select board, standing, display_name, value from public.leaderboard('solo', '30d')
     where board = 'games' $$,
  $$ values ('games', 1, 'Alpha', 25::numeric), ('games', 1, 'Hotel', 25), ('games', 3, 'Echo Fruit', 20) $$,
  'last 30 days: a week ago is in, 40 days ago is out');
select is_empty(
  $$ select 1 from public.leaderboard('solo', 'all')
     where display_name in ('Charlie', 'Delta') or display_name is null
        or user_id in ('00000000-0000-4000-8000-00000000000c', '00000000-0000-4000-8000-00000000000d',
                       '00000000-0000-4000-8000-00000000000f') $$,
  'a private profile, a player under the minimum and a profile with no name are not on any board');
select is(
  (select count(*)::int from public.leaderboard('solo', 'all') where display_name = 'Echo Fruit'), 6,
  'a public player with 20 games is on every board those games back (no shop numbers: 5 boards and 1 hero)');
select is_empty(
  $$ select 1 from public.leaderboard('solo', 'all') where display_name ~ '^P[0-9]+$' or display_name = 'Golf' $$,
  'Duos players are not on the Solo boards');
select results_eq(
  $$ select standing, display_name, value from public.leaderboard('duos', '7d')
     where board = 'avg_place' and standing = 1 $$,
  $$ values (1, 'Golf', 1.00::numeric) $$,
  'Duos boards rank Duos places (per team)');
select is(
  (select games from public.leaderboard('duos', '7d') where board = 'games' and display_name = 'Golf'), 21,
  'a game dated tomorrow (UTC) counts in the last 7 days');
select is(
  (select count(*)::int from public.leaderboard('duos', '7d') where board = 'avg_place'), 50,
  'a board lists at most 50 rows');
select is(
  (select count(distinct standing)::int from public.leaderboard('duos', '7d')
   where board = 'avg_place' and standing > 1), 1,
  'the 55 equal players share one standing');
select is(
  (select count(*)::int from public.leaderboard('duos', '7d') where board = 'avg_place' and standing = 2), 49,
  'and the cap keeps the first 49 of them, in a stable order');
select results_eq(
  $$ select display_name from public.leaderboard('duos', '7d') where board = 'avg_place' and standing = 2
     order by display_name limit 1 $$,
  $$ select display_name from public.leaderboard('duos', '7d') where board = 'avg_place' and standing = 2
     order by display_name limit 1 $$,
  'two reads return the same rows');
select results_eq(
  $$ select array_agg(user_id order by user_id) from (
       select user_id from public.leaderboard('duos', '7d') where board = 'avg_place' and standing = 2) x $$,
  $$ select array_agg(user_id order by user_id) from (
       select p.user_id from public.profiles p where p.display_name ~ '^P[0-9]+$'
       order by p.user_id limit 49) x $$,
  'ties go by account id after games');
select throws_ok($$ select * from public.leaderboard('ranked', 'all') $$, '22023', null,
  'an unknown mode is refused');
select throws_ok($$ select * from public.leaderboard('solo', '365d') $$, '22023', null,
  'an unknown period is refused');
select throws_ok($$ select * from public.leaderboard(null, 'all') $$, '22023', null,
  'no mode is refused');
select is_empty(
  $$ select 1 from public.leaderboard('solo', 'all') where value is null or games < 1 $$,
  'every row has a value and the games behind it');

-- Clients cannot reach the snapshot or rebuild it.
select throws_ok($$ select * from private.leaderboard_rows $$, '42501', null,
  'anonymous cannot read the snapshot');
select throws_ok($$ select private.refresh_leaderboards() $$, '42501', null,
  'anonymous cannot rebuild the boards');
select throws_ok($$ select private.display_name_allowed('x') $$, '42501', null,
  'anonymous cannot probe the word list');

-- ---------------------------------------------------------------- another user, the private owner
reset role;
set local role authenticated;
select set_config('request.jwt.claims',
  '{"sub": "00000000-0000-4000-8000-00000000000c", "role": "authenticated"}', true);
select is_empty(
  $$ select 1 from public.leaderboard('solo', 'all') where display_name = 'Charlie' $$,
  'a private profile is not on the boards, not even for its owner');
select is((select count(*)::int from public.leaderboard('solo', 'all')), 13 + 6 + 6 + 5,
  'a signed-in user reads the same boards (A 13, B 6, E 6, H 5 rows)');
select throws_ok($$ select * from private.leaderboard_rows $$, '42501', null,
  'a signed-in user cannot read the snapshot');
select throws_ok($$ select private.refresh_leaderboards() $$, '42501', null,
  'a signed-in user cannot rebuild the boards');
select throws_ok($$ insert into private.name_blocklist values ('zzz', 'part', 'offensive') $$, '42501', null,
  'a signed-in user cannot change the word list');

-- ---------------------------------------------------------------- the name filter, as the owner
reset role;
set local role authenticated;
select set_config('request.jwt.claims',
  '{"sub": "00000000-0000-4000-8000-00000000000d", "role": "authenticated"}', true);
select results_eq(
  $$ select n, pg_temp.name_refused(n) from unnest(array[
       'fuckface', 'F u c k', 'f_u_c_k', 'sh1t lord', 'n1gger', 'Ｆｕｃｋ', 'MotherFlipper',
       'Big ASS', 'kys', 'Blizzard Fan', 'TavernLedger', 'admin', 'Official', '1488']) as n $$,
  $$ select n, 'display_name_not_allowed' from unnest(array[
       'fuckface', 'F u c k', 'f_u_c_k', 'sh1t lord', 'n1gger', 'Ｆｕｃｋ', 'MotherFlipper',
       'Big ASS', 'kys', 'Blizzard Fan', 'TavernLedger', 'admin', 'Official', '1488']) as n $$,
  'listed words are refused: anywhere, spaced out, in digits, full-width, as whole words');
select results_eq(
  $$ select n, pg_temp.name_refused(n) from unnest(array[
       'Bass Player', 'Class Act', 'Grape', 'Badminton', 'Peacock', 'Sussex', 'Álvaro Ñ',
       'Therapist', 'Ashkenazi', 'Fukuoka', 'Delta']) as n $$,
  $$ select n, 'saved' from unnest(array[
       'Bass Player', 'Class Act', 'Grape', 'Badminton', 'Peacock', 'Sussex', 'Álvaro Ñ',
       'Therapist', 'Ashkenazi', 'Fukuoka', 'Delta']) as n $$,
  'ordinary names that contain a whole-word entry are kept');
select is(pg_temp.name_refused(null), 'saved', 'clearing the name always works');
select throws_ok(
  $$ update public.profiles set display_name = 'Name#1234' where user_id = '00000000-0000-4000-8000-00000000000d' $$,
  '23514', null, 'the old rule still holds (no BattleTag shapes)');

-- ---------------------------------------------------------------- a name that fails later
reset role;
insert into private.name_blocklist values ('fruit', 'part', 'offensive');
set local role anon;
select set_config('request.jwt.claims', '{"role": "anon"}', true);
select is_empty(
  $$ select 1 from public.leaderboard('solo', 'all') where display_name = 'Echo Fruit' $$,
  'a stored name that fails the filter now is hidden from the boards at once');
reset role;
set local role authenticated;
select set_config('request.jwt.claims',
  '{"sub": "00000000-0000-4000-8000-00000000000e", "role": "authenticated"}', true);
select lives_ok(
  $$ update public.profiles set display_name = 'Echo Fruit', is_public = false
     where user_id = '00000000-0000-4000-8000-00000000000e' $$,
  'the owner of a refused name can still make the profile private (the same name sent again)');
select lives_ok(
  $$ update public.profiles set display_name = 'Echo', is_public = true
     where user_id = '00000000-0000-4000-8000-00000000000e' $$,
  'and can change it to an allowed one');
reset role;
select private.refresh_leaderboards();
set local role anon;
select set_config('request.jwt.claims', '{"role": "anon"}', true);
select is((select count(*)::int from public.leaderboard('solo', 'all') where display_name = 'Echo'), 6,
  'with an allowed name the player is back after the next rebuild');
select is_empty(
  $$ select 1 from public.leaderboard('solo', 'all') where display_name ~* 'fruit' $$,
  'the refused name is shown nowhere');

-- ---------------------------------------------------------------- opting out
reset role;
set local role authenticated;
select set_config('request.jwt.claims',
  '{"sub": "00000000-0000-4000-8000-00000000000a", "role": "authenticated"}', true);
update public.profiles set display_name = 'Alpha Two' where user_id = '00000000-0000-4000-8000-00000000000a';
select is((select count(*)::int from public.leaderboard('solo', 'all') where display_name = 'Alpha Two'), 13,
  'a new name shows at once (11 boards and 2 heroes)');
update public.profiles set is_public = false where user_id = '00000000-0000-4000-8000-00000000000a';
select is_empty(
  $$ select 1 from public.leaderboard('solo', '7d') where user_id = '00000000-0000-4000-8000-00000000000a'
     union all
     select 1 from public.leaderboard('solo', 'all') where user_id = '00000000-0000-4000-8000-00000000000a' $$,
  'a profile made private leaves every board at once');
select results_eq(
  $$ select standing, display_name from public.leaderboard('solo', '7d') where board = 'avg_place' $$,
  $$ values (1, 'Echo'), (2, 'Hotel') $$,
  'standings count only the players shown');
reset role;
select is((select count(*)::int from private.leaderboard_rows
           where user_id = '00000000-0000-4000-8000-00000000000a'), 0,
  'and its rows are deleted from the snapshot at the same moment');
update public.profiles set is_public = true where user_id = '00000000-0000-4000-8000-00000000000a';
select is((select count(*)::int from private.leaderboard_rows
           where user_id = '00000000-0000-4000-8000-00000000000a'), 0,
  'made public again, it waits for the next rebuild');
select private.refresh_leaderboards();
select ok((select count(*) from private.leaderboard_rows
           where user_id = '00000000-0000-4000-8000-00000000000a') > 0,
  'and is back after it');
select is((select players from private.leaderboard_state), 60,
  'the rebuild counts the players on any board (A, B, E, G, H and P1-P55)');

-- ---------------------------------------------------------------- export and deletion
set local role authenticated;
select set_config('request.jwt.claims',
  '{"sub": "00000000-0000-4000-8000-00000000000a", "role": "authenticated"}', true);
select is(jsonb_array_length(public.export_my_data() -> 'leaderboard_rows'), 13 * 3,
  'the export has the player''s own leaderboard rows (all time, 30 and 7 days)');
select ok(
  not exists (
    select 1 from jsonb_array_elements(public.export_my_data() -> 'leaderboard_rows') e
    where e ? 'user_id' or e ? 'display_name'
  ),
  'export rows carry the numbers only');
reset role;
select is((public.delete_user_data('00000000-0000-4000-8000-00000000000a') ->> 'profiles')::int, 1,
  'account deletion removes the profile');
select is((select count(*)::int from private.leaderboard_rows
           where user_id = '00000000-0000-4000-8000-00000000000a'), 0,
  'and its leaderboard rows with it');

-- ---------------------------------------------------------------- privileges and shape
select ok(
  has_function_privilege('anon', 'public.leaderboard(text, text)', 'execute')
  and has_function_privilege('authenticated', 'public.leaderboard(text, text)', 'execute')
  and has_function_privilege('anon', 'public.leaderboard_refreshed_at()', 'execute'),
  'everyone can read the boards');
select ok(
  not has_function_privilege('anon', 'private.refresh_leaderboards()', 'execute')
  and not has_function_privilege('authenticated', 'private.refresh_leaderboards()', 'execute')
  and not has_function_privilege('service_role', 'private.refresh_leaderboards()', 'execute'),
  'only the database itself rebuilds the boards');
select is_empty(
  $$ select table_name, privilege_type from information_schema.role_table_grants
     where table_schema = 'private'
       and table_name in ('leaderboard_rows', 'leaderboard_state', 'name_blocklist')
       and grantee in ('anon', 'authenticated', 'service_role', 'PUBLIC') $$,
  'no client role has any right on the snapshot or the word list');
select is(
  (select string_agg(p.parameter_name::text, ',' order by p.ordinal_position)
   from information_schema.parameters p
   join information_schema.routines r using (specific_schema, specific_name)
   where r.routine_schema = 'public' and r.routine_name = 'leaderboard' and p.parameter_mode = 'OUT'),
  'board,standing,user_id,display_name,hero,value,games',
  'a row is the board, the standing, the account id, the display name, the hero, the value and its games');
select ok(
  (select prosecdef and proconfig @> array['search_path=""'] from pg_proc
   where oid = 'public.leaderboard(text, text)'::regprocedure),
  'the read runs with a fixed, empty search path');
select is_empty(
  $$ select tgname from pg_trigger
     where tgrelid = 'public.profiles'::regclass and not tgisinternal and tgenabled <> 'O' $$,
  'the profile triggers are on');
select results_eq(
  $$ select private.display_name_allowed(n) from unnest(array['Hotel', 'fuck']) as n $$,
  $$ values (true), (false) $$,
  'the same check is used by the trigger and the boards');
select ok(private.leaderboard_min_games() = 20 and private.leaderboard_min_hero_games() = 10,
  'the minimums are 20 games per board and 10 per hero');
select ok(
  not private.leaderboard_game_plausible(jsonb_populate_record(null::public.game_stats, '{"apm": 100.5}'))
  and private.leaderboard_game_plausible(jsonb_populate_record(null::public.game_stats, '{"apm": 100}'))
  and not private.leaderboard_game_plausible(jsonb_populate_record(null::public.game_stats,
    '{"final_board": ["A", "B", "C", "D", "E", "F", "G", "H"]}')),
  'an APM above 100 or 8 minions on a board make a game impossible; unknown values do not');

-- Clearing the name: the stored rows go at once, like making the profile private.
update public.profiles set display_name = null where user_id = '00000000-0000-4000-8000-00000000001b';
select is((select count(*)::int from private.leaderboard_rows
           where user_id = '00000000-0000-4000-8000-00000000001b'), 0,
  'a profile whose display name is cleared leaves the snapshot at once');

select * from finish();
rollback;

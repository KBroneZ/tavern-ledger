-- Community hero stats and their opt-in (T-507, T-401, D-085).
-- Run: npx supabase test db. Users and games are made up; everything rolls
-- back at the end.
begin;
create extension if not exists pgtap with schema extensions;
select plan(62);

-- New functions are closed to clients by default; the helpers below run under their roles.
alter default privileges for role postgres grant execute on functions to public;

-- Players P1..P11 (ids from pg_temp.pid). Opted in: P1..P8 and P10; P1..P3
-- have public profiles, P4..P8 and P10 private ones. P9 has a public profile
-- and never opted in. P11 loses its profile (a broken account).
--
-- Solo, yesterday unless said otherwise:
--   H1 BG20_HERO_202  P1..P6, 10 games each (P6 with a skin), places
--                     1..8, 1, 2 (avg 3.90, top 4 60%, wins 20%); each
--                     choice offered H1, H2, H3, H4
--   H2 BG22_HERO_000  P1..P7, 7 games each = 49: one short of 50; choices
--                     offered H2 twice (two skins) and H1
--   H3 BG23_HERO_303  P1..P4, 15 games each in 8th, and P10, 20 in 3rd:
--                     80 games from 5 players until P10 opts out; P9's 100
--                     wins with it never count; one more P1 game dated today
--                     waits for its day to end
--   H4 BG24_HERO_404  P1 31 games, P2..P5 29: one player has more than half
--   H6 BG26_HERO_606  40 days ago: P1 25 games, P2..P5 25: exactly 50, 5
--                     players, one with half (all time only)
--   H7 BG27_HERO_707  P1..P5, 10 games each in 2nd (P1's on the first of the
--                     30 days); and one game of P6 the day before: all time
--                     minus 30 days would be that one game, so all time hides it
--   H8 BG28_HERO_808  P1 25, P2 25, P3..P5 10: two players hold over three quarters
-- Duos, yesterday:
--   H5 BG25_HERO_505  P1..P5, 10 wins each
create function pg_temp.pid(i int) returns uuid language sql immutable as $$
  select ('00000000-0000-4000-8000-' || lpad(to_hex(8192 + i), 12, '0'))::uuid
$$;

insert into auth.users (id, email)
select pg_temp.pid(i), 'ch' || i || '@example.test' from generate_series(1, 11) as i;
update public.profiles set is_public = true
where user_id in (pg_temp.pid(1), pg_temp.pid(2), pg_temp.pid(3), pg_temp.pid(9));

-- n finished games for one player, stored as the upload function stores
-- them; place null cycles 1..8 from the first game.
create function pg_temp.seed(
  uid uuid, first_index int, n int, game_type text, place int, hero text, played date, offered text[]
) returns int language sql as $$
  select count(*)::int from generate_series(first_index, first_index + n - 1) as i,
  lateral public.upload_commit(
    uid, 'Hearthstone_2026_10_09_18_30_00', i, game_type, 'ok', hero,
    coalesce(place, ((i - first_index) % 8) + 1), 253216, played, '{}'::jsonb, 1791000000,
    repeat('a', 64), 2000, '0.1.0+r7',
    jsonb_build_object('rounds', 10, 'heroes_offered', to_jsonb(offered)), 1
  ) as r
  where r = 'created';
$$;

create function pg_temp.as_user(i int) returns text language sql as $$
  select set_config('request.jwt.claims', format('{"sub": "%s", "role": "authenticated"}', pg_temp.pid(i)), true);
$$;

-- ---------------------------------------------------------------- before any data
select ok(private.refresh_community_heroes() = 0, 'with no games the rebuild stores no hero');
set local role anon;
select set_config('request.jwt.claims', '{"role": "anon"}', true);
select is_empty($$ select * from public.community_heroes() $$,
  'no hero is shown while there is too little data');
select results_eq(
  $$ select refreshed_at is not null, counted_until = current_date - 1, available from public.community_heroes_state() $$,
  $$ values (true, true, true) $$,
  'the state says when it was built, the last day it counts (yesterday) and that it is current');
reset role;

-- ---------------------------------------------------------------- the switch, off by default
select is((select count(*)::int from public.community_consent), 0,
  'no account has opted in until it uses the switch (existing and new accounts are off)');
select ok(
  not exists (select 1 from information_schema.columns
              where table_schema = 'public' and table_name = 'profiles' and column_name ~ 'community|consent'),
  'the choice is not on the profile, which everyone can read for public profiles');

set local role authenticated;
select pg_temp.as_user(1);
select is(public.set_community_consent(true), true, 'the owner turns the switch on');
select results_eq($$ select hero_stats, policy_version from public.community_consent $$,
  $$ values (true, '2026-10-11'::text) $$,
  'and reads it back, with the privacy policy version it was given under');
select throws_ok(
  format($$ insert into public.community_consent (user_id, hero_stats, policy_version) values ('%s', true, '2026-10-11') $$, pg_temp.pid(2)),
  '42501', null, 'nobody writes the table directly, not even for another account');
select throws_ok(
  $$ update public.community_consent set hero_stats = false $$,
  '42501', null, 'nor updates it directly');
select throws_ok($$ select public.set_community_consent(null) $$, '22023', null,
  'the switch is on or off, never unknown');
reset role;
set local role authenticated;
select pg_temp.as_user(2);
select is_empty($$ select * from public.community_consent $$,
  'another account cannot see someone else''s choice');
select is(public.set_community_consent(false), false, 'an account can save "off"');
reset role;
delete from public.profiles where user_id = pg_temp.pid(11);
set local role authenticated;
select pg_temp.as_user(11);
select throws_ok($$ select public.set_community_consent(true) $$, '42501', null,
  'an account with no profile gets a clean refusal');
reset role;
set local role anon;
select set_config('request.jwt.claims', '{"role": "anon"}', true);
select throws_ok($$ select public.set_community_consent(true) $$, '42501', null,
  'an anonymous visitor cannot set anything');
select throws_ok($$ select * from public.community_consent $$, '42501', null,
  'nor read the choices');
reset role;
select throws_ok($$ select public.set_community_consent(true) $$, '42501', null,
  'without a signed-in user (the service role or the database) the function refuses');

-- The rest of the opted-in players, as the function would store them.
insert into public.community_consent (user_id, hero_stats, policy_version)
select pg_temp.pid(i), true, '2026-10-11' from unnest(array[2, 3, 4, 5, 6, 7, 8, 10]) as i
on conflict (user_id) do update set hero_stats = true;

-- ---------------------------------------------------------------- games
select is(
  (select sum(pg_temp.seed(pg_temp.pid(i), 1, 10, 'GT_BATTLEGROUNDS', null,
     case when i = 6 then 'BG20_HERO_202_SKIN_C' else 'BG20_HERO_202' end, current_date - 1,
     array['BG20_HERO_202', 'BG22_HERO_000', 'BG23_HERO_303', 'BG24_HERO_404']))::int
   from generate_series(1, 6) as i),
  60, 'H1: 60 games');
select is(
  (select sum(pg_temp.seed(pg_temp.pid(i), 11, 7, 'GT_BATTLEGROUNDS', 4,
     case when i % 2 = 0 then 'BG22_HERO_000_SKIN_A' else 'BG22_HERO_000' end, current_date - 1,
     array['BG22_HERO_000_SKIN_A', 'BG22_HERO_000', 'BG20_HERO_202']))::int
   from generate_series(1, 7) as i),
  49, 'H2: 49 games');
select is(
  (select sum(pg_temp.seed(pg_temp.pid(i), 21, 15, 'GT_BATTLEGROUNDS', 8, 'BG23_HERO_303',
     current_date - 1, null))::int
   from generate_series(1, 4) as i)
  + pg_temp.seed(pg_temp.pid(10), 21, 20, 'GT_BATTLEGROUNDS', 3, 'BG23_HERO_303', current_date - 1, null)
  + pg_temp.seed(pg_temp.pid(9), 21, 100, 'GT_BATTLEGROUNDS', 1, 'BG23_HERO_303', current_date - 1, null)
  + pg_temp.seed(pg_temp.pid(1), 36, 1, 'GT_BATTLEGROUNDS', 1, 'BG23_HERO_303', current_date, null),
  181, 'H3: 80 games from opted-in players, 100 from a public profile that did not opt in, 1 today');
select is(
  pg_temp.seed(pg_temp.pid(1), 41, 31, 'GT_BATTLEGROUNDS', 2, 'BG24_HERO_404', current_date - 1, null)
  + (select sum(pg_temp.seed(pg_temp.pid(i), 41, case when i = 5 then 8 else 7 end, 'GT_BATTLEGROUNDS', 2,
       'BG24_HERO_404', current_date - 1, null))::int
     from generate_series(2, 5) as i),
  60, 'H4: 60 games, 31 of them from one player');
select is(
  pg_temp.seed(pg_temp.pid(1), 101, 25, 'GT_BATTLEGROUNDS', 5, 'BG26_HERO_606', current_date - 40, null)
  + (select sum(pg_temp.seed(pg_temp.pid(i), 101, case when i = 5 then 7 else 6 end, 'GT_BATTLEGROUNDS', 5,
       'BG26_HERO_606', current_date - 40, null))::int
     from generate_series(2, 5) as i),
  50, 'H6: 50 games 40 days ago, 25 of them from one player');
select is(
  (select sum(pg_temp.seed(pg_temp.pid(i), 131, 10, 'GT_BATTLEGROUNDS', 2, 'BG27_HERO_707',
     case when i = 1 then current_date - 30 else current_date - 1 end, null))::int
   from generate_series(1, 5) as i)
  + pg_temp.seed(pg_temp.pid(6), 131, 1, 'GT_BATTLEGROUNDS', 2, 'BG27_HERO_707', current_date - 31, null),
  51, 'H7: 50 games in the 30 days (P1''s on the first of them) and one just before');
select is(
  (select sum(pg_temp.seed(pg_temp.pid(i), 151, case when i <= 2 then 25 when i = 3 then 4 else 3 end,
     'GT_BATTLEGROUNDS', 3, 'BG28_HERO_808', current_date - 1, null))::int
   from generate_series(1, 5) as i),
  60, 'H8: 60 games, 50 of them from two players');
select is(
  (select sum(pg_temp.seed(pg_temp.pid(i), 201, 10, 'GT_BATTLEGROUNDS_DUO', 1, 'BG25_HERO_505',
     current_date - 1, null))::int
   from generate_series(1, 5) as i),
  50, 'H5: 50 Duos wins');
-- Games that never count: an impossible one (8 golden minions), one dated the
-- day after tomorrow, an unfinished one and one with no server summary.
select pg_temp.seed(pg_temp.pid(1), 301, 1, 'GT_BATTLEGROUNDS', 1, 'BG23_HERO_303', current_date - 1, null);
update public.game_stats set golden_max = 8 where user_id = pg_temp.pid(1) and game_index = 301;
select pg_temp.seed(pg_temp.pid(1), 302, 1, 'GT_BATTLEGROUNDS', 1, 'BG23_HERO_303', current_date + 2, null);
select public.upload_commit(pg_temp.pid(1), 'Hearthstone_2026_10_09_18_30_00', 303, 'GT_BATTLEGROUNDS',
  'incomplete', 'BG23_HERO_303', null, 253216, current_date - 1, '{}'::jsonb, 1791000000,
  repeat('a', 64), 2000, '0.1.0+r7', '{"rounds": 3}'::jsonb, 1);
select public.upload_commit(pg_temp.pid(1), 'Hearthstone_2026_10_09_18_30_00', 304, 'GT_BATTLEGROUNDS',
  'ok', 'BG23_HERO_303', 1, 253216, current_date - 1, '{}'::jsonb, 1791000000,
  repeat('a', 64), 2000, '0.1.0+r7');

-- ---------------------------------------------------------------- the rebuild
select is(private.refresh_community_heroes(), 6 + 2 + 7 + 2,
  'one row per mode, period and hero (Solo: 6 heroes in 30 days, 2 before, 7 all time; Duos: H5 twice)');
select is((select excluded_games from private.community_state), 2,
  'the impossible games (8 golden minions, a date after tomorrow) are counted as excluded');
select is(
  (select games from private.community_hero_rows where mode = 'solo' and period = '30d' and hero = 'BG23_HERO_303'),
  80, 'a game dated today waits for its day to end');
select results_eq(
  $$ select schedule, command, active from cron.job where jobname = 'community-heroes-refresh' $$,
  $$ values ('5 4 * * *'::text, 'select private.refresh_community_heroes()'::text, true) $$,
  'the stats are rebuilt once a day');
select is(
  (select offered_games from private.community_hero_rows
   where mode = 'solo' and period = '30d' and hero = 'BG22_HERO_000'),
  60 + 49, 'two skins of one hero in one choice count once');

-- ---------------------------------------------------------------- reads, as anonymous
set local role anon;
select set_config('request.jwt.claims', '{"role": "anon"}', true);
select results_eq(
  $$ select hero, games, players, avg_place, top_half_rate, win_rate, offered_games, offered_players, pick_rate
     from public.community_heroes() where mode = 'solo' and period = '30d' $$,
  $$ values
     ('BG23_HERO_303'::text, 80, 5, 6.8::numeric, 25, 0, 60, 5, 0),
     ('BG20_HERO_202', 60, 5, 3.9, 60, 20, 100, 5, 55),
     ('BG27_HERO_707', 50, 5, 2.0, 100, 0, null::int, null::int, null::int) $$,
  'Solo, 30 days: heroes with 50 games from 5 players, most games first; counts floored, values rounded');
select is_empty(
  $$ select 1 from public.community_heroes()
     where hero in ('BG22_HERO_000', 'BG24_HERO_404', 'BG28_HERO_808') $$,
  '49 games, one player over half, or two players over three quarters are not enough');
select results_eq(
  $$ select hero, games, players, avg_place from public.community_heroes() where mode = 'solo' and period = 'all' $$,
  $$ values ('BG23_HERO_303'::text, 80, 5, 6.8::numeric), ('BG20_HERO_202', 60, 5, 3.9),
            ('BG26_HERO_606', 50, 5, 5.0) $$,
  'all time: the hero of 40 days ago shows (exactly 50 games, 5 players, one with half); H7 does not, '
  'since all time minus 30 days would be its one older game');
select results_eq(
  $$ select period, hero, games, players, avg_place, top_half_rate, win_rate from public.community_heroes()
     where mode = 'duos' $$,
  $$ values ('30d'::text, 'BG25_HERO_505'::text, 50, 5, 1.0::numeric, 100, 100),
            ('all', 'BG25_HERO_505', 50, 5, 1.0, 100, 100) $$,
  'Duos apart, top 2 as the top half');
select throws_ok($$ select * from private.community_hero_rows $$, '42501', null,
  'an anonymous visitor cannot read the snapshot');
select throws_ok($$ select private.refresh_community_heroes() $$, '42501', null,
  'nor rebuild it');
reset role;

-- ---------------------------------------------------------------- reads, signed in
set local role authenticated;
select pg_temp.as_user(1);
select is((select count(*)::int from public.community_heroes()), 3 + 3 + 2,
  'a signed-in player reads the same rows');
select throws_ok($$ select * from private.community_state $$, '42501', null,
  'and cannot read the state table');
reset role;

-- ---------------------------------------------------------------- a failed rebuild
update private.community_state set refreshed_at = now() - interval '27 hours';
set local role anon;
select set_config('request.jwt.claims', '{"role": "anon"}', true);
select is_empty($$ select * from public.community_heroes() $$,
  'a snapshot older than 26 hours is not shown: a withdrawn consent never lingers');
select is((select available from public.community_heroes_state()), false, 'and the state says so');
reset role;
select private.refresh_community_heroes();

-- ---------------------------------------------------------------- opting out
set local role authenticated;
select pg_temp.as_user(10);
select is(public.set_community_consent(false), false, 'P10 turns the switch off');
reset role;
set local role anon;
select set_config('request.jwt.claims', '{"role": "anon"}', true);
select is((select count(*)::int from public.community_heroes() where hero = 'BG23_HERO_303'), 2,
  'until the next rebuild the numbers do not change');
reset role;
select private.refresh_community_heroes();
set local role anon;
select set_config('request.jwt.claims', '{"role": "anon"}', true);
select results_eq(
  $$ select hero, games from public.community_heroes() where mode = 'solo' and period = '30d' $$,
  $$ values ('BG20_HERO_202'::text, 60), ('BG27_HERO_707', 50) $$,
  'at the next rebuild P10''s games are out, and H3 (now 4 players) is no longer shown');
reset role;

-- ---------------------------------------------------------------- export and deletion
set local role authenticated;
select pg_temp.as_user(1);
select is(public.export_my_data() -> 'community_consent' -> 'hero_stats', 'true'::jsonb,
  'the export has the switch''s state');
select ok(public.export_my_data() -> 'community_consent' ->> 'updated_at' is not null
  and public.export_my_data() -> 'community_consent' ->> 'policy_version' = '2026-10-11',
  'and when and under which policy it was set');
reset role;
set local role authenticated;
select pg_temp.as_user(9);
select is(public.export_my_data() -> 'community_consent',
  '{"hero_stats": false, "updated_at": null, "policy_version": null}'::jsonb,
  'an account that never used the switch exports it as off');
reset role;
select is((public.delete_user_data(pg_temp.pid(1)) ->> 'profiles')::int, 1,
  'account deletion removes the profile');
select is((select count(*)::int from public.community_consent where user_id = pg_temp.pid(1)), 0,
  'and the choice with it');
select private.refresh_community_heroes();
select results_eq(
  $$ select hero, games, players from public.community_heroes() where mode = 'solo' and period = '30d' $$,
  $$ values ('BG20_HERO_202'::text, 50, 5) $$,
  'at the next rebuild the deleted account''s games are out of every number');

-- ---------------------------------------------------------------- the pick rate's own rule
-- 50 games of a new hero from P2..P6 (10 each), and 50 more games of P2
-- with another hero whose choices offered it: P2 holds 60 of its 100 offers,
-- over half, so the games pass and the pick rate does not.
select pg_temp.seed(pg_temp.pid(i), 401, 10, 'GT_BATTLEGROUNDS', 4, 'BG29_HERO_909', current_date - 2,
  array['BG29_HERO_909'])
from generate_series(2, 6) as i;
select pg_temp.seed(pg_temp.pid(2), 411, 50, 'GT_BATTLEGROUNDS', 4, 'BG20_HERO_202', current_date - 2,
  array['BG29_HERO_909']);
select private.refresh_community_heroes();
select results_eq(
  $$ select games, offered_games, pick_rate from public.community_heroes()
     where hero = 'BG29_HERO_909' and period = '30d' $$,
  $$ values (50, null::int, null::int) $$,
  'a pick rate needs its own offers to pass the rule: here one player has over half of them');

-- ---------------------------------------------------------------- no player in the snapshot or the reads
select is_empty(
  $$ select table_name, column_name from information_schema.columns
     where table_schema = 'private'
       and table_name in ('community_hero_rows', 'community_state')
       and (column_name ~ 'user|name|session|email' or data_type = 'uuid') $$,
  'the snapshot has no account id, name or game: sums and counts only');
select is(
  (select string_agg(p.parameter_name::text, ',' order by p.ordinal_position)
   from information_schema.parameters p
   join information_schema.routines r using (specific_schema, specific_name)
   where r.routine_schema = 'public' and r.routine_name = 'community_heroes' and p.parameter_mode = 'OUT'),
  'mode,period,hero,games,players,avg_place,top_half_rate,win_rate,offered_games,offered_players,pick_rate',
  'a row is a hero and its numbers, never a player');
select ok(
  not exists (select 1 from public.community_heroes() where period not in ('30d', 'all')),
  'the slice before the 30 days is only checked, never shown');
select ok(
  not exists (select 1 from public.community_heroes()
              where games % 10 <> 0 or players % 5 <> 0 or offered_games % 10 <> 0 or offered_players % 5 <> 0),
  'counts are floored to tens of games and fives of players');
select is_empty(
  $$ select table_name, privilege_type from information_schema.role_table_grants
     where table_schema = 'private'
       and table_name in ('community_hero_rows', 'community_state')
       and grantee in ('anon', 'authenticated', 'service_role', 'PUBLIC') $$,
  'no client role has any right on the snapshot');
select is_empty(
  $$ select privilege_type from information_schema.role_table_grants
     where table_schema = 'public' and table_name = 'community_consent'
       and grantee in ('anon', 'PUBLIC') $$,
  'anonymous visitors have no right on the choices');
select is(
  (select string_agg(privilege_type::text, ',') from information_schema.role_table_grants
   where table_schema = 'public' and table_name = 'community_consent' and grantee = 'authenticated'),
  'SELECT',
  'signed-in users may only read (their own row, by the policy)');
select ok(
  has_function_privilege('anon', 'public.community_heroes()', 'execute')
  and has_function_privilege('authenticated', 'public.community_heroes()', 'execute')
  and has_function_privilege('anon', 'public.community_heroes_state()', 'execute'),
  'everyone can read the community stats');
select ok(
  not has_function_privilege('anon', 'public.set_community_consent(boolean)', 'execute')
  and has_function_privilege('authenticated', 'public.set_community_consent(boolean)', 'execute'),
  'only signed-in users can call the switch');
select ok(
  not has_function_privilege('anon', 'private.refresh_community_heroes()', 'execute')
  and not has_function_privilege('authenticated', 'private.refresh_community_heroes()', 'execute')
  and not has_function_privilege('service_role', 'private.refresh_community_heroes()', 'execute'),
  'only the database itself rebuilds the stats');
select ok(
  (select bool_and(prosecdef and proconfig @> array['search_path=""']) from pg_proc
   where oid in ('public.community_heroes()'::regprocedure,
                 'public.community_heroes_state()'::regprocedure,
                 'public.set_community_consent(boolean)'::regprocedure)),
  'the functions run with a fixed, empty search path');
select ok(private.community_min_games() = 50 and private.community_min_players() = 5
  and private.community_max_age() = interval '26 hours',
  'the thresholds are 50 games from 5 players, and a snapshot is shown for 26 hours');
select ok(
  private.community_enough(50, 5, 25, 37) and not private.community_enough(49, 5, 10, 20)
  and not private.community_enough(50, 4, 20, 30) and not private.community_enough(51, 5, 26, 30)
  and private.community_enough(60, 5, 25, 45) and not private.community_enough(60, 5, 25, 46),
  'nobody over half, and the top two at most three quarters');

select * from finish();
rollback;

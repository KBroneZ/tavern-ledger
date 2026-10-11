-- Per-game summary and the website's views (T-502, D-071). Run: npx supabase test db
-- Users and games are made up; everything rolls back at the end.
begin;
create extension if not exists pgtap with schema extensions;
select plan(61);

insert into auth.users (id, email) values
  ('00000000-0000-4000-8000-00000000000a', 'a@example.test'),
  ('00000000-0000-4000-8000-00000000000b', 'b@example.test');

-- Player A: four Solo games and two Duos games, stored the way the upload
-- function stores them (upload_commit, service role; here postgres).
create temp table seed (
  idx int, game_type text, status text, hero text, place int, played date, stats jsonb
) on commit drop;
insert into seed values
  (1, 'GT_BATTLEGROUNDS', 'ok', 'BG20_HERO_202', 1, '2026-10-01',
   '{"rounds": 12, "combats_won": 8, "combats_lost": 3, "combats_tied": 1, "combats_unknown": 0,
     "combats_inferred": 0, "win_streak": 5, "win_streak_inferred": false,
     "damage_taken": 25, "damage_taken_combats": 12, "damage_taken_max": 9,
     "damage_dealt": 60, "damage_dealt_combats": 11, "damage_dealt_max": 15,
     "shop_turns": 12, "final_tier": 6, "gold_spent": 90, "gold_spent_turns": 12,
     "gold_beyond_income": 14, "gold_beyond_turns": 12, "sell_gold": 9, "sell_gold_turns": 12,
     "rolls": 30, "free_rolls": 4, "buys": 25, "spell_buys": 3, "sells": 9, "freezes": 2,
     "actions": 300, "apm": 21.5, "triples": 3, "final_board": ["BG_A", null, "BG_B"],
     "final_board_round": 12, "board_stats_max": 210, "golden_max": 2,
     "heroes_offered": ["BG20_HERO_202", "BG22_HERO_000"], "hero_rerolls": 1}'),
  (2, 'GT_BATTLEGROUNDS', 'ok', 'BG20_HERO_202_SKIN_A', 5, '2026-10-02',
   '{"rounds": 9, "combats_won": 3, "combats_lost": 4, "combats_tied": 0, "combats_unknown": 2,
     "combats_inferred": 1, "win_streak": 2, "win_streak_inferred": true,
     "damage_taken_max": 14, "damage_taken": 30, "damage_taken_combats": 8,
     "rolls": 40, "free_rolls": 0, "apm": 30.25, "triples": 1,
     "heroes_offered": ["BG20_HERO_202_SKIN_A"], "hero_rerolls": 0}'),
  (3, 'GT_BATTLEGROUNDS', 'ok', 'BG22_HERO_000', 3, '2026-10-03',
   '{"rounds": 10, "triples": 0}'),
  -- Unfinished: counted as a game, never for a place.
  (4, 'GT_BATTLEGROUNDS', 'incomplete', 'BG22_HERO_000', null, '2026-10-04', null),
  (5, 'GT_BATTLEGROUNDS_DUO', 'ok', 'BG24_HERO_100', 2, '2026-10-05',
   '{"teammate_hero": "BG22_HERO_000", "rounds": 11}'),
  (6, 'GT_BATTLEGROUNDS_DUO', 'ok', 'BG24_HERO_100', 3, '2026-10-06', null);

select is(
  (select count(*)::int from seed s, lateral (
     select public.upload_commit(
       '00000000-0000-4000-8000-00000000000a', 'Hearthstone_2026_10_09_18_30_00', s.idx,
       s.game_type, s.status, s.hero, s.place, 253216, s.played, '{}'::jsonb, 1791000000,
       repeat('a', 64), 2000, '0.1.0+r6', s.stats, case when s.stats is null then null else 1 end
     ) as r) c where c.r = 'created'),
  6, 'upload_commit stores every game, with its summary when there is one'
);
-- Player B: one private game.
select public.upload_commit(
  '00000000-0000-4000-8000-00000000000b', 'Hearthstone_2026_10_09_18_30_00', 1,
  'GT_BATTLEGROUNDS', 'ok', 'BG20_HERO_202', 2, 253216, '2026-10-01', '{}'::jsonb, 1791000000,
  repeat('b', 64), 2000, '0.1.0+r6', '{"rounds": 7, "triples": 4}'::jsonb, 1);

-- ---------------------------------------------------------------- storage of the summary
select is((select count(*)::int from public.game_stats), 5,
  'one summary row per game sent with a summary');
select is(
  (select final_board from public.game_stats where game_index = 1
     and user_id = '00000000-0000-4000-8000-00000000000a'),
  array['BG_A', null, 'BG_B']::text[], 'the final board keeps an unknown card as null');
select is(
  (select row(damage_dealt, gold_spent)::text from public.game_stats where game_index = 2
     and user_id = '00000000-0000-4000-8000-00000000000a'),
  '(,)', 'a value missing from the summary is unknown (null), never zero');
select throws_ok(
  $$ select public.upload_commit(
       '00000000-0000-4000-8000-00000000000a', 'Hearthstone_2026_10_09_18_30_00', 9,
       'GT_BATTLEGROUNDS', 'ok', 'BG20_HERO_202', 2, 253216, '2026-10-01', '{}', 1791000000,
       repeat('a', 64), 2000, null, '{"rounds": 7, "battletag": "x"}', 1) $$,
  '22023', null, 'a summary key that is not a column is refused');
select throws_ok(
  $$ select public.upload_commit(
       '00000000-0000-4000-8000-00000000000a', 'Hearthstone_2026_10_09_18_30_00', 9,
       'GT_BATTLEGROUNDS', 'ok', 'BG20_HERO_202', 2, 253216, '2026-10-01', '{}', 1791000000,
       repeat('a', 64), 2000, null, '{"user_id": "00000000-0000-4000-8000-00000000000b"}', 1) $$,
  '22023', null, 'the summary cannot name the row it belongs to');
select throws_ok(
  $$ select public.upload_commit(
       '00000000-0000-4000-8000-00000000000a', 'Hearthstone_2026_10_09_18_30_00', 9,
       'GT_BATTLEGROUNDS', 'ok', 'BG20_HERO_202', 2, 253216, '2026-10-01', '{}', 1791000000,
       repeat('a', 64), 2000, null, '{"rounds": 7}', null) $$,
  '22023', null, 'a summary needs its version');
select throws_ok(
  $$ select public.upload_commit(
       '00000000-0000-4000-8000-00000000000a', 'Hearthstone_2026_10_09_18_30_00', 9,
       'GT_BATTLEGROUNDS', 'ok', 'BG20_HERO_202', 2, 253216, '2026-10-01', '{}', 1791000000,
       repeat('a', 64), 2000, null, '{"rounds": 3, "combats_won": 1, "combats_lost": 1,
         "combats_tied": 0, "combats_unknown": 0}', 1) $$,
  '23514', null, 'the combats must add up to the rounds');
select throws_ok(
  $$ select public.upload_commit(
       '00000000-0000-4000-8000-00000000000a', 'Hearthstone_2026_10_09_18_30_00', 9,
       'GT_BATTLEGROUNDS', 'ok', 'BG20_HERO_202', 2, 253216, '2026-10-01', '{}', 1791000000,
       repeat('a', 64), 2000, null, '{"final_board": ["BG<script>"]}', 1) $$,
  '23514', null, 'the final board holds card ids only');
select throws_ok(
  $$ select public.upload_commit(
       '00000000-0000-4000-8000-00000000000a', 'Hearthstone_2026_10_09_18_30_00', 9,
       'GT_BATTLEGROUNDS', 'ok', 'BG20_HERO_202', 2, 253216, '2026-10-01', '{}', 1791000000,
       repeat('a', 64), 2000, null, '{"heroes_offered": ["Name#1234"]}', 1) $$,
  '23514', null, 'heroes offered are card ids only');
select throws_ok(
  $$ select public.upload_commit(
       '00000000-0000-4000-8000-00000000000a', 'Hearthstone_2026_10_09_18_30_00', 9,
       'GT_BATTLEGROUNDS', 'ok', 'BG20_HERO_202', 2, 253216, '2026-10-01', '{}', 1791000000,
       repeat('a', 64), 2000, null, '{"rolls": 2, "free_rolls": 3}', 1) $$,
  '23514', null, 'free rolls cannot pass the rolls');
select is((select count(*)::int from public.games where game_index = 9), 0,
  'a refused summary stores no game row either');
select throws_ok(
  $$ insert into public.game_stats (user_id, session, game_index, summary_version)
     values ('00000000-0000-4000-8000-00000000000a', 'Hearthstone_2026_10_09_18_30_00', 99, 1) $$,
  '23503', null, 'a summary cannot exist without its game');

-- A replace without a summary (an older upload function) drops the stale one.
select is(
  public.upload_commit(
    '00000000-0000-4000-8000-00000000000a', 'Hearthstone_2026_10_09_18_30_00', 3,
    'GT_BATTLEGROUNDS', 'ok', 'BG22_HERO_000', 3, 253216, '2026-10-03', '{}', 1791000001,
    repeat('c', 64), 2000, '0.1.0+r6'),
  'replaced', 'the call without a summary still works (the deployed function before T-502)');
select is((select count(*)::int from public.game_stats where game_index = 3
  and user_id = '00000000-0000-4000-8000-00000000000a'), 0,
  'a replaced game keeps no summary of its older copy');
select is(
  public.upload_commit(
    '00000000-0000-4000-8000-00000000000a', 'Hearthstone_2026_10_09_18_30_00', 3,
    'GT_BATTLEGROUNDS', 'ok', 'BG22_HERO_000', 3, 253216, '2026-10-03', '{}', 1791000000,
    repeat('d', 64), 2000, '0.1.0+r6', '{"rounds": 1}', 1),
  'older', 'an older copy changes neither the game nor its summary');
select is((select count(*)::int from public.game_stats where game_index = 3
  and user_id = '00000000-0000-4000-8000-00000000000a'), 0,
  'an older copy writes no summary');

-- ---------------------------------------------------------------- backfill (the sweep)
select results_eq(
  $$ select session, game_index, content_sha256 from public.game_stats_backfill_list(1, 100)
     where user_id = '00000000-0000-4000-8000-00000000000a' order by game_index $$,
  $$ values ('Hearthstone_2026_10_09_18_30_00'::text, 3, repeat('c', 64)),
            ('Hearthstone_2026_10_09_18_30_00'::text, 4, repeat('a', 64)),
            ('Hearthstone_2026_10_09_18_30_00'::text, 6, repeat('a', 64)) $$,
  'the backfill lists the games with no summary'
);
select is((select count(*)::int from public.game_stats_backfill_list(2, 100)), 7,
  'a newer summary version lists every game again');
select is((select count(*)::int from public.game_stats_backfill_list(1, 1)), 1,
  'the backfill list is capped');
select is(
  public.game_stats_backfill_put('00000000-0000-4000-8000-00000000000a',
    'Hearthstone_2026_10_09_18_30_00', 3, repeat('a', 64), 1, '{"rounds": 10}'),
  'changed', 'a summary of another copy of the game is not stored');
select is(
  public.game_stats_backfill_put('00000000-0000-4000-8000-00000000000a',
    'Hearthstone_2026_10_09_18_30_00', 3, repeat('c', 64), 1, '{"rounds": 10, "triples": 0}'),
  'stored', 'the backfill stores the summary of the stored copy');
select is(
  public.game_stats_backfill_put('00000000-0000-4000-8000-00000000000a',
    'Hearthstone_2026_10_09_18_30_00', 3, repeat('c', 64), 0, '{"rounds": 2}'),
  'newer', 'a summary never replaces one of a newer version');
select is(
  public.game_stats_backfill_put('00000000-0000-4000-8000-00000000000a',
    'Hearthstone_2026_10_09_18_30_00', 77, repeat('c', 64), 1, '{}'),
  'missing', 'a game that is gone gets no summary');
select is((select rounds from public.game_stats where game_index = 3
  and user_id = '00000000-0000-4000-8000-00000000000a'), 10::smallint,
  'the stored summary is the stored copy''s');

-- ---------------------------------------------------------------- shape of the API
select is_empty(
  $$ select c.relname from pg_class c join pg_namespace n on n.oid = c.relnamespace
     where n.nspname = 'public' and c.relkind = 'v'
       and not coalesce(c.reloptions @> array['security_invoker=true'], false) $$,
  'every public view runs with the caller''s rights, so row-level security applies'
);
select is_empty(
  $$ select table_name, column_name from information_schema.columns
     where table_schema = 'public'
       and column_name ~* '(mmr|rating|battle_?tag|player_?name|account_?id|rank|email)' $$,
  'no name, BattleTag, rating, MMR, rank or email column in the tables or views'
);
select is_empty(
  $$ select table_name, privilege_type from information_schema.role_table_grants
     where table_schema = 'public' and grantee in ('anon', 'authenticated')
       and table_name in ('game_stats', 'game_summaries', 'user_overview', 'user_heroes',
                          'user_records')
       and privilege_type <> 'SELECT' $$,
  'clients can only read the summary and the views'
);
select ok(
  not has_function_privilege('anon', 'public.game_stats_backfill_list(integer, integer)', 'execute')
  and not has_function_privilege('authenticated',
    'public.game_stats_backfill_put(uuid, text, integer, text, integer, jsonb)', 'execute')
  and not has_function_privilege('authenticated',
    'public.upload_commit(uuid, text, integer, text, text, text, integer, integer, date, jsonb, bigint, text, integer, text, jsonb, integer)',
    'execute'),
  'clients cannot call the write or backfill functions'
);

-- ---------------------------------------------------------------- anon, private profiles
set local role anon;
select set_config('request.jwt.claims', '{"role": "anon"}', true);
select is_empty($$ select * from public.game_stats $$, 'anon sees no summary of a private profile');
select is_empty($$ select * from public.game_summaries $$, 'anon sees no game of a private profile');
select is_empty($$ select * from public.user_overview $$, 'anon sees no overview of a private profile');
select is_empty($$ select * from public.user_heroes $$, 'anon sees no heroes of a private profile');
select is_empty($$ select * from public.user_records $$, 'anon sees no records of a private profile');
select throws_ok($$ select * from public.game_stats_backfill_list(1, 10) $$, '42501', null,
  'anon cannot list the backfill');

-- ---------------------------------------------------------------- user B
reset role;
set local role authenticated;
select set_config('request.jwt.claims',
  '{"sub": "00000000-0000-4000-8000-00000000000b", "role": "authenticated"}', true);
select results_eq($$ select distinct user_id from public.game_summaries $$,
  $$ values ('00000000-0000-4000-8000-00000000000b'::uuid) $$,
  'another user sees only their own games');
select results_eq($$ select user_id, games from public.user_overview $$,
  $$ values ('00000000-0000-4000-8000-00000000000b'::uuid, 1::bigint) $$,
  'another user''s overview holds only their own games');
select is_empty(
  $$ select * from public.user_records where user_id = '00000000-0000-4000-8000-00000000000a' $$,
  'another user sees no record of a private profile');
select is_empty(
  $$ select * from public.user_heroes where user_id = '00000000-0000-4000-8000-00000000000a' $$,
  'another user sees no hero of a private profile');
select throws_ok(
  $$ insert into public.game_stats (user_id, session, game_index, summary_version)
     values ('00000000-0000-4000-8000-00000000000b', 'Hearthstone_2026_10_09_18_30_00', 1, 1) $$,
  '42501', null, 'a client cannot write a summary');
select throws_ok($$ update public.game_stats set triples = 9 $$, '42501', null,
  'a client cannot change a summary');

-- ---------------------------------------------------------------- user A, their own data
reset role;
set local role authenticated;
select set_config('request.jwt.claims',
  '{"sub": "00000000-0000-4000-8000-00000000000a", "role": "authenticated"}', true);

select results_eq(
  $$ select mode, games, placed_games, avg_place, top_half, top_half_pct, wins, win_pct,
            incomplete_games
     from public.user_overview order by mode $$,
  $$ values ('duos'::text, 2::bigint, 2::bigint, 2.50::numeric, 1::bigint, 50.0::numeric,
             0::bigint, 0.0::numeric, 0::bigint),
            ('solo'::text, 4::bigint, 3::bigint, 3.00::numeric, 2::bigint, 66.7::numeric,
             1::bigint, 33.3::numeric, 1::bigint) $$,
  'overview per mode: places only from finished games; top 4 in Solo, top 2 in Duos'
);
select results_eq(
  $$ select triples, triples_games, rolls, shop_games, avg_apm, apm_games, gold_spent,
            gold_spent_games
     from public.user_overview where mode = 'solo' $$,
  $$ values (4::bigint, 3::bigint, 70::bigint, 2::bigint, 25.88::numeric, 2::bigint,
             90::bigint, 1::bigint) $$,
  'totals say how many games back them; a game without the number is left out, not zero'
);
select results_eq(
  $$ select hero, games, placed_games, avg_place, top_half, best_place, worst_place,
            offered_games, picked_games
     from public.user_heroes where mode = 'solo' order by hero $$,
  $$ values ('BG20_HERO_202'::text, 2::bigint, 2::bigint, 3.00::numeric, 1::bigint, 1::smallint,
             5::smallint, 2::bigint, 2::bigint),
            ('BG22_HERO_000'::text, 2::bigint, 1::bigint, 3.00::numeric, 1::bigint, 3::smallint,
             3::smallint, 1::bigint, 0::bigint) $$,
  'heroes: skins grouped under the base hero, picks counted where the hero choice is known'
);
select results_eq(
  $$ select hero, hero_card_id from public.game_summaries where game_index = 2 $$,
  $$ values ('BG20_HERO_202'::text, 'BG20_HERO_202_SKIN_A'::text) $$,
  'a game keeps its own hero card and the base hero it counts under'
);
select results_eq(
  $$ select value, game_index, games, inferred from public.user_records
     where mode = 'solo' and record = 'win_streak' $$,
  $$ values (5::numeric, 1, 2::bigint, false) $$,
  'records: the best win streak, its game and how many games back it'
);
select results_eq(
  $$ select value, game_index, inferred from public.user_records
     where mode = 'solo' and record = 'damage_taken_max' $$,
  $$ values (14::numeric, 2, true) $$,
  'records from health are marked inferred'
);
select results_eq(
  $$ select value, game_index, games from public.user_records
     where mode = 'solo' and record = 'top_half_streak' $$,
  $$ values (1::numeric, 1, 3::bigint) $$,
  'the longest run of top finishes in a row'
);
select results_eq(
  $$ select value, game_index from public.user_records
     where mode = 'solo' and record = 'apm' $$,
  $$ values (30.25::numeric, 2) $$,
  'the highest APM keeps its decimals'
);
select is_empty(
  $$ select * from public.user_records where mode = 'duos' and record = 'triples' $$,
  'a record no game backs is absent, not zero'
);
select results_eq(
  $$ select count(*)::int from public.game_summaries
     where mode = 'solo' and hero = 'BG22_HERO_000' and played_on >= '2026-10-03' $$,
  $$ values (2) $$,
  'the games list filters by mode, hero and date'
);
select results_eq(
  $$ select game_index from public.game_summaries
     order by played_on desc, session desc, game_index desc limit 2 offset 1 $$,
  $$ values (5), (4) $$,
  'the games list pages in a stable order'
);
select results_eq(
  $$ select counted_place, top_half, won from public.game_summaries where game_index = 4 $$,
  $$ values (null::smallint, null::boolean, null::boolean) $$,
  'an unfinished game has no counted place'
);

-- ---------------------------------------------------------------- export
select is(
  (select jsonb_array_length(public.export_my_data() -> 'game_stats')), 4,
  'export has every summary of the user and no one else''s'
);
select is(
  (select public.export_my_data() -> 'game_stats' -> 0 ?& array(
     select column_name::text from information_schema.columns
     where table_schema = 'public' and table_name = 'game_stats')),
  true, 'exported summaries carry every column'
);

-- ---------------------------------------------------------------- public profile
update public.profiles set display_name = 'Tester', is_public = true;
reset role;
set local role anon;
select set_config('request.jwt.claims', '{"role": "anon"}', true);
select is((select count(*)::int from public.game_summaries), 6,
  'a public profile''s games are readable by anyone');
select is((select count(*)::int from public.user_overview), 2,
  'a public profile''s overview is readable by anyone');
select is((select count(*)::int from public.game_stats
  where user_id = '00000000-0000-4000-8000-00000000000b'), 0,
  'a private profile stays private next to a public one');

-- ---------------------------------------------------------------- deletion
reset role;
select is(
  (public.delete_user_data('00000000-0000-4000-8000-00000000000a') ->> 'game_stats')::int, 4,
  'account deletion removes the user''s summaries and says how many'
);
select is((select count(*)::int from public.game_stats), 1,
  'only the other user''s summary is left');
delete from public.games where user_id = '00000000-0000-4000-8000-00000000000b';
select is((select count(*)::int from public.game_stats), 0,
  'a summary goes with its game');

select * from finish();
rollback;

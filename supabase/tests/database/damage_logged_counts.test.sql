-- The per-game damage says how many of its combats are the game's own hit
-- (T-559), so the dashboard marks it inferred only when it is.
-- Run: npx supabase test db
-- Users and games are made up; everything rolls back at the end.
begin;
create extension if not exists pgtap with schema extensions;
select plan(17);

insert into auth.users (id, email) values
  ('00000000-0000-4000-8000-00000000000b', 'b@example.test'),
  ('00000000-0000-4000-8000-00000000000c', 'c@example.test');

create temp table seed (idx int, game_type text, played date, stats jsonb) on commit drop;
insert into seed values
  -- Solo: every combat behind both totals is the game's hit.
  (1, 'GT_BATTLEGROUNDS', '2026-10-01',
   '{"rounds": 8, "damage_dealt": 30, "damage_dealt_combats": 8, "damage_dealt_logged": 8,
     "damage_taken": 20, "damage_taken_combats": 8, "damage_taken_logged": 8}'),
  -- Solo: the dealt total is part worked out from health; the taken one all of it.
  (2, 'GT_BATTLEGROUNDS', '2026-10-02',
   '{"rounds": 8, "damage_dealt": 28, "damage_dealt_combats": 8, "damage_dealt_logged": 5,
     "damage_taken": 22, "damage_taken_combats": 8, "damage_taken_logged": 0}'),
  -- Duos: a summary older than the counts (null), and one with no damage at all.
  (3, 'GT_BATTLEGROUNDS_DUO', '2026-10-03',
   '{"rounds": 8, "damage_dealt": 40, "damage_dealt_combats": 8,
     "damage_taken": 10, "damage_taken_combats": 8}'),
  (4, 'GT_BATTLEGROUNDS_DUO', '2026-10-04', '{"rounds": 1}');

select is(
  (select count(*)::int from seed s, lateral (
     select public.upload_commit(
       '00000000-0000-4000-8000-00000000000b', 'Hearthstone_2026_10_09_18_30_00', s.idx,
       s.game_type, 'ok', 'BG20_HERO_202', 2, 253216, s.played, '{}'::jsonb, 1791000000,
       repeat('b', 64), 2000, '0.1.0+r6', s.stats, 4
     ) as r) c where c.r = 'created'),
  4, 'games with the counts in their summary are stored'
);
select is(
  (select count(*)::int from public.game_stats
   where user_id = '00000000-0000-4000-8000-00000000000b' and summary_version = 4),
  4, 'and so are their summaries, none refused'
);

-- Another player, private, with a summary of their own.
select is(
  public.upload_commit(
    '00000000-0000-4000-8000-00000000000c', 'Hearthstone_2026_10_09_18_30_00', 1,
    'GT_BATTLEGROUNDS', 'ok', 'BG20_HERO_202', 3, 253216, '2026-10-05', '{}'::jsonb, 1791000000,
    repeat('c', 64), 2000, '0.1.0+r6',
    '{"rounds": 5, "damage_dealt": 9, "damage_dealt_combats": 5, "damage_dealt_logged": 2}', 4),
  'created', 'a private player''s game is stored'
);

-- A summary of version 1 took every combat from health: the migration's backfill makes its counts 0.
update public.game_stats set summary_version = 1, damage_dealt_logged = null, damage_taken_logged = null
where user_id = '00000000-0000-4000-8000-00000000000c';
update public.game_stats
set damage_taken_logged = case when damage_taken_combats is not null then 0 end,
    damage_dealt_logged = case when damage_dealt_combats is not null then 0 end
where summary_version = 1;
select results_eq(
  $$ select damage_dealt_logged, damage_taken_logged from public.game_stats
     where user_id = '00000000-0000-4000-8000-00000000000c' $$,
  $$ values (0::smallint, null::smallint) $$,
  'the version-1 backfill sets 0 where a total is known and leaves an unknown one null'
);

select has_column('public', 'game_stats', 'damage_taken_logged', 'the summary counts the taken combats from the hit');
select has_column('public', 'game_stats', 'damage_dealt_logged', 'the summary counts the dealt combats from the hit');

select throws_ok(
  $$ select public.game_stats_backfill_put('00000000-0000-4000-8000-00000000000b',
       'Hearthstone_2026_10_09_18_30_00', 1, repeat('b', 64), 4,
       '{"rounds": 7, "damage_dealt": 9, "damage_dealt_combats": 3, "damage_dealt_logged": 4}') $$,
  '23514', null, 'more combats from the hit than combats is refused'
);
select throws_ok(
  $$ select public.game_stats_backfill_put('00000000-0000-4000-8000-00000000000b',
       'Hearthstone_2026_10_09_18_30_00', 1, repeat('b', 64), 4,
       '{"rounds": 7, "damage_taken_logged": 2}') $$,
  '23514', null, 'a count with no combats behind it is refused'
);
select throws_ok(
  $$ select public.game_stats_backfill_put('00000000-0000-4000-8000-00000000000b',
       'Hearthstone_2026_10_09_18_30_00', 1, repeat('b', 64), 4,
       '{"rounds": 7, "damage_taken": 5, "damage_taken_combats": 2, "damage_taken_logged": -1}') $$,
  '23514', null, 'a negative count is refused'
);

reset role;
set local role authenticated;
select set_config('request.jwt.claims',
  '{"sub": "00000000-0000-4000-8000-00000000000b", "role": "authenticated"}', true);

select results_eq(
  $$ select damage_dealt_inferred_games, damage_taken_inferred_games from public.user_overview
     where user_id = '00000000-0000-4000-8000-00000000000b' and mode = 'solo' $$,
  $$ values (1::bigint, 1::bigint) $$,
  'Solo: a total part or wholly worked out counts as inferred; one all from the hit does not'
);
select results_eq(
  $$ select damage_dealt_inferred_games, damage_taken_inferred_games from public.user_overview
     where user_id = '00000000-0000-4000-8000-00000000000b' and mode = 'duos' $$,
  $$ values (1::bigint, 1::bigint) $$,
  'Duos: a summary with no counts stays inferred; a game with no damage is not counted'
);
select results_eq(
  $$ select damage_dealt_games, damage_taken_games from public.user_overview where user_id = '00000000-0000-4000-8000-00000000000b' and mode = 'duos' $$,
  $$ values (1::bigint, 1::bigint) $$,
  'the games behind the damage totals are unchanged'
);
select results_eq(
  $$ select damage_dealt_logged, damage_taken_logged from public.game_summaries
     where user_id = '00000000-0000-4000-8000-00000000000b' and game_index = 2 $$,
  $$ values (5::smallint, 0::smallint) $$,
  'the games list carries both counts'
);
select results_eq(
  $$ select damage_dealt_logged, damage_taken_logged from public.game_summaries
     where user_id = '00000000-0000-4000-8000-00000000000b' and game_index = 3 $$,
  $$ values (null::smallint, null::smallint) $$,
  'a missing count is unknown (null), not 0'
);
select is_empty(
  $$ select 1 from public.user_overview where user_id = '00000000-0000-4000-8000-00000000000c' $$,
  'a player sees no other private player''s overview'
);
select is_empty(
  $$ select 1 from public.game_summaries where user_id = '00000000-0000-4000-8000-00000000000c' $$,
  'nor their games list with its counts'
);

reset role;
set local role anon;
select set_config('request.jwt.claims', '{"role": "anon"}', true);
select is_empty(
  $$ select 1 from public.user_overview
     where user_id in ('00000000-0000-4000-8000-00000000000b', '00000000-0000-4000-8000-00000000000c') $$,
  'a visitor sees no private overview'
);

select * from finish();
rollback;

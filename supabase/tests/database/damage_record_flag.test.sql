-- The "most damage in one combat" records say whether the value is the game's
-- own hit (D-116 gap, D-125). Run: npx supabase test db
-- Users and games are made up; everything rolls back at the end.
begin;
create extension if not exists pgtap with schema extensions;
select plan(13);

insert into auth.users (id, email) values
  ('00000000-0000-4000-8000-00000000000a', 'a@example.test');

create temp table seed (idx int, game_type text, played date, stats jsonb) on commit drop;
insert into seed values
  -- Solo: the same max damage dealt twice, health-based first, from the hit later.
  (1, 'GT_BATTLEGROUNDS', '2026-10-01',
   '{"rounds": 8, "damage_dealt": 30, "damage_dealt_combats": 8, "damage_dealt_max": 15,
     "damage_dealt_max_inferred": true, "damage_taken": 20, "damage_taken_combats": 8,
     "damage_taken_max": 9}'),
  (2, 'GT_BATTLEGROUNDS', '2026-10-02',
   '{"rounds": 8, "damage_dealt": 28, "damage_dealt_combats": 8, "damage_dealt_max": 15,
     "damage_dealt_max_inferred": false, "damage_taken": 22, "damage_taken_combats": 8,
     "damage_taken_max": 12, "damage_taken_max_inferred": true}'),
  -- Duos: a max from the hit, and one from a summary older than the flag.
  (3, 'GT_BATTLEGROUNDS_DUO', '2026-10-03',
   '{"rounds": 8, "damage_dealt": 40, "damage_dealt_combats": 8, "damage_dealt_max": 20,
     "damage_taken": 10, "damage_taken_combats": 8, "damage_taken_max": 7,
     "damage_taken_max_inferred": false}'),
  -- A recorded tie: a max of 0 from the log.
  (4, 'GT_BATTLEGROUNDS_DUO', '2026-10-04',
   '{"rounds": 1, "damage_dealt": 0, "damage_dealt_combats": 1, "damage_dealt_max": 0,
     "damage_dealt_max_inferred": false}');

select is(
  (select count(*)::int from seed s, lateral (
     select public.upload_commit(
       '00000000-0000-4000-8000-00000000000a', 'Hearthstone_2026_10_09_18_30_00', s.idx,
       s.game_type, 'ok', 'BG20_HERO_202', 2, 253216, s.played, '{}'::jsonb, 1791000000,
       repeat('a', 64), 2000, '0.1.0+r6', s.stats, 3
     ) as r) c where c.r = 'created'),
  4, 'games with the flags in their summary are stored'
);

select has_column('public', 'game_stats', 'damage_dealt_max_inferred', 'the summary has the flag for the dealt max');
select has_column('public', 'game_stats', 'damage_taken_max_inferred', 'the summary has the flag for the taken max');

select throws_ok(
  $$ select public.game_stats_backfill_put('00000000-0000-4000-8000-00000000000a',
       'Hearthstone_2026_10_09_18_30_00', 1, repeat('a', 64), 3,
       '{"rounds": 7, "damage_taken_max_inferred": true}') $$,
  '23514', null, 'a flag with no max is refused'
);
select throws_ok(
  $$ select public.game_stats_backfill_put('00000000-0000-4000-8000-00000000000a',
       'Hearthstone_2026_10_09_18_30_00', 1, repeat('a', 64), 3,
       '{"rounds": 7, "damage_dealt_max": 5, "damage_dealt_max_inferred": "maybe"}') $$,
  '22P02', null, 'a flag that is not a boolean is refused'
);

reset role;
set local role authenticated;
select set_config('request.jwt.claims',
  '{"sub": "00000000-0000-4000-8000-00000000000a", "role": "authenticated"}', true);

select results_eq(
  $$ select value, game_index, inferred from public.user_records
     where mode = 'solo' and record = 'damage_dealt_max' $$,
  $$ values (15::numeric, 2, false) $$,
  'a record from the hit is not marked inferred, and at equal value it beats an inferred one'
);
select results_eq(
  $$ select value, game_index, inferred from public.user_records
     where mode = 'solo' and record = 'damage_taken_max' $$,
  $$ values (12::numeric, 2, true) $$,
  'a record the summary says is worked out from health stays inferred'
);
select results_eq(
  $$ select value, game_index, inferred from public.user_records
     where mode = 'duos' and record = 'damage_dealt_max' $$,
  $$ values (20::numeric, 3, true) $$,
  'a summary with no flag (older than the flag) stays inferred'
);
select results_eq(
  $$ select value, game_index, inferred from public.user_records
     where mode = 'duos' and record = 'damage_taken_max' $$,
  $$ values (7::numeric, 3, false) $$,
  'the taken record has its own flag'
);
select results_eq(
  $$ select inferred from public.user_records
     where mode = 'duos' and record = 'win_streak' $$,
  $$ select false where false $$,
  'records with no game behind them are absent'
);
select results_eq(
  $$ select damage_dealt_max_inferred, damage_taken_max_inferred from public.game_summaries
     where game_index = 2 $$,
  $$ values (false, true) $$,
  'the games list carries both flags'
);
select results_eq(
  $$ select damage_dealt_max_inferred, damage_taken_max_inferred from public.game_summaries
     where game_index = 1 $$,
  $$ values (true, null::boolean) $$,
  'a missing flag is unknown (null), not false'
);
select is(
  (select inferred from public.user_records where mode = 'duos' and record = 'damage_dealt_max'
     and value = 0),
  null, 'the best value wins, not the log: a zero from a tie does not replace 20'
);

select * from finish();
rollback;

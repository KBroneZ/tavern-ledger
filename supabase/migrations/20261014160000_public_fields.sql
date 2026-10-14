-- Fewer fields on public profiles (T-572, data inventory section 11).
--
-- A public profile showed more than the pages need: the exact start time of
-- each game (updated_at, saved_at, created_at), the file hash, the size, the
-- mark that a game's file has expired and the stats' computed_at. Clients
-- (anon and authenticated alike) can now read only the columns the website
-- uses from profiles, games and game_stats; the owner gets the rest through
-- the data export, and the games view still tells the owner (only) when a
-- game's file expired. Nothing public carries a time finer than the day a
-- game was played (played_on).
--
-- public.game_summaries now runs with the view owner's rights (the clients no
-- longer can read the hidden columns) and applies the same row rule as the
-- tables' policies: the caller's own games, or the games of a public profile.
-- The views built on it stay security_invoker.

revoke select on public.profiles from anon, authenticated;
grant select (user_id, display_name, is_public) on public.profiles to anon, authenticated;

revoke select on public.games from anon, authenticated;
grant select (
  user_id, session, game_index, game_type, status, hero_card_id, final_place,
  build, played_on, tribes_offered, storage_path, parser_version
) on public.games to anon, authenticated;

revoke select on public.game_stats from anon, authenticated;
grant select (
  user_id, session, game_index, summary_version, teammate_hero, rounds, combats_won,
  combats_lost, combats_tied, combats_unknown, combats_inferred, win_streak,
  win_streak_inferred, damage_taken, damage_taken_combats, damage_taken_max, damage_dealt,
  damage_dealt_combats, damage_dealt_max, shop_turns, final_tier, gold_spent,
  gold_spent_turns, gold_beyond_income, gold_beyond_turns, sell_gold, sell_gold_turns,
  rolls, free_rolls, buys, spell_buys, sells, freezes, actions, apm, triples, final_board,
  final_board_round, board_stats_max, golden_max, heroes_offered, hero_rerolls,
  damage_taken_max_inferred, damage_dealt_max_inferred, damage_taken_logged,
  damage_dealt_logged
) on public.game_stats to anon, authenticated;

-- As in 20261013120000_damage_logged_counts.sql: owner's rights, the row rule
-- in the view, and the expiry mark for the game's owner only.
create or replace view public.game_summaries with (security_invoker = false) as
select
  g.user_id,
  g.session,
  g.game_index,
  case g.game_type when 'GT_BATTLEGROUNDS' then 'solo' else 'duos' end as mode,
  g.game_type,
  g.status,
  g.played_on,
  g.hero_card_id,
  coalesce(substring(g.hero_card_id from '^(.+)_SKIN_[A-Za-z0-9]+$'), g.hero_card_id) as hero,
  g.final_place,
  p.counted_place,
  p.counted_place <= case g.game_type when 'GT_BATTLEGROUNDS' then 4 else 2 end as top_half,
  p.counted_place = 1 as won,
  g.build,
  g.parser_version,
  g.tribes_offered,
  g.storage_path,
  s.summary_version,
  s.teammate_hero,
  s.rounds,
  s.combats_won,
  s.combats_lost,
  s.combats_tied,
  s.combats_unknown,
  s.combats_inferred,
  s.win_streak,
  s.win_streak_inferred,
  s.damage_taken,
  s.damage_taken_combats,
  s.damage_taken_max,
  s.damage_dealt,
  s.damage_dealt_combats,
  s.damage_dealt_max,
  s.shop_turns,
  s.final_tier,
  s.gold_spent,
  s.gold_spent_turns,
  s.gold_beyond_income,
  s.gold_beyond_turns,
  s.sell_gold,
  s.sell_gold_turns,
  s.rolls,
  s.free_rolls,
  s.buys,
  s.spell_buys,
  s.sells,
  s.freezes,
  s.actions,
  s.apm,
  s.triples,
  s.final_board,
  s.final_board_round,
  s.board_stats_max,
  s.golden_max,
  s.heroes_offered,
  s.hero_rerolls,
  case when g.user_id = (select auth.uid()) then g.file_expired_at end as file_expired_at,
  s.damage_taken_max_inferred,
  s.damage_dealt_max_inferred,
  s.damage_taken_logged,
  s.damage_dealt_logged
from public.games g
cross join lateral (
  select case when g.status = 'ok' then g.final_place end as counted_place
) p
left join public.game_stats s
  on s.user_id = g.user_id and s.session = g.session and s.game_index = g.game_index
where g.user_id = (select auth.uid())
  or exists (
    select 1 from public.profiles pr
    where pr.user_id = g.user_id and pr.is_public
  );

comment on view public.game_summaries is
  'One row per game with its summary (T-502). Runs with the owner''s rights and shows the caller''s own games, or those of a public profile. file_expired_at is the owner''s only. Null is unknown.';

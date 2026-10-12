-- The per-game damage on the dashboard says whether it is the game's own hit
-- (T-559; D-116 and D-125 did the same for each combat and for the records).
--
-- * public.game_stats gets damage_taken_logged and damage_dealt_logged: of the
--   combats behind damage_taken / damage_dealt (damage_*_combats), how many
--   are the game's own hit; the rest were worked out from health. null when
--   the total is unknown or the summary is older than version 4. Written with
--   the rest of the summary (SUMMARY_VERSION 4), so the daily sweep fills
--   them for the games whose file is still stored.
-- * Existing rows: a summary of version 1 took every combat from health, so
--   its counts are 0; later rows stay null until the sweep summarizes them
--   again. Games whose file has expired keep null, read as inferred.
-- * public.game_summaries shows both counts at the end, and
--   public.user_overview counts, per mode, the games whose damage total is
--   not all the game's hit (damage_dealt_inferred_games,
--   damage_taken_inferred_games); create or replace keeps the grants.

alter table public.game_stats
  add column damage_taken_logged smallint,
  add column damage_dealt_logged smallint,
  add constraint game_stats_damage_taken_logged
    check (damage_taken_logged is null
      or (damage_taken_combats is not null
        and damage_taken_logged between 0 and damage_taken_combats)),
  add constraint game_stats_damage_dealt_logged
    check (damage_dealt_logged is null
      or (damage_dealt_combats is not null
        and damage_dealt_logged between 0 and damage_dealt_combats));

comment on column public.game_stats.damage_taken_logged is
  'Of damage_taken_combats, how many are the game''s own hit (the rest worked out from health); null: unknown (no total, or a summary older than version 4).';
comment on column public.game_stats.damage_dealt_logged is
  'Of damage_dealt_combats, how many are the game''s own hit (the rest worked out from health); null: unknown (no total, or a summary older than version 4).';

update public.game_stats
set damage_taken_logged = case when damage_taken_combats is not null then 0 end,
    damage_dealt_logged = case when damage_dealt_combats is not null then 0 end
where summary_version = 1;

-- As in 20261012210000_damage_record_flag.sql, plus the two counts at the end.
create or replace view public.game_summaries with (security_invoker = true) as
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
  g.file_expired_at,
  s.damage_taken_max_inferred,
  s.damage_dealt_max_inferred,
  s.damage_taken_logged,
  s.damage_dealt_logged
from public.games g
cross join lateral (
  select case when g.status = 'ok' then g.final_place end as counted_place
) p
left join public.game_stats s
  on s.user_id = g.user_id and s.session = g.session and s.game_index = g.game_index;

-- As in 20261011120000_game_stats.sql, plus the games with inferred damage at the end.
create or replace view public.user_overview with (security_invoker = true) as
select
  user_id,
  mode,
  count(*) as games,
  count(*) filter (where status = 'incomplete') as incomplete_games,
  count(*) filter (where status = 'unsupported') as unsupported_games,
  count(counted_place) as placed_games,
  round(avg(counted_place), 2) as avg_place,
  count(*) filter (where top_half) as top_half,
  round(100.0 * count(*) filter (where top_half) / nullif(count(counted_place), 0), 1)
    as top_half_pct,
  count(*) filter (where won) as wins,
  round(100.0 * count(*) filter (where won) / nullif(count(counted_place), 0), 1) as win_pct,
  min(played_on) as first_played_on,
  max(played_on) as last_played_on,
  count(summary_version) as summarized_games,
  sum(combats_won) as combats_won,
  sum(combats_lost) as combats_lost,
  sum(combats_tied) as combats_tied,
  sum(combats_unknown) as combats_unknown,
  sum(combats_inferred) as combats_inferred,
  count(combats_won) as combat_games,
  sum(damage_dealt) as damage_dealt,
  count(damage_dealt) as damage_dealt_games,
  sum(damage_taken) as damage_taken,
  count(damage_taken) as damage_taken_games,
  sum(gold_spent) as gold_spent,
  count(gold_spent) as gold_spent_games,
  sum(gold_beyond_income) as gold_beyond_income,
  count(gold_beyond_income) as gold_beyond_games,
  sum(sell_gold) as sell_gold,
  count(sell_gold) as sell_gold_games,
  sum(rolls) as rolls,
  sum(free_rolls) as free_rolls,
  sum(buys) as buys,
  sum(sells) as sells,
  count(rolls) as shop_games,
  round(avg(apm), 2) as avg_apm,
  count(apm) as apm_games,
  sum(triples) as triples,
  count(triples) as triples_games,
  -- Games whose damage total is not all the game's own hit (or not known to
  -- be: a summary older than version 4), so the per-game damage is inferred.
  count(*) filter (where damage_dealt is not null
    and damage_dealt_logged is distinct from damage_dealt_combats) as damage_dealt_inferred_games,
  count(*) filter (where damage_taken is not null
    and damage_taken_logged is distinct from damage_taken_combats) as damage_taken_inferred_games
from public.game_summaries
group by user_id, mode;

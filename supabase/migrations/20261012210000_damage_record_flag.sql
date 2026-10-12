-- The "most damage in one combat" records say whether the value is the
-- game's own hit (D-116 left them marked inferred, D-125).
--
-- * public.game_stats gets damage_taken_max_inferred and damage_dealt_max_inferred:
--   true when no combat at the max value has the game's own hit (the max is
--   worked out from health), false when one has, null when the max is unknown
--   or the summary is older than the flag. They are written with the rest of
--   the summary (SUMMARY_VERSION 3), so the daily sweep fills them for the
--   games whose file is still stored.
-- * Existing rows: a summary of version 1 (before T-545) took every combat's
--   damage from health, so its flag is true; later rows stay null until the
--   sweep summarizes them again (the stored row does not say which combats
--   had the hit). Games whose file has expired keep null.
-- * public.game_summaries shows both flags at the end (create or replace
--   keeps the grants); public.user_records marks a damage record inferred
--   unless its flag is false, and at equal value a record from the game's hit
--   beats an inferred one.

alter table public.game_stats
  add column damage_taken_max_inferred boolean,
  add column damage_dealt_max_inferred boolean,
  add constraint game_stats_damage_taken_flag
    check (damage_taken_max_inferred is null or damage_taken_max is not null),
  add constraint game_stats_damage_dealt_flag
    check (damage_dealt_max_inferred is null or damage_dealt_max is not null);

comment on column public.game_stats.damage_taken_max_inferred is
  'true: no combat at damage_taken_max has the game''s own hit (worked out from health); false: one has; null: unknown (no max, or a summary older than version 3).';
comment on column public.game_stats.damage_dealt_max_inferred is
  'true: no combat at damage_dealt_max has the game''s own hit (worked out from health); false: one has; null: unknown (no max, or a summary older than version 3).';

update public.game_stats
set damage_taken_max_inferred = (damage_taken_max is not null) or null,
    damage_dealt_max_inferred = (damage_dealt_max is not null) or null
where summary_version = 1;

-- As in 20261012180000_file_retention.sql, plus the two flags at the end.
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
  s.damage_dealt_max_inferred
from public.games g
cross join lateral (
  select case when g.status = 'ok' then g.final_place end as counted_place
) p
left join public.game_stats s
  on s.user_id = g.user_id and s.session = g.session and s.game_index = g.game_index;

-- As in 20261011120000_game_stats.sql, with the damage records marked from the flag.
create or replace view public.user_records with (security_invoker = true) as
with streaks as (
  -- Runs of top finishes in a row over the finished games, in play order.
  select user_id, mode, session, game_index, played_on, top_half,
    count(*) over (partition by user_id, mode) as placed,
    row_number() over (partition by user_id, mode order by played_on, session, game_index)
      - row_number() over (
          partition by user_id, mode, top_half order by played_on, session, game_index
        ) as run
  from public.game_summaries
  where counted_place is not null
),
top_runs as (
  -- Each run with its last game.
  select distinct on (user_id, mode, run)
    user_id, mode, run, placed, session, game_index, played_on,
    count(*) over (partition by user_id, mode, run) as len
  from streaks
  where top_half
  order by user_id, mode, run, played_on desc, session desc, game_index desc
),
candidates as (
  select g.user_id, g.mode, r.record, r.value, g.session, g.game_index, g.played_on,
    r.inferred, r.tiebreak
  from public.game_summaries g
  cross join lateral (values
    -- At equal value, a record from the log beats an inferred one.
    ('win_streak', g.win_streak::numeric, coalesce(g.win_streak_inferred, false),
      coalesce(g.win_streak_inferred, false)::int),
    -- The max is from the game's own hit when the summary says so (false);
    -- true or unknown (a summary written before the flag) stays inferred.
    ('damage_dealt_max', g.damage_dealt_max::numeric,
      g.damage_dealt_max_inferred is distinct from false,
      (g.damage_dealt_max_inferred is distinct from false)::int),
    ('damage_taken_max', g.damage_taken_max::numeric,
      g.damage_taken_max_inferred is distinct from false,
      (g.damage_taken_max_inferred is distinct from false)::int),
    ('board_stats_max', g.board_stats_max::numeric, false, 0),
    ('golden_max', g.golden_max::numeric, false, 0),
    ('gold_spent', g.gold_spent::numeric, false, 0),
    ('gold_beyond_income', g.gold_beyond_income::numeric, false, 0),
    ('rolls', g.rolls::numeric, false, 0),
    ('triples', g.triples::numeric, false, 0),
    ('apm', g.apm, false, 0),
    ('rounds', g.rounds::numeric, false, 0)
  ) as r(record, value, inferred, tiebreak)
  where r.value is not null
),
counted as (
  select user_id, mode, record, value, session, game_index, played_on, inferred, tiebreak,
    count(*) over (partition by user_id, mode, record) as games
  from candidates
  union all
  select user_id, mode, 'top_half_streak', len::numeric, session, game_index, played_on,
    false, 0, placed
  from top_runs
)
select distinct on (user_id, mode, record)
  user_id, mode, record, value, session, game_index, played_on, games, inferred
from counted
order by user_id, mode, record, value desc, tiebreak, played_on, session, game_index;
